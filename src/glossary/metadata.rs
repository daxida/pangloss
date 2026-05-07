use crate::formats::{mdict::StyleSheet, yomitan::TagBank};

#[derive(Debug, Default)]
pub struct GlossaryMetadata {
    // Yomitan definition tag metadata
    pub tag_bank: Option<TagBank>,
    // Mdict inline styles
    pub stylesheet: Option<StyleSheet>,
}
