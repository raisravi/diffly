use std::path::PathBuf;

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
}

pub type Result<T, E = DiffError> = std::result::Result<T, E>;
