//! A simple json format based on the text format.

use std::{fs, path::Path};

use anyhow::{Result, bail};
use indexmap::IndexMap;
use serde_json::Value;

use crate::{
    Context, Reader, Writer,
    glossary::{Definition, Entry, Glossary, GlossaryInfo},
};

pub struct JsonFormat;

impl Reader for JsonFormat {
    fn read_with_context(&self, path: &Path, ctx: &Context) -> Result<Glossary> {
        read_with_context(path, ctx)
    }
}

fn read_with_context(path: &Path, _: &Context) -> Result<Glossary> {
    if path.extension().and_then(|e| e.to_str()) != Some("json") {
        bail!(
            "Expected a file with .json extension but got {}",
            path.display()
        );
    }

    let content = fs::read_to_string(path)?;
    let doc: IndexMap<String, Value> = serde_json::from_str(&content)?;

    let mut info = GlossaryInfo::new();
    let mut entries = Vec::new();

    for (key, value) in doc {
        let value_str = match value {
            Value::String(s) => s,
            other => other.to_string(),
        };

        if key.is_empty() || value_str.is_empty() {
            continue;
        }

        if let Some(info_key) = key.strip_prefix("##") {
            info.insert(info_key, value_str);
        } else {
            entries.push(Entry::new(key, Definition::Html(value_str)));
        }
    }

    Ok(Glossary {
        entries,
        info,
        ..Default::default()
    })
}

impl Writer for JsonFormat {
    fn write_with_context(&self, path: &Path, glossary: &Glossary, ctx: &Context) -> Result<()> {
        write_with_context(path, glossary, ctx)
    }
}

fn write_with_context(path: &Path, glossary: &Glossary, _: &Context) -> Result<()> {
    let mut doc: IndexMap<String, Value> = IndexMap::new();
    let alt_map = &glossary.alt_map;

    for (key, value) in &glossary.info {
        doc.insert(format!("##{key}"), Value::String(value.clone()));
    }

    for entry in &glossary.entries {
        doc.insert(
            entry.s_terms(alt_map),
            Value::String(entry.definition().to_text()),
        );
    }

    let mut output = serde_json::to_string_pretty(&doc)?;

    // TODO: remove this
    // serde_json uses 2-space indent by default; swap to tabs to match C# output
    output = indent_with_tabs(&output);

    // Good POSIX manners: trailing newline
    // https://stackoverflow.com/questions/729692/why-should-text-files-end-with-a-newline
    if !output.ends_with('\n') {
        output.push('\n');
    }

    fs::write(path, output)?;
    Ok(())
}

fn indent_with_tabs(json: &str) -> String {
    json.lines()
        .map(|line| {
            let spaces = line.len() - line.trim_start().len();
            let depth = spaces / 2;
            format!("{}{}", "\t".repeat(depth), line.trim_start())
        })
        .collect::<Vec<_>>()
        .join("\n")
}
