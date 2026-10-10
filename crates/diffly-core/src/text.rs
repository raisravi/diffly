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
    let diff = match mode {
        DiffMode::Line => similar::TextDiff::from_lines(left, right),
        DiffMode::Word => similar::TextDiff::from_words(left, right),
        DiffMode::Char => similar::TextDiff::from_chars(left, right),
    };

    let mut changes = Vec::new();
    let mut stats = DiffStats::default();
    for change in diff.iter_all_changes() {
        let kind = match change.tag() {
            ChangeTag::Equal => TextChangeKind::Equal,
            ChangeTag::Insert => {
                stats.inserted += 1;
                TextChangeKind::Insert
            }
            ChangeTag::Delete => {
                stats.deleted += 1;
                TextChangeKind::Delete
            }
        };
        changes.push(TextChange {
            kind,
            value: change.value().to_owned(),
        });
    }

    DiffResult {
        stats,
        body: DiffBody::Text(TextDiff { mode, changes }),
    }
}

/// One line of one side in a side-by-side view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideLine {
    /// 1-based line number in that side's input.
    pub number: usize,
    /// The line without its line ending.
    pub text: String,
    pub kind: TextChangeKind,
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
    /// Unchanged lines share a row. Within a run of changes, deleted and
    /// inserted lines are paired in order and leftovers get an empty cell.
    /// Intended for [`DiffMode::Line`]; in other modes each cell is one token.
    #[must_use]
    pub fn side_by_side(&self) -> Vec<SideBySideRow> {
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
            }
        };

        for change in &self.changes {
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
