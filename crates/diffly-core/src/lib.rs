//! UI-agnostic diff engine for diffly.
//!
//! This crate must not depend on GPUI, clap or anyhow: it produces a plain
//! [`DiffResult`] model that front-ends (CLI, desktop app) render.

mod error;
mod mode;
mod source;
mod text;

pub use error::{DiffError, Result};
pub use mode::DiffMode;
pub use source::load_source;
pub use text::{Change, ChangeKind, DiffResult, DiffStats, diff_text};
