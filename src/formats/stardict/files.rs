use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::scan::{DictionaryFiles, companion};

/// Every file is named after the .ifo, as koreader expects.
pub struct StardictFiles {
    pub ifo: PathBuf,
    pub idx: PathBuf,
    pub dict: PathBuf,
    pub syn: Option<PathBuf>,
    pub css: Option<PathBuf>,
}

impl DictionaryFiles for StardictFiles {
    const MAIN_EXTENSION: &'static str = "ifo";

    fn find(ifo: &Path) -> Result<Self> {
        let required = |suffixes: &[&str]| {
            companion(ifo, suffixes)
                .with_context(|| format!("No .{} found for {}", suffixes[0], ifo.display()))
        };
        Ok(Self {
            ifo: ifo.to_path_buf(),
            idx: required(&["idx", "idx.gz", "idx.dz"])?,
            dict: required(&["dict", "dict.dz"])?,
            syn: companion(ifo, &["syn", "syn.gz", "syn.dz"]),
            css: companion(ifo, &["css"]),
        })
    }
}
