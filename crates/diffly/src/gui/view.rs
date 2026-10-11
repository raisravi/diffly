use std::collections::HashSet;
use std::fmt;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::Context as _;
use diffly_core::{
    DiffBody, DiffMode, DiffOptions, DiffResult, DiffStats, InputKind, JsonChangeKind, JsonNode,
    JsonNodeKind, JsonPath, PathSegment, SideBySideRow, SideLine, TextChangeKind, diff,
    load_source,
};
use gpui_kit::base::{TestSupportExt as _, Theme};
use gpui_kit::component::Disableable as _;
use gpui_kit::component::button::{Button, Toggle, ToggleGroup, ToggleVariants as _};
use gpui_kit::{
    AnyElement, App, Context, Div, ElementId, ExternalPaths, HighlightStyle, Hsla, IntoElement,
    ParentElement, PathPromptOptions, Render, Role, SharedString, Stateful,
    StatefulInteractiveElement, Styled, StyledText, Window, div, hsla, prelude::*, px,
    uniform_list,
};

/// Which diff to run. `Auto` detects JSON from the file extensions, like the CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KindChoice {
    Auto,
    Text,
    Json,
}

/// How a JSON diff is shown: as a tree of both documents, or as their text side by side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JsonView {
    Tree,
    Text,
}

#[derive(Debug, Clone, Copy)]
enum Pane {
    Left,
    Right,
}

impl Pane {
    fn name(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

/// The comparison window: two sources, a toolbar, and the diff between them.
#[derive(Debug)]
pub(crate) struct DiffView {
    left: Option<PathBuf>,
    right: Option<PathBuf>,
    kind: KindChoice,
    mode: DiffMode,
    json_view: JsonView,
    /// Tree branches the user expanded or collapsed, away from their default
    /// (expanded when something inside changed, collapsed otherwise).
    toggled: HashSet<JsonPath>,
    content: Content,
    /// The window title last set, so it's only pushed to the platform on change.
    title: String,
}

#[derive(Debug)]
enum Content {
    /// At least one side has no file yet.
    Empty,
    Text {
        /// Shared with the virtualized list's row builder.
        rows: Rc<[SideBySideRow]>,
        stats: DiffStats,
    },
    Json {
        tree: JsonNode,
        /// The tree's visible rows, rebuilt when a branch is toggled.
        rows: Rc<[TreeRow]>,
        stats: DiffStats,
    },
    /// Shown instead of a diff when the files can't be compared.
    Error(SharedString),
}

impl DiffView {
    pub(crate) fn new(left: Option<PathBuf>, right: Option<PathBuf>) -> Self {
        let mut view = Self {
            left,
            right,
            kind: KindChoice::Auto,
            mode: DiffMode::default(),
            json_view: JsonView::Tree,
            toggled: HashSet::new(),
            content: Content::Empty,
            title: String::new(),
        };
        view.refresh();
        view
    }

    /// Re-runs the diff from the current sources and settings.
    fn refresh(&mut self) {
        let (Some(left), Some(right)) = (&self.left, &self.right) else {
            self.content = Content::Empty;
            return;
        };
        self.content = match compare(left, right, self.diff_kind()) {
            Ok(result) => self.content_for(result),
            Err(err) => Content::Error(format!("{err:#}").into()),
        };
    }

    fn content_for(&self, result: DiffResult) -> Content {
        let stats = result.stats;
        match result.body {
            DiffBody::Text(text) => Content::Text {
                rows: text.side_by_side().into(),
                stats,
            },
            DiffBody::Json(json) => Content::Json {
                rows: tree_rows(&json.tree, &self.toggled).into(),
                tree: json.tree,
                stats,
            },
        }
    }

    /// Expands a collapsed tree branch, or collapses an expanded one.
    fn toggle_branch(&mut self, path: &JsonPath, cx: &mut Context<Self>) {
        if !self.toggled.remove(path) {
            self.toggled.insert(path.clone());
        }
        if let Content::Json { tree, rows, .. } = &mut self.content {
            *rows = tree_rows(tree, &self.toggled).into();
        }
        cx.notify();
    }

    fn source(&self, pane: Pane) -> Option<&PathBuf> {
        match pane {
            Pane::Left => self.left.as_ref(),
            Pane::Right => self.right.as_ref(),
        }
    }

    /// "diffly — old.txt ↔ new.txt", naming whichever files are chosen.
    fn title(&self) -> String {
        let name = |pane| {
            self.source(pane).map_or("?".into(), |path: &PathBuf| {
                path.file_name()
                    .unwrap_or(path.as_os_str())
                    .to_string_lossy()
                    .into_owned()
            })
        };
        if self.left.is_none() && self.right.is_none() {
            return "diffly".into();
        }
        format!(
            "diffly \u{2014} {} \u{2194} {}",
            name(Pane::Left),
            name(Pane::Right)
        )
    }

    fn input_kind(&self) -> InputKind {
        let text = InputKind::Text(self.mode);
        match self.kind {
            KindChoice::Text => text,
            KindChoice::Json => InputKind::Json,
            KindChoice::Auto => match (&self.left, &self.right) {
                (Some(left), Some(right)) => InputKind::detect_pair(left, right).unwrap_or(text),
                _ => text,
            },
        }
    }

    /// The diff to run: the input kind, except that JSON's text view diffs it as text.
    fn diff_kind(&self) -> InputKind {
        match self.input_kind() {
            InputKind::Json if self.json_view == JsonView::Text => InputKind::Text(self.mode),
            kind => kind,
        }
    }

    fn set_source(&mut self, pane: Pane, path: PathBuf, cx: &mut Context<Self>) {
        let source = match pane {
            Pane::Left => &mut self.left,
            Pane::Right => &mut self.right,
        };
        *source = Some(path);
        self.toggled.clear();
        self.refresh();
        cx.notify();
    }

    fn choose_file(pane: Pane, cx: &mut Context<Self>) {
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(format!("Choose the {} file", pane.name()).into()),
        });
        cx.spawn(async move |this, cx| {
            let picked = match picked.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                Ok(Ok(None)) | Err(_) => None,
                Ok(Err(err)) => {
                    tracing::warn!("file picker failed: {err:#}");
                    None
                }
            };
            if let Some(path) = picked {
                this.update(cx, |this, cx| this.set_source(pane, path, cx))
                    .ok();
            }
        })
        .detach();
    }
}

