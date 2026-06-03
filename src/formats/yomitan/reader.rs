//! Reader for [Yomitan](https://github.com/yomidevs/yomitan) dictionary archives.

use std::{fs::File, io::Read, path::Path, sync::LazyLock};

use anyhow::{Context as _, Result, bail};
use indexmap::IndexMap;
use rayon::prelude::*;
use regex::Regex;
use serde_json::Value;
use zip::ZipArchive;

use crate::{
    Context, Reader,
    formats::yomitan::{
        YomitanFormat,
        model::{TagBank, TermBank, TermMetaBank, YomitanDefinition},
    },
    glossary::{
        AltEntry, AltMap, DataEntry, Definition, Entry, Glossary, GlossaryInfo, GlossaryMetadata,
    },
};

static TERM_BANK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^term_bank_(\d+)\.json$").unwrap());
static TERM_META_BANK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^term_meta_bank_(\d+)\.json$").unwrap());
static TAG_BANK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^tag_bank_(\d+)\.json$").unwrap());
static KANJI_BANK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^kanji_bank_(\d+)\.json$").unwrap());
static KANJI_META_BANK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^kanji_meta_bank_(\d+)\.json$").unwrap());

// note that kanji_banks and kanji_meta_banks are skipped
struct ZipContents {
    // list of names
    term_banks: Vec<String>,
    term_meta_banks: Vec<String>,
    tag_banks: Vec<String>,
    // bytes of index.json
    index: Vec<u8>,
    // media file names (including styles.css etc.) and their bytes
    media: Vec<(String, Vec<u8>)>,
}

impl Reader for YomitanFormat {
    fn read_with_context(&self, path: &Path, ctx: &Context) -> Result<Glossary> {
        read_with_context(path, ctx)
    }
}

fn read_with_context(path: &Path, _: &Context) -> Result<Glossary> {
    let file = File::open(path)?;
    let mut zip = ZipArchive::new(file)?;

    let zip_contents = collect_zip_contents(&mut zip)?;
    let info = parse_index_file(&zip_contents.index)?;

    if zip_contents.media.len() > 0 {
        tracing::debug!("Found {} media files", zip_contents.media.len());
    }

    let (mut entries, alt_map) = read_term_banks(&mut zip, &zip_contents.term_banks)?;
    let term_meta_bank = read_term_meta_banks(&mut zip, &zip_contents.term_meta_banks)?;
    tracing::debug!("Found {} term meta bank entries", term_meta_bank.len());
    // TODO: This should be added to read_term_meta_banks fn
    for term_meta_bank_entry in term_meta_bank {
        entries.push(Entry::new(
            term_meta_bank_entry.term().clone(),
            Definition::Yomitan(YomitanDefinition::TermMetaBankEntry(term_meta_bank_entry)),
        ));
    }

    let tag_bank = read_tag_banks(&mut zip, &zip_contents.tag_banks)?;
    tracing::debug!("Found {} tag bank entries", tag_bank.len());
    let metadata = GlossaryMetadata {
        tag_bank: Some(tag_bank),
        ..Default::default()
    };

    let data_entries = zip_contents
        .media
        .into_iter()
        .map(|(fname, bytes)| DataEntry::new(fname, bytes))
        .collect();

    Ok(Glossary {
        entries,
        data_entries,
        alt_map,
        info,
        metadata,
    })
}

