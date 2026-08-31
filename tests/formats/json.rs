use std::path::Path;

use pretty_assertions::assert_eq;

use pangloss::{AltEntry, Definition, Entry, Glossary, Reader, Writer, formats::json::JsonFormat};

#[test]
fn do_undo() {
    let ipath = Path::new("tests/fixtures/formats/dict.json");
    let opath = tempfile::NamedTempFile::new().expect("failed to create temp file");

    let fmt = JsonFormat;
    let glossary = fmt.read(ipath).expect("failed to read");

    fmt.write(opath.path(), &glossary).expect("failed to write");

    let expected = std::fs::read_to_string(ipath).expect("failed to read fixture");
    let actual = std::fs::read_to_string(opath.path()).expect("failed to read output");

    assert_eq!(expected, actual);
}

#[test]
fn alts_survive_a_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let opath = dir.path().join("out.json");

    let glossary = Glossary {
        entries: vec![
            Entry::new(
                "bank".to_string(),
                Definition::Text("a riverside".to_string()),
            )
            .with_alts(vec![AltEntry::only_term("banks".to_string())]),
        ],
        ..Default::default()
    };
    JsonFormat
        .write(&opath, &glossary)
        .expect("failed to write");

    let read = JsonFormat.read(&opath).expect("failed to read");
    assert_eq!(read.entries[0].term(), "bank");
    assert_eq!(read.entries[0].alts().len(), 1);
    assert_eq!(read.entries[0].alts()[0].term(), "banks");
}
