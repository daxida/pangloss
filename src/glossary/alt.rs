//! An alternative form of an entry's headword.
//!
//! Alts live on the [`Entry`](crate::Entry) itself, positionally, the way
//! stardict keys its .syn records by entry index. Keying them by term instead
//! cannot tell two entries sharing a headword apart, and they leak between them.
//!
//! Unlike pyglossary, which stores alts as bare strings, an alt can carry a
//! definition: formats like Yomitan hold extra information about the relation
//! (the causal chain) that a plain alias would lose.
//!
//! Does not reuse the [`Entry`](crate::Entry) type since, here, we expect MOST of
//! the definitions to be None (i.e. the most common case is a simple term/alts pair),
//! while an Entry without definition is a pathological case.

use crate::glossary::Definition;

#[derive(Clone, PartialEq, Debug)]
pub struct AltEntry {
    term: String,
    definition: Option<Definition>,
}

impl AltEntry {
    pub const fn new(term: String, definition: Definition) -> Self {
        Self {
            term,
            definition: Some(definition),
        }
    }

    pub const fn only_term(term: String) -> Self {
        Self {
            term,
            definition: None,
        }
    }

    pub fn term(&self) -> &str {
        &self.term
    }

    pub const fn definition(&self) -> Option<&Definition> {
        self.definition.as_ref()
    }
}
