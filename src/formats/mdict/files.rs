use anyhow::Result;

use crate::scan::{DictionaryFiles, has_extension, with_extension};

pub struct MdictFiles {
    pub mdx: String,
    pub mdd: Vec<String>,
    pub css: Vec<String>,
}

impl DictionaryFiles for MdictFiles {
    const MAIN: &'static str = "*.mdx";

    fn find(mdx: &str, names: &[String]) -> Result<Self> {
        Ok(Self {
            mdx: mdx.to_string(),
            mdd: mdd_volumes(mdx, names),
            css: css_files(mdx, names),
        })
    }
}

/// `foo.mdd`, then `foo.1.mdd`, `foo.2.mdd`... until one is missing, as goldendict-ng does.
fn mdd_volumes(mdx: &str, names: &[String]) -> Vec<String> {
    let mut volumes = Vec::new();
    let mut name = with_extension(mdx, "mdd");
    while names.contains(&name) {
        volumes.push(name);
        name = with_extension(mdx, &format!("{}.mdd", volumes.len()));
    }
    volumes
}

/// goldendict-ng serves any css beside the .mdx that an entry links, so take them all,
/// unless other dictionaries share the folder: then only `foo.css` is surely ours.
fn css_files(mdx: &str, names: &[String]) -> Vec<String> {
    let alone = !names
        .iter()
        .any(|name| has_extension(name, "mdx") && name != mdx);
    let own = with_extension(mdx, "css");
    let mut css: Vec<_> = names
        .iter()
        .filter(|name| has_extension(name, "css") && (alone || **name == own))
        .cloned()
        .collect();
    css.sort();
    css
}
