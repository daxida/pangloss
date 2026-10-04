use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::scan::{DictionaryFiles, siblings};

pub struct MdictFiles {
    pub mdx: PathBuf,
    pub mdd: Vec<PathBuf>,
    pub css: Vec<PathBuf>,
}

impl DictionaryFiles for MdictFiles {
    const MAIN_EXTENSION: &'static str = "mdx";

    fn find(mdx: &Path) -> Result<Self> {
        let mut mdd = Vec::new();
        let mut css = Vec::new();
        for path in siblings(mdx)? {
            match path.extension().and_then(|e| e.to_str()) {
                Some("mdx") => (),
                Some("mdd") => mdd.push(path),
                Some("css") => css.push(path),
                _ => tracing::warn!("Ignoring unsupported file: {}", path.display()),
            }
        }
        Ok(Self {
            mdx: mdx.to_path_buf(),
            mdd,
            css,
        })
    }
}
