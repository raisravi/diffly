use std::path::Path;

use crate::{DiffError, Result};

/// Reads a file as UTF-8 text.
pub fn load_source(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    let bytes = std::fs::read(path).map_err(|source| DiffError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    String::from_utf8(bytes).map_err(|_| DiffError::NotUtf8 {
        path: path.to_path_buf(),
    })
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn reads_utf8_file() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all("héllo".as_bytes()).unwrap();
        assert_eq!(load_source(file.path()).unwrap(), "héllo");
    }

    #[test]
    fn missing_file_is_io_error() {
        let err = load_source("/definitely/not/here").unwrap_err();
        assert!(matches!(err, DiffError::Io { .. }));
    }

    #[test]
    fn invalid_utf8_is_rejected() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(&[0xff, 0xfe, 0xfd]).unwrap();
        let err = load_source(file.path()).unwrap_err();
        assert!(matches!(err, DiffError::NotUtf8 { .. }));
    }
}
