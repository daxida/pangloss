use std::path::Path;

use pangloss::{Reader, formats::yomitan::YomitanFormat};

#[test]
fn reads_single_css_data_entry() {
    let path = Path::new("tests/fixtures/data_entry/100-base-with-css.zip");
    let glossary = YomitanFormat.read(&path).unwrap();
    assert_eq!(glossary.css_files().count(), 1);
}
