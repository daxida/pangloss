//! Map from term to alts, with possibly extra information of the relation.
//!
//! Compare to pyglossary, which stores alts directly on the Entry. Their
//! approach is fundamentally lossy, and can only work with simple term/alts
//! pair. It can not deal with formats like Yomitan that contain extra information
//! about the pair (i.e. the causal chain).
//!
//! Does not reuse the [`crate::entry::Entry`] type since, here, we expect MOST of
//! the definitions to be None (i.e. the most common case is a simple term/alts pair),
//! while an Entry without definition is a pathological case.

use indexmap::IndexMap;

use crate::glossary::Definition;

// IndexMap for reproducibility, not really needed
pub type AltMap = IndexMap<String, Vec<AltEntry>>;

#[derive(Clone, Debug)]
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
