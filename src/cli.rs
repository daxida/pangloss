use std::path::PathBuf;

use clap::Parser;

use crate::registry::{ReaderFormat, WriterFormat};

#[derive(Parser, Debug, Clone)]
#[command(name = "pangloss", about = "Convert between glossary formats", version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    #[arg(help = "Path to the input dictionary: its main file, or a folder or archive holding it")]
    pub input: PathBuf,

    #[arg(help = "Path to the output dictionary, or a folder with --batch")]
    pub output: PathBuf,

    #[arg(
        long,
        help = "Convert every dictionary under the input folder, mirroring it in the output folder"
    )]
    pub batch: bool,

    #[arg(long, help = "Read format")]
    pub rformat: Option<ReaderFormat>,

    #[arg(long, help = "Write format")]
    pub wformat: Option<WriterFormat>,

    #[arg(long, help = "Overwrite the dictionary name")]
    pub name: Option<String>,

    // It requires the reader to store definitions in a String-compatible matter.
    #[arg(long, help = "Strip this pattern from every definition.")]
    pub strip_pattern: Option<String>,

    #[arg(short, long, help = "Enable verbose logging")]
    pub verbose: bool,
}

impl Cli {
    pub fn parse_cli() -> Self {
        Self::parse()
    }
}
