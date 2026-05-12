//! CLI arguments and other data passed to both Reader and Writer.
//!
//! Ideally it is simple enough so it becomes clear who is using what.

use crate::{ReaderFormat, WriterFormat, cli::Cli};

// Wrapper over Config, in case we end up passing more internal information
// that is not part of the CLI arguments (for instance, "extra needed files"
// was here at some point, but was moved to main.rs to modify the glossary instead)
#[derive(Debug, Default)]
pub struct Context {
    pub config: Config,
}

impl Context {
    pub const fn from_config(config: Config) -> Self {
        Self { config }
    }
}

/// Helper struct over CLI arguments.
#[derive(Debug, Default, Clone, Copy)]
pub struct Config {
    pub rformat: Option<ReaderFormat>,
    pub wformat: Option<WriterFormat>,
}

impl Config {
    pub const fn from_cli(args: &Cli) -> Self {
        Self {
            rformat: args.rformat,
            wformat: args.wformat,
        }
    }
}
