use std::path::Path;

use anyhow::{Result, bail};

use crate::{
    Context, Reader,
    formats::dsl::{DslFormat, files::DslFiles},
    glossary::Glossary,
    scan::DictionaryFiles,
};

impl Reader for DslFormat {
    fn read_with_context(&self, path: &Path, _: &Context) -> Result<Glossary> {
        let (_, files) = DslFiles::scan(path)?;
        bail!("Reading {} is not implemented yet", files.dsl)
    }
}
