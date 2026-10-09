use anyhow::Result;

use crate::scan::DictionaryFiles;

const ABBREVIATIONS: &str = "_abrv";

/// Every file is named after the .dsl, as goldendict-ng expects.
pub struct DslFiles {
    /// `foo.dsl`, or `foo.dsl.dz` when dictzipped.
    pub dsl: String,
    /// `foo.ann`: a description of the dictionary.
    pub ann: Option<String>,
    /// `foo.dsl.files.zip`: the media that cards show.
    pub resources: Option<String>,
}

impl DslFiles {
    /// `foo` for `dir/foo.dsl`.
    pub fn name(&self) -> &str {
        let stem = stem(&self.dsl).unwrap_or(&self.dsl);
        stem.rsplit('/').next().unwrap_or(stem)
    }
}

impl DictionaryFiles for DslFiles {
    const MAIN: &'static str = "*.dsl";

    /// `foo_abrv.dsl` is not a dictionary but the labels of `foo.dsl`.
    fn is_main(name: &str) -> bool {
        stem(name).is_some_and(|stem| !stem.ends_with(ABBREVIATIONS))
    }

    fn find(dsl: &str, names: &[String]) -> Result<Self> {
        let stem = stem(dsl).unwrap_or(dsl);
        let present = |name: String| names.contains(&name).then_some(name);
        Ok(Self {
            dsl: dsl.to_string(),
            ann: present(format!("{stem}.ann")),
            resources: present(format!("{stem}.dsl.files.zip")),
        })
    }
}

/// `foo` for `foo.dsl` or `foo.dsl.dz`.
fn stem(name: &str) -> Option<&str> {
    let lower = name.to_ascii_lowercase();
    [".dsl", ".dsl.dz"]
        .iter()
        .find(|ext| lower.ends_with(*ext))
        .map(|ext| &name[..name.len() - ext.len()])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mains() {
        assert!(DslFiles::is_main("dict.dsl"));
        assert!(DslFiles::is_main("dir/dict.dsl.dz"));
        assert!(DslFiles::is_main("DICT.DSL"));
        assert!(!DslFiles::is_main("dict_abrv.dsl"));
        assert!(!DslFiles::is_main("dict.dsl.files.zip"));
        assert!(!DslFiles::is_main("dict.ann"));
    }

    #[test]
    fn companions() {
        let names: Vec<String> = ["d/x.dsl.dz", "d/x.ann", "d/x.dsl.files.zip", "d/x_abrv.dsl"]
            .map(String::from)
            .into();
        let files = DslFiles::find("d/x.dsl.dz", &names).unwrap();
        assert_eq!(files.ann.as_deref(), Some("d/x.ann"));
        assert_eq!(files.resources.as_deref(), Some("d/x.dsl.files.zip"));
    }
}
