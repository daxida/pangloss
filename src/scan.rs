//! Finding the files of multi-file dictionaries (Stardict, Mdict).

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Result, bail};

use crate::utils::parent_dir;

pub trait DictionaryFiles: Sized {
    /// The extension of the file the user points at, f.e. "ifo".
    const MAIN_EXTENSION: &'static str;

    /// Fails if a required file is missing.
    fn find(main: &Path) -> Result<Self>;

    fn scan(main: &Path) -> Result<Self> {
        if main
            .extension()
            .is_none_or(|ext| !ext.eq_ignore_ascii_case(Self::MAIN_EXTENSION))
        {
            bail!(
                "Expected a file with .{} extension but got {}",
                Self::MAIN_EXTENSION,
                main.display()
            );
        }
        Self::find(main)
    }
}

/// The first existing `foo.<suffix>` beside `foo.<ext>`.
pub fn companion(main: &Path, suffixes: &[&str]) -> Option<PathBuf> {
    suffixes
        .iter()
        .map(|suffix| main.with_extension(suffix))
        .find(|path| path.is_file())
}

/// Every file beside `main`, unordered, spelled like `main` so they compare with it.
pub fn siblings(main: &Path) -> Result<Vec<PathBuf>> {
    fs::read_dir(parent_dir(main))?
        .map(|entry| Ok(main.with_file_name(entry?.file_name())))
        .collect()
}
