use std::path::PathBuf;

use crate::Side;

/// Errors produced by the diff engine.
#[derive(Debug, thiserror::Error)]
pub enum DiffError {
    #[error("failed to read {path}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("{path} is not valid UTF-8")]
    NotUtf8 { path: PathBuf },

    /// The source carries the parser's message and line/column.
    #[error("{side} input is not valid JSON")]
    InvalidJson {
        side: Side,
        #[source]
        source: serde_json::Error,
    },
}

pub type Result<T, E = DiffError> = std::result::Result<T, E>;
