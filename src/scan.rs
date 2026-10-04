//! Finding the files of a dictionary, in a folder or inside a zip.

use std::{
    fs,
    io::{Cursor, Read},
    path::{Component, Path, PathBuf},
};

use anyhow::{Result, bail};
use zip::ZipArchive;

use crate::utils::parent_dir;

pub trait DictionaryFiles: Sized {
    /// The file the user points at: `*.<ext>`, or an exact name like "index.json".
    const MAIN: &'static str;

    /// `main` is the name of the file the user points at, `names` every file beside it.
    /// Fails if a required file is missing.
    fn find(main: &str, names: &[String]) -> Result<Self>;

    fn is_main(name: &str) -> bool {
        match Self::MAIN.strip_prefix("*.") {
            Some(ext) => has_extension(name, ext),
            None => name == Self::MAIN,
        }
    }

    /// `path` is either the main file, or a zip holding it.
    fn scan(path: &Path) -> Result<(Source, Self)> {
        let zipped = has_extension(path, "zip");
        let source = if zipped {
            Source::zip(path)?
        } else {
            Source::Dir(parent_dir(path).to_path_buf())
        };
        let names = source.names()?;

        let mains: Vec<String> = if zipped {
            names.clone()
        } else {
            vec![
                path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
            ]
        };
        let mut mains = mains.into_iter().filter(|name| Self::is_main(name));
        let main = match (mains.next(), mains.next()) {
            (Some(main), None) => main,
            (None, _) => bail!("No {} found at {}", Self::MAIN, path.display()),
            (Some(_), Some(_)) => bail!("More than one {} in {}", Self::MAIN, path.display()),
        };

        let files = Self::find(&main, &names)?;
        Ok((source, files))
    }
}

/// Where the files of a dictionary live.
pub enum Source {
    Dir(PathBuf),
    Zip(ZipArchive<fs::File>),
}

impl Source {
    pub fn zip(path: &Path) -> Result<Self> {
        let archive = ZipArchive::new(fs::File::open(path)?)?;
        Ok(Self::Zip(archive))
    }

    /// The names of every file (not folders), in the zip's order or unordered for a folder.
    pub fn names(&self) -> Result<Vec<String>> {
        match self {
            Self::Dir(dir) => {
                let mut names = Vec::new();
                for entry in fs::read_dir(dir)? {
                    let entry = entry?;
                    if entry.path().is_file() {
                        names.push(entry.file_name().to_string_lossy().into_owned());
                    }
                }
                Ok(names)
            }
            Self::Zip(archive) => Ok(archive
                .file_names()
                .filter(|name| !name.ends_with('/'))
                .map(String::from)
                .collect()),
        }
    }

    pub fn open(&mut self, name: &str) -> Result<Box<dyn Read>> {
        match self {
            Self::Dir(dir) => Ok(Box::new(fs::File::open(inside(dir, name)?)?)),
            // A zip entry borrows the archive, so read it whole
            Self::Zip(_) => Ok(Box::new(Cursor::new(self.read(name)?))),
        }
    }

    pub fn read(&mut self, name: &str) -> Result<Vec<u8>> {
        match self {
            Self::Dir(dir) => Ok(fs::read(inside(dir, name)?)?),
            Self::Zip(archive) => {
                let mut bytes = Vec::new();
                archive.by_name(name)?.read_to_end(&mut bytes)?;
                Ok(bytes)
            }
        }
    }
}

/// `dir/name`, refusing names that would leave `dir`, since they may come from the dictionary.
fn inside(dir: &Path, name: &str) -> Result<PathBuf> {
    let plain = |part| matches!(part, Component::Normal(_));
    if !Path::new(name).components().all(plain) {
        bail!("Refusing to read {name} outside of {}", dir.display());
    }
    Ok(dir.join(name))
}

pub fn has_extension(name: impl AsRef<Path>, ext: &str) -> bool {
    name.as_ref()
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(ext))
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
