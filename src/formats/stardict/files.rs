use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::scan::{DictionaryFiles, single_file};

pub struct StardictFiles {
    pub ifo: PathBuf,
    pub idx: PathBuf,
    pub dict: PathBuf,
    pub syn: Option<PathBuf>,
    // Can there be more than one?
    pub css: Option<PathBuf>,
}

impl DictionaryFiles for StardictFiles {
    const MAIN_EXTENSION: &'static str = "ifo";

    fn find(ifo: &Path) -> Result<Self> {
        Ok(Self {
            ifo: ifo.to_path_buf(),
            idx: single_file(ifo, &["*.idx", "*.idx.dz", "*.idx.gz"])?,
            dict: single_file(ifo, &["*.dict", "*.dict.dz"])?,
            syn: single_file(ifo, &["*.syn", "*.syn.dz", "*.syn.gz"]).ok(),
            css: single_file(ifo, &["*.css"]).ok(),
        })
    }
}
