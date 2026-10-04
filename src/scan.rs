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
            .is_none_or(|ext| ext != Self::MAIN_EXTENSION)
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

/// The only file beside `main` matching one of the glob `patterns`.
pub fn single_file(main: &Path, patterns: &[&str]) -> Result<PathBuf> {
    let dir = parent_dir(main);
    let mut matches = Vec::new();
    for pattern in patterns {
        for entry in glob::glob(&format!("{}/{pattern}", dir.display()))? {
            matches.push(entry?);
        }
    }
    if matches.len() != 1 {
        let (patterns, dir, n) = (patterns.join("/"), dir.display(), matches.len());
        bail!("Expected exactly one {patterns} file in {dir}, found {n}");
    }
    Ok(matches.remove(0))
}

/// Every file beside `main`, unordered.
pub fn siblings(main: &Path) -> Result<Vec<PathBuf>> {
    fs::read_dir(parent_dir(main))?
        .map(|entry| Ok(entry?.path()))
        .collect()
}
