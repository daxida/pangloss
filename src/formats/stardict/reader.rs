use std::{
    collections::HashMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::{Result, bail};

use crate::{
    Context, DataEntry, Reader,
    formats::stardict::{StardictFormat, sts::SameTypeSequence},
    glossary::{AltEntry, AltMap, Entry, Glossary, GlossaryInfo},
};

fn get_single_file(path: &Path, patterns: &[&str]) -> Result<PathBuf> {
    let mut matches = Vec::new();
    for pattern in patterns {
        let glob = format!("{}/{}", path.display(), pattern);
        for entry in glob::glob(&glob)? {
            matches.push(entry?);
        }
    }
    match matches.len() {
        1 => Ok(matches.remove(0)),
        0 => bail!("No {} file found in {}", patterns.join("/"), path.display()),
        n => bail!(
            "Expected exactly one {} file in {}, found {}",
            patterns.join("/"),
            path.display(),
            n
        ),
    }
}

impl Reader for StardictFormat {
    fn read_with_context(&self, path: &Path, ctx: &Context) -> Result<Glossary> {
        read_with_context(path, ctx)
    }
}

fn read_with_context(path: &Path, _: &Context) -> Result<Glossary> {
    if path.extension().and_then(|e| e.to_str()) != Some("ifo") {
        bail!("Stardict reader expects a ifo file, got {}", path.display());
    }
    let Some(parent) = path.parent() else {
        bail!("The ifo file was found at a location with no parent.");
    };

    let info = read_ifo_file(path)?;
    let sts = SameTypeSequence::from_info(&info);

    // In theory, we only care about 32
    let is_large_file = match info.get("idxoffsetbits") {
        Some("32") | None => false,
        Some("64") => true,
        Some(other) => bail!("Invalid idxoffsetbits value: {other}"),
    };

    let idx_path = get_single_file(parent, &["*.idx", "*.idx.dz", "*.idx.gz"])?;
    let idx = read_idx_file(&idx_path, is_large_file)?;

    let dict_path = get_single_file(parent, &["*.dict", "*.dict.dz"])?;

    let syn = if let Ok(syn_path) = get_single_file(parent, &["*.syn", "*.syn.dz", "*.syn.gz"]) {
        read_syn_file(&syn_path, idx.len())?
    } else {
        tracing::info!("No synonym file found.");
        HashMap::new()
    };

    let (entries, alt_map) = read_entries(sts, &idx, &syn, &dict_path)?;

    // Can there be more than one?
    let mut data_entries = Vec::new();
    if let Ok(css_path) = get_single_file(parent, &["*.css"])
        && let Ok(content) = fs::read(&css_path)
    {
        let fname = css_path.file_name().unwrap().to_string_lossy().to_string();
        data_entries.push(DataEntry::new(fname, content));
    }

    Ok(Glossary {
        entries,
        data_entries,
        alt_map,
        info,
        ..Default::default()
    })
}

fn read_entries(
    sts: SameTypeSequence,
    index_data: &[(Vec<u8>, u64, u32)],
    syn_dict: &HashMap<usize, Vec<String>>,
    dict_path: &Path,
) -> Result<(Vec<Entry>, AltMap)> {
    let dict_bytes = read_possibly_compressed(dict_path)?;
    let mut entries = Vec::new();
    let mut alt_map = AltMap::new();

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
        entries.push(Entry::new(term.clone(), sts.as_definition(defi)));
        // if there's no syn dict...
        if let Some(alts) = syn_dict.get(&entry_index).cloned() {
            let alts_entries: Vec<AltEntry> = alts.into_iter().map(AltEntry::only_term).collect();
            alt_map.insert(term, alts_entries);
        }
    }

    Ok((entries, alt_map))
}

fn read_syn_file(syn_path: &Path, entry_count: usize) -> Result<HashMap<usize, Vec<String>>> {
    let syn_bytes = read_possibly_compressed(syn_path)?;
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

fn read_idx_file(path: &Path, is_large_file: bool) -> Result<Vec<(Vec<u8>, u64, u32)>> {
    let idx_bytes = read_possibly_compressed(path)?;
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
pub fn read_ifo_file(ifo_path: &Path) -> Result<GlossaryInfo> {
    let mut info = GlossaryInfo::new();

    for line in fs::read_to_string(ifo_path)?.lines() {
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

    info.insert("sourceLang", "English".to_string());
    info.insert("targetLang", "French".to_string());

    Ok(info)
}

fn read_possibly_compressed(path: &Path) -> Result<Vec<u8>> {
    let ext = path.extension().and_then(|e| e.to_str());
    match ext {
        Some("dz" | "gz") => {
            let file = fs::File::open(path)?;
            let mut decoder = flate2::read::GzDecoder::new(file);
            let mut buf = Vec::new();
            decoder.read_to_end(&mut buf)?;
            Ok(buf)
        }
        _ => Ok(fs::read(path)?),
    }
}