fn compare(left: &Path, right: &Path, kind: InputKind) -> anyhow::Result<DiffResult> {
    let left_text =
        load_source(left).with_context(|| format!("loading left input {}", left.display()))?;
    let right_text =
        load_source(right).with_context(|| format!("loading right input {}", right.display()))?;
    diff(&left_text, &right_text, &DiffOptions::from(kind))
        .with_context(|| format!("comparing {} with {}", left.display(), right.display()))
}

const ROW_HEIGHT: f32 = 20.;
const NUMBER_WIDTH: f32 = 48.;
const TREE_INDENT: f32 = 16.;

impl Render for DiffView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx).tokens;
        let title = self.title();
        if title != self.title {
            window.set_window_title(&title);
            self.title = title;
        }

        let header = div()
            .flex()
            .border_b_1()
            .border_color(theme.colors.border)
            .child(self.render_source(Pane::Left, cx))
            .child(self.render_source(Pane::Right, cx));

        let (body, summary) = self.render_body(cx);
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
            .child(self.render_toolbar(cx))
            .child(header)
            .child(body)
            .child(footer)
    }
}

impl DiffView {
    /// The main area for the current content, and the summary for the footer.
    fn render_body(&self, cx: &mut Context<Self>) -> (AnyElement, String) {
        let theme = Theme::global(cx).tokens;
        let monospace = |list: gpui_kit::UniformList| {
            list.flex_1()
                .font_family(theme.typography.mono.clone())
                .text_size(theme.typography.mono_md.size)
        };
        match &self.content {
            Content::Empty => {
                // The whole empty column is a drop target, not just its header.
                let mut panel = |pane: Pane| {
                    let name = pane.name();
                    let missing = self.source(pane).is_none();
                    drop_target(
                        div().id(SharedString::from(format!("drop-{name}-area"))),
                        pane,
                        cx,
                    )
                    .test_support()
                    .test_support()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(missing, |panel| {
                        panel.child(format!("Drop the {name} file here"))
                    })
                };
                let message = "Choose a file for each side";
                let notice = div()
                    .id("empty")
                    .test_support()
                    .aria_label(message)
                    .flex_1()
                    .flex()
                    .text_color(theme.colors.muted_foreground)
                    .child(panel(Pane::Left))
                    .child(panel(Pane::Right));
                (notice.into_any_element(), String::new())
            }
            Content::Text { rows, stats } => {
                let rows = Rc::clone(rows);
                let list = uniform_list("rows", rows.len(), move |range, _, cx| {
                    range.map(|ix| render_row(ix, &rows[ix], cx)).collect()
                });
                let summary = format!(
                    "{} insertion(s), {} deletion(s) ({} mode)",
                    stats.inserted, stats.deleted, self.mode
                );
                (monospace(list).into_any_element(), summary)
            }
            Content::Json { rows, stats, .. } => {
                let rows = Rc::clone(rows);
                let list = uniform_list(
                    "json-tree",
                    rows.len(),
                    cx.processor(move |_, range: Range<usize>, _, cx| {
                        range.map(|ix| render_tree_row(&rows[ix], cx)).collect()
                    }),
                );
                let summary = format!(
                    "{} added, {} removed, {} changed (json)",
                    stats.inserted, stats.deleted, stats.changed
                );
                (monospace(list).into_any_element(), summary)
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
        }
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let kinds = [
            (KindChoice::Auto, "kind-auto", "Auto"),
            (KindChoice::Text, "kind-text", "Text"),
            (KindChoice::Json, "kind-json", "JSON"),
        ];
        let modes = [
            (DiffMode::Line, "mode-line", "Line"),
            (DiffMode::Word, "mode-word", "Word"),
            (DiffMode::Char, "mode-char", "Char"),
        ];
        let views = [
            (JsonView::Tree, "view-tree", "Tree"),
            (JsonView::Text, "view-text", "Text"),
        ];
        // Granularity only applies to text; the JSON tree is compared by structure.
        let is_text = matches!(self.diff_kind(), InputKind::Text(_));
        let is_json = self.input_kind() == InputKind::Json;
        let kind_group = choice_group("kind", kinds, self.kind, |this, kind| this.kind = kind, cx);
        let view_group = choice_group(
            "view",
            views,
            self.json_view,
            |this, view| this.json_view = view,
            cx,
        )
        .disabled(!is_json);
        let mode_group = choice_group("mode", modes, self.mode, |this, mode| this.mode = mode, cx)
            .disabled(!is_text);

        div()
            .flex()
            .items_center()
            .gap_3()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(Theme::global(cx).tokens.colors.border)
            .child(kind_group)
            .child(view_group)
            .child(mode_group)
    }

    /// One column's source: its path (or a prompt), a picker button, and a drop target.
    fn render_source(&self, pane: Pane, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::global(cx).tokens;
        let shown = self
            .source(pane)
            .map(|path| SharedString::from(path.display().to_string()));
        let name = pane.name();

        drop_target(
            div().id(SharedString::from(format!("drop-{name}"))),
            pane,
            cx,
        )
        .test_support()
        .test_support()
        .flex()
        .flex_1()
        .min_w_0()
        .items_center()
        .gap_2()
        .px_3()
        .py_1()
        .child(match shown {
            Some(shown) => div()
                .id(SharedString::from(format!("{name}-path")))
                .test_support()
                .aria_label(shown.clone())
                .flex_1()
                .min_w_0()
                .truncate()
                .child(shown)
                .into_any_element(),
            None => div()
                .flex_1()
                .text_color(theme.colors.muted_foreground)
                .child(format!("Drop the {name} file here, or"))
                .into_any_element(),
        })
        .child(
            Button::new(SharedString::from(format!("choose-{name}")))
                .label("Choose\u{2026}")
                .on_click(cx.listener(move |_, _, _, cx| Self::choose_file(pane, cx))),
        )
    }
}

/// A segmented single-choice group of `(value, id, label)` options; a click
/// stores the picked value with `set` and re-runs the diff.
fn choice_group<T: Copy + PartialEq + 'static, const N: usize>(
    id: &'static str,
    options: [(T, &'static str, &'static str); N],
    current: T,
    set: fn(&mut DiffView, T),
    cx: &mut Context<DiffView>,
) -> ToggleGroup {
    ToggleGroup::new(id)
        .segmented()
        .outline()
        .children(
            options
                .map(|(value, id, label)| Toggle::new(id).label(label).checked(value == current)),
        )
        .on_click(cx.listener(move |this, checks: &Vec<bool>, _, cx| {
            let before = options.map(|(value, ..)| value == current);
            if let Some(ix) = toggled_index(&before, checks) {
                set(this, options[ix].0);
                this.refresh();
                cx.notify();
            }
        }))
}

