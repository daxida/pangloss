mod definition;
pub use definition::{Definition, HtmlConverter};

mod data_entry;
pub use data_entry::DataEntry;

mod conversion;

use crate::glossary::AltEntry;

pub const TERM_SEPARATOR: char = '|';

/// An entry. That is, a (string) headword and a [`Definition`].
///
/// A [`Glossary`](crate::Glossary) is composed of these, and some metadata.
//
// Right now there is a broken assumption: that the term/definition pair is
// separated from alts, and can not coincide. Unfortunately, the yomitan format
// is rich enough to give alt information via the term/definition.
//
// Note that the fields are not public because I haven't decided yet on the internals.
// Mainly, whether to store the term as a String or some other type. This results
// in a less ergonomic API.
#[derive(Clone, PartialEq, Debug)]
pub struct Entry {
    term: String,
    definition: Definition,
    alts: Vec<AltEntry>,
}

impl Entry {
    pub const fn new(term: String, definition: Definition) -> Self {
        Self {
            term,
            definition,
            alts: Vec::new(),
        }
    }

    pub const fn with_html(term: String, definition: String) -> Self {
        Self {
            term,
            definition: Definition::Html(definition),
            alts: Vec::new(),
        }
    }

    pub fn term(&self) -> &str {
        &self.term
    }

    pub const fn term_mut(&mut self) -> &mut String {
        &mut self.term
    }

    pub const fn definition(&self) -> &Definition {
        &self.definition
    }

    pub const fn definition_mut(&mut self) -> &mut Definition {
        &mut self.definition
    }

    pub fn alts(&self) -> &[AltEntry] {
        &self.alts
    }

    pub const fn alts_mut(&mut self) -> &mut Vec<AltEntry> {
        &mut self.alts
    }

    // [L]ist terms and [S]tring terms
    // https://github.com/ilius/pyglossary/blob/master/pyglossary/entry.py#L266
    fn l_terms(&self) -> Vec<String> {
        std::iter::once(self.term.clone())
            .chain(self.alts.iter().map(|alt| alt.term().to_string()))
            .collect()
    }

    pub fn s_terms(&self) -> String {
        self.l_terms().join(&TERM_SEPARATOR.to_string())
    }

    // Binary methods
    pub const fn b_term(&self) -> &[u8] {
        self.term.as_bytes()
    }

    pub fn b_alts(&self) -> Vec<Vec<u8>> {
        self.alts
            .iter()
            .map(|alt| alt.term().as_bytes().to_vec())
            .collect()
    }

    // Some modifiers (return new owned Entry, like C# With*)
    pub fn with_term(self, term: String) -> Self {
        Self { term, ..self }
    }

    pub fn with_definition(self, definition: Definition) -> Self {
        Self { definition, ..self }
    }

    pub fn with_alts(self, alts: Vec<AltEntry>) -> Self {
        Self { alts, ..self }
    }
}
