use std::{fs::File, io::Read, path::Path};

use pangloss::formats::yomitan::{TagBankEntry, TermBank};

// This test is intentionally trivial. We do not need to match exactly the Yomitan render.
// Instead, we need just enough for the css to kick in (and I can't be bothered to trim
// the superfluous css/html tags...)
fn test_model(ipath: &Path) {
    let mut file = File::open(ipath).expect("Failed to open file");
    let mut buf = String::new();
    file.read_to_string(&mut buf).expect("Failed to read file");
    // Read as serde_json::Value to check that the json is not malformed
    let _result: serde_json::Value = serde_json::from_str(&buf).expect("Failed to parse JSON");
    // Then serialize
    let term_bank: TermBank = serde_json::from_str(&buf).unwrap();
    let tag_bank_entry = TagBankEntry {
        short_tag: "名".into(),
        category: "partOfSpeech".into(),
        sort_order: 1,
        long_tag: "名詞".into(),
        popularity_score: 1,
    };
    let received = &term_bank[0].to_html(&[tag_bank_entry]);
    assert!(received.contains("<div class=\"entry\">"));
    assert!(received.contains("partOfSpeech"));
    assert!(received.contains("名詞"));
}

#[test]
fn table1() {
    test_model(Path::new(
        "tests/fixtures/models/yomitan/term_bank/011-simple-entry.json",
    ));
}