/// The index whose checked state differs between `before` and `after` a group click.
fn toggled_index(before: &[bool], after: &[bool]) -> Option<usize> {
    before
        .iter()
        .zip(after)
        .position(|(before, after)| before != after)
}

/// Makes `element` accept a dropped file for `pane`, highlighting it while a file hovers.
fn drop_target(element: Stateful<Div>, pane: Pane, cx: &mut Context<DiffView>) -> Stateful<Div> {
    let accent = Theme::global(cx).tokens.colors.accent;
    element
        .drag_over::<ExternalPaths>(move |style, _, _, _| style.bg(accent))
        .on_drop(cx.listener(move |this, paths: &ExternalPaths, _, cx| {
            if let Some(path) = paths.paths().first() {
                this.set_source(pane, path.clone(), cx);
            }
        }))
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
    let kind = match line.kind {
        TextChangeKind::Equal => "unchanged",
        TextChangeKind::Delete => "deleted",
        TextChangeKind::Insert => "inserted",
    };
    let background = tint(line.kind).map(|(line, _)| line);
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
        .child(div().whitespace_nowrap().child(line_text(line)))
        .into_any_element()
}

/// The line's text, with word or char changes inside it drawn on a stronger background.
fn line_text(line: &SideLine) -> StyledText {
    let mut highlights = Vec::new();
    let mut start = 0;
    for span in &line.inline {
        let end = start + span.value.len();
        if let Some((_, background)) = tint(span.kind) {
            let style = HighlightStyle {
                background_color: Some(background),
                ..HighlightStyle::default()
            };
            highlights.push((start..end, style));
        }
        start = end;
    }
    StyledText::new(line.text.clone()).with_highlights(highlights)
}

