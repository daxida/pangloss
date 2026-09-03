// cargo bench --bench mdict

use std::path::PathBuf;

use pangloss::{
    Definition, Entry, Glossary, GlossaryInfo, Writer,
    formats::mdict::{CompressionKind, MdictFormat},
};

fn main() {
    divan::main();
}

const SIZES: [usize; 3] = [1_000, 10_000, 100_000];

/// A glossary of `n` entries.
fn glossary(n: usize) -> Glossary {
    let entries = (0..n)
        .map(|i| {
            let key = (i * 2_654_435_761) % 1_000_000_007;
            Entry::new(
                format!("term{key:010}"),
                Definition::Html(format!(
                    "<div><b>sense {i}</b> — a definition with some length to it.</div>"
                )),
            )
        })
        .collect();

    let mut info = GlossaryInfo::new();
    info.insert("name", "Bench".to_string());

    Glossary {
        entries,
        info,
        ..Default::default()
    }
}

struct Output {
    path: PathBuf,
    _dir: tempfile::TempDir,
}

impl Output {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("failed to create a temporary directory");
        Self {
            path: dir.path().join("bench.mdx"),
            _dir: dir,
        }
    }
}

#[divan::bench(args = SIZES)]
fn zip(bencher: divan::Bencher, n: usize) {
    let glossary = glossary(n);
    bencher
        .with_inputs(Output::new)
        .bench_refs(|out| MdictFormat::new(CompressionKind::Zip).write(&out.path, &glossary));
}

#[divan::bench(args = SIZES)]
fn uncompressed(bencher: divan::Bencher, n: usize) {
    let glossary = glossary(n);
    bencher
        .with_inputs(Output::new)
        .bench_refs(|out| MdictFormat::new(CompressionKind::None).write(&out.path, &glossary));
}
