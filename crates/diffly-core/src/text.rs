use similar::{ChangeTag, TextDiff};

use crate::{Change, ChangeKind, DiffMode, DiffResult, DiffStats};

/// Diffs two strings at the given granularity.
pub(crate) fn diff_text(left: &str, right: &str, mode: DiffMode) -> DiffResult {
    let diff = match mode {
        DiffMode::Line => TextDiff::from_lines(left, right),
        DiffMode::Word => TextDiff::from_words(left, right),
        DiffMode::Char => TextDiff::from_chars(left, right),
    };

    let mut changes = Vec::new();
    let mut stats = DiffStats::default();
    for change in diff.iter_all_changes() {
        let kind = match change.tag() {
            ChangeTag::Equal => ChangeKind::Equal,
            ChangeTag::Insert => {
                stats.inserted += 1;
                ChangeKind::Insert
            }
            ChangeTag::Delete => {
                stats.deleted += 1;
                ChangeKind::Delete
            }
        };
        changes.push(Change {
            kind,
            value: change.value().to_owned(),
        });
    }

    DiffResult {
        mode,
        changes,
        stats,
    }
}
