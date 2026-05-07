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
