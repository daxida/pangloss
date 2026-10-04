//! Finding the files of multi-file dictionaries (Stardict, Mdict).

use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use anyhow::{Result, bail};

use crate::utils::parent_dir;

pub trait DictionaryFiles: Sized {
    /// The extension of the file the user points at, f.e. "ifo".
    const MAIN_EXTENSION: &'static str;

    /// `main` is the name of the file the user points at, `names` every file beside it.
    /// Fails if a required file is missing.
    fn find(main: &str, names: &[String]) -> Result<Self>;

    fn scan(path: &Path) -> Result<(Source, Self)> {
        if path
            .extension()
            .is_none_or(|ext| !ext.eq_ignore_ascii_case(Self::MAIN_EXTENSION))
        {
            bail!(
                "Expected a file with .{} extension but got {}",
                Self::MAIN_EXTENSION,
                path.display()
            );
        }
        let main = path.file_name().unwrap_or_default().to_string_lossy();
        let source = Source::Dir(parent_dir(path).to_path_buf());
        let files = Self::find(&main, &source.names()?)?;
        Ok((source, files))
    }
}

/// Where the files of a dictionary live.
pub enum Source {
    Dir(PathBuf),
}

impl Source {
    /// The names of every file, unordered.
    pub fn names(&self) -> Result<Vec<String>> {
        match self {
            Self::Dir(dir) => fs::read_dir(dir)?
                .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
                .collect(),
        }
    }

    pub fn open(&self, name: &str) -> Result<Box<dyn Read>> {
        match self {
            Self::Dir(dir) => Ok(Box::new(fs::File::open(dir.join(name))?)),
        }
    }

    pub fn read(&self, name: &str) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        self.open(name)?.read_to_end(&mut bytes)?;
        Ok(bytes)
    }
}

/// `foo.<ext>` for `foo.<main ext>`.
pub fn with_extension(main: &str, ext: &str) -> String {
    Path::new(main)
        .with_extension(ext)
        .to_string_lossy()
        .into_owned()
}

/// The first `foo.<suffix>` among `names`.
pub fn companion(main: &str, names: &[String], suffixes: &[&str]) -> Option<String> {
    suffixes
        .iter()
        .map(|suffix| with_extension(main, suffix))
        .find(|name| names.contains(name))
}
