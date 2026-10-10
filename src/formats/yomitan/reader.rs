//! Reader for [Yomitan](https://github.com/yomidevs/yomitan) dictionary archives.

use std::{collections::HashMap, path::Path};

use anyhow::{Result, bail};
use indexmap::{IndexMap, IndexSet};
use rayon::prelude::*;
use serde_json::Value;

use crate::{
    Context, Reader,
    formats::yomitan::{
        YomitanFormat,
        files::YomitanFiles,
        model::{TagBank, TermBank, TermBankEntry, TermMetaBank, YomitanDefinition},
    },
    glossary::{AltEntry, DataEntry, Definition, Entry, Glossary, GlossaryInfo, GlossaryMetadata},
    scan::DictionaryFiles,
};

impl Reader for YomitanFormat {
    fn read_with_context(&self, path: &Path, ctx: &Context) -> Result<Glossary> {
        read_with_context(path, ctx)
    }
}

fn read_with_context(path: &Path, _: &Context) -> Result<Glossary> {
    let (mut source, files) = YomitanFiles::scan(path)?;
    let info = parse_index_file(&source.read(&files.index)?)?;
    let mut read_all = |names: &[String]| -> Result<Vec<(String, Vec<u8>)>> {
        names
            .iter()
            .map(|name| Ok((name.clone(), source.read(name)?)))
            .collect()
    };

    let (mut entries, inflections) = read_term_banks(&read_all(&files.term_banks)?)?;
    attach_inflections(&mut entries, inflections);
    let term_meta_bank = read_term_meta_banks(&read_all(&files.term_meta_banks)?)?;
    let tag_bank = read_tag_banks(&read_all(&files.tag_banks)?)?;

    tracing::debug!("Found {} term meta bank entries", term_meta_bank.len());
    tracing::debug!("Found {} tag bank entries", tag_bank.len());

    for term_meta_bank_entry in term_meta_bank {
        entries.push(Entry::new(
            term_meta_bank_entry.term().clone(),
            Definition::from(YomitanDefinition::TermMetaBankEntry(term_meta_bank_entry)),
        ));
    }

    let metadata = GlossaryMetadata {
        tag_bank: Some(tag_bank),
        ..Default::default()
    };

    let mut media = Vec::from_iter(files.styles.as_deref());
    for entry in &entries {
        if let Definition::Yomitan(def) = entry.definition()
            && let YomitanDefinition::TermBankEntry(entry) = def.as_ref()
        {
            for definition in &entry.definitions {
                definition.image_paths(&mut media);
            }
        }
    }
    let media: IndexSet<&str> = media.into_iter().collect();
    tracing::debug!("Found {} media files", media.len());

    let mut data_entries = Vec::new();
    for path in media {
        match source.read(path) {
            Ok(bytes) => data_entries.push(DataEntry::new(path, bytes)),
            Err(err) => tracing::warn!("Missing image {path}: {err}"),
        }
    }

    Ok(Glossary {
        entries,
        data_entries,
        info,
        metadata,
    })
}

fn parse_index_file(json: &[u8]) -> Result<GlossaryInfo> {
    let index: IndexMap<String, Value> = serde_json::from_slice(json)?;

    let version = index
        .get("version")
        .or_else(|| index.get("format"))
        .and_then(Value::as_i64);
    match version {
        None => bail!("Missing 'version' or 'format' field in index.json"),
        Some(3) => {}
        Some(v) => bail!("Unsupported Yomitan version {v}, only version 3 is supported"),
    }

    let mut info = GlossaryInfo::new();
    for (key, value) in index {
        let value_str = match value {
            Value::String(s) => s,
            other => other.to_string(),
        };
        info.insert(&key, value_str);
    }
    Ok(info)
}

/// An inflection and the term it inflects from, before it finds its entry.
type Inflections = Vec<(String, TermBankEntry)>;

fn read_term_bank(
    json: &[u8],
    entries: &mut Vec<Entry>,
    inflections: &mut Inflections,
) -> Result<()> {
    // This can fail if our logic doesn't cover the full schema
    let term_bank: TermBank = serde_json::from_slice(json)?;

    // Unfortunately there is no way to maintain the order: it does
    // not matter for the dictionary but it makes testing harder.
    for term_bank_entry in term_bank {
        if let Some(source) = term_bank_entry.inflection_source() {
            inflections.push((source.to_string(), term_bank_entry));
        } else {
            entries.push(Entry::new(
                term_bank_entry.term.clone(),
                Definition::from(YomitanDefinition::TermBankEntry(term_bank_entry)),
            ));
        }
    }

    Ok(())
}

