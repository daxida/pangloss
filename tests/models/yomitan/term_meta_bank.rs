use std::{fs::File, io::Read, path::Path};

use pangloss::formats::yomitan::TermMetaBank;

fn test_model(ipath: &Path) {
    let mut file = File::open(ipath).expect("Failed to open file");
    let mut buf = String::new();
    file.read_to_string(&mut buf).expect("Failed to read file");
    // Read as serde_json::Value to check that the json is not malformed
    let _: serde_json::Value = serde_json::from_str(&buf).expect("Failed to parse JSON");
    // Then serialize
    let _: TermMetaBank = serde_json::from_str(&buf).unwrap();
}

#[test]
fn model_from_yomitan_repo() {
    test_model(Path::new(
        "tests/fixtures/models/yomitan/term_meta_bank/term_meta_bank_1.json",
    ));
}
