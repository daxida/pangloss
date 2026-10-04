use anyhow::{Context, Result};

use crate::scan::{DictionaryFiles, companion};

/// Every file is named after the .ifo, as koreader expects.
pub struct StardictFiles {
    pub ifo: String,
    pub idx: String,
    pub dict: String,
    pub syn: Option<String>,
    pub css: Option<String>,
}

impl DictionaryFiles for StardictFiles {
    const MAIN_EXTENSION: &'static str = "ifo";

    fn find(ifo: &str, names: &[String]) -> Result<Self> {
        let required = |suffixes: &[&str]| {
            companion(ifo, names, suffixes)
                .with_context(|| format!("No .{} found for {ifo}", suffixes[0]))
        };
        Ok(Self {
            ifo: ifo.to_string(),
            idx: required(&["idx", "idx.gz", "idx.dz"])?,
            dict: required(&["dict", "dict.dz"])?,
            syn: companion(ifo, names, &["syn", "syn.gz", "syn.dz"]),
            css: companion(ifo, names, &["css"]),
        })
    }
}
