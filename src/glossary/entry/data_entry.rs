use std::{ffi::OsStr, path::PathBuf};

/// A data entry.
#[derive(Debug)]
pub struct DataEntry {
    pub fname: PathBuf,
    pub bytes: Vec<u8>,
}

impl DataEntry {
    pub fn new<P: Into<PathBuf>>(fname: P, bytes: Vec<u8>) -> Self {
        let fname = fname.into();
        debug_assert!(fname.exists());
        Self { fname, bytes }
    }

    pub fn extension(&self) -> Option<&OsStr> {
        self.fname.extension()
    }

    pub fn extension_or_empty(&self) -> &OsStr {
        self.fname.extension().unwrap_or_else(|| OsStr::new(""))
    }
}
