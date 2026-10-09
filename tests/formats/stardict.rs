use std::{io::Write, path::Path};

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
        // Sorted, as read back, with the spec's example of a nested name
        data_entries: vec![
            DataEntry::new("image.gif", b"GIF89a".to_vec()),
            DataEntry::new("pic/example.jpg", b"jpg".to_vec()),
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

    // And comes back from it
    let read = StardictFormat.read(&opath).expect("failed to read");
    assert_eq!(read.data_entries, glossary.data_entries);
}

/// The names and bytes of the media of `glossary`.
fn media(glossary: &Glossary) -> Vec<(String, Vec<u8>)> {
    glossary
        .data_entries
        .iter()
        .map(|d| (d.fname().to_string_lossy().to_string(), d.bytes().to_vec()))
        .collect()
}

/// Modelled on the sample dictionary of stardict-3.
const RES: &str = "tests/fixtures/formats/stardict/07-res";

/// The media of the 07-res fixture.
fn res_media() -> Vec<(String, Vec<u8>)> {
    ["apple.png", "pic/flag.gif"]
        .map(|name| {
            let bytes = std::fs::read(Path::new(RES).join("res").join(name)).unwrap();
            (name.to_string(), bytes)
        })
        .into()
}

#[test]
fn media_is_read_from_res() {
    let glossary = StardictFormat
        .read(&Path::new(RES).join("sample.ifo"))
        .unwrap();
    assert_eq!(media(&glossary), res_media());
}

#[test]
fn media_is_read_from_the_res_beside_the_ifo_in_an_archive() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("sample.zip");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    let options = zip::write::SimpleFileOptions::default();
    for name in ["sample.ifo", "sample.idx", "sample.syn", "sample.dict.dz"] {
        zip.start_file(format!("nested/{name}"), options).unwrap();
        zip.write_all(&std::fs::read(Path::new(RES).join(name)).unwrap())
            .unwrap();
    }
    for (name, bytes) in res_media() {
        zip.start_file(format!("nested/res/{name}"), options)
            .unwrap();
        zip.write_all(&bytes).unwrap();
    }
    // Not beside the .ifo, so not its media
    zip.start_file("res/other.png", options).unwrap();
    zip.write_all(b"not ours").unwrap();
    zip.finish().unwrap();

    let glossary = StardictFormat.read(&path).unwrap();
    assert_eq!(media(&glossary), res_media());
}

#[test]
fn media_named_outside_res_is_skipped() {
    let dir = tempdir().unwrap();
    let opath = dir.path().join("out.ifo");
    let glossary = Glossary {
        entries: vec![Entry::new(
            "apple".to_string(),
            Definition::Text("a fruit".to_string()),
        )],
        data_entries: vec![
            DataEntry::new("../escaped.png", b"no".to_vec()),
            DataEntry::new("kept.png", b"yes".to_vec()),
        ],
        ..Default::default()
    };
    StardictFormat.write(&opath, &glossary).unwrap();
    assert!(dir.path().join("res/kept.png").is_file());
    assert!(!dir.path().join("escaped.png").exists());
}
