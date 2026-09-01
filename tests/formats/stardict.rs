use std::path::Path;

use pretty_assertions::assert_eq;
use tempfile::tempdir;

use pangloss::{
    DataEntry, Definition, Entry, Glossary, Reader, Writer, formats::stardict::StardictFormat,
};

fn do_undo(ipath: &Path) {
    let tmpdir = tempfile::TempDir::new().expect("failed to create temp dir");
    let opath = tmpdir.path().join("out.ifo");

    let fmt = StardictFormat;
    let glossary = fmt.read(ipath).expect("failed to read");

    fmt.write(&opath, &glossary).expect("failed to write");

    let idir = ipath.parent().expect("ipath has no parent directory");
    for entry in std::fs::read_dir(idir).expect("failed to read input dir") {
        let entry = entry.expect("failed to read dir entry");
        let filename = entry.file_name();
        let expected_path = idir.join(&filename);
        let ext = expected_path.extension().unwrap_or_default();
        let actual_path = tmpdir.path().join("out").with_extension(ext);

        let expected = std::fs::read(&expected_path).expect("failed to read expected file");
        let actual = std::fs::read(&actual_path).expect("failed to read actual file");
        assert_eq!(expected, actual, "mismatch for file {:?}", filename);
    }
}

#[test]
fn do_undo_base() {
    do_undo(Path::new(
        "tests/fixtures/formats/stardict/01-base/dict.ifo",
    ));
}

#[test]
fn do_undo_larousse_extract() {
    do_undo(Path::new(
        "tests/fixtures/formats/stardict/05-larousse-extract/larousse.ifo",
    ));
}

#[test]
fn do_undo_syns() {
    // This test requires sorting entries!
    do_undo(Path::new(
        "tests/fixtures/formats/stardict/02-syns/syns.ifo",
    ));
}

#[test]
fn do_undo_syns_long() {
    do_undo(Path::new(
        "tests/fixtures/formats/stardict/03-syns-100/100-ja-en.ifo",
    ));
}

#[test]
fn do_undo_bar() {
    do_undo(Path::new(
        "tests/fixtures/formats/stardict/04-syns-bar/bar.ifo",
    ));
}

#[test]
fn test_idx_entries_written_in_sorted_order() {
    let dir = tempdir().unwrap();
    let ifo_path = dir.path().join("test.ifo");

    let entries = vec![
        Entry::new(
            "zebra".to_string(),
            Definition::Text("last alphabetically".to_string()),
        ),
        Entry::new(
            "apple".to_string(),
            Definition::Text("first alphabetically".to_string()),
        ),
        Entry::new(
            "mango".to_string(),
            Definition::Text("middle alphabetically".to_string()),
        ),
    ];
    let glossary = Glossary {
        entries,
        ..Default::default()
    };

    StardictFormat.write(&ifo_path, &glossary).unwrap();

    let read_glossary = StardictFormat.read(&ifo_path).unwrap();
    let read_terms: Vec<_> = read_glossary.entries.iter().map(Entry::term).collect();

    let mut expected = read_terms.clone();
    expected.sort_by_key(|t| t.to_lowercase());
    assert_eq!(
        read_terms, expected,
        "idx entries should be in lexicographic order"
    );
}

#[test]
fn media_reaches_the_res_directory() {
    let dir = tempdir().unwrap();
    let opath = dir.path().join("out.ifo");

    let glossary = Glossary {
        entries: vec![Entry::new(
            "apple".to_string(),
            Definition::Text("a fruit".to_string()),
        )],
        data_entries: vec![
            DataEntry::new("image.gif", b"GIF89a".to_vec()),
            DataEntry::new("styles.css", b"body {}".to_vec()),
        ],
        ..Default::default()
    };

    StardictFormat
        .write(&opath, &glossary)
        .expect("failed to write");

    let res = dir.path().join("res");
    assert_eq!(
        std::fs::read(res.join("image.gif")).unwrap(),
        b"GIF89a".to_vec()
    );
    assert_eq!(
        std::fs::read(res.join("styles.css")).unwrap(),
        b"body {}".to_vec()
    );
}
