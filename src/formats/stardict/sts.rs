use crate::glossary::{Definition, GlossaryInfo};

// https://code.google.com/archive/p/babiloo/wikis/StarDict_format.wiki
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

    pub fn detect_from_info(info: &GlossaryInfo) -> Self {
        // We only trust the metadata: we won't go scanning definitions to see if the user
        // forgot to add the information in the .ifo file...
        Self::from_opt_str(info.get("sametypesequence"))
    }

    pub const fn as_definition(self, s: String) -> Definition {
        match self {
            Self::Html => Definition::Html(s),
            _ => Definition::Text(s),
        }
    }
}