/// Backgrounds for a changed line and for the changed spans inside it.
fn tint(kind: TextChangeKind) -> Option<(Hsla, Hsla)> {
    match kind {
        TextChangeKind::Equal => None,
        TextChangeKind::Delete => Some((DELETED, DELETED_STRONG)),
        TextChangeKind::Insert => Some((INSERTED, INSERTED_STRONG)),
    }
}

/// One visible node of the JSON tree, flattened for the virtualized list.
#[derive(Debug)]
struct TreeRow {
    path: JsonPath,
    /// Left padding: one [`TREE_INDENT`] per level below the root.
    indent: f32,
    /// A branch's open/closed chevron, or `+`/`−` for an added/removed leaf.
    gutter: &'static str,
    /// The node's key, `[index]`, or `$` for the root.
    name: String,
    /// The value as drawn: `{2}` for an object, `30 → 31` for a changed leaf.
    value: String,
    /// The value as described to assistive tech and tests, e.g. `added 1`.
    accessible_value: String,
    background: Option<Hsla>,
    /// `Some(expanded)` for an object or array, `None` for a leaf.
    expanded: Option<bool>,
}

/// The rows of `tree` that are on screen, depth-first, skipping collapsed branches.
fn tree_rows(tree: &JsonNode, toggled: &HashSet<JsonPath>) -> Vec<TreeRow> {
    let mut rows = Vec::new();
    push_tree_rows(tree, 0., toggled, &mut rows);
    rows
}

