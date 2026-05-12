use pangloss::{Definition, Entry, Glossary, GlossaryInfo};

// We may want to add data_entries etc.

#[test]
fn add_entry() {
    let entry = Entry::new("apple".to_string(), Definition::Text("a fruit".to_string()));
    let glossary = Glossary {
        entries: vec![entry],
        ..Default::default()
    };
    assert_eq!(glossary.entries.len(), 1);
}

#[test]
fn add_info_pair() {
    let mut glossary = Glossary::default();
    assert_eq!(glossary.info.len(), 1); // The "name" key
    let _ = glossary.info.insert("snake_case", "value".to_string());
    assert_eq!(glossary.info.len(), 2);
}

#[test]
fn glossary_info() {
    let mut info = GlossaryInfo::default();
    let pairs = [("sametypesequence", "h"), ("source_language", "en")];
    for (key, value) in pairs {
        info.insert(key, value.to_string());
    }
    assert_eq!(info.len(), 3);
    assert_eq!(info.get("sourceLanguage"), Some("en"));
}
