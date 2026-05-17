use std::{ffi::OsStr, path::PathBuf};

use anyhow::{Result, ensure};

/// A data entry.
///
/// INVARIANT: fname must exist.
#[derive(Debug)]
pub struct DataEntry {
    pub fname: PathBuf,
    pub bytes: Vec<u8>,
}

impl DataEntry {
    pub fn new<P: Into<PathBuf>>(fname: P, bytes: Vec<u8>) -> Result<Self> {
        let fname = fname.into();
        ensure!(fname.exists(), "file does not exist: {}", fname.display());
        Ok(Self { fname, bytes })
    }

    pub fn extension(&self) -> Option<&OsStr> {
        self.fname.extension()
    }

    pub fn is_css(&self) -> bool {
        self.extension().is_some_and(|ext| ext == "css")
    }

    pub fn extension_or_empty(&self) -> &OsStr {
        self.fname.extension().unwrap_or_else(|| OsStr::new(""))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn new_with_existing_file_succeeds() {
        let file = NamedTempFile::new().unwrap();
        let entry = DataEntry::new(file.path(), vec![1, 2, 3]).unwrap();
        assert_eq!(entry.bytes, vec![1, 2, 3]);
        assert_eq!(entry.fname, file.path());
    }

    #[test]
    fn new_with_missing_file_returns_err() {
        let result = DataEntry::new("/nonexistent/path/file.bin", vec![]);
        assert_eq!(
            result.unwrap_err().to_string(),
            "file does not exist: /nonexistent/path/file.bin"
        );
    }
}