// For index and styles file, store the bytes in memory.
// For banks, collect the number (to sort them) and name.
fn collect_zip_contents(zip: &mut ZipArchive<File>) -> Result<ZipContents> {
    let mut term_banks = Vec::new();
    let mut term_meta_banks = Vec::new();
    let mut tag_banks = Vec::new();
    let mut media = Vec::new();

    let mut index = None;

    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        let name = file.name().to_string();
        let mut buf = Vec::new();

        if let Some(captures) = TERM_BANK_RE.captures(&name) {
            let n = captures.get(1).unwrap().as_str().parse::<u32>()?;
            term_banks.push((n, name));
        } else if let Some(captures) = TERM_META_BANK_RE.captures(&name) {
            let n = captures.get(1).unwrap().as_str().parse::<u32>()?;
            term_meta_banks.push((n, name));
        } else if let Some(captures) = TAG_BANK_RE.captures(&name) {
            let n = captures.get(1).unwrap().as_str().parse::<u32>()?;
            tag_banks.push((n, name));
        } else if KANJI_BANK_RE.captures(&name).is_some()
            || KANJI_META_BANK_RE.captures(&name).is_some()
        {
            tracing::warn!("Unsupported kanji file in zip: {name}");
        } else if name == "index.json" {
            buf.clear();
            file.read_to_end(&mut buf)?;
            index = Some(buf);
        } else if name.ends_with("json") {
            tracing::warn!("Unrecognized json file in zip: {name}");
        } else {
            if name == "styles.css" {
                tracing::debug!("Detected styles file: {name}");
            }
            buf.clear();
            file.read_to_end(&mut buf)?;
            media.push((name, buf));
        }
    }

    term_banks.sort_by_key(|(n, _)| *n);
    term_meta_banks.sort_by_key(|(n, _)| *n);
    tag_banks.sort_by_key(|(n, _)| *n);

    Ok(ZipContents {
        term_banks: term_banks.into_iter().map(|(_, name)| name).collect(),
        term_meta_banks: term_meta_banks.into_iter().map(|(_, name)| name).collect(),
        tag_banks: tag_banks.into_iter().map(|(_, name)| name).collect(),
        index: index.context("No index.json found in zip")?,
        media,
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

#[allow(unused)]
fn read_term_bank(json: &[u8], entries: &mut Vec<Entry>, alt_map: &mut AltMap) -> Result<()> {
    // This can fail if our logic doesn't cover the full schema
    let term_bank: TermBank = serde_json::from_slice(json)?;

    // Unfortunately there is no way to maintain the order: it does
    // not matter for the dictionary but it makes testing harder.
    for term_bank_entry in term_bank {
        // TODO: is_inflection is not perfect, it assumes that an
        // inflection is made from an homogeneous Vec<inflections>,
        // when in reality they could be mixed.
        if term_bank_entry.is_inflection() {
            alt_map
                .entry(term_bank_entry.term.clone())
                .or_default()
                .push(AltEntry::new(
                    term_bank_entry.term.clone(),
                    Definition::Yomitan(YomitanDefinition::TermBankEntry(term_bank_entry)),
                ));
        } else {
            entries.push(Entry::new(
                term_bank_entry.term.clone(),
                Definition::Yomitan(YomitanDefinition::TermBankEntry(term_bank_entry)),
            ));
        }
    }

    Ok(())
}

fn read_term_banks(
    zip: &mut ZipArchive<File>,
    term_banks: &[String],
) -> Result<(Vec<Entry>, AltMap)> {
    let banks_bytes: Vec<_> = term_banks
        .iter()
        .map(|name| {
            let mut entry = zip.by_name(name)?;
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            Ok(buf)
        })
        .collect::<Result<_>>()?;

    // Parse in parallel
    let result: (Vec<_>, AltMap) = banks_bytes
        .par_iter()
        .map(|bytes| {
            let mut entries = Vec::new();
            let mut alt_map = AltMap::new();
            read_term_bank(bytes, &mut entries, &mut alt_map)?;
            Ok((entries, alt_map))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .fold(
            (Vec::new(), AltMap::new()),
            |(mut entries, mut alt_map), (e, a)| {
                entries.extend(e);
                for (term, alts) in a {
                    alt_map.entry(term).or_default().extend(alts);
                }
                (entries, alt_map)
            },
        );

    Ok(result)
}

// Read all <T> banks into a single one.
//
// The simple version, for when we don't need to separate data.
fn read_banks<T>(zip: &mut ZipArchive<File>, names: &[String]) -> Result<T>
where
    T: Send + for<'de> serde::Deserialize<'de> + IntoIterator + FromIterator<T::Item>,
    T::Item: Send,
{
    let banks_bytes: Vec<_> = names
        .iter()
        .map(|name| {
            let mut entry = zip.by_name(name)?;
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            Ok(buf)
        })
        .collect::<Result<_>>()?;

    let result: T = banks_bytes
        .par_iter()
        .map(|bytes| Ok(serde_json::from_slice::<T>(bytes)?))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();
    Ok(result)
}

fn read_term_meta_banks(
    zip: &mut ZipArchive<File>,
    term_meta_banks: &[String],
) -> Result<TermMetaBank> {
    read_banks::<TermMetaBank>(zip, term_meta_banks)
}

fn read_tag_banks(zip: &mut ZipArchive<File>, tag_banks: &[String]) -> Result<TagBank> {
    read_banks::<TagBank>(zip, tag_banks)
}
