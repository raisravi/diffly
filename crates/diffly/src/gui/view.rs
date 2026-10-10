use std::path::Path;
use std::rc::Rc;

use anyhow::Context as _;
use diffly_core::{
    DiffBody, DiffMode, DiffOptions, DiffStats, SideBySideRow, SideLine, TextChangeKind, diff,
    load_source,
};
use gpui_kit::base::{TestSupportExt as _, Theme};
use gpui_kit::{
    AnyElement, App, Context, ElementId, Hsla, IntoElement, ParentElement, Render, Role,
    SharedString, StatefulInteractiveElement, Styled, Window, div, hsla, prelude::*, px,
    uniform_list,
};

/// Side-by-side view of a line diff between two files.
#[derive(Debug)]
pub(crate) struct DiffView {
    left: SharedString,
    right: SharedString,
    content: Content,
}

#[derive(Debug)]
enum Content {
    Rows {
        /// Shared with the virtualized list's row builder.
        rows: Rc<[SideBySideRow]>,
        stats: DiffStats,
    },
    /// Shown instead of the rows when the files can't be compared.
    Error(SharedString),
}

impl DiffView {
    /// Loads and diffs both files; failures become an on-screen error, never a panic.
    pub(crate) fn load(left: &Path, right: &Path) -> Self {
        let content = match compare(left, right) {
            Ok(content) => content,
            Err(err) => Content::Error(format!("{err:#}").into()),
        };
        Self {
            left: left.display().to_string().into(),
            right: right.display().to_string().into(),
            content,
        }
    }
}

/// The desktop app shows line diffs for now; picking kind and mode comes with #7.
fn compare(left: &Path, right: &Path) -> anyhow::Result<Content> {
    let left_text =
        load_source(left).with_context(|| format!("loading left input {}", left.display()))?;
    let right_text =
        load_source(right).with_context(|| format!("loading right input {}", right.display()))?;
    let result = diff(&left_text, &right_text, &DiffOptions::text(DiffMode::Line))
        .with_context(|| format!("comparing {} with {}", left.display(), right.display()))?;
    let rows = match result.body {
        DiffBody::Text(text) => text.side_by_side().into(),
        // Unreachable while the options above are fixed; choosing the kind comes with #7.
        DiffBody::Json(_) => anyhow::bail!("the desktop app only shows text diffs so far"),
    };
    Ok(Content::Rows {
        rows,
        stats: result.stats,
    })
}

const ROW_HEIGHT: f32 = 20.;
const NUMBER_WIDTH: f32 = 48.;

impl Render for DiffView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx).tokens;
        // Paths are often long; each column's header truncates on its own.
        let path_header = |path: &SharedString| {
            div()
                .flex_1()
                .min_w_0()
                .px_3()
                .truncate()
                .child(path.clone())
        };
        let header = div()
            .flex()
            .py_2()
            .border_b_1()
            .border_color(theme.colors.border)
            .child(path_header(&self.left))
            .child(path_header(&self.right));

        let (body, summary) = match &self.content {
            Content::Rows { rows, stats } => {
                let rows = Rc::clone(rows);
                let list = uniform_list("rows", rows.len(), move |range, _, cx| {
                    range.map(|ix| render_row(ix, &rows[ix], cx)).collect()
                })
                .flex_1()
                .font_family(theme.typography.mono.clone())
                .text_size(theme.typography.mono_md.size);
                let summary = format!(
                    "{} insertion(s), {} deletion(s)",
                    stats.inserted, stats.deleted
                );
                (list.into_any_element(), summary)
            }
            Content::Error(message) => {
                let panel = div()
                    .id("load-error")
                    .test_support()
                    .role(Role::Alert)
                    .aria_label(message.clone())
                    .m_3()
                    .p_3()
                    .rounded_md()
                    .bg(theme.colors.destructive)
                    .text_color(theme.colors.destructive_foreground)
                    .child(message.clone());
                (panel.into_any_element(), String::new())
            }
        };
        let footer = div()
            .px_3()
            .py_1()
            .border_t_1()
            .border_color(theme.colors.border)
            .text_color(theme.colors.muted_foreground)
            .child(summary);

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.colors.background)
            .text_color(theme.colors.foreground)
            .child(header)
            .child(body)
            .child(footer)
    }
}

fn render_row(ix: usize, row: &SideBySideRow, cx: &App) -> AnyElement {
    div()
        .flex()
        .w_full()
        .h(px(ROW_HEIGHT))
        .child(render_cell(("left", ix).into(), row.left.as_ref(), cx))
        .child(render_cell(("right", ix).into(), row.right.as_ref(), cx))
        .into_any_element()
}

