use std::path::Path;

use pangloss::{
    Entry, Glossary, Reader, ReaderFormat, WriterFormat,
    formats::{mdict::MdictFormat, stardict::StardictFormat},
};

/// A fresh folder holding these fixtures (relative to `tests/fixtures/formats`) under new names.
fn folder_with(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (from, to) in files {
        let from = Path::new("tests/fixtures/formats").join(from);
        std::fs::copy(from, dir.path().join(to)).unwrap();
    }
    dir
}

fn add_css(dir: &tempfile::TempDir, names: &[&str]) {
    for name in names {
        std::fs::write(dir.path().join(name), format!("/* {name} */")).unwrap();
    }
}

fn data_names(glossary: &Glossary) -> Vec<String> {
    glossary
        .data_entries
        .iter()
        .map(|d| d.fname().to_string_lossy().to_string())
        .collect()
}

fn terms(glossary: &Glossary) -> Vec<&str> {
    glossary.entries.iter().map(Entry::term).collect()
}

#[test]
fn reader_format_is_detected_from_the_path() {
    let cases = [
        ("dict.ifo", Some(ReaderFormat::Stardict)),
        ("dict.mdx", Some(ReaderFormat::Mdict)),
        ("dict.zip", Some(ReaderFormat::Yomitan)),
        ("dict.json", Some(ReaderFormat::Json)),
        ("dict.txt", Some(ReaderFormat::Text)),
        ("some/nested/dir/dict.mdx", Some(ReaderFormat::Mdict)),
        ("./dict.v2.ifo", Some(ReaderFormat::Stardict)),
        ("DICT.MDX", Some(ReaderFormat::Mdict)),
        ("Dict.Ifo", Some(ReaderFormat::Stardict)),
        // Companions are not dictionaries on their own
        ("dict.mdd", None),
        ("dict.idx", None),
        ("dict.css", None),
        // Html is write only
        ("out.hdir", None),
        ("dict", None),
        ("dir/", None),
    ];
    for (path, expected) in cases {
        assert_eq!(
            ReaderFormat::try_from_path(Path::new(path)),
            expected,
            "{path}"
        );
    }
}

#[test]
fn writer_format_is_detected_from_the_path() {
    let cases = [
        ("out.ifo", Some(WriterFormat::Stardict)),
        ("out.mdx", Some(WriterFormat::Mdict)),
        ("out.zip", Some(WriterFormat::Yomitan)),
        ("out.json", Some(WriterFormat::Json)),
        ("out.txt", Some(WriterFormat::Text)),
        ("out.hdir", Some(WriterFormat::Html)),
        ("OUT.ZIP", Some(WriterFormat::Yomitan)),
        ("out.mdd", None),
        ("out", None),
    ];
    for (path, expected) in cases {
        assert_eq!(
            WriterFormat::try_from_path(Path::new(path)),
            expected,
            "{path}"
        );
    }
}

#[test]
fn an_uppercase_extension_is_read() {
    let dir = folder_with(&[("mdict/001-entry1.mdx", "DICT.MDX")]);
    MdictFormat::default()
        .read(&dir.path().join("DICT.MDX"))
        .expect("failed to read");
}

#[test]
fn mdicts_sharing_a_folder_only_take_their_own_files() {
    let dir = folder_with(&[
        ("mdict/005-picture/005-picture.mdx", "a.mdx"),
        ("mdict/005-picture/005-picture.mdd", "a.mdd"),
        ("mdict/001-entry1.mdx", "b.mdx"),
    ]);
    add_css(&dir, &["a.css", "b.css", "shared.css"]);
    let read = |name| MdictFormat::default().read(&dir.path().join(name)).unwrap();
    assert_eq!(data_names(&read("a.mdx")), ["a.css", "apple.png"]);
    assert_eq!(data_names(&read("b.mdx")), ["b.css"]);
}

#[test]
fn an_mdict_alone_in_its_folder_takes_every_css() {
    let dir = folder_with(&[("mdict/001-entry1.mdx", "a.mdx")]);
    add_css(&dir, &["b.css", "a.css"]);
    let glossary = MdictFormat::default()
        .read(&dir.path().join("a.mdx"))
        .unwrap();
    assert_eq!(data_names(&glossary), ["a.css", "b.css"]);
}

#[test]
fn mdd_volumes_are_read_in_order_until_one_is_missing() {
    let mdd = "mdict/005-picture/005-picture.mdd";
    let dir = folder_with(&[
        ("mdict/005-picture/005-picture.mdx", "a.mdx"),
        (mdd, "a.mdd"),
        (mdd, "a.1.mdd"),
        // No a.2.mdd, so this one is not a volume
        (mdd, "a.3.mdd"),
    ]);
    let glossary = MdictFormat::default()
        .read(&dir.path().join("a.mdx"))
        .unwrap();
    assert_eq!(data_names(&glossary), ["apple.png", "apple.png"]);
}

#[test]
fn stardicts_sharing_a_folder_only_take_their_own_files() {
    let dir = folder_with(&[
        ("stardict/01-base/dict.ifo", "dict.ifo"),
        ("stardict/01-base/dict.idx", "dict.idx"),
        ("stardict/01-base/dict.dict", "dict.dict"),
        ("stardict/02-syns/syns.ifo", "syns.ifo"),
        ("stardict/02-syns/syns.idx", "syns.idx"),
        ("stardict/02-syns/syns.dict", "syns.dict"),
        ("stardict/02-syns/syns.syn", "syns.syn"),
    ]);
    for fixture in ["01-base/dict.ifo", "02-syns/syns.ifo"] {
        let alone = Path::new("tests/fixtures/formats/stardict").join(fixture);
        let shared = dir.path().join(alone.file_name().unwrap());
        assert_eq!(
            terms(&StardictFormat.read(&shared).expect("failed to read")),
            terms(&StardictFormat.read(&alone).unwrap()),
        );
    }
}
