use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

/// A data entry.
///
/// Note: we cannot enforce a "file must exist" invariant here because
/// entries can be constructed from in-memory sources (e.g. zip archives)
/// that have no corresponding path on disk.
#[derive(Debug, PartialEq)]
pub struct DataEntry {
    fname: PathBuf,
    bytes: Vec<u8>,
}

impl DataEntry {
    pub fn new<P: Into<PathBuf>>(fname: P, bytes: Vec<u8>) -> Self {
        let fname = fname.into();
        if !fname.exists() {
            tracing::warn!(fname = %fname.display(), "file does not exist");
        }
        Self { fname, bytes }
    }

    pub fn fname(&self) -> &Path {
        &self.fname
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
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
        let entry = DataEntry::new(file.path(), vec![1, 2, 3]);
        assert_eq!(entry.bytes, vec![1, 2, 3]);
        assert_eq!(entry.fname, file.path());
    }
}
