use std::path::Path;

use pretty_assertions::assert_eq;

use pangloss::{
    Reader, Writer,
    formats::mdict::{CompressionKind, MdictFormat},
};

fn do_undo(ipath: &Path, compression: CompressionKind) {
    let opath = tempfile::NamedTempFile::new().expect("failed to create temp file");

    let fmt = MdictFormat::new(compression);
    let glossary = fmt.read(ipath).expect("failed to read");

    fmt.write(opath.path(), &glossary).expect("failed to write");

    let expected = std::fs::read(ipath).expect("failed to read fixture");
    let actual = std::fs::read(opath.path()).expect("failed to read output");

    let diff_pos = expected
        .iter()
        .zip(actual.iter())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| expected.len().min(actual.len()));
    eprintln!(
        "outputs differ at byte {diff_pos}\n  expected: {:?}\n  actual:   {:?}",
        &expected[diff_pos..],
        &actual[diff_pos..],
    );

    assert_eq!(expected, actual);
}

#[test]
fn read_info_entry1() {
    let ipath = Path::new("tests/fixtures/formats/mdict/001-entry1.mdx");
    let glossary = MdictFormat::default().read(ipath).expect("failed to read");
    assert!(
        glossary.info.get("GeneratedByEngineVersion").is_some(),
        "Couldn't find Mdict version in {:?}",
        glossary.info
    );
    // The number of keys in ATTR_ORDER
    assert_eq!(glossary.info.len(), 16);
}

#[test]
fn do_undo_one_entry1_uncompressed() {
    do_undo(
        Path::new("tests/fixtures/formats/mdict/001-entry1-uncompressed.mdx"),
        CompressionKind::None,
    );
}

#[test]
fn do_undo_one_entry3_uncompressed() {
    do_undo(
        Path::new("tests/fixtures/formats/mdict/003-entry3-uncompressed.mdx"),
        CompressionKind::None,
    );
}

#[test]
fn do_undo_one_entry1() {
    do_undo(
        Path::new("tests/fixtures/formats/mdict/001-entry1.mdx"),
        CompressionKind::Zip,
    );
}

#[test]
fn do_undo_repeated_headword_uncompressed() {
    do_undo(
        Path::new("tests/fixtures/formats/mdict/004-repeated-headword.mdx"),
        CompressionKind::None,
    );
}

#[test]
fn a_picture_is_read_from_the_mdd() {
    let glossary = MdictFormat::default()
        .read(Path::new(
            "tests/fixtures/formats/mdict/005-picture/005-picture.mdx",
        ))
        .expect("failed to read");

    let names: Vec<_> = glossary
        .data_entries
        .iter()
        .map(|d| d.fname().to_string_lossy().to_string())
        .collect();
    assert_eq!(names, ["apple.png"]);
    assert_eq!(&glossary.data_entries[0].bytes()[..4], b"\x89PNG");
}

#[test]
fn do_undo_picture_with_mdd() {
    let dir = tempfile::tempdir().unwrap();
    let opath = dir.path().join("out.mdx");

    let fmt = MdictFormat::new(CompressionKind::None);
    let glossary = fmt
        .read(Path::new(
            "tests/fixtures/formats/mdict/005-picture/005-picture.mdx",
        ))
        .expect("failed to read");
    fmt.write(&opath, &glossary).expect("failed to write");

    assert!(dir.path().join("out.mdd").exists(), "no .mdd was written");

    let back = fmt.read(&opath).expect("failed to read the pair back");
    assert_eq!(back.data_entries.len(), 1);
    assert_eq!(back.data_entries[0].fname().to_string_lossy(), "apple.png");
    assert_eq!(
        back.data_entries[0].bytes(),
        glossary.data_entries[0].bytes()
    );
}
