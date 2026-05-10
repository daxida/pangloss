mod entry;
pub use entry::{DataEntry, Definition, Entry, HtmlConverter, TERM_SEPARATOR};

mod alt_map;
pub use alt_map::{AltEntry, AltMap};

mod glossary_info;
pub use glossary_info::GlossaryInfo;

mod metadata;
pub use metadata::GlossaryMetadata;

/// The universal intermediary data format passed between readers and writers.
///
/// All format-specific readers produce one, and all writers consume one.
#[derive(Debug, Default)]
pub struct Glossary {
    pub entries: Vec<Entry>,
    pub data_entries: Vec<DataEntry>,
    pub alt_map: AltMap,
    pub info: GlossaryInfo,
    pub metadata: GlossaryMetadata,
}

impl Glossary {
    pub fn css_files(&self) -> impl Iterator<Item = &DataEntry> {
        self.data_entries.iter().filter(|dentry| dentry.is_css())
    }

    pub fn diagnostics(&self) {
        tracing::info!(
            "Found {} entries, {} data_entries, and {} alts",
            self.entries.len(),
            self.data_entries.len(),
            self.alt_map.values().map(Vec::len).sum::<usize>()
        );
    }
}
