use anyhow::Result;

use pangloss::{
    Definition, Entry, Glossary, ReaderWriter,
    formats::{
        json::JsonFormat, mdict::MdictFormat, stardict::StardictFormat, text::TextFormat,
        yomitan::YomitanFormat,
    },
};

// TODO: the filename is needed because there is currently no strict connexion between format
// and extension.
fn roundtrip<T: ReaderWriter>(rw: T, glossary: &Glossary, filename: &str) -> Result<Glossary> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join(filename);
    rw.write(&path, glossary)?;
    rw.read(&path)
}

// TODO: the filename is needed because there is currently no strict connexion between format
// and extension.
fn mktest_roundtrip<T: ReaderWriter>(rw: T, filename: &str) {
    let entries = vec![Entry::new(
        "apple".to_string(),
        Definition::Text("a fruit".to_string()),
    )];
    // TODO: add a builder...
    let glossary = Glossary {
        entries,
        ..Default::default()
    };
    let result = roundtrip(rw, &glossary, filename).unwrap();
    // We can't compare glossaries directly because info/metadata differs across formats.
    // We can't compare definitions directly because formats may change the variant
    // (e.g. Text -> Yomitan). We compare to_text() as the lowest common denominator.
    for (original, roundtripped) in glossary.entries.iter().zip(result.entries.iter()) {
        assert_eq!(original.term(), roundtripped.term());
        assert_eq!(
            original.definition().to_text(),
            roundtripped.definition().to_text()
        );
    }
}

#[test]
fn test_roundtrip_text() {
    mktest_roundtrip(TextFormat, "test.txt");
}

#[test]
fn test_roundtrip_json() {
    mktest_roundtrip(JsonFormat, "test.json");
}

#[test]
fn test_roundtrip_mdict() {
    mktest_roundtrip(MdictFormat::default(), "test.mdx");
}

#[test]
fn test_roundtrip_stardict() {
    mktest_roundtrip(StardictFormat, "test.ifo");
}

#[test]
fn test_roundtrip_yomitan() {
    mktest_roundtrip(YomitanFormat, "test.zip");
}