/// One half of a row. An absent line renders as a blank, unidentified filler.
fn render_cell(id: ElementId, line: Option<&SideLine>, cx: &App) -> AnyElement {
    let theme = Theme::global(cx).tokens;
    let cell = div().flex().flex_1().min_w_0().h_full().overflow_hidden();
    let Some(line) = line else {
        return cell.bg(theme.colors.muted).into_any_element();
    };
    let (kind, background) = match line.kind {
        TextChangeKind::Equal => ("unchanged", None),
        TextChangeKind::Delete => ("deleted", Some(DELETED)),
        TextChangeKind::Insert => ("inserted", Some(INSERTED)),
    };
    cell.id(id)
        .test_support()
        .aria_label(format!("{} {kind}: {}", line.number, line.text))
        .when_some(background, Styled::bg)
        .child(
            div()
                .flex_none()
                .w(px(NUMBER_WIDTH))
                .pr_2()
                .text_right()
                .text_color(theme.colors.muted_foreground)
                .child(line.number.to_string()),
        )
        .child(div().whitespace_nowrap().child(line.text.clone()))
        .into_any_element()
}

/// Translucent so they read on light and dark themes alike.
const DELETED: Hsla = hsla(0., 0.7, 0.5, 0.25);
const INSERTED: Hsla = hsla(130. / 360., 0.6, 0.45, 0.25);

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;
    use std::fs;

    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{
        AnyWindowHandle, AppContext as _, Bounds, ElementId, Point, TestAppContext, WindowBounds,
        WindowOptions, px, size,
    };
    use tempfile::TempDir;

    use super::DiffView;

    fn left(row: usize) -> ElementId {
        ("left", row).into()
    }

    fn right(row: usize) -> ElementId {
        ("right", row).into()
    }

    /// Opens a 900x600 window comparing two files written with `left` and `right`.
    fn open(
        cx: &mut TestAppContext,
        left: Option<&str>,
        right: &str,
    ) -> (AnyWindowHandle, TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let (left_path, right_path) = (dir.path().join("left.txt"), dir.path().join("right.txt"));
        if let Some(left) = left {
            fs::write(&left_path, left).unwrap();
        }
        fs::write(&right_path, right).unwrap();

        cx.update(gpui_kit::init);
        let window = cx.update(|cx| {
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point::default(),
                    size: size(px(900.), px(600.)),
                })),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |_, cx| {
                cx.new(|_| DiffView::load(&left_path, &right_path))
            })
            .unwrap()
            .0
        });
        (window, dir)
    }

    #[gpui_kit::test]
    fn changed_lines_sit_side_by_side(cx: &mut TestAppContext) {
        let (window, _dir) = open(cx, Some("a\nb\nz\n"), "a\nc\nz\n");

        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            assert_eq!(window.find(left(0)).label(), Some("1 unchanged: a"));
            assert_eq!(window.find(right(0)).label(), Some("1 unchanged: a"));
            assert_eq!(window.find(left(1)).label(), Some("2 deleted: b"));
            assert_eq!(window.find(right(1)).label(), Some("2 inserted: c"));
            assert_eq!(window.find(left(2)).label(), Some("3 unchanged: z"));
            let (deleted, inserted) = (window.find(left(1)), window.find(right(1)));
            assert!(deleted.bounds().right() <= inserted.bounds().left());
        })
        .unwrap();
    }

    #[gpui_kit::test]
    fn a_side_without_a_line_has_no_cell(cx: &mut TestAppContext) {
        let (window, _dir) = open(cx, Some("a\nz\n"), "a\nnew\nz\n");

        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            assert!(window.try_find(left(1)).is_none());
            assert_eq!(window.find(right(1)).label(), Some("2 inserted: new"));
            assert_eq!(window.find(left(2)).label(), Some("2 unchanged: z"));
        })
        .unwrap();
    }

    #[gpui_kit::test]
    fn only_visible_rows_of_a_large_file_are_built(cx: &mut TestAppContext) {
        let mut text = String::new();
        for n in 1..=10_000 {
            writeln!(text, "line {n}").unwrap();
        }
        let (window, _dir) = open(cx, Some(&text), &text);

        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            assert_eq!(window.find(left(0)).label(), Some("1 unchanged: line 1"));
            assert!(window.try_find(left(9_999)).is_none());
        })
        .unwrap();
    }

    #[gpui_kit::test]
    fn load_error_is_shown_in_the_window(cx: &mut TestAppContext) {
        let (window, dir) = open(cx, None, "a\n");
        let missing = dir.path().join("left.txt");

        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            let error = window.find("load-error");
            let message = error.label().unwrap();
            assert!(
                message.starts_with(&format!("loading left input {}", missing.display())),
                "{message}"
            );
            assert!(window.try_find(left(0)).is_none());
        })
        .unwrap();
    }
}
