use std::{fs::File, io::Write, path::Path};

use anyhow::{Result, bail};
use indexmap::IndexMap;
use serde_json::Value;
use zip::{ZipWriter, write::SimpleFileOptions};

use crate::{
    Context, Definition, Glossary, ReaderFormat, Writer,
    formats::yomitan::{TermBankEntry, YomitanFormat, model::YomitanDefinition},
    transform::rewrite_css_classes,
};

impl Writer for YomitanFormat {
    fn write_with_context(&self, path: &Path, glossary: &Glossary, ctx: &Context) -> Result<()> {
        write_with_context(path, glossary, ctx)
    }
}

fn write_with_context(path: &Path, glossary: &Glossary, ctx: &Context) -> Result<()> {
    let chunk_size = 1000;
    let file = File::create(path)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default();

    write_index(&mut zip, &options, glossary)?;

    let mut term_entries = Vec::with_capacity(glossary.entries.len());
    let mut term_meta_entries = Vec::new();

    for e in &glossary.entries {
        match e.definition().to_yomitan(e.term()) {
            YomitanDefinition::TermBankEntry(e) => term_entries.push(e),
            YomitanDefinition::TermMetaBankEntry(e) => term_meta_entries.push(e),
        }
    }

    for (term, alts) in &glossary.alt_map {
        for alt in alts {
            match alt.definition() {
                Some(Definition::Yomitan(YomitanDefinition::TermBankEntry(e))) => {
                    term_entries.push(e.clone());
                }
                Some(Definition::Yomitan(YomitanDefinition::TermMetaBankEntry(_))) => {
                    bail!("Term meta bank entry leaked to alt map")
                }
                Some(_) => bail!("Rich alt entry coming from unexpected format"),
                None => term_entries.push(TermBankEntry::raw_inflection(
                    term.clone(),
                    alt.term().to_string(),
                )),
            }
        }
    }

    for (i, chunk) in term_entries.chunks(chunk_size).enumerate() {
        let name = format!("term_bank_{}.json", i + 1);
        zip.start_file(&name, options)?;
        zip.write_all(serde_json::to_string(&chunk)?.as_bytes())?;
        // serde_json::to_writer(&mut zip, &chunk)?; // same speed
    }

    for (i, chunk) in term_meta_entries.chunks(chunk_size).enumerate() {
        let name = format!("term_meta_bank_{}.json", i + 1);
        zip.start_file(&name, options)?;
        zip.write_all(serde_json::to_string(&chunk)?.as_bytes())?;
    }

    // Yomitan only accepts a single css file name 'styles.css', so we write
    // any css files of the Glossary to that destination.
    // https://github.com/yomidevs/yomitan/blob/master/ext/js/dictionary/dictionary-importer.js#L297
    let css_bytes: Vec<_> = glossary
        .data_entries
        .iter()
        .filter(|e| e.is_css())
        .flat_map(|e| e.bytes.iter().copied()) // unfortunate copy
        .collect();
    if !css_bytes.is_empty() {
        zip.start_file("styles.css", options)?;
        // Transform css only if we are certain that the Glossary came from
        // a non-Yomitan reader.
        match ctx.config.rformat {
            Some(ReaderFormat::Yomitan) | None => {
                zip.write_all(&css_bytes)?;
            }
            _ => {
                tracing::debug!("Transforming css classes...");
                let css = String::from_utf8_lossy(&css_bytes);
                let rewritten = rewrite_css_classes(&css);
                zip.write_all(rewritten.as_bytes())?;
            }
        }
    }

    for data_entry in glossary.data_entries.iter().filter(|e| !e.is_css()) {
        let fname = data_entry.fname.to_string_lossy();
        zip.start_file(fname, options)?;
        zip.write_all(&data_entry.bytes)?;
    }

    zip.finish()?;
    Ok(())
}

// https://github.com/yomidevs/yomitan/blob/master/ext/data/schemas/dictionary-index-schema.json
fn write_index(
    zip: &mut ZipWriter<File>,
    options: &SimpleFileOptions,
    glossary: &Glossary,
) -> Result<()> {
    zip.start_file("index.json", *options)?;

    // We have to undo some of the normalization to comply with the schema
    let mut info: IndexMap<String, Value> = glossary
        .info
        .into_iter()
        .filter_map(|(k, v)| {
            let key = match k.as_str() {
                "name" => "title".to_string(),
                _ => k.clone(),
            };
            // For some fields, try to parse an int/bool.
            // If it fails, skip the key-value pair, otherwise yomitan won't import this.
            // (there are many other fields we don't deal with for now)
            let value = match k.as_str() {
                "version" | "format" => v.parse::<i64>().ok().map(Value::from),
                "sequenced" | "isUpdatable" => v.parse::<bool>().ok().map(Value::from),
                _ => Some(Value::from(v.as_str())),
            };

            value.map(|val| (key, val))
        })
        .collect();

    // The schema requirres "title", "revision" and one of ("version"|"format").
    // We already guarantee title, let's add the rest if not present:
    info.entry("revision".to_string())
        .or_insert_with(|| Value::from("1"));
    if !info.contains_key("version") && !info.contains_key("format") {
        info.entry("version".to_string())
            .or_insert_with(|| Value::from(3));
    }

    zip.write_all(serde_json::to_string_pretty(&info)?.as_bytes())?;
    Ok(())
}
