//! Putting the css files of a [`Glossary`] together.

use crate::{DataEntry, Glossary};

/// The bytes of all the css files in order, with a newline between two files that
/// don't already end in one. A single file comes out unchanged.
pub fn concat_css_files(glossary: &Glossary) -> Vec<u8> {
    let mut out = Vec::new();
    for (i, entry) in glossary.css_files().enumerate() {
        if i > 0 && !out.ends_with(b"\n") {
            out.push(b'\n');
        }
        out.extend_from_slice(entry.bytes());
    }
    out
}

/// Replace the css files with their [`concat_css_files`], so Mdict links one file per
/// entry instead of several. Skipped when the join could change the css (`@import`,
/// `@charset`, non UTF-8).
///
/// Only for glossaries whose definitions carry no css links of their own (Yomitan),
/// since those would be left pointing at the files that were merged away.
pub fn merge_css_files(glossary: &mut Glossary) {
    if glossary.css_files().nth(1).is_none() {
        return;
    }
    let joinable = glossary.css_files().all(|entry| {
        std::str::from_utf8(entry.bytes()).is_ok_and(|text| {
            let lower = text.to_ascii_lowercase();
            !lower.contains("@import") && !lower.contains("@charset")
        })
    });
    if !joinable {
        return;
    }

    let name = glossary.css_files().next().map(|e| e.fname().to_path_buf());
    let mut merged = name.map(|name| DataEntry::new(name, concat_css_files(glossary)));
    glossary.data_entries = std::mem::take(&mut glossary.data_entries)
        .into_iter()
        .filter_map(|entry| {
            if entry.is_css() {
                merged.take()
            } else {
                Some(entry)
            }
        })
        .collect();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn glossary(files: &[(&str, &str)]) -> Glossary {
        Glossary {
            data_entries: files
                .iter()
                .map(|(name, bytes)| DataEntry::new(*name, bytes.as_bytes().to_vec()))
                .collect(),
            ..Default::default()
        }
    }

    fn files(glossary: &Glossary) -> Vec<(String, String)> {
        glossary
            .data_entries
            .iter()
            .map(|e| {
                (
                    e.fname().to_string_lossy().into_owned(),
                    String::from_utf8_lossy(e.bytes()).into_owned(),
                )
            })
            .collect()
    }

    #[test]
    fn merge_keeps_order_and_position_of_the_first() {
        let mut g = glossary(&[
            ("a.css", ".a{}"),
            ("img.png", "png"),
            ("b.css", ".b{}\n"),
            ("c.css", ".c{}"),
        ]);
        merge_css_files(&mut g);
        assert_eq!(
            files(&g),
            [
                ("a.css".to_string(), ".a{}\n.b{}\n.c{}".to_string()),
                ("img.png".to_string(), "png".to_string()),
            ]
        );
    }

    #[test]
    fn merge_skips_files_that_cannot_be_joined() {
        for bad in ["@import 'x.css';", "@CHARSET \"utf-8\";"] {
            let mut g = glossary(&[("a.css", ".a{}"), ("b.css", bad)]);
            merge_css_files(&mut g);
            assert_eq!(files(&g).len(), 2, "{bad} should not be merged");
        }
        let mut g = glossary(&[("a.css", ".a{}")]);
        g.data_entries
            .push(DataEntry::new("b.css", vec![0xff, 0xfe, 0xfd]));
        merge_css_files(&mut g);
        assert_eq!(g.data_entries.len(), 2);
    }

    #[test]
    fn concat_separates_files_that_do_not_end_in_a_newline() {
        let single = glossary(&[("a.css", ".a{}")]);
        assert_eq!(concat_css_files(&single), b".a{}");

        let g = glossary(&[
            ("a.css", ".a{}"),
            ("b.css", ".b{}\n"),
            ("img.png", "png"),
            ("c.css", ".c{}"),
        ]);
        assert_eq!(concat_css_files(&g), b".a{}\n.b{}\n.c{}");
    }
}
