use std::{io::Write, path::Path};

use pangloss::{
    Entry, Glossary, Reader, ReaderFormat, WriterFormat,
    formats::{mdict::MdictFormat, stardict::StardictFormat, yomitan::YomitanFormat},
};

const FIXTURES: &str = "tests/fixtures/formats";

/// A fresh folder holding these fixtures (relative to `tests/fixtures/formats`) under new names.
fn folder_with(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for (from, to) in files {
        let from = Path::new(FIXTURES).join(from);
        std::fs::copy(from, dir.path().join(to)).unwrap();
    }
    dir
}

fn add_css(dir: &tempfile::TempDir, names: &[&str]) {
    for name in names {
        std::fs::write(dir.path().join(name), format!("/* {name} */")).unwrap();
    }
}

fn data_names(glossary: &Glossary) -> Vec<String> {
    glossary
        .data_entries
        .iter()
        .map(|d| d.fname().to_string_lossy().to_string())
        .collect()
}

fn terms(glossary: &Glossary) -> Vec<&str> {
    glossary.entries.iter().map(Entry::term).collect()
}

#[test]
fn reader_format_is_detected_from_the_path() {
    let cases = [
        ("dict.ifo", Some(ReaderFormat::Stardict)),
        ("dict.mdx", Some(ReaderFormat::Mdict)),
        ("dict.json", Some(ReaderFormat::Json)),
        ("unzipped/index.json", Some(ReaderFormat::Yomitan)),
        // A zip is detected by what it holds, see below
        ("missing.zip", None),
        ("dict.txt", Some(ReaderFormat::Text)),
        ("some/nested/dir/dict.mdx", Some(ReaderFormat::Mdict)),
        ("./dict.v2.ifo", Some(ReaderFormat::Stardict)),
        ("DICT.MDX", Some(ReaderFormat::Mdict)),
        ("Dict.Ifo", Some(ReaderFormat::Stardict)),
        // Companions are not dictionaries on their own
        ("dict.mdd", None),
        ("dict.idx", None),
        ("dict.css", None),
        // Html is write only
        ("out.hdir", None),
        ("dict", None),
        ("dir/", None),
    ];
    for (path, expected) in cases {
        assert_eq!(
            ReaderFormat::try_from_path(Path::new(path)),
            expected,
            "{path}"
        );
    }
}

#[test]
fn writer_format_is_detected_from_the_path() {
    let cases = [
        ("out.ifo", Some(WriterFormat::Stardict)),
        ("out.mdx", Some(WriterFormat::Mdict)),
        ("out.zip", Some(WriterFormat::Yomitan)),
        ("out.json", Some(WriterFormat::Json)),
        ("out.txt", Some(WriterFormat::Text)),
        ("out.hdir", Some(WriterFormat::Html)),
        ("OUT.ZIP", Some(WriterFormat::Yomitan)),
        ("out.mdd", None),
        ("out", None),
    ];
    for (path, expected) in cases {
        assert_eq!(
            WriterFormat::try_from_path(Path::new(path)),
            expected,
            "{path}"
        );
    }
}

#[test]
fn an_uppercase_extension_is_read() {
    let dir = folder_with(&[("mdict/001-entry1.mdx", "DICT.MDX")]);
    MdictFormat::default()
        .read(&dir.path().join("DICT.MDX"))
        .expect("failed to read");
}

#[test]
fn mdicts_sharing_a_folder_only_take_their_own_files() {
    let dir = folder_with(&[
        ("mdict/005-picture/005-picture.mdx", "a.mdx"),
        ("mdict/005-picture/005-picture.mdd", "a.mdd"),
        ("mdict/001-entry1.mdx", "b.mdx"),
    ]);
    add_css(&dir, &["a.css", "b.css", "shared.css"]);
    let read = |name| MdictFormat::default().read(&dir.path().join(name)).unwrap();
    assert_eq!(data_names(&read("a.mdx")), ["a.css", "apple.png"]);
    assert_eq!(data_names(&read("b.mdx")), ["b.css"]);
}

#[test]
fn an_mdict_alone_in_its_folder_takes_every_css() {
    let dir = folder_with(&[("mdict/001-entry1.mdx", "a.mdx")]);
    add_css(&dir, &["b.css", "a.css"]);
    let glossary = MdictFormat::default()
        .read(&dir.path().join("a.mdx"))
        .unwrap();
    assert_eq!(data_names(&glossary), ["a.css", "b.css"]);
}

