//! A simple tab-separated text format.
//!
//! Each line is either:
//! - A comment/info line starting with `##`, e.g. `##name\tMy Dictionary`
//! - A term/definition pair separated by a tab, e.g. `hello\tA greeting`

use std::{
    fs,
    io::{BufRead, BufReader},
    path::Path,
};

use anyhow::{Result, bail};

use crate::{
    Context, Reader, Writer,
    glossary::{AltEntry, AltMap, Definition, Entry, Glossary, GlossaryInfo, TERM_SEPARATOR},
};

pub struct TextFormat;

impl Reader for TextFormat {
    fn read_with_context(&self, path: &Path, ctx: &Context) -> Result<Glossary> {
        read_with_context(path, ctx)
    }
}

fn read_with_context(path: &Path, _: &Context) -> Result<Glossary> {
    if path.extension().and_then(|e| e.to_str()) != Some("txt") {
        bail!(
            "Expected a file with .txt extension but got {}",
            path.display()
        );
    }

    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);

    let mut info = GlossaryInfo::new();
    let mut entries = Vec::new();
    // TODO: extract alts!!!
    let mut alt_map = AltMap::new();

    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let (key, value) = line.split_once('\t').unwrap_or((&line, ""));
        if let Some(info_key) = key.strip_prefix("##") {
            info.insert(info_key, value.to_string());
        } else {
            // Use the first one as term; the rest as alts
            let mut parts = key.split(TERM_SEPARATOR);
            let term = parts.next().unwrap_or(key).to_string();
            let mut alts = parts
                .map(|alt| AltEntry::only_term(alt.to_string()))
                .peekable();
            entries.push(Entry::new(
                term.clone(),
                Definition::Text(value.to_string()),
            ));
            if alts.peek().is_some() {
                alt_map.entry(term).or_default().extend(alts);
            }
        }
    }

    Ok(Glossary {
        entries,
        alt_map,
        info,
        ..Default::default()
    })
}

impl Writer for TextFormat {
    fn write_with_context(&self, path: &Path, glossary: &Glossary, ctx: &Context) -> Result<()> {
        write_with_context(path, glossary, ctx)
    }
}

fn write_with_context(path: &Path, glossary: &Glossary, _: &Context) -> Result<()> {
    let mut lines = Vec::new();

    for (key, value) in &glossary.info {
        let clean_value = if key == "description" {
            value.replace('\n', "\\n")
        } else {
            value.clone()
        };
        lines.push(format!("##{key}\t{clean_value}"));
    }

    let alt_map = &glossary.alt_map;
    for entry in &glossary.entries {
        lines.push(format!(
            "{}\t{}",
            entry.s_terms(alt_map),
            entry.definition().to_text()
        ));
    }

    let mut output = lines.join("\n");

    // Good POSIX manners: trailing newline
    // https://stackoverflow.com/questions/729692/why-should-text-files-end-with-a-newline
    if !output.ends_with('\n') {
        output.push('\n');
    }

    fs::write(path, output)?;
    Ok(())
}
