//! Tests separatedly `index.json`, and the rest of `test_bank_x.json`

use std::{fs::File, io::Read, path::Path};

use pretty_assertions::assert_eq;
use zip::ZipArchive;

use pangloss::{Reader, Writer, formats::yomitan::YomitanFormat};

struct ZipContents {
    index: String,
    // (name, content)
    banks: Vec<(String, String)>,
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
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).unwrap();
        let name = entry.name().to_string();
        let mut content = String::new();
        entry.read_to_string(&mut content).unwrap();
        if name == "index.json" {
            index = content;
        } else {
            banks.push((name, content));
        }
    }
    assert!(!banks.is_empty()); // No test with empty banks (for now at least)
    ZipContents { index, banks }
}

// Compare as json: we don't care about formatting/order of entries
fn assert_json_eq(expected: &str, actual: &str, name: &str) {
    let expected: serde_json::Value =
        serde_json::from_str(expected).expect("failed to parse expected JSON");
    let actual: serde_json::Value =
        serde_json::from_str(actual).expect("failed to parse actual JSON");
    // assert_eq!(expected, actual, "mismatch for zip entry {name}");

    if expected != actual {
        let expected_pretty = serde_json::to_string_pretty(&expected).unwrap();
        let actual_pretty = serde_json::to_string_pretty(&actual).unwrap();
        assert_eq!(
            expected_pretty, actual_pretty,
            "mismatch for zip entry {name}"
        );
    }
}

#[test]
fn do_undo_index1() {
    let ipath = Path::new("tests/fixtures/formats/yomitan/010-base.zip");
    let (expected, actual) = read_and_write(ipath);
    assert_eq!(expected.index, actual.index);
}

#[test]
fn do_undo_term_banks1() {
    let ipath = Path::new("tests/fixtures/formats/yomitan/010-base.zip");
    let (expected, actual) = read_and_write(ipath);
    for ((name, expected_bank), (_, actual_bank)) in expected.banks.iter().zip(actual.banks.iter())
    {
        assert_json_eq(expected_bank, actual_bank, name);
    }
}

#[test]
fn do_undo_index2() {
    let ipath = Path::new("tests/fixtures/formats/yomitan/020-deinflection.zip");
    let (expected, actual) = read_and_write(ipath);
    assert_eq!(expected.index, actual.index);
}

#[test]
fn do_undo_term_banks2() {
    let ipath = Path::new("tests/fixtures/formats/yomitan/020-deinflection.zip");
    let (expected, actual) = read_and_write(ipath);
    for ((name, expected_bank), (_, actual_bank)) in expected.banks.iter().zip(actual.banks.iter())
    {
        assert_json_eq(expected_bank, actual_bank, name);
    }
}

#[test]
fn model_table1() {
    let ipath = Path::new("tests/fixtures/formats/yomitan/001-table.zip");
    let (expected, actual) = read_and_write(ipath);
    for ((name, expected_bank), (_, actual_bank)) in expected.banks.iter().zip(actual.banks.iter())
    {
        assert_json_eq(expected_bank, actual_bank, name);
    }
}

#[test]
fn model_table2() {
    let ipath = Path::new("tests/fixtures/formats/yomitan/002-table.zip");
    let (expected, actual) = read_and_write(ipath);
    for ((name, expected_bank), (_, actual_bank)) in expected.banks.iter().zip(actual.banks.iter())
    {
        assert_json_eq(expected_bank, actual_bank, name);
    }
}

#[test]
fn model_jitendex() {
    let ipath = Path::new("tests/fixtures/formats/yomitan/003-jitendex.zip");
    let (expected, actual) = read_and_write(ipath);
    for ((name, expected_bank), (_, actual_bank)) in expected.banks.iter().zip(actual.banks.iter())
    {
        assert_json_eq(expected_bank, actual_bank, name);
    }
}

// Test the coverage of our model.
#[test]
fn model_final() {
    let ipath = Path::new("tests/fixtures/formats/yomitan/000-model.zip");
    YomitanFormat.read(ipath).expect("failed to read");
}
