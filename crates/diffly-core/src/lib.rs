//! UI-agnostic diff engine for diffly.
//!
//! This crate must not depend on GPUI, clap or anyhow: it produces a plain
//! [`DiffResult`] model that front-ends (CLI, desktop app) render.

mod diff;
mod error;
mod mode;
mod result;
mod source;
mod text;

pub use diff::{DiffOptions, InputKind, diff};
pub use error::{DiffError, Result};
pub use mode::DiffMode;
pub use result::{Change, ChangeKind, DiffResult, DiffStats};
pub use source::load_source;
