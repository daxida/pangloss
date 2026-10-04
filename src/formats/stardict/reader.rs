use std::{collections::HashMap, io::Read, path::Path};

use anyhow::{Result, bail};

use crate::{
    Context, DataEntry, Reader,
    formats::stardict::{StardictFormat, files::StardictFiles, sts::SameTypeSequence},
    glossary::{AltEntry, Entry, Glossary, GlossaryInfo},
    scan::{DictionaryFiles, Source},
};

impl Reader for StardictFormat {
    fn read_with_context(&self, path: &Path, ctx: &Context) -> Result<Glossary> {
        read_with_context(path, ctx)
    }
}

fn read_with_context(path: &Path, _: &Context) -> Result<Glossary> {
    let (mut source, files) = StardictFiles::scan(path)?;
    let mut read = |name: &str| read_possibly_compressed(&mut source, name);

    let info = read_ifo_file(&String::from_utf8_lossy(&read(&files.ifo)?));
    let sts = SameTypeSequence::from_info(&info);

    // In theory, we only care about 32
    let is_large_file = match info.get("idxoffsetbits") {
        Some("32") | None => false,
        Some("64") => true,
        Some(other) => bail!("Invalid idxoffsetbits value: {other}"),
    };

    let idx = read_idx_file(&read(&files.idx)?, is_large_file)?;

    let syn = if let Some(syn) = &files.syn {
        read_syn_file(&read(syn)?, idx.len())?
    } else {
        tracing::info!("No synonym file found.");
        HashMap::new()
    };

    let entries = read_entries(sts, &idx, &syn, &read(&files.dict)?);

    let data_entries = files
        .css
        .iter()
        .flat_map(|css| source.read(css).map(|bytes| DataEntry::new(css, bytes)))
        .collect();

    Ok(Glossary {
        entries,
        data_entries,
        info,
        ..Default::default()
    })
}

fn read_entries(
    sts: SameTypeSequence,
    index_data: &[(Vec<u8>, u64, u32)],
    syn_dict: &HashMap<usize, Vec<String>>,
    dict_bytes: &[u8],
) -> Vec<Entry> {
    let mut entries = Vec::new();

    for (entry_index, (b_term, defi_offset, defi_size)) in index_data.iter().enumerate() {
        if b_term.is_empty() {
            tracing::warn!("Empty b_term");
            continue;
        }
        let offset = *defi_offset as usize;
        let size = *defi_size as usize;

        if offset + size > dict_bytes.len() {
            tracing::error!(
                "Unable to read definition for word {}",
                String::from_utf8_lossy(b_term)
            );
            continue;
        }

        let defi = String::from_utf8_lossy(&dict_bytes[offset..offset + size]).into_owned();
        let term = String::from_utf8_lossy(b_term).into_owned();
        let alts = syn_dict
            .get(&entry_index)
            .map(|alts| alts.iter().cloned().map(AltEntry::only_term).collect())
            .unwrap_or_default();
        entries.push(Entry::new(term, sts.as_definition(defi)).with_alts(alts));
    }

    entries
}

fn read_syn_file(syn_bytes: &[u8], entry_count: usize) -> Result<HashMap<usize, Vec<String>>> {
    let mut syn_dict: HashMap<usize, Vec<String>> = HashMap::new();
    let mut pos = 0;

    while pos < syn_bytes.len() {
        let beg = pos;
        let null_pos = syn_bytes[beg..].iter().position(|&b| b == 0);
        let Some(rel) = null_pos else {
            tracing::error!("Synonym file is corrupted");
            break;
        };
        pos = beg + rel;
        let b_alt = syn_bytes[beg..pos].to_vec();
        pos += 1;

        if pos + 4 > syn_bytes.len() {
            tracing::error!("Synonym file is corrupted");
            break;
        }

        let entry_index = u32::from_be_bytes(syn_bytes[pos..pos + 4].try_into()?) as usize;
        pos += 4;

        if entry_index >= entry_count {
            tracing::error!(
                "Corrupted synonym file. Word {} references invalid item",
                String::from_utf8_lossy(&b_alt)
            );
            continue;
        }

        let s_alt = String::from_utf8_lossy(&b_alt).into_owned();
        syn_dict.entry(entry_index).or_default().push(s_alt);
    }

    Ok(syn_dict)
}

fn read_idx_file(idx_bytes: &[u8], is_large_file: bool) -> Result<Vec<(Vec<u8>, u64, u32)>> {
    let step = if is_large_file { 8 } else { 4 };
    let mut index_data = Vec::new();
    let mut pos = 0;

    while pos < idx_bytes.len() {
        let beg = pos;
        let null_pos = idx_bytes[beg..].iter().position(|&b| b == 0);
        let Some(rel) = null_pos else {
            tracing::error!("Index file is corrupted (no null terminator)");
            break;
        };
        pos = beg + rel;
        let term = idx_bytes[beg..pos].to_vec();
        pos += 1;

        if pos + step + 4 > idx_bytes.len() {
            tracing::error!("Index file is corrupted (pos overflowed)");
            break;
        }

        let offset: u64 = if is_large_file {
            let v = u64::from_be_bytes(idx_bytes[pos..pos + 8].try_into()?);
            pos += 8;
            v
        } else {
            let v = u64::from(u32::from_be_bytes(idx_bytes[pos..pos + 4].try_into()?));
            pos += 4;
            v
        };

        let size = u32::from_be_bytes(idx_bytes[pos..pos + 4].try_into()?);
        pos += 4;

        index_data.push((term, offset, size));
    }

    Ok(index_data)
}

// https://github.com/ilius/pyglossary/blob/master/pyglossary/plugins/stardict/reader.py#L140
fn read_ifo_file(text: &str) -> GlossaryInfo {
    let mut info = GlossaryInfo::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "StarDict's dict ifo file" {
            continue;
        }
        let Some(sep) = trimmed.find('=') else {
            continue;
        };
        let key = &trimmed[..sep];
        let value = &trimmed[sep + 1..];
        if value.is_empty() {
            continue;
        }
        info.insert(key, value.to_string());
    }

    info
}

fn read_possibly_compressed(source: &mut Source, name: &str) -> Result<Vec<u8>> {
    let mut reader = source.open(name)?;
    if matches!(
        Path::new(name).extension().and_then(|e| e.to_str()),
        Some("dz" | "gz")
    ) {
        reader = Box::new(flate2::read::GzDecoder::new(reader));
    }
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf)?;
    Ok(buf)
}
