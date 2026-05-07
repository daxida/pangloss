use std::{fs::File, io::Read, path::Path};

use pangloss::formats::yomitan::TermBank;

fn test_model(ipath: &Path) {
    let mut file = File::open(ipath).expect("Failed to open file");
    let mut buf = String::new();
    file.read_to_string(&mut buf).expect("Failed to read file");
    // Read as serde_json::Value to check that the json is not malformed
    let _result: serde_json::Value = serde_json::from_str(&buf).expect("Failed to parse JSON");
    // Then serialize
    let _term_bank: TermBank = serde_json::from_str(&buf).unwrap();
}

#[test]
fn table1() {
    test_model(Path::new(
        "tests/fixtures/models/yomitan/term_bank/001-table.json",
    ));
}

#[test]
fn table2() {
    test_model(Path::new(
        "tests/fixtures/models/yomitan/term_bank/002-table.json",
    ));
}

#[test]
fn image() {
    test_model(Path::new(
        "tests/fixtures/models/yomitan/term_bank/003-image.json",
    ));
}

#[test]
fn model_from_yomitan_repo1() {
    test_model(Path::new(
        "tests/fixtures/models/yomitan/term_bank/009-model.json",
    ));
}

#[test]
fn model_from_yomitan_repo2() {
    test_model(Path::new(
        "tests/fixtures/models/yomitan/term_bank/010-model.json",
    ));
}

#[test]
fn jitendex() {
    test_model(Path::new(
        "tests/fixtures/models/yomitan/term_bank/020-jitendex.json",
    ));
}
