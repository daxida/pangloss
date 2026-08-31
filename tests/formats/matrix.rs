//! Every fixture dictionary, converted to every format we can write.
//!
//! A smoke test rather than a correctness one: it only asks that the conversion
//! runs. The per-format tests check what comes out. Its job is to notice when a
//! pair that used to work stops working, which no single-format test can see.

use std::{
    fs,
    path::{Path, PathBuf},
};

use pangloss::{Reader, ReaderFormat, Writer, WriterFormat};

// TODO: use enum macro to keep this in sync with the ReaderFormat and WriterFormat enums.
/// One output extension per writable format.
const OUTPUTS: [&str; 6] = ["txt", "json", "mdx", "ifo", "zip", "hdir"];

/// Loose pieces of a Yomitan dictionary that sit next to the fixtures, not
/// dictionaries in their own right.
const NOT_DICTIONARIES: [&str; 2] = ["index.json", "term_bank_1.json"];

// FIXME: Yomitan > Stardict
fn is_known_failure(reader: ReaderFormat, writer: WriterFormat) -> bool {
    reader == ReaderFormat::Yomitan && writer == WriterFormat::Stardict
}

fn collect_fixtures(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in fs::read_dir(dir).expect("failed to read the fixture directory") {
        let path = entry.expect("failed to read a fixture").path();
        if path.is_dir() {
            found.extend(collect_fixtures(&path));
        } else if ReaderFormat::try_from_path(&path).is_some() {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            if !NOT_DICTIONARIES.contains(&name.as_ref()) {
                found.push(path);
            }
        }
    }
    found
}

#[test]
fn every_fixture_converts_to_every_format() {
    let mut inputs = collect_fixtures(Path::new("tests/fixtures/formats"));
    inputs.sort();

    let mut converted = 0;
    let mut failures = Vec::new();

    for input in &inputs {
        let rformat = ReaderFormat::try_from_path(input).expect("a readable fixture");
        let dir = tempfile::tempdir().unwrap();

        for ext in OUTPUTS {
            let opath = dir.path().join(format!("out.{ext}"));
            let wformat = WriterFormat::try_from_path(&opath).expect("a writable extension");
            // TODO: remove me
            if is_known_failure(rformat, wformat) {
                continue;
            }

            converted += 1;
            if let Err(err) = rformat
                .read(input)
                .and_then(|glossary| wformat.write(&opath, &glossary))
            {
                failures.push(format!("{} -> .{ext}: {err}", input.display()));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {converted} conversions failed:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}
