#![allow(clippy::match_same_arms)] // TODO: for now...

use crate::{
    formats::yomitan::{
        TermBankEntry,
        model::{TagBankEntry, YomitanDefinition},
    },
    glossary::entry::conversion,
};

pub use conversion::HtmlConverter;

// Wrapping seems better than having definition as a different attrribute of Entry
// in order to force every caller to think about what they are adding/consuming to the glossary.
#[derive(Clone, Debug)]
pub enum Definition {
    // There should be another "Raw" variant, which is just text but we guarantee
    // that every conversion to it is trivial. This should fix the case where sometimes
    // we want html > text via tag removal, and sometimes html > text via identity
    // (just print the html) for debugging purposes etc.
    // At the moment Text *is* the "Raw" variant.
    Text(String),               // m (the default when we don't know)
    Html(String),               // h
    Yomitan(YomitanDefinition), // TODO: use box here: size too big
}

// TODO: better than cow would be to pass by value, which makes sense since they are
// "into" consuming versions. It does require some sort of consuming iteration over a Glossary.
impl Definition {
    // TODO: use cow, don't clone
    pub fn to_text(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            // Intentionally lossy. We don't want html in a yomitan definition.
            // Self::Html(s) => strip_html(s),
            Self::Html(s) => s.clone(),
            Self::Yomitan(def) => match def {
                YomitanDefinition::TermBankEntry(term_bank_entry) => term_bank_entry
                    .definitions
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n"),
                YomitanDefinition::TermMetaBankEntry(_term_meta_bank_entry) => {
                    String::from("unimplemented to_text for TermMetaBankEntry")
                }
            },
        }
    }

    // TODO: use cow, don't clone
    // Only call this via HtmlConverter
    pub fn to_html(&self, tag_bank: Option<&[TagBankEntry]>) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::Html(s) => s.clone(),
            Self::Yomitan(def) => match def {
                YomitanDefinition::TermBankEntry(term_bank_entry) => {
                    // Here we can get rid of the option since we know we are
                    // dealing with Yomitan.
                    term_bank_entry.to_html(tag_bank.unwrap_or_default())
                }
                YomitanDefinition::TermMetaBankEntry(term_meta_bank_entry) => {
                    term_meta_bank_entry.to_html() // TODO: use tags
                }
            },
        }
    }

    // We require access to the term to create a TermInformation
    pub fn to_yomitan(&self, term: &str) -> YomitanDefinition {
        debug_assert!(!term.is_empty());
        match self {
            Self::Text(s) => {
                YomitanDefinition::TermBankEntry(TermBankEntry::raw(term.to_string(), s.clone()))
            }
            Self::Html(s) => YomitanDefinition::TermBankEntry(TermBankEntry {
                term: term.to_string(),
                definitions: vec![conversion::html_to_structured_content(s)],
                ..Default::default()
            }),
            Self::Yomitan(defs) => defs.clone(),
        }
    }
}
