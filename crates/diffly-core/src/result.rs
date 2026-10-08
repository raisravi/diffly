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
