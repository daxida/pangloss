mod entry;
pub use entry::{DataEntry, Definition, Entry, HtmlConverter, TERM_SEPARATOR};

mod alt;
pub use alt::AltEntry;

mod glossary_info;
pub use glossary_info::GlossaryInfo;

mod metadata;
pub use metadata::GlossaryMetadata;

/// The universal intermediary data format passed between readers and writers.
///
/// All format-specific readers produce one, and all writers consume one.
#[derive(Debug, PartialEq, Default)]
pub struct Glossary {
    pub entries: Vec<Entry>,
    pub data_entries: Vec<DataEntry>,
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
            self.entries.iter().map(|e| e.alts().len()).sum::<usize>()
        );
    }
}