fn push_tree_rows(
    node: &JsonNode,
    indent: f32,
    toggled: &HashSet<JsonPath>,
    rows: &mut Vec<TreeRow>,
) {
    // `change` is `(word for the accessible label, marker drawn in the gutter)`.
    let (children, value, change, background) = match &node.kind {
        JsonNodeKind::Object(children) => (
            Some(children),
            format!("{{{}}}", children.len()),
            None,
            None,
        ),
        JsonNodeKind::Array(children) => {
            (Some(children), format!("[{}]", children.len()), None, None)
        }
        JsonNodeKind::Unchanged(value) => (None, value.to_string(), None, None),
        JsonNodeKind::Change(JsonChangeKind::Added(value)) => (
            None,
            value.to_string(),
            Some(("added", "+")),
            Some(INSERTED),
        ),
        JsonNodeKind::Change(JsonChangeKind::Removed(value)) => (
            None,
            value.to_string(),
            Some(("removed", "\u{2212}")),
            Some(DELETED),
        ),
        JsonNodeKind::Change(JsonChangeKind::Changed { left, right }) => (
            None,
            format!("{left} \u{2192} {right}"),
            None,
            Some(CHANGED),
        ),
    };
    let accessible_value =
        change.map_or_else(|| value.clone(), |(word, _)| format!("{word} {value}"));
    let name = match node.path.0.last() {
        None => "$".to_owned(),
        Some(PathSegment::Key(key)) => key.clone(),
        Some(PathSegment::Index(index)) => format!("[{index}]"),
    };
    let expanded = children.map(|_| node.changed != toggled.contains(&node.path));
    let gutter = match (expanded, change) {
        (Some(true), _) => "\u{25be}",
        (Some(false), _) => "\u{25b8}",
        (None, Some((_, marker))) => marker,
        (None, None) => "",
    };
    rows.push(TreeRow {
        path: node.path.clone(),
        indent,
        gutter,
        name,
        value,
        accessible_value,
        background,
        expanded,
    });
    if let (Some(children), Some(true)) = (children, expanded) {
        for child in children {
            push_tree_rows(child, indent + TREE_INDENT, toggled, rows);
        }
    }
}

/// The element id of the tree row for the node at `path` (a [`JsonPath`] or its text).
fn tree_row_id(path: impl fmt::Display) -> ElementId {
    SharedString::from(format!("json-{path}")).into()
}

/// One tree row; clicking a branch expands or collapses it.
fn render_tree_row(row: &TreeRow, cx: &mut Context<DiffView>) -> AnyElement {
    let theme = Theme::global(cx).tokens;
    let path = &row.path;
    let element = div()
        .id(tree_row_id(path))
        .test_support()
        .aria_label(format!("{path}: {}", row.accessible_value))
        .flex()
        .w_full()
        .h(px(ROW_HEIGHT))
        .pl(px(row.indent))
        .whitespace_nowrap()
        .overflow_hidden()
        .when_some(row.background, Styled::bg)
        .child(
            div()
                .flex_none()
                .w(px(TREE_INDENT))
                .text_color(theme.colors.muted_foreground)
                .child(row.gutter),
        )
        .child(format!("{}: {}", row.name, row.value));
    if row.expanded.is_none() {
        return element.into_any_element();
    }
    let path = path.clone();
    element
        .cursor_pointer()
        .hover(|style| style.bg(theme.colors.muted))
        .on_click(cx.listener(move |this, _, _, cx| this.toggle_branch(&path, cx)))
        .into_any_element()
}

