use std::path::Path;

use pretty_assertions::assert_eq;

use pangloss::{Reader, Writer, formats::json::JsonFormat};

#[test]
fn do_undo() {
    let ipath = Path::new("tests/fixtures/formats/dict.json");
    let opath = tempfile::NamedTempFile::new().expect("failed to create temp file");

    let fmt = JsonFormat;
    let glossary = fmt.read(ipath).expect("failed to read");

    fmt.write(opath.path(), &glossary).expect("failed to write");

    let expected = std::fs::read_to_string(ipath).expect("failed to read fixture");
    let actual = std::fs::read_to_string(opath.path()).expect("failed to read output");

    assert_eq!(expected, actual);
}
