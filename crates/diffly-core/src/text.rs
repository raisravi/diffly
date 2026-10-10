use similar::ChangeTag;

use crate::{DiffBody, DiffMode, DiffResult, DiffStats};

/// What happened to a span of text between the left and right input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextChangeKind {
    Equal,
    Insert,
    Delete,
}

/// A contiguous span of text with a single [`TextChangeKind`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextChange {
    pub kind: TextChangeKind,
    pub value: String,
}

/// Changes found by a text comparison, in input order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextDiff {
    pub mode: DiffMode,
    pub changes: Vec<TextChange>,
}

/// Diffs two strings at the given granularity.
pub(crate) fn diff_text(left: &str, right: &str, mode: DiffMode) -> DiffResult {
    let changes = tokenize(left, right, mode);
    let mut stats = DiffStats::default();
    for change in &changes {
        match change.kind {
            TextChangeKind::Equal => {}
            TextChangeKind::Insert => stats.inserted += 1,
            TextChangeKind::Delete => stats.deleted += 1,
        }
    }

    DiffResult {
        stats,
        body: DiffBody::Text(TextDiff { mode, changes }),
    }
}

/// The changes between `left` and `right` at `mode` granularity.
fn tokenize(left: &str, right: &str, mode: DiffMode) -> Vec<TextChange> {
    let diff = match mode {
        DiffMode::Line => similar::TextDiff::from_lines(left, right),
        DiffMode::Word => similar::TextDiff::from_words(left, right),
        DiffMode::Char => similar::TextDiff::from_chars(left, right),
    };
    diff.iter_all_changes()
        .map(|change| TextChange {
            kind: match change.tag() {
                ChangeTag::Equal => TextChangeKind::Equal,
                ChangeTag::Insert => TextChangeKind::Insert,
                ChangeTag::Delete => TextChangeKind::Delete,
            },
            value: change.value().to_owned(),
        })
        .collect()
}

/// One line of one side in a side-by-side view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideLine {
    /// 1-based line number in that side's input.
    pub number: usize,
    /// The line without its line ending.
    pub text: String,
    pub kind: TextChangeKind,
    /// For a changed line paired with one on the other side in word or char
    /// mode: `text` split into `Equal` spans and spans of this line's `kind`.
    /// Empty otherwise.
    pub inline: Vec<TextChange>,
}

/// A row of a side-by-side view; a side is `None` where it has no line to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideBySideRow {
    pub left: Option<SideLine>,
    pub right: Option<SideLine>,
}

impl TextDiff {
    /// Aligns the changes into rows for a two-column view.
    ///
    /// Rows are always whole lines. Unchanged lines share a row. Within a run
    /// of changes, deleted and inserted lines are paired in order and
    /// leftovers get an empty cell. In word and char mode, each paired line
    /// also gets [`SideLine::inline`] spans at that granularity.
    #[must_use]
    pub fn side_by_side(&self) -> Vec<SideBySideRow> {
        if self.mode == DiffMode::Line {
            return align_lines(&self.changes);
        }
        let mut rows = align_lines(&tokenize(
            &self.side(TextChangeKind::Delete),
            &self.side(TextChangeKind::Insert),
            DiffMode::Line,
        ));
        for row in &mut rows {
            if let (Some(left), Some(right)) = (&mut row.left, &mut row.right)
                && left.kind == TextChangeKind::Delete
            {
                let spans = tokenize(&left.text, &right.text, self.mode);
                left.inline = spans_for(&spans, TextChangeKind::Delete);
                right.inline = spans_for(&spans, TextChangeKind::Insert);
            }
        }
        rows
    }

    /// Reassembles one input: the equal spans plus the spans of `kind`.
    fn side(&self, kind: TextChangeKind) -> String {
        self.changes
            .iter()
            .filter(|change| change.kind == TextChangeKind::Equal || change.kind == kind)
            .map(|change| change.value.as_str())
            .collect()
    }
}

/// One side's view of `spans`, with neighbouring spans of the same kind merged.
fn spans_for(spans: &[TextChange], kind: TextChangeKind) -> Vec<TextChange> {
    let mut merged: Vec<TextChange> = Vec::new();
    for span in spans {
        if span.kind != TextChangeKind::Equal && span.kind != kind {
            continue;
        }
        match merged.last_mut() {
            Some(last) if last.kind == span.kind => last.value.push_str(&span.value),
            _ => merged.push(span.clone()),
        }
    }
    merged
}

/// Aligns line-granularity changes into rows.
fn align_lines(changes: &[TextChange]) -> Vec<SideBySideRow> {
    let mut rows = Vec::new();
    let (mut left_number, mut right_number) = (0, 0);
    let mut deleted = Vec::new();
    let mut inserted = Vec::new();

    let cell = |number: &mut usize, change: &TextChange| {
        *number += 1;
        SideLine {
            number: *number,
            text: strip_line_ending(&change.value).to_owned(),
            kind: change.kind,
            inline: Vec::new(),
        }
    };

    for change in changes {
        match change.kind {
            TextChangeKind::Delete => deleted.push(cell(&mut left_number, change)),
            TextChangeKind::Insert => inserted.push(cell(&mut right_number, change)),
            TextChangeKind::Equal => {
                pair_up(&mut rows, &mut deleted, &mut inserted);
                rows.push(SideBySideRow {
                    left: Some(cell(&mut left_number, change)),
                    right: Some(cell(&mut right_number, change)),
                });
            }
        }
    }
    pair_up(&mut rows, &mut deleted, &mut inserted);
    rows
}

/// Emits one row per line of the longer run, draining both runs.
fn pair_up(
    rows: &mut Vec<SideBySideRow>,
    deleted: &mut Vec<SideLine>,
    inserted: &mut Vec<SideLine>,
) {
    let mut left = deleted.drain(..);
    let mut right = inserted.drain(..);
    loop {
        match (left.next(), right.next()) {
            (None, None) => break,
            (left, right) => rows.push(SideBySideRow { left, right }),
        }
    }
}

fn strip_line_ending(line: &str) -> &str {
    line.strip_suffix("\r\n")
        .or_else(|| line.strip_suffix('\n'))
        .unwrap_or(line)
}