#[test]
fn mdd_volumes_are_read_in_order_until_one_is_missing() {
    let mdd = "mdict/005-picture/005-picture.mdd";
    let dir = folder_with(&[
        ("mdict/005-picture/005-picture.mdx", "a.mdx"),
        (mdd, "a.mdd"),
        (mdd, "a.1.mdd"),
        // No a.2.mdd, so this one is not a volume
        (mdd, "a.3.mdd"),
    ]);
    let glossary = MdictFormat::default()
        .read(&dir.path().join("a.mdx"))
        .unwrap();
    assert_eq!(data_names(&glossary), ["apple.png", "apple.png"]);
}

#[test]
fn stardicts_sharing_a_folder_only_take_their_own_files() {
    let dir = folder_with(&[
        ("stardict/01-base/dict.ifo", "dict.ifo"),
        ("stardict/01-base/dict.idx", "dict.idx"),
        ("stardict/01-base/dict.dict", "dict.dict"),
        ("stardict/02-syns/syns.ifo", "syns.ifo"),
        ("stardict/02-syns/syns.idx", "syns.idx"),
        ("stardict/02-syns/syns.dict", "syns.dict"),
        ("stardict/02-syns/syns.syn", "syns.syn"),
    ]);
    for fixture in ["01-base/dict.ifo", "02-syns/syns.ifo"] {
        let alone = Path::new("tests/fixtures/formats/stardict").join(fixture);
        let shared = dir.path().join(alone.file_name().unwrap());
        assert_eq!(
            terms(&StardictFormat.read(&shared).expect("failed to read")),
            terms(&StardictFormat.read(&alone).unwrap()),
        );
    }
}

/// A zip at `path` holding these fixtures under new names.
fn zip_with(path: &Path, files: &[(&str, &str)]) {
    let mut zip = zip::ZipWriter::new(std::fs::File::create(path).unwrap());
    for (from, to) in files {
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file(*to, options).unwrap();
        zip.write_all(&std::fs::read(Path::new(FIXTURES).join(from)).unwrap())
            .unwrap();
    }
    zip.finish().unwrap();
}

#[test]
fn a_zip_is_detected_from_what_it_holds() {
    let dir = tempfile::tempdir().unwrap();
    let mdx = ("mdict/001-entry1.mdx", "dict.mdx");
    let ifo = ("stardict/01-base/dict.ifo", "dict.ifo");
    let index = ("yomitan/index.json", "index.json");
    let cases = [
        (vec![index], Some(ReaderFormat::Yomitan)),
        (vec![mdx], Some(ReaderFormat::Mdict)),
        (
            vec![("mdict/001-entry1.mdx", "nested/DICT.MDX")],
            Some(ReaderFormat::Mdict),
        ),
        (vec![ifo], Some(ReaderFormat::Stardict)),
        // Several dictionaries of one format are still that format
        (
            vec![mdx, ("mdict/001-entry1.mdx", "other.mdx")],
            Some(ReaderFormat::Mdict),
        ),
        // Yomitan only looks at the root
        (vec![("yomitan/index.json", "nested/index.json")], None),
        (vec![index, mdx], None),
        (vec![], None),
    ];
    for (i, (files, expected)) in cases.into_iter().enumerate() {
        let path = dir.path().join(format!("{i}.zip"));
        zip_with(&path, &files);
        assert_eq!(ReaderFormat::try_from_path(&path), expected, "{files:?}");
    }
    let real = Path::new(FIXTURES).join("yomitan/010-base.zip");
    assert_eq!(
        ReaderFormat::try_from_path(&real),
        Some(ReaderFormat::Yomitan)
    );
    for (archive, expected) in [
        ("mdict/006-archived.7z", ReaderFormat::Mdict),
        ("stardict/06-archived.7z", ReaderFormat::Stardict),
    ] {
        let path = Path::new(FIXTURES).join(archive);
        assert_eq!(
            ReaderFormat::try_from_path(&path),
            Some(expected),
            "{archive}"
        );
    }
}

#[test]
fn a_zipped_mdict_reads_like_an_unzipped_one() {
    let dir = tempfile::tempdir().unwrap();
    let zip = dir.path().join("dict.zip");
    zip_with(
        &zip,
        &[
            ("mdict/005-picture/005-picture.mdx", "005-picture.mdx"),
            ("mdict/005-picture/005-picture.mdd", "005-picture.mdd"),
        ],
    );
    let zipped = MdictFormat::default().read(&zip).unwrap();
    let unzipped = MdictFormat::default()
        .read(&Path::new(FIXTURES).join("mdict/005-picture/005-picture.mdx"))
        .unwrap();
    assert_eq!(terms(&zipped), terms(&unzipped));
    assert_eq!(data_names(&zipped), data_names(&unzipped));
}

