use std::collections::HashSet;

use crate::{
    Glossary,
    glossary::{Definition, GlossaryInfo},
};

// https://code.google.com/archive/p/babiloo/wikis/StarDict_format.wiki
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SameTypeSequence {
    Html, // h
    Text, // m
    None, // ""
}

impl SameTypeSequence {
    pub const fn to_format_string(self) -> &'static str {
        match self {
            Self::Html => "h",
            Self::Text => "m",
            Self::None => "",
        }
    }

    pub fn from_opt_str(s: Option<&str>) -> Self {
        match s {
            Some("h") => Self::Html,
            Some("m") => Self::Text,
            Some("") | None => Self::None,
            Some(other) => {
                tracing::warn!("Unknown sametypesequence {other}. Defaulting to none.");
                Self::None
            }
        }
    }

    // Try the metadata and then fallback to a quick glossary scan
    pub fn from_glossary(glossary: &Glossary) -> Self {
        match Self::from_info(&glossary.info) {
            SameTypeSequence::None => {
                tracing::info!("No sts in info. Detecting from entries.");
                let kinds: HashSet<Self> = glossary
                    .entries
                    .iter()
                    .map(|e| match e.definition() {
                        Definition::Text(_) => Self::Text,
                        Definition::Html(_) => Self::Html,
                        Definition::Yomitan(_) => unreachable!(),
                    })
                    .collect();
                if kinds.len() == 1 {
                    kinds.into_iter().next().unwrap()
                } else {
                    SameTypeSequence::None
                }
            }
            other => other,
        }
    }

    // Try the metadata: this is our only choice when reading an .ifo file
    // since at that point we don't have any definition in memory.
    pub fn from_info(info: &GlossaryInfo) -> Self {
        Self::from_opt_str(info.get("sametypesequence"))
    }

    pub const fn as_definition(self, s: String) -> Definition {
        match self {
            Self::Html => Definition::Html(s),
            _ => Definition::Text(s),
        }
    }
}
