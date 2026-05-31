//! Yomitan tag bank data model.
//!
//! Ported from the typescript [yomitan-dict-builder] library.
//!
//! [yomitan-dict-builder]: https://github.com/MarvNC/yomichan-dict-builder/blob/master/src/types/yomitan/tagbank.ts

use serde::{Deserialize, Serialize, Serializer, ser::SerializeTuple};

pub type TagBank = Vec<TagBankEntry>;

/// A tag. Attribute names come from [wty].
///
/// [wty]: https://github.com/yomidevs/wiktionary-to-yomitan/blob/master/src/models/yomitan.rs
#[derive(Debug, PartialEq, Clone, Deserialize)]
pub struct TagBankEntry {
    pub short_tag: String, // tagName
    pub category: String,  // category
    pub sort_order: i32,   // sortingOrder
    pub long_tag: String,  // notes (only this changes)
    pub popularity_score: i32,
}

impl Serialize for TagBankEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tup = serializer.serialize_tuple(5)?;
        tup.serialize_element(&self.short_tag)?;
        tup.serialize_element(&self.category)?;
        tup.serialize_element(&self.sort_order)?;
        tup.serialize_element(&self.long_tag)?;
        tup.serialize_element(&self.popularity_score)?;
        tup.end()
    }
}
