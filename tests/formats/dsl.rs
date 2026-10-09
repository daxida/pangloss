use std::{io::Write, path::Path};

use pretty_assertions::assert_eq;

use pangloss::{Glossary, Reader, ReaderFormat, formats::dsl::DslFormat};

const BASE: &str = "tests/fixtures/formats/dsl/01-base";

/// Every entry as its terms, joined by '|', and its definition.
fn entries(glossary: &Glossary) -> Vec<(String, String)> {
    glossary
        .entries
        .iter()
        .map(|entry| (entry.s_terms(), entry.definition().to_text()))
        .collect()
}

fn data_names(glossary: &Glossary) -> Vec<String> {
    glossary
        .data_entries
        .iter()
        .map(|d| d.fname().to_string_lossy().to_string())
        .collect()
}

// Checked against pyglossary, which only differs where noted
const EXPECTED: [(&str, &str); 4] = [
    (
        "λέξη|λέξις",
        concat!(
            // pyglossary adds a title="ουσιαστικό" tooltip, from the _abrv.dsl
            r#"<p style="padding-left:1em;margin:0"><i class="p"><font color="green">ουσ.</font></i> a <b>word</b></p>"#,
            r#"<p style="padding-left:2em;margin:0">see <a href="bword://λόγος">λόγος</a>, "#,
            r#"<span class="ex"><font color="steelblue">λέξη of mouth</font></span></p>"#,
        ),
    ),
    (
        "colour|color",
        concat!(
            r#"<p style="padding-left:1em;margin:0"><img align="top" src="pic.png" alt="pic.png" /> a hue</p>"#,
            r#"<p style="padding-left:2em;margin:0"><a href="bword://colour scheme">colour scheme</a>"#,
        ),
    ),
    (
        "colour scheme",
        r#"<p style="padding-left:1em;margin:0">a set of colours</p>"#,
    ),
    // pyglossary drops [br]: "movealong"
    ("go", "<b>to go</b><br/>move<br/>along"),
];

fn expected() -> Vec<(String, String)> {
    EXPECTED
        .iter()
        .map(|(terms, html)| ((*terms).to_string(), (*html).to_string()))
        .collect()
}

#[test]
fn reads_cards_info_and_media() {
    let glossary = DslFormat.read(&Path::new(BASE).join("dict.dsl")).unwrap();
    assert_eq!(entries(&glossary), expected());
    assert_eq!(glossary.info.name(), "Test Dictionary");
    assert_eq!(glossary.info.get("sourceLang"), Some("Greek"));
    assert_eq!(glossary.info.get("targetLang"), Some("English"));
    assert_eq!(glossary.info.get("description"), Some("A test dictionary."));
    assert_eq!(data_names(&glossary), ["pic.png"]);
}

#[test]
fn the_abbreviations_are_not_a_dictionary() {
    assert_eq!(
        ReaderFormat::try_from_path(&Path::new(BASE).join("dict.dsl")),
        Some(ReaderFormat::Dsl)
    );
    assert_eq!(
        ReaderFormat::try_from_path(&Path::new(BASE).join("dict_abrv.dsl")),
        None
    );
    // The folder holds one dictionary, the abbreviations notwithstanding
    assert_eq!(
        ReaderFormat::try_from_path(Path::new(BASE)),
        Some(ReaderFormat::Dsl)
    );
}

#[test]
fn reads_a_dictzipped_dsl_in_a_zip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dict.zip");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    let options = zip::write::SimpleFileOptions::default();

    let dsl = std::fs::read(Path::new(BASE).join("dict.dsl")).unwrap();
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gz.write_all(&dsl).unwrap();
    zip.start_file("nested/dict.dsl.dz", options).unwrap();
    zip.write_all(&gz.finish().unwrap()).unwrap();
    for name in ["dict_abrv.dsl", "dict.ann", "dict.dsl.files.zip"] {
        zip.start_file(format!("nested/{name}"), options).unwrap();
        zip.write_all(&std::fs::read(Path::new(BASE).join(name)).unwrap())
            .unwrap();
    }
    zip.finish().unwrap();

    assert_eq!(ReaderFormat::try_from_path(&path), Some(ReaderFormat::Dsl));
    let glossary = DslFormat.read(&path).unwrap();
    assert_eq!(entries(&glossary), expected());
    assert_eq!(glossary.info.get("description"), Some("A test dictionary."));
    assert_eq!(data_names(&glossary), ["pic.png"]);
}

#[test]
fn loose_media_is_found_in_folders_beside_the_dsl() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("dict.dsl"),
        "#NAME\t\"Media\"\n\nword\n\t[s]img/pic.png[/s]\n",
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("img")).unwrap();
    std::fs::write(dir.path().join("img/pic.png"), b"png").unwrap();

    let glossary = DslFormat.read(&dir.path().join("dict.dsl")).unwrap();
    assert_eq!(data_names(&glossary), ["img/pic.png"]);
}
