use std::{
    path::{Path, PathBuf},
    process::Command,
};

use pangloss::{
    ReaderFormat, WriterFormat,
    batch::{find_dictionaries, output_path},
};

const FIXTURES: &str = "tests/fixtures/formats";

/// A tree of dictionaries in every format the batch looks for, plus things it should skip.
fn tree() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    for (from, to) in [
        ("mdict/005-picture/005-picture.mdx", "a/pic.mdx"),
        ("mdict/005-picture/005-picture.mdd", "a/pic.mdd"),
        ("stardict/02-syns/syns.ifo", "b/syns.ifo"),
        ("stardict/02-syns/syns.idx", "b/syns.idx"),
        ("stardict/02-syns/syns.dict", "b/syns.dict"),
        ("stardict/02-syns/syns.syn", "b/syns.syn"),
        ("yomitan/index.json", "b/c/unzipped/index.json"),
        ("yomitan/term_bank_1.json", "b/c/unzipped/term_bank_1.json"),
        ("yomitan/010-base.zip", "zipped.zip"),
        ("mdict/001-entry1.mdx", ".hidden/skipped.mdx"),
    ] {
        let to = root.path().join(to);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::copy(Path::new(FIXTURES).join(from), to).unwrap();
    }
    std::fs::write(root.path().join("notes.txt"), "not a dictionary").unwrap();
    root
}

fn relative(root: &Path, found: Vec<(PathBuf, ReaderFormat)>) -> Vec<(String, ReaderFormat)> {
    found
        .into_iter()
        .map(|(path, format)| {
            let path = path.strip_prefix(root).unwrap();
            (path.to_string_lossy().replace('\\', "/"), format)
        })
        .collect()
}

#[test]
fn dictionaries_are_found_in_every_folder() {
    let root = tree();
    let found = find_dictionaries(root.path(), None).unwrap();
    assert_eq!(
        relative(root.path(), found),
        [
            ("a/pic.mdx".to_string(), ReaderFormat::Mdict),
            ("b/c/unzipped/index.json".to_string(), ReaderFormat::Yomitan),
            ("b/syns.ifo".to_string(), ReaderFormat::Stardict),
            ("zipped.zip".to_string(), ReaderFormat::Yomitan),
        ]
    );
}

#[test]
fn an_rformat_only_finds_that_format() {
    let root = tree();
    let found = find_dictionaries(root.path(), Some(ReaderFormat::Text)).unwrap();
    assert_eq!(
        relative(root.path(), found),
        [("notes.txt".to_string(), ReaderFormat::Text)]
    );
}

#[test]
fn outputs_mirror_the_input_tree() {
    let cases = [
        ("in/a/pic.mdx", "out/a/pic.zip"),
        ("in/zipped.zip", "out/zipped.zip"),
        ("in/b/c/unzipped/index.json", "out/b/c/unzipped.zip"),
        ("in/index.json", "out/in.zip"),
    ];
    for (input, expected) in cases {
        let output = output_path(
            Path::new("in"),
            Path::new(input),
            Path::new("out"),
            WriterFormat::Yomitan,
        );
        assert_eq!(output, Path::new(expected), "{input}");
    }
}

#[test]
fn a_folder_is_converted_past_a_broken_dictionary() {
    let root = tree();
    std::fs::write(root.path().join("a/broken.mdx"), "not an mdx").unwrap();
    let out = tempfile::tempdir().unwrap();

    let run = Command::new(env!("CARGO_BIN_EXE_pangloss"))
        .arg(root.path())
        .arg(out.path())
        .arg("--batch")
        .arg("--wformat=yomitan")
        .env("RUST_LOG", "off")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(!run.status.success());
    assert!(
        stdout.contains("Converted 4, skipped 0, failed 1"),
        "{stdout}"
    );
    for converted in ["a/pic.zip", "b/syns.zip", "b/c/unzipped.zip", "zipped.zip"] {
        assert!(
            out.path().join(converted).is_file(),
            "{converted} is missing"
        );
    }
}

#[test]
fn a_folder_converted_into_itself_keeps_its_inputs() {
    let root = tree();
    let zipped = std::fs::read(root.path().join("zipped.zip")).unwrap();

    let run = Command::new(env!("CARGO_BIN_EXE_pangloss"))
        .arg(root.path())
        .arg(root.path())
        .arg("--batch")
        .arg("--wformat=yomitan")
        .env("RUST_LOG", "off")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(run.status.success(), "{stdout}");
    // zipped.zip would be written over itself
    assert!(
        stdout.contains("Converted 3, skipped 1, failed 0"),
        "{stdout}"
    );
    assert_eq!(
        std::fs::read(root.path().join("zipped.zip")).unwrap(),
        zipped
    );
    assert!(root.path().join("a/pic.zip").is_file());
}

#[test]
fn a_folder_of_dictionaries_needs_batch() {
    let root = tree();
    let out = tempfile::tempdir().unwrap();

    let run = Command::new(env!("CARGO_BIN_EXE_pangloss"))
        .arg(root.path())
        .arg(out.path().join("out.zip"))
        .env("RUST_LOG", "off")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&run.stderr);
    assert!(!run.status.success());
    assert!(stderr.contains("pass --batch"), "{stderr}");
}
