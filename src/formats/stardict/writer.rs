use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};

use anyhow::{Result, bail};
use byteorder::{BigEndian, WriteBytesExt};

use crate::{
    Context, Writer,
    formats::stardict::{StardictFormat, sts::SameTypeSequence},
    glossary::{Entry, Glossary, HtmlConverter},
};

impl Writer for StardictFormat {
    fn write_with_context(&self, path: &Path, glossary: &Glossary, ctx: &Context) -> Result<()> {
        write_with_context(path, glossary, ctx)
    }
}

fn write_with_context(path: &Path, glossary: &Glossary, _: &Context) -> Result<()> {
    if path.extension().and_then(|e| e.to_str()) != Some("ifo") {
        bail!("Stardict reader expects a ifo file, got {}", path.display());
    }

    let mut sts = SameTypeSequence::detect_from_info(&glossary.info);
    if matches!(sts, SameTypeSequence::None) {
        tracing::warn!("In theory, the writer doesn't work with same type sequence equal to None.");
        sts = SameTypeSequence::Html;
        tracing::info!("Auto-selecting sametypesequence=h");
    }

    write_compact(sts, path, glossary)?;

    // SAFETY: There should always be a parent by main.rs logic.
    let parent = path.parent().unwrap();
    let opath = parent.join("res"); // stardict convention
    let _ = fs::create_dir(&opath);
    for data_entry in glossary.css_files() {
        let fname = opath.join(&data_entry.fname);
        fs::write(&fname, &data_entry.bytes)?;
    }

    Ok(())
}

pub fn write_compact(sts: SameTypeSequence, path: &Path, glossary: &Glossary) -> Result<()> {
    let mut alt_index_list: Vec<(Vec<u8>, usize)> = Vec::new();
    let alt_map = &glossary.alt_map;

    let dict_path = path.with_extension("dict");
    let idx_path = path.with_extension("idx");
    tracing::info!("Writing {}", dict_path.display());

    let mut dict_file = BufWriter::new(File::create(&dict_path)?);
    let mut idx_file = BufWriter::new(File::create(&idx_path)?);

    let mut entry_idx = 0usize;
    let mut dict_mark = 0u32;

    // A bit hacky way to sort keys, as this format requires
    tracing::debug!("Sorting {:?} entries...", glossary.entries.len());
    let mut entries: Vec<&Entry> = glossary.entries.iter().collect();
    entries.sort_by_key(|a| a.term().to_lowercase());

    let converter = HtmlConverter::new(glossary);

    for entry in &glossary.entries {
        let b_term = entry.b_term();
        for b_alt in entry.b_alts(alt_map) {
            alt_index_list.push((b_alt, entry_idx));
        }

        let b_dict_block = converter.convert(entry.definition()).into_bytes();
        dict_file.write_all(&b_dict_block)?;

        idx_file.write_all(b_term)?;
        idx_file.write_u8(0)?;
        idx_file.write_u32::<BigEndian>(dict_mark)?;
        idx_file.write_u32::<BigEndian>(b_dict_block.len() as u32)?;

        dict_mark += b_dict_block.len() as u32;
        entry_idx += 1;
    }

    // Don't remove those: the BufWriter for idx_file might still hold buffered bytes
    // when we query the file size in write_syn_file. This guarantees flush.
    drop(dict_file);
    drop(idx_file);

    write_syn_file(path, &mut alt_index_list)?;
    write_ifo_file(path, glossary, sts, entry_idx, alt_index_list.len())?;
    Ok(())
}

fn write_syn_file(path: &Path, alt_index_list: &mut Vec<(Vec<u8>, usize)>) -> Result<()> {
    if alt_index_list.is_empty() {
        tracing::debug!("Empty alt list. Skipping writing syn file.");
        return Ok(());
    }

    alt_index_list.sort_by(|a, b| a.0.cmp(&b.0));

    let syn_path = path.with_extension("syn");
    tracing::info!("Writing {}", syn_path.display());
    tracing::debug!("Writing {} synonyms", alt_index_list.len());

    let mut syn_file = BufWriter::new(File::create(&syn_path)?);
    for (b_alt, entry_index) in alt_index_list {
        syn_file.write_all(b_alt)?;
        syn_file.write_u8(0)?;
        syn_file.write_u32::<BigEndian>(*entry_index as u32)?;
    }
    Ok(())
}

fn write_ifo_file(
    path: &Path,
    glossary: &Glossary,
    sts: SameTypeSequence,
    entry_count: usize,
    syn_word_count: usize,
) -> Result<()> {
    let ifo_path = path.with_extension("ifo");
    tracing::info!("Writing {}", ifo_path.display());

    let idx_path = path.with_extension("idx");
    let index_file_size = fs::metadata(&idx_path)?.len();

    let mut ifo_dict = indexmap::IndexMap::new();
    ifo_dict.insert("version", "3.0.0".to_string());
    ifo_dict.insert("bookname", glossary.info.name().replace('\n', " "));
    ifo_dict.insert("wordcount", entry_count.to_string());
    ifo_dict.insert("idxfilesize", index_file_size.to_string());
    ifo_dict.insert("sametypesequence", sts.to_format_string().to_string());

    if syn_word_count > 0 {
        ifo_dict.insert("synwordcount", syn_word_count.to_string());
    }

    if let Some(desc) = glossary.info.get("description")
        && !desc.is_empty()
    {
        ifo_dict.insert("description", desc.replace('\n', " "));
    }

    // I don't think this is necessary but pyglossary does it so let's do it
    // too while we test
    ifo_dict.entry("description").or_insert(String::new());

    let mut ifo_file = BufWriter::new(File::create(&ifo_path)?);
    writeln!(ifo_file, "StarDict's dict ifo file")?;
    for (key, value) in &ifo_dict {
        writeln!(ifo_file, "{key}={value}")?;
    }

    Ok(())
}
