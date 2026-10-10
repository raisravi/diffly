//! UI-agnostic diff engine for diffly.
//!
//! This crate must not depend on GPUI, clap or anyhow: it produces a plain
//! [`DiffResult`] model that front-ends (CLI, desktop app) render.

mod diff;
mod error;
mod json;
mod mode;
mod result;
mod side;
mod source;
mod text;

pub use diff::{DiffOptions, InputKind, diff};
pub use error::{DiffError, Result};
pub use json::{JsonChange, JsonChangeKind, JsonDiff, JsonPath, PathSegment};
pub use mode::DiffMode;
pub use result::{DiffBody, DiffResult, DiffStats};
pub use side::Side;
pub use source::load_source;
pub use text::{SideBySideRow, SideLine, TextChange, TextChangeKind, TextDiff};
