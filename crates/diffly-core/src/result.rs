use crate::{JsonDiff, TextDiff};

/// Kind-specific detail of a diff.
///
/// Deliberately exhaustive: adding an input kind should make every front-end
/// fail to compile until it knows how to render the new body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffBody {
    Text(TextDiff),
    Json(JsonDiff),
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
