use similar::{ChangeTag, TextDiff};

use crate::DiffMode;

/// What happened to a span of text between the old and new input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChangeKind {
    Equal,
    Insert,
    Delete,
}

/// A contiguous span of text with a single [`ChangeKind`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub kind: ChangeKind,
    pub value: String,
}

/// Counts of changed tokens (lines, words or chars depending on the mode).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DiffStats {
    pub inserted: usize,
    pub deleted: usize,
}

/// Front-end neutral result of a diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffResult {
    pub mode: DiffMode,
    pub changes: Vec<Change>,
    pub stats: DiffStats,
}

impl DiffResult {
    #[must_use]
    pub fn is_identical(&self) -> bool {
        self.stats.inserted == 0 && self.stats.deleted == 0
    }
}

/// Diffs two strings at the given granularity.
#[must_use]
pub fn diff_text(old: &str, new: &str, mode: DiffMode) -> DiffResult {
    let diff = match mode {
        DiffMode::Line => TextDiff::from_lines(old, new),
        DiffMode::Word => TextDiff::from_words(old, new),
        DiffMode::Char => TextDiff::from_chars(old, new),
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

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    fn kinds(result: &DiffResult) -> Vec<(ChangeKind, &str)> {
        result
            .changes
            .iter()
            .map(|c| (c.kind, c.value.as_str()))
            .collect()
    }

    #[test]
    fn identical_inputs() {
        let result = diff_text("a\nb\n", "a\nb\n", DiffMode::Line);
        assert!(result.is_identical());
    }

    #[test]
    fn empty_inputs() {
        let result = diff_text("", "", DiffMode::Char);
        assert!(result.is_identical());
        assert!(result.changes.is_empty());
    }

    #[test]
    fn line_mode() {
        let result = diff_text("a\nb\n", "a\nc\n", DiffMode::Line);
        assert_eq!(
            kinds(&result),
            vec![
                (ChangeKind::Equal, "a\n"),
                (ChangeKind::Delete, "b\n"),
                (ChangeKind::Insert, "c\n"),
            ]
        );
        assert_eq!(
            result.stats,
            DiffStats {
                inserted: 1,
                deleted: 1
            }
        );
    }

    #[test]
    fn word_mode() {
        let result = diff_text("hello world", "hello there", DiffMode::Word);
        assert_eq!(
            result.stats,
            DiffStats {
                inserted: 1,
                deleted: 1
            }
        );
        assert!(
            result
                .changes
                .iter()
                .any(|c| c.kind == ChangeKind::Insert && c.value == "there")
        );
    }

    #[test]
    fn char_mode() {
        let result = diff_text("cat", "cut", DiffMode::Char);
        assert_eq!(
            kinds(&result),
            vec![
                (ChangeKind::Equal, "c"),
                (ChangeKind::Delete, "a"),
                (ChangeKind::Insert, "u"),
                (ChangeKind::Equal, "t"),
            ]
        );
    }
}
