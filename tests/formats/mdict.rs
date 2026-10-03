use std::path::Path;

use pretty_assertions::assert_eq;

use pangloss::{
    Reader, Writer,
    formats::mdict::{CompressionKind, MdictFormat},
};

fn do_undo(ipath: &Path, compression: CompressionKind) {
    let opath = tempfile::NamedTempFile::new().expect("failed to create temp file");

    let fmt = MdictFormat::new(compression);
    let glossary = fmt.read(ipath).expect("failed to read");

    fmt.write(opath.path(), &glossary).expect("failed to write");

    let expected = std::fs::read(ipath).expect("failed to read fixture");
    let actual = std::fs::read(opath.path()).expect("failed to read output");

    let diff_pos = expected
        .iter()
        .zip(actual.iter())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| expected.len().min(actual.len()));
    eprintln!(
        "outputs differ at byte {diff_pos}\n  expected: {:?}\n  actual:   {:?}",
        &expected[diff_pos..],
        &actual[diff_pos..],
    );

    assert_eq!(expected, actual);
}

#[test]
fn read_info_entry1() {
    let ipath = Path::new("tests/fixtures/formats/mdict/001-entry1.mdx");
    let glossary = MdictFormat::default().read(ipath).expect("failed to read");
    assert!(
        glossary.info.get("GeneratedByEngineVersion").is_some(),
        "Couldn't find Mdict version in {:?}",
        glossary.info
    );
    // The number of keys in ATTR_ORDER
    assert_eq!(glossary.info.len(), 16);
}

#[test]
fn do_undo_one_entry1_uncompressed() {
    do_undo(
        Path::new("tests/fixtures/formats/mdict/001-entry1-uncompressed.mdx"),
        CompressionKind::None,
    );
}

#[test]
fn do_undo_one_entry3_uncompressed() {
    do_undo(
        Path::new("tests/fixtures/formats/mdict/003-entry3-uncompressed.mdx"),
        CompressionKind::None,
    );
}

#[test]
fn do_undo_one_entry1() {
    do_undo(
        Path::new("tests/fixtures/formats/mdict/001-entry1.mdx"),
        CompressionKind::Zip,
    );
}

#[test]
fn do_undo_repeated_headword_uncompressed() {
    do_undo(
        Path::new("tests/fixtures/formats/mdict/004-repeated-headword.mdx"),
        CompressionKind::None,
    );
}

#[test]
fn a_picture_is_read_from_the_mdd() {
    let glossary = MdictFormat::default()
        .read(Path::new(
            "tests/fixtures/formats/mdict/005-picture/005-picture.mdx",
        ))
        .expect("failed to read");

    let names: Vec<_> = glossary
        .data_entries
        .iter()
        .map(|d| d.fname().to_string_lossy().to_string())
        .collect();
    assert_eq!(names, ["apple.png"]);
    assert_eq!(&glossary.data_entries[0].bytes()[..4], b"\x89PNG");
}

#[test]
fn do_undo_picture_with_mdd() {
    let dir = tempfile::tempdir().unwrap();
    let opath = dir.path().join("out.mdx");

    let fmt = MdictFormat::new(CompressionKind::None);
    let glossary = fmt
        .read(Path::new(
            "tests/fixtures/formats/mdict/005-picture/005-picture.mdx",
        ))
        .expect("failed to read");
    fmt.write(&opath, &glossary).expect("failed to write");

    assert!(dir.path().join("out.mdd").exists(), "no .mdd was written");

    let back = fmt.read(&opath).expect("failed to read the pair back");
    assert_eq!(back.data_entries.len(), 1);
    assert_eq!(back.data_entries[0].fname().to_string_lossy(), "apple.png");
    assert_eq!(
        back.data_entries[0].bytes(),
        glossary.data_entries[0].bytes()
    );
}

/// Where each record starts and the size of each record block, read from an
/// uncompressed .mdx holding one entry per given size.
fn layout(sizes: &[usize]) -> (Vec<u64>, Vec<u64>) {
    use pangloss::{Definition, Entry, Glossary};

    let glossary = Glossary {
        entries: sizes
            .iter()
            .enumerate()
            .map(|(i, &n)| Entry::new(format!("term{i:03}"), Definition::Html("x".repeat(n))))
            .collect(),
        ..Default::default()
    };
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("out.mdx");
    MdictFormat::new(CompressionKind::None)
        .write(&path, &glossary)
        .unwrap();
    let bytes = std::fs::read(path).unwrap();

    let u64_at = |pos: usize| u64::from_be_bytes(bytes[pos..pos + 8].try_into().unwrap());
    let header_len = u32::from_be_bytes(bytes[..4].try_into().unwrap()) as usize;
    let key_header = 4 + header_len + 4;
    let key_block = key_header + 40 + 4 + u64_at(key_header + 24) as usize;
    let key_block_len = u64_at(key_header + 32) as usize;

    // The key block is an offset then a null terminated term, after an 8 byte block header.
    let keys = &bytes[key_block + 8..key_block + key_block_len];
    let mut offsets = Vec::new();
    let mut pos = 0;
    while pos < keys.len() {
        offsets.push(u64::from_be_bytes(keys[pos..pos + 8].try_into().unwrap()));
        pos += 8 + keys[pos + 8..].iter().position(|&b| b == 0).unwrap() + 1;
    }

    let records = key_block + key_block_len;
    let blocks = u64_at(records) as usize;
    let block_sizes = (0..blocks)
        .map(|i| u64_at(records + 32 + 16 * i + 8))
        .collect();
    (offsets, block_sizes)
}

// goldendict-ng reads a record out of a single decompressed block, so a block
// has to end where a record starts.
#[test]
fn record_blocks_end_on_a_record_start() {
    let (offsets, sizes) = layout(&[100_000; 80]);
    assert!(offsets.contains(&sizes[0]));
}

// Even when the record that fills the block is bigger than the block.
#[test]
fn a_record_bigger_than_a_block_is_not_split() {
    let (offsets, sizes) = layout(&[10, 5 << 20, 10]);
    assert!(offsets.contains(&sizes[0]));
}
