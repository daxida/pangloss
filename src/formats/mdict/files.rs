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
        Ok(Self {
            mdx: mdx.to_path_buf(),
            mdd: mdd_volumes(mdx),
            css: css_files(mdx)?,
        })
    }
}

/// `foo.mdd`, then `foo.1.mdd`, `foo.2.mdd`... until one is missing, as goldendict-ng does.
fn mdd_volumes(mdx: &Path) -> Vec<PathBuf> {
    let mut volumes = Vec::new();
    let mut path = mdx.with_extension("mdd");
    while path.is_file() {
        volumes.push(path);
        path = mdx.with_extension(format!("{}.mdd", volumes.len()));
    }
    volumes
}

/// goldendict-ng serves any css beside the .mdx that an entry links, so take them all,
/// unless other dictionaries share the folder: then only `foo.css` is surely ours.
fn css_files(mdx: &Path) -> Result<Vec<PathBuf>> {
    let siblings = siblings(mdx)?;
    let is = |path: &Path, ext: &str| path.extension().is_some_and(|e| e == ext);
    let alone = !siblings.iter().any(|path| is(path, "mdx") && path != mdx);
    let own = mdx.with_extension("css");
    let mut css: Vec<_> = siblings
        .into_iter()
        .filter(|path| is(path, "css") && (alone || *path == own))
        .collect();
    css.sort();
    Ok(css)
}
