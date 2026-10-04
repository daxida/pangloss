use std::sync::LazyLock;

use anyhow::Result;
use regex::Regex;

use crate::scan::DictionaryFiles;

static BANK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(term|term_meta|tag|kanji|kanji_meta)_bank_(\d+)\.json$").unwrap()
});

/// Banks are sorted by their number.
pub struct YomitanFiles {
    pub index: String,
    pub term_banks: Vec<String>,
    pub term_meta_banks: Vec<String>,
    pub tag_banks: Vec<String>,
    /// Everything that is not json, `styles.css` included.
    pub media: Vec<String>,
}

impl DictionaryFiles for YomitanFiles {
    const MAIN: &'static str = "index.json";

    fn find(index: &str, names: &[String]) -> Result<Self> {
        let mut term_banks = Vec::new();
        let mut term_meta_banks = Vec::new();
        let mut tag_banks = Vec::new();
        let mut media = Vec::new();

        for name in names {
            if let Some(captures) = BANK_RE.captures(name) {
                let n = captures[2].parse::<u32>()?;
                match &captures[1] {
                    "term" => term_banks.push((n, name.clone())),
                    "term_meta" => term_meta_banks.push((n, name.clone())),
                    "tag" => tag_banks.push((n, name.clone())),
                    _ => tracing::warn!("Unsupported kanji file: {name}"),
                }
            } else if name.ends_with("json") {
                if name != index {
                    tracing::warn!("Unrecognized json file: {name}");
                }
            } else {
                media.push(name.clone());
            }
        }

        Ok(Self {
            index: index.to_string(),
            term_banks: sorted(term_banks),
            term_meta_banks: sorted(term_meta_banks),
            tag_banks: sorted(tag_banks),
            media,
        })
    }
}

fn sorted(mut banks: Vec<(u32, String)>) -> Vec<String> {
    banks.sort();
    banks.into_iter().map(|(_, name)| name).collect()
}
