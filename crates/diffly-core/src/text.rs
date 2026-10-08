use similar::ChangeTag;

use crate::{DiffBody, DiffMode, DiffResult, DiffStats, TextChange, TextChangeKind, TextDiff};

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