/// Hand every inflection to the entry it inflects from.
///
/// The entry's own term is the *inflected* form; the term it inflects from is
/// inside the definition. A headword can appear on more than one entry and the
/// format cannot say which is meant, so the first one takes them.
fn attach_inflections(entries: &mut Vec<Entry>, inflections: Inflections) {
    // Resolve every source to an index first
    let targets: Vec<Option<usize>> = {
        let mut first_by_term: HashMap<&str, usize> = HashMap::new();
        for (index, entry) in entries.iter().enumerate() {
            first_by_term.entry(entry.term()).or_insert(index);
        }
        inflections
            .iter()
            .map(|(source, _)| first_by_term.get(source.as_str()).copied())
            .collect()
    };

    let mut orphans = Vec::new();
    for (target, (_, term_bank_entry)) in targets.into_iter().zip(inflections) {
        let term = term_bank_entry.term.clone();
        let definition = Definition::from(YomitanDefinition::TermBankEntry(term_bank_entry));
        match target {
            Some(index) => entries[index]
                .alts_mut()
                .push(AltEntry::new(term, definition)),
            None => orphans.push(Entry::new(term, definition)),
        }
    }
    entries.extend(orphans);
}

fn read_term_banks(term_banks: &[(String, Vec<u8>)]) -> Result<(Vec<Entry>, Inflections)> {
    Ok(term_banks
        .par_iter()
        .map(|(_, bytes)| {
            let mut entries = Vec::new();
            let mut inflections = Inflections::new();
            read_term_bank(bytes, &mut entries, &mut inflections)?;
            Ok((entries, inflections))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .fold(
            (Vec::new(), Inflections::new()),
            |(mut entries, mut inflections), (e, i)| {
                entries.extend(e);
                inflections.extend(i);
                (entries, inflections)
            },
        ))
}

// Read all <T> banks into a single one.
//
// The simple version, for when we don't need to separate data.
fn read_banks<T>(banks: &[(String, Vec<u8>)]) -> Result<T>
where
    T: Send + for<'de> serde::Deserialize<'de> + IntoIterator + FromIterator<T::Item>,
    T::Item: Send,
{
    Ok(banks
        .par_iter()
        .map(|(_, bytes)| Ok(serde_json::from_slice::<T>(bytes)?))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<T>())
}

fn read_term_meta_banks(term_meta_banks: &[(String, Vec<u8>)]) -> Result<TermMetaBank> {
    read_banks::<TermMetaBank>(term_meta_banks)
}

fn read_tag_banks(tag_banks: &[(String, Vec<u8>)]) -> Result<TagBank> {
    read_banks::<TagBank>(tag_banks)
}

#[cfg(test)]
mod tests {
    use super::*;

    // A lemma, and an inflection of it. Note where the two terms sit:
    //
    //   ["nieves", ..., [["nieve", ["plural"]]], ...]
    //     ^^^^^^                    the inflected form, as the entry's own term
    //                      ^^^^^    the term it inflects from, inside the definition
    //
    // so the alt has to land on the "nieve" entry, not on a "nieves" one, or it
    // never reaches the entry it belongs to.
    const BANK: &str = r#"[
        ["nieve",  "", "", "",  0, ["snow"],                0, ""],
        ["nieves", "", "", "n", 0, [["nieve", ["plural"]]], 0, ""]
    ]"#;

    #[test]
    fn an_inflection_is_filed_under_the_term_it_inflects_from() {
        let mut entries = Vec::new();
        let mut inflections = Inflections::new();
        read_term_bank(BANK.as_bytes(), &mut entries, &mut inflections).unwrap();
        attach_inflections(&mut entries, inflections);

        // Only the lemma is an entry; the inflection became one of its alts.
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].term(), "nieve");

        let alts: Vec<_> = entries[0].alts().iter().map(AltEntry::term).collect();
        assert_eq!(alts, ["nieves"]);

        // Which is the whole point: the alt comes back out of its entry.
        assert_eq!(entries[0].s_terms(), "nieve|nieves");
    }
}