/// Translucent so they read on light and dark themes alike.
const DELETED: Hsla = hsla(0., 0.7, 0.5, 0.25);
const INSERTED: Hsla = hsla(130. / 360., 0.6, 0.45, 0.25);
const CHANGED: Hsla = hsla(45. / 360., 0.9, 0.5, 0.25);
const DELETED_STRONG: Hsla = hsla(0., 0.7, 0.5, 0.5);
const INSERTED_STRONG: Hsla = hsla(130. / 360., 0.6, 0.45, 0.5);

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;
    use std::fs;
    use std::path::Path;

    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{
        AnyWindowHandle, AppContext as _, Bounds, ElementId, ExternalPaths, FileDropEvent,
        PlatformInput, Point, TestAppContext, VisualTestContext, WindowBounds, WindowOptions, px,
        size,
    };
    use tempfile::TempDir;

    use super::DiffView;

    fn left(row: usize) -> ElementId {
        ("left", row).into()
    }

    fn right(row: usize) -> ElementId {
        ("right", row).into()
    }

    /// A JSON tree row, by the path of its node: `json("$.user.age")`.
    fn json(path: &str) -> ElementId {
        super::tree_row_id(path)
    }

    /// Writes `(name, contents)` files into a fresh directory.
    fn files(files: &[(&str, &str)]) -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (name, contents) in files {
            fs::write(dir.path().join(name), contents).unwrap();
        }
        dir
    }

    /// Opens a 900x600 window comparing `left` with `right`, either of which may be unset.
    fn open(cx: &mut TestAppContext, left: Option<&Path>, right: Option<&Path>) -> AnyWindowHandle {
        let (left, right) = (left.map(Path::to_owned), right.map(Path::to_owned));
        cx.update(gpui_kit::init);
        cx.update(|cx| {
            let options = WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point::default(),
                    size: size(px(900.), px(600.)),
                })),
                ..Default::default()
            };
            gpui_kit::open_window(options, cx, |_, cx| cx.new(|_| DiffView::new(left, right)))
                .unwrap()
                .0
        })
    }

    /// Opens a window comparing `left.txt` with `right.txt` holding the given text.
    fn open_texts(cx: &mut TestAppContext, left: &str, right: &str) -> (AnyWindowHandle, TempDir) {
        let dir = files(&[("left.txt", left), ("right.txt", right)]);
        let window = open(
            cx,
            Some(&dir.path().join("left.txt")),
            Some(&dir.path().join("right.txt")),
        );
        (window, dir)
    }

    fn label(cx: &mut TestAppContext, window: AnyWindowHandle, id: ElementId) -> Option<String> {
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window
                .try_find(id)
                .and_then(|e| e.label().map(str::to_owned))
        })
        .unwrap()
    }

    fn click(cx: &mut TestAppContext, window: AnyWindowHandle, id: impl Into<ElementId>) {
        let id = id.into();
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            window.click(id, cx);
        })
        .unwrap();
        cx.run_until_parked();
    }

    #[gpui_kit::test]
    fn changed_lines_sit_side_by_side(cx: &mut TestAppContext) {
        let (window, _dir) = open_texts(cx, "a\nb\nz\n", "a\nc\nz\n");

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
        let (window, _dir) = open_texts(cx, "a\nz\n", "a\nnew\nz\n");

        assert_eq!(label(cx, window, left(1)), None);
        assert_eq!(
            label(cx, window, right(1)).as_deref(),
            Some("2 inserted: new")
        );
        assert_eq!(
            label(cx, window, left(2)).as_deref(),
            Some("2 unchanged: z")
        );
    }

    #[gpui_kit::test]
    fn only_visible_rows_of_a_large_file_are_built(cx: &mut TestAppContext) {
        let mut text = String::new();
        for n in 1..=10_000 {
            writeln!(text, "line {n}").unwrap();
        }
        let (window, _dir) = open_texts(cx, &text, &text);

        assert_eq!(
            label(cx, window, left(0)).as_deref(),
            Some("1 unchanged: line 1")
        );
        assert_eq!(label(cx, window, left(9_999)), None);
    }

    #[gpui_kit::test]
    fn load_error_is_shown_in_the_window(cx: &mut TestAppContext) {
        let dir = files(&[("right.txt", "a\n")]);
        let missing = dir.path().join("left.txt");
        let window = open(cx, Some(&missing), Some(&dir.path().join("right.txt")));

        let message = label(cx, window, "load-error".into()).unwrap();
        assert!(
            message.starts_with(&format!("loading left input {}", missing.display())),
            "{message}"
        );
        assert_eq!(label(cx, window, left(0)), None);
    }

    #[gpui_kit::test]
    fn without_paths_the_window_asks_for_files(cx: &mut TestAppContext) {
        let window = open(cx, None, None);

        assert_eq!(
            label(cx, window, "empty".into()).as_deref(),
            Some("Choose a file for each side")
        );
        assert_eq!(label(cx, window, left(0)), None);
    }

    #[gpui_kit::test]
    fn command_line_paths_prefill_both_sides(cx: &mut TestAppContext) {
        let (window, dir) = open_texts(cx, "a\n", "b\n");

        let shown = label(cx, window, "left-path".into()).unwrap();
        assert_eq!(shown, dir.path().join("left.txt").display().to_string());
        let shown = label(cx, window, "right-path".into()).unwrap();
        assert_eq!(shown, dir.path().join("right.txt").display().to_string());
    }

    #[gpui_kit::test]
    fn choosing_a_file_fills_that_side(cx: &mut TestAppContext) {
        let dir = files(&[("left.txt", "a\nb\n"), ("right.txt", "a\nc\n")]);
        let picked = dir.path().join("left.txt");
        let window = open(cx, None, Some(&dir.path().join("right.txt")));

        click(cx, window, "choose-left");
        assert!(cx.did_prompt_for_paths());
        cx.simulate_path_prompt_response(|_| Some(vec![picked.clone()]));
        cx.run_until_parked();

        assert_eq!(
            label(cx, window, "left-path".into()),
            Some(picked.display().to_string())
        );
        assert_eq!(label(cx, window, left(1)).as_deref(), Some("2 deleted: b"));
        assert_eq!(label(cx, window, "empty".into()), None);
    }

    #[gpui_kit::test]
    fn dropping_a_file_fills_that_side(cx: &mut TestAppContext) {
        let dir = files(&[("left.txt", "a\nb\n"), ("right.txt", "a\nc\n")]);
        let dropped = dir.path().join("right.txt");
        let window = open(cx, Some(&dir.path().join("left.txt")), None);

        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            let position = window.find("drop-right-area").bounds().center();
            let paths = ExternalPaths(vec![dropped.clone()].into());
            for event in [
                FileDropEvent::Entered { position, paths },
                FileDropEvent::Submit { position },
            ] {
                window.dispatch_event(PlatformInput::FileDrop(event), cx);
            }
        })
        .unwrap();
        cx.run_until_parked();

        assert_eq!(
            label(cx, window, "right-path".into()),
            Some(dropped.display().to_string())
        );
        assert_eq!(
            label(cx, window, right(1)).as_deref(),
            Some("2 inserted: c")
        );
    }

    /// Opens `a.json` and `b.json`; their extension makes `Auto` pick the JSON diff.
    fn open_jsons(cx: &mut TestAppContext, left: &str, right: &str) -> (AnyWindowHandle, TempDir) {
        let dir = files(&[("a.json", left), ("b.json", right)]);
        let window = open(
            cx,
            Some(&dir.path().join("a.json")),
            Some(&dir.path().join("b.json")),
        );
        (window, dir)
    }

    const USER_LEFT: &str =
        r#"{"user": {"age": 30, "name": "ann"}, "meta": {"v": 1}, "old": true}"#;
    const USER_RIGHT: &str = r#"{"user": {"age": 31, "name": "ann"}, "meta": {"v": 1}, "x": 1}"#;

    #[gpui_kit::test]
    fn json_files_are_shown_as_a_tree_with_changes_expanded(cx: &mut TestAppContext) {
        let (window, _dir) = open_jsons(cx, USER_LEFT, USER_RIGHT);

        assert_eq!(label(cx, window, json("$")).as_deref(), Some("$: {4}"));
        assert_eq!(
            label(cx, window, json("$.user")).as_deref(),
            Some("$.user: {2}")
        );
        assert_eq!(
            label(cx, window, json("$.user.age")).as_deref(),
            Some("$.user.age: 30 \u{2192} 31")
        );
        assert_eq!(
            label(cx, window, json("$.user.name")).as_deref(),
            Some(r#"$.user.name: "ann""#)
        );
        assert_eq!(
            label(cx, window, json("$.x")).as_deref(),
            Some("$.x: added 1")
        );
        assert_eq!(
            label(cx, window, json("$.old")).as_deref(),
            Some("$.old: removed true")
        );
        // An unchanged subtree starts collapsed.
        assert_eq!(
            label(cx, window, json("$.meta")).as_deref(),
            Some("$.meta: {1}")
        );
        assert_eq!(label(cx, window, json("$.meta.v")), None);
    }

    #[gpui_kit::test]
    fn clicking_a_branch_expands_or_collapses_it(cx: &mut TestAppContext) {
        let (window, _dir) = open_jsons(cx, USER_LEFT, USER_RIGHT);

        click(cx, window, json("$.meta"));
        assert_eq!(
            label(cx, window, json("$.meta.v")).as_deref(),
            Some("$.meta.v: 1")
        );

        click(cx, window, json("$.user"));
        assert_eq!(label(cx, window, json("$.user.age")), None);
        assert!(label(cx, window, json("$.meta.v")).is_some());

        // Clicking a leaf does nothing.
        click(cx, window, json("$.x"));
        assert!(label(cx, window, json("$.x")).is_some());
    }

    #[gpui_kit::test]
    fn json_can_be_viewed_as_side_by_side_text(cx: &mut TestAppContext) {
        let (window, _dir) = open_jsons(cx, "{\n  \"a\": 1\n}\n", "{\n  \"a\": 2\n}\n");

        // Granularity means nothing to the tree, so the mode toggles stop responding.
        click(cx, window, "mode-word");
        click(cx, window, "view-text");
        assert_eq!(label(cx, window, json("$")), None);
        assert_eq!(
            label(cx, window, left(1)).as_deref(),
            Some("2 deleted:   \"a\": 1")
        );
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            assert_eq!(window.find("mode-line").checked(), Some(true));
        })
        .unwrap();

        // In the text view they apply again.
        click(cx, window, "mode-word");
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            assert_eq!(window.find("mode-word").checked(), Some(true));
        })
        .unwrap();

        click(cx, window, "view-tree");
        assert!(label(cx, window, json("$.a")).is_some());
        assert_eq!(label(cx, window, left(1)), None);
    }

    #[gpui_kit::test]
    fn invalid_json_shows_the_parse_error_instead_of_the_tree(cx: &mut TestAppContext) {
        let (window, _dir) = open_jsons(cx, r#"{"a": }"#, r#"{"a": 1}"#);

        let message = label(cx, window, "load-error".into()).unwrap();
        assert!(
            message.contains("left input is not valid JSON") && message.contains("line 1"),
            "{message}"
        );
        assert_eq!(label(cx, window, json("$")), None);
    }

    #[gpui_kit::test]
    fn window_title_names_the_chosen_files(cx: &mut TestAppContext) {
        let (window, _dir) = open_texts(cx, "a\n", "b\n");

        let mut cx = VisualTestContext::from_window(window, cx);
        cx.run_until_parked();
        assert_eq!(
            cx.window_title().as_deref(),
            Some("diffly \u{2014} left.txt \u{2194} right.txt")
        );
    }

    #[gpui_kit::test]
    fn choosing_a_kind_reruns_the_diff(cx: &mut TestAppContext) {
        let (window, _dir) = open_texts(cx, r#"{"age": 30}"#, r#"{"age": 31}"#);
        assert_eq!(label(cx, window, json("$.age")), None);

        click(cx, window, "kind-json");
        assert_eq!(
            label(cx, window, json("$.age")).as_deref(),
            Some("$.age: 30 \u{2192} 31")
        );

        click(cx, window, "kind-text");
        assert_eq!(label(cx, window, json("$.age")), None);
        assert!(label(cx, window, left(0)).is_some());
    }

    #[gpui_kit::test]
    fn mode_is_chosen_in_the_toolbar_and_disabled_for_json(cx: &mut TestAppContext) {
        let (window, _dir) = open_texts(cx, "the quick fox\n", "the slow fox\n");

        click(cx, window, "mode-word");
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            assert_eq!(window.find("mode-word").checked(), Some(true));
            assert_eq!(window.find("mode-line").checked(), Some(false));
            assert_eq!(
                window.find(left(0)).label(),
                Some("1 deleted: the quick fox")
            );
        })
        .unwrap();

        // Granularity means nothing for JSON, so the mode toggles stop responding.
        click(cx, window, "kind-json");
        click(cx, window, "mode-char");
        click(cx, window, "kind-text");
        cx.update_window(window, |_, window, cx| {
            window.render_frame(cx);
            assert_eq!(window.find("mode-word").checked(), Some(true));
            assert_eq!(window.find("mode-char").checked(), Some(false));
        })
        .unwrap();
    }
}
