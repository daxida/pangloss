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
    glossary::{AltEntry, Definition, Entry, Glossary, GlossaryInfo, TERM_SEPARATOR},
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

    for line in reader.lines() {
        let line = line?;
        if line.is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('\t') else {
            tracing::warn!("Skipping line with no tab: {line}");
            continue;
        };
        if let Some(info_key) = key.strip_prefix("##") {
            info.insert(info_key, value.to_string());
        } else {
            // Use the first one as term; the rest as alts
            let mut parts = key.split(TERM_SEPARATOR);
            let term = parts.next().unwrap_or(key).to_string();
            if term.is_empty() {
                tracing::warn!("Skipping line with an empty headword: {line}");
                continue;
            }
            let alts: Vec<_> = parts
                .map(|alt| AltEntry::only_term(alt.to_string()))
                .collect();
            entries.push(Entry::new(term, Definition::from_raw_text(value)).with_alts(alts));
        }
    }

    Ok(Glossary {
        entries,
        info,
        ..Default::default()
    })
}

impl Writer for TextFormat {
    fn write_with_context(&self, path: &Path, glossary: &Glossary, ctx: &Context) -> Result<()> {
        write_with_context(path, glossary, ctx)
    }
}

fn escape_newlines(s: &str) -> String {
    s.replace("\r\n", "\\n").replace(['\n', '\r'], "\\n")
}

fn write_with_context(path: &Path, glossary: &Glossary, _: &Context) -> Result<()> {
    let mut lines = Vec::new();

    for (key, value) in &glossary.info {
        lines.push(format!("##{key}\t{}", escape_newlines(value)));
    }

    for entry in &glossary.entries {
        lines.push(format!(
            "{}\t{}",
            entry.s_terms(),
            escape_newlines(&entry.definition().to_text())
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
