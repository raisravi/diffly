use crate::{DiffMode, DiffResult, Result, text};

/// What kind of data the two inputs hold, which decides how they are compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum InputKind {
    /// Plain text at the given granularity.
    Text(DiffMode),
}

impl Default for InputKind {
    fn default() -> Self {
        Self::Text(DiffMode::default())
    }
}

/// How to compare two inputs.
///
/// Build it with a constructor or [`Default`]; fields will be added as new options land.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct DiffOptions {
    pub kind: InputKind,
}

impl DiffOptions {
    /// Plain-text comparison at the given granularity.
    #[must_use]
    pub fn text(mode: DiffMode) -> Self {
        Self {
            kind: InputKind::Text(mode),
        }
    }
}

/// Compares two in-memory inputs. This is the single entry point front-ends use.
///
/// Fallible because structured kinds must parse their inputs first.
pub fn diff(left: &str, right: &str, options: &DiffOptions) -> Result<DiffResult> {
    match options.kind {
        InputKind::Text(mode) => Ok(text::diff_text(left, right, mode)),
    }
}
