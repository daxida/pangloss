//! Tests separatedly `index.json`, and the rest of `test_bank_x.json`

use std::{fs::File, io::Read, path::Path};

use pretty_assertions::assert_eq;
use serde_json::Value;
use zip::ZipArchive;

use pangloss::{Reader, Writer, formats::yomitan::YomitanFormat};

struct ZipContents {
    index: String,
    // (name, content)
    banks: Vec<(String, String)>,
    media: Vec<(String, Vec<u8>)>,
}

fn read_and_write(ipath: &Path) -> (ZipContents, ZipContents) {
    let opath = tempfile::NamedTempFile::new().expect("failed to create temp file");
    let fmt = YomitanFormat;
    let glossary = fmt.read(ipath).expect("failed to read");
    fmt.write(opath.path(), &glossary).expect("failed to write");
    (extract(ipath), extract(opath.path()))
}

fn extract(path: &Path) -> ZipContents {
    let mut zip = ZipArchive::new(File::open(path).unwrap()).unwrap();
    let mut index = String::new();
    let mut banks = Vec::new();
    let mut media = Vec::new();
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).unwrap();
        let name = entry.name().to_string();
        if name.ends_with(".json") {
            let mut content = String::new();
            entry.read_to_string(&mut content).unwrap();
            if name == "index.json" {
                index = content;
            } else {
                banks.push((name, content));
            }
        } else {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).unwrap();
            media.push((name, buf)); // maybe add the path too
        }
    }
    assert!(!banks.is_empty()); // No test with empty banks (for now at least)
    ZipContents {
        index,
        banks,
        media,
    }
}

// Compare as json: we don't care about formatting/order of entries
fn assert_json_eq(expected: &str, received: &str, name: &str) {
    let expected: Value = serde_json::from_str(expected).unwrap();
    let received: Value = serde_json::from_str(received).unwrap();
    if expected != received {
        let expected_pretty = serde_json::to_string_pretty(&expected).unwrap();
        let received_pretty = serde_json::to_string_pretty(&received).unwrap();
        assert_eq!(
            expected_pretty, received_pretty,
            "mismatch for zip entry {name}"
        );
    }
}

fn assert_index_eq(expected: &ZipContents, received: &ZipContents) {
    assert_eq!(expected.index, received.index);
}

fn assert_banks_eq(expected: &ZipContents, received: &ZipContents) {
    assert_eq!(
        expected.banks.len(),
        received.banks.len(),
        "bank count mismatch"
    );
    for ((name, expected_bank), (_, received_bank)) in
        expected.banks.iter().zip(received.banks.iter())
    {
        assert_json_eq(expected_bank, received_bank, name);
    }
}

fn assert_media_eq(expected: &ZipContents, received: &ZipContents) {
    assert_eq!(
        expected.media.len(),
        received.media.len(),
        "media count mismatch"
    );
    for ((name, expected_bytes), (_, received_bytes)) in
        expected.media.iter().zip(received.media.iter())
    {
        assert_eq!(expected_bytes, received_bytes, "media mismatch for {name}");
    }
}

fn assert_zip_contents_eq(expected: &ZipContents, received: &ZipContents) {
    assert_json_eq(&expected.index, &received.index, "index.json mismatch");
    assert_banks_eq(expected, received);
    assert_media_eq(expected, received);
}

#[test]
fn do_undo_index1() {
    let ipath = Path::new("tests/fixtures/formats/yomitan/010-base.zip");
    let (expected, received) = read_and_write(ipath);
    assert_index_eq(&expected, &received);
}

#[test]
fn do_undo_with_media() {
    let ipath = Path::new("tests/fixtures/formats/yomitan/011-base-with-gif.zip");
    let (expected, received) = read_and_write(ipath);
    assert_zip_contents_eq(&expected, &received);
}

#[test]
fn do_undo_term_banks1() {
    let ipath = Path::new("tests/fixtures/formats/yomitan/010-base.zip");
    let (expected, received) = read_and_write(ipath);
    assert_banks_eq(&expected, &received);
}

#[test]
fn do_undo_index2() {
    let ipath = Path::new("tests/fixtures/formats/yomitan/020-deinflection.zip");
    let (expected, received) = read_and_write(ipath);
    assert_index_eq(&expected, &received);
}

#[test]
fn do_undo_term_banks2() {
    let ipath = Path::new("tests/fixtures/formats/yomitan/020-deinflection.zip");
    let (expected, received) = read_and_write(ipath);
    assert_banks_eq(&expected, &received);
}

#[test]
fn model_table1() {
    let ipath = Path::new("tests/fixtures/formats/yomitan/001-table.zip");
    let (expected, received) = read_and_write(ipath);
    assert_banks_eq(&expected, &received);
}

#[test]
fn model_table2() {
    let ipath = Path::new("tests/fixtures/formats/yomitan/002-table.zip");
    let (expected, received) = read_and_write(ipath);
    assert_banks_eq(&expected, &received);
}

#[test]
fn model_jitendex() {
    let ipath = Path::new("tests/fixtures/formats/yomitan/003-jitendex.zip");
    let (expected, received) = read_and_write(ipath);
    assert_banks_eq(&expected, &received);
}

// Test the coverage of our model.
#[test]
fn model_final() {
    let ipath = Path::new("tests/fixtures/formats/yomitan/000-model.zip");
    YomitanFormat.read(ipath).expect("failed to read");
}
