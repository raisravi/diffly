use std::path::Path;

use crate::{DiffMode, DiffResult, Result, html, json, text};

/// What kind of data the two inputs hold, which decides how they are compared.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum InputKind {
    /// Plain text at the given granularity.
    Text(DiffMode),
    /// JSON documents, compared as parsed values.
    Json,
    /// HTML documents, compared by element structure, attributes and text.
    Html,
}

impl InputKind {
    /// Recognises structured kinds from a file extension. Returns `None` for
    /// anything else; callers fall back to a text diff.
    #[must_use]
    pub fn detect(path: &Path) -> Option<Self> {
        let extension = path.extension()?.to_str()?.to_ascii_lowercase();
        match extension.as_str() {
            "json" => Some(Self::Json),
            "html" | "htm" => Some(Self::Html),
            _ => None,
        }
    }

    /// Detects the kind for a pair of inputs: both must be recognised as the same
    /// structured kind, otherwise `None` (compare as text).
    #[must_use]
    pub fn detect_pair(left: &Path, right: &Path) -> Option<Self> {
        Self::detect(left).filter(|kind| Self::detect(right) == Some(*kind))
    }
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

    /// Structural comparison of two JSON documents.
    #[must_use]
    pub fn json() -> Self {
        Self {
            kind: InputKind::Json,
        }
    }

    /// Structural comparison of two HTML documents.
    #[must_use]
    pub fn html() -> Self {
        Self {
            kind: InputKind::Html,
        }
    }
}

impl From<InputKind> for DiffOptions {
    fn from(kind: InputKind) -> Self {
        Self { kind }
    }
}

/// Compares two in-memory inputs. This is the single entry point front-ends use.
///
/// Fallible because structured kinds must parse their inputs first.
pub fn diff(left: &str, right: &str, options: &DiffOptions) -> Result<DiffResult> {
    match options.kind {
        InputKind::Text(mode) => Ok(text::diff_text(left, right, mode)),
        InputKind::Json => json::diff_json(left, right),
        InputKind::Html => Ok(html::diff_html(left, right)),
    }
}
