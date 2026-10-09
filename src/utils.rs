use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;

/// Like [`Path::parent`], but never empty.
///
/// A bare filename has `""` as its parent, which is neither a readable
/// directory nor a usable glob prefix. Readers use this to find a dictionary's
/// companion files (css, .idx, .syn, ...).
pub fn parent_dir(path: &Path) -> &Path {
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    }
}

/// Every file under `dir`, sorted, skipping hidden ones.
pub fn files_under(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut dirs = vec![dir.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            if entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
            } else {
                files.push(path);
            }
        }
    }
    files.sort();
    Ok(files)
}

pub fn unescape_html(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&amp;", "&")
}

pub fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parent_dir_of_a_bare_filename_is_the_current_dir() {
        assert_eq!(parent_dir(Path::new("dict.ifo")), Path::new("."));
        assert_eq!(parent_dir(Path::new("./dict.ifo")), Path::new("."));
        assert_eq!(parent_dir(Path::new("a/b/dict.ifo")), Path::new("a/b"));
        assert_eq!(parent_dir(Path::new("/dict.ifo")), Path::new("/"));
    }
}
