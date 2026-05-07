use std::path::Path;

use anyhow::Result;

mod encryption;
mod utils;

pub mod cli;
pub mod formats;
pub mod transformer;

mod registry;
pub use registry::{ReaderFormat, WriterFormat};

mod glossary;
pub use glossary::{DataEntry, Definition, Glossary, HtmlConverter};

mod context;
pub use context::{Config, Context};

pub trait Reader {
    fn read_with_context(&self, path: &Path, ctx: &Context) -> Result<Glossary>;

    fn read(&self, path: &Path) -> Result<Glossary> {
        self.read_with_context(path, &Context::default())
    }
}

pub trait Writer {
    fn write_with_context(&self, path: &Path, glossary: &Glossary, ctx: &Context) -> Result<()>;

    fn write(&self, path: &Path, glossary: &Glossary) -> Result<()> {
        self.write_with_context(path, glossary, &Context::default())
    }
}
