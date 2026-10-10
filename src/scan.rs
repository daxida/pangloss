//! Finding the files of a dictionary: beside its main file, or in a container holding it.

use std::{
    collections::HashMap,
    fs,
    io::{BufReader, Cursor, Read, Seek},
    path::{Component, Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use sevenz_rust2::{ArchiveReader, Password};
use zip::ZipArchive;

use crate::utils::{files_under, parent_dir};

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

    /// `path` is either the main file, or a container holding it.
    fn scan(path: &Path) -> Result<(Source, Self)> {
        let container = Source::container(path)?;
        let contained = container.is_some();
        let source = container.unwrap_or_else(|| Source::Dir(parent_dir(path).to_path_buf()));
        let names = source.names()?;

        let mains: Vec<String> = if contained {
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
    /// Decoded whole on the first read: in a solid archive, reading an entry decodes
    /// everything before it, so reading entries one by one decodes the start over and over.
    SevenZ {
        // Boxed, it is much bigger than the other variants
        reader: Box<ArchiveReader<BufReader<fs::File>>>,
        entries: Option<HashMap<String, Vec<u8>>>,
    },
}

/// Every file in the archive, decoded in one pass.
fn decode_all(reader: &mut ArchiveReader<BufReader<fs::File>>) -> Result<HashMap<String, Vec<u8>>> {
    let mut entries = HashMap::new();
    reader.for_each_entries(|entry, data| {
        if !entry.is_directory() {
            // Sized up front, unless the archive claims an impossible size
            let mut bytes = Vec::new();
            let _ = bytes.try_reserve_exact(usize::try_from(entry.size()).unwrap_or(0));
            data.read_to_end(&mut bytes)?;
            entries.insert(entry.name().to_string(), bytes);
        }
        Ok(true)
    })?;
    Ok(entries)
}

impl Source {
    /// The container at `path`, if it is one we can look into.
    pub fn container(path: &Path) -> Result<Option<Self>> {
        if path.is_dir() {
            Ok(Some(Self::Dir(path.to_path_buf())))
        } else if has_extension(path, "zip") {
            Ok(Some(Self::Zip(ZipArchive::new(fs::File::open(path)?)?)))
        } else if has_extension(path, "7z") {
            // Buffered: the PPMd decoder pulls its input one byte at a time
            let file = BufReader::new(fs::File::open(path)?);
            let reader = Box::new(ArchiveReader::new(file, Password::empty())?);
            Ok(Some(Self::SevenZ {
                reader,
                entries: None,
            }))
        } else {
            Ok(None)
        }
    }

    /// The names of every file (not folders), in the archive's order or unordered for a folder.
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
            Self::Zip(archive) => zip_file_names(archive),
            Self::SevenZ { reader, .. } => Ok(reader
                .archive()
                .files
                .iter()
                .filter(|entry| !entry.is_directory())
                .map(|entry| entry.name().to_string())
                .collect()),
        }
    }

    /// The files under `dir` at any depth, sorted, named as in [`Self::names`].
    pub fn names_under(&self, dir: &str) -> Result<Vec<String>> {
        let prefix = format!("{dir}/");
        let mut names = match self {
            Self::Dir(root) if !root.join(dir).is_dir() => Vec::new(),
            Self::Dir(root) => files_under(&root.join(dir))?
                .iter()
                .map(|path| {
                    path.strip_prefix(root)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .replace('\\', "/")
                })
                .collect(),
            Self::Zip(_) | Self::SevenZ { .. } => self.names()?,
        };
        names.retain(|name| name.starts_with(&prefix));
        names.sort();
        Ok(names)
    }

    pub fn open(&mut self, name: &str) -> Result<Box<dyn Read>> {
        match self {
            Self::Dir(dir) => Ok(Box::new(fs::File::open(inside(dir, name)?)?)),
            // An entry borrows the archive, so read it whole
            Self::Zip(_) | Self::SevenZ { .. } => Ok(Box::new(Cursor::new(self.read(name)?))),
        }
    }

    /// The bytes of `name`, decompressed if it is a `.gz`, or a `.dz` (dictzip is gzip).
    pub fn read_possibly_compressed(&mut self, name: &str) -> Result<Vec<u8>> {
        let mut reader = self.open(name)?;
        if has_extension(name, "dz") || has_extension(name, "gz") {
            reader = Box::new(flate2::read::GzDecoder::new(reader));
        }
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf)?;
        Ok(buf)
    }

    pub fn read(&mut self, name: &str) -> Result<Vec<u8>> {
        match self {
            Self::Dir(dir) => Ok(fs::read(inside(dir, name)?)?),
            Self::Zip(archive) => {
                let mut bytes = Vec::new();
                archive.by_name(name)?.read_to_end(&mut bytes)?;
                Ok(bytes)
            }
            Self::SevenZ { reader, entries } => {
                if entries.is_none() {
                    *entries = Some(decode_all(reader)?);
                }
                let bytes = entries.as_ref().and_then(|entries| entries.get(name));
                Ok(bytes
                    .with_context(|| format!("No {name} in the 7z"))?
                    .clone())
            }
        }
    }
}

/// The names of every file (not folder) in a zip, in the archive's order.
pub fn zip_file_names<R: Read + Seek>(archive: &ZipArchive<R>) -> Result<Vec<String>> {
    let mut names = Vec::new();
    for name in archive.file_names() {
        let name = name?;
        if !name.ends_with('/') {
            names.push(name.into_owned());
        }
    }
    Ok(names)
}

/// `dir/name`, refusing names that would leave `dir`, since they may come from the dictionary.
pub fn inside(dir: &Path, name: &str) -> Result<PathBuf> {
    let plain = |part| matches!(part, Component::Normal(_));
    if !Path::new(name).components().all(plain) {
        bail!("{name} is outside of {}", dir.display());
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
