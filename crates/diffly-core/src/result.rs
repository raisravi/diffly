use crate::DiffMode;

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

/// Kind-specific detail of a diff.
///
/// Deliberately exhaustive: adding an input kind should make every front-end
/// fail to compile until it knows how to render the new body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffBody {
    Text(TextDiff),
}

/// How much differs. For text these count tokens (lines, words or chars);
/// `changed` is only used by structured kinds, where a value can be replaced in place.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DiffStats {
    pub inserted: usize,
    pub deleted: usize,
    pub changed: usize,
}

/// Front-end neutral result of a diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffResult {
    pub stats: DiffStats,
    pub body: DiffBody,
}

impl DiffResult {
    #[must_use]
    pub fn is_identical(&self) -> bool {
        self.stats == DiffStats::default()
    }
}
