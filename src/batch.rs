//! Converting every dictionary under a folder.

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::{ReaderFormat, WriterFormat, utils::files_under};

/// Every dictionary under `root`, sorted, with its format.
///
/// Without `rformat`, Text and Json are not looked for: every Yomitan bank would pass for Json.
pub fn find_dictionaries(
    root: &Path,
    rformat: Option<ReaderFormat>,
) -> Result<Vec<(PathBuf, ReaderFormat)>> {
    let wanted = |format| match rformat {
        Some(rformat) => format == rformat,
        None => !matches!(format, ReaderFormat::Text | ReaderFormat::Json),
    };
    Ok(files_under(root)?
        .into_iter()
        .filter_map(|path| {
            let format = ReaderFormat::try_from_path(&path)?;
            wanted(format).then_some((path, format))
        })
        .collect())
}

/// Where `input`, found under `root`, goes under `out_root`: the same place, with the
/// extension of `wformat`.
pub fn output_path(root: &Path, input: &Path, out_root: &Path, wformat: WriterFormat) -> PathBuf {
    let mut named = input.strip_prefix(root).unwrap_or(input).to_path_buf();
    // An unzipped Yomitan is named after its folder
    if named.ends_with("index.json") {
        named.pop();
    }
    if named.as_os_str().is_empty() {
        named = root.file_name().unwrap_or_default().into();
    }
    out_root.join(named).with_extension(wformat.extension())
}
