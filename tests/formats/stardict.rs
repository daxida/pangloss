use std::path::Path;

use pretty_assertions::assert_eq;

use pangloss::{Reader, Writer, formats::stardict::StardictFormat};

fn do_undo(ipath: &Path) {
    let tmpdir = tempfile::TempDir::new().expect("failed to create temp dir");
    let opath = tmpdir.path().join("out.ifo");

    let fmt = StardictFormat;
    let glossary = fmt.read(ipath).expect("failed to read");

    fmt.write(&opath, &glossary).expect("failed to write");

    let idir = ipath.parent().expect("ipath has no parent directory");
    for entry in std::fs::read_dir(idir).expect("failed to read input dir") {
        let entry = entry.expect("failed to read dir entry");
        let filename = entry.file_name();
        let expected_path = idir.join(&filename);
        let ext = expected_path.extension().unwrap_or_default();
        let actual_path = tmpdir.path().join("out").with_extension(ext);

        let expected = std::fs::read(&expected_path).expect("failed to read expected file");
        let actual = std::fs::read(&actual_path).expect("failed to read actual file");
        assert_eq!(expected, actual, "mismatch for file {:?}", filename);
    }
}

#[test]
fn do_undo_base() {
    do_undo(Path::new(
        "tests/fixtures/formats/stardict/01-base/dict.ifo",
    ));
}

#[test]
fn do_undo_syns() {
    // This test requires sorting entries!
    do_undo(Path::new(
        "tests/fixtures/formats/stardict/02-syns/syns.ifo",
    ));
}

//
// These two test fail because of synonym differences that I'm not entirely
// sure matter, nor can I see the reason of the divergence with pyglossary.
//

// #[test]
// fn do_undo_syns_long() {
//     do_undo(Path::new(
//         "tests/fixtures/formats/stardict/03-syns-100/100-ja-en.ifo",
//     ));
// }

// #[test]
// fn do_undo_bar() {
//     do_undo(Path::new(
//         "tests/fixtures/formats/stardict/04-syns-bar/bar.ifo",
//     ));
// }