#[test]
fn a_zipped_stardict_reads_like_an_unzipped_one() {
    let dir = tempfile::tempdir().unwrap();
    let zip = dir.path().join("dict.zip");
    zip_with(
        &zip,
        &[
            ("stardict/02-syns/syns.ifo", "syns.ifo"),
            ("stardict/02-syns/syns.idx", "syns.idx"),
            ("stardict/02-syns/syns.dict", "syns.dict"),
            ("stardict/02-syns/syns.syn", "syns.syn"),
        ],
    );
    let zipped = StardictFormat.read(&zip).unwrap();
    let unzipped = StardictFormat
        .read(&Path::new(FIXTURES).join("stardict/02-syns/syns.ifo"))
        .unwrap();
    assert_eq!(terms(&zipped), terms(&unzipped));
}

#[test]
fn an_unzipped_yomitan_reads_like_a_zipped_one() {
    let zip = Path::new(FIXTURES).join("yomitan/011-base-with-gif.zip");
    let dir = tempfile::tempdir().unwrap();
    zip::ZipArchive::new(std::fs::File::open(&zip).unwrap())
        .unwrap()
        .extract(dir.path())
        .unwrap();
    let zipped = YomitanFormat.read(&zip).unwrap();
    let unzipped = YomitanFormat.read(&dir.path().join("index.json")).unwrap();
    assert_eq!(terms(&zipped), terms(&unzipped));
    assert_eq!(data_names(&zipped), data_names(&unzipped));
}

#[test]
fn a_7z_reads_like_its_unarchived_dictionary() {
    let mdict = MdictFormat::default();
    let archived = mdict
        .read(&Path::new(FIXTURES).join("mdict/006-archived.7z"))
        .unwrap();
    let unarchived = mdict
        .read(&Path::new(FIXTURES).join("mdict/005-picture/005-picture.mdx"))
        .unwrap();
    assert_eq!(terms(&archived), terms(&unarchived));
    assert_eq!(data_names(&archived), data_names(&unarchived));

    let archived = StardictFormat
        .read(&Path::new(FIXTURES).join("stardict/06-archived.7z"))
        .unwrap();
    let unarchived = StardictFormat
        .read(&Path::new(FIXTURES).join("stardict/02-syns/syns.ifo"))
        .unwrap();
    assert_eq!(terms(&archived), terms(&unarchived));
}

#[test]
fn a_zip_with_two_dictionaries_of_a_format_is_not_read() {
    let dir = tempfile::tempdir().unwrap();
    let zip = dir.path().join("dict.zip");
    zip_with(
        &zip,
        &[
            ("mdict/001-entry1.mdx", "a.mdx"),
            ("mdict/001-entry1.mdx", "b.mdx"),
        ],
    );
    let err = MdictFormat::default().read(&zip).unwrap_err();
    assert!(err.to_string().contains("More than one *.mdx"), "{err}");
}

#[test]
fn an_unzipped_yomitan_takes_only_the_media_yomitan_imports() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("dict");
    std::fs::create_dir_all(dir.join("img")).unwrap();
    std::fs::copy(
        Path::new(FIXTURES).join("yomitan/index.json"),
        dir.join("index.json"),
    )
    .unwrap();
    let bank = r#"[
        ["a", "", "", "", 0, [{"type": "structured-content", "content": [
            {"tag": "span", "content": {"tag": "img", "path": "img/a.png"}}
        ]}], 0, ""],
        ["b", "", "", "", 0, [{"type": "image", "path": "../secret.png"}], 0, ""]
    ]"#;
    std::fs::write(dir.join("term_bank_1.json"), bank).unwrap();
    for (name, bytes) in [
        ("img/a.png", "a"),
        ("img/unused.png", "unused"),
        ("styles.css", "b {}"),
        ("notes.txt", "not part of the dictionary"),
    ] {
        std::fs::write(dir.join(name), bytes).unwrap();
    }
    std::fs::write(root.path().join("secret.png"), "outside").unwrap();

    let glossary = YomitanFormat.read(&dir.join("index.json")).unwrap();
    assert_eq!(data_names(&glossary), ["styles.css", "img/a.png"]);
}
