use std::{
    collections::HashSet,
    fmt::Write as _,
    io::{Cursor, Read},
    path::Path,
};

use anyhow::{Context as _, Result};
use indexmap::IndexMap;
use rayon::prelude::*;
use zip::ZipArchive;

use crate::{
    Context, DataEntry, Reader,
    formats::dsl::{DslFormat, files::DslFiles, headword, markup},
    glossary::{AltEntry, Definition, Entry, Glossary, GlossaryInfo},
    scan::{DictionaryFiles, Source, zip_file_names},
};

impl Reader for DslFormat {
    fn read_with_context(&self, path: &Path, ctx: &Context) -> Result<Glossary> {
        read_with_context(path, ctx)
    }
}

fn read_with_context(path: &Path, _: &Context) -> Result<Glossary> {
    let (mut source, files) = DslFiles::scan(path)?;

    let mut text = decode(source.read_possibly_compressed(&files.dsl)?)
        .with_context(|| format!("Failed to decode {}", files.dsl))?;
    // As Python's universal newlines, which pyglossary reads with
    if text.contains('\r') {
        text = text.replace("\r\n", "\n").replace('\r', "\n");
    }

    let (mut info, body) = read_header(&text, files.name());
    if let Some(ann) = &files.ann {
        match decode(source.read(ann)?) {
            Ok(description) if !description.trim().is_empty() => {
                info.insert("description", description.trim().replace("\r\n", "\n"));
            }
            Ok(_) => (),
            Err(err) => tracing::warn!("Ignoring {ann}: {err}"),
        }
    }

    let cards: Vec<_> = split_cards(body).par_iter().map(Card::to_entries).collect();
    let mut entries = Vec::new();
    let mut referenced = HashSet::new();
    let mut warnings: IndexMap<String, usize> = IndexMap::new();
    for card in cards {
        entries.extend(card.entries);
        referenced.extend(card.resources);
        for warning in card.warnings {
            *warnings.entry(warning).or_default() += 1;
        }
    }
    for (warning, count) in warnings {
        tracing::warn!("{warning} ({count} times)");
    }

    let data_entries = read_resources(&mut source, &files, referenced)?;

    Ok(Glossary {
        entries,
        data_entries,
        info,
        ..Default::default()
    })
}

/// The text, by its byte order mark, else UTF-8.
//
// TODO: the spec allows ANSI too, naming its code page in #SOURCE_CODE_PAGE.
fn decode(bytes: Vec<u8>) -> Result<String> {
    let utf16 = |bytes: &[u8], from_bytes: fn([u8; 2]) -> u16| {
        let units = bytes.as_chunks().0.iter().copied().map(from_bytes);
        char::decode_utf16(units)
            .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect::<String>()
    };
    if let Some(rest) = bytes.strip_prefix(b"\xFF\xFE") {
        Ok(utf16(rest, u16::from_le_bytes))
    } else if let Some(rest) = bytes.strip_prefix(b"\xFE\xFF") {
        Ok(utf16(rest, u16::from_be_bytes))
    } else {
        let mut text =
            String::from_utf8(bytes).context("Neither UTF-8, nor UTF-16 with a byte order mark")?;
        if text.starts_with('\u{feff}') {
            text.drain(..'\u{feff}'.len_utf8());
        }
        Ok(text)
    }
}

/// The info from the `#` lines at the top, and the text after them.
/// The dictionary goes by `name` unless the header names it.
fn read_header<'a>(text: &'a str, name: &str) -> (GlossaryInfo, &'a str) {
    let mut info = GlossaryInfo::new();
    info.insert("name", name.to_string());
    let mut header_len = 0;
    for line in text.split_inclusive('\n') {
        let line_len = line.len();
        let line = line.trim_end();
        if !line.is_empty() && !line.starts_with('#') {
            break;
        }
        header_len += line_len;
        let (directive, value) = line.split_once([' ', '\t']).unwrap_or((line, ""));
        let key = match directive {
            "#NAME" => "name",
            "#INDEX_LANGUAGE" => "sourceLang",
            "#CONTENTS_LANGUAGE" => "targetLang",
            // TODO: none of the dictionaries we have seen merges others in
            "#INCLUDE" => {
                tracing::warn!("Ignoring {line}: #INCLUDE is not supported");
                continue;
            }
            _ => continue,
        };
        info.insert(key, unwrap_quotes(value.trim()).to_string());
    }
    (info, &text[header_len..])
}

fn unwrap_quotes(s: &str) -> &str {
    let mut chars = s.chars();
    match (chars.next(), chars.next_back()) {
        (Some(open @ ('"' | '\'')), Some(close)) if open == close => chars.as_str(),
        _ => s,
    }
}

/// What a [`Card`] holds.
#[derive(Default)]
struct CardEntries {
    entries: Vec<Entry>,
    /// The media it refers to.
    resources: Vec<String>,
    warnings: Vec<String>,
}

/// A card: its headword lines, then its body lines, which are indented.
/// Lines keep their `\n`, as the markup tells line breaks from them.
#[derive(Default)]
struct Card<'a> {
    headwords: Vec<&'a str>,
    body: Vec<&'a str>,
}

fn split_cards(text: &str) -> Vec<Card<'_>> {
    let mut cards = Vec::new();
    let mut card = Card::default();
    for line in text.split_inclusive('\n') {
        if line.trim().is_empty() {
            continue;
        }
        if line.starts_with([' ', '\t']) {
            card.body.push(line);
            continue;
        }
        if !card.body.is_empty() {
            cards.push(std::mem::take(&mut card));
        }
        card.headwords.push(line);
    }
    // Headwords without a body are dropped
    if !card.body.is_empty() {
        cards.push(card);
    }
    cards
}

impl Card<'_> {
    /// The entries of the card.
    ///
    /// A card can hold subentries, each starting with an `@ headword` line and ending
    /// with an `@` one. They become entries of their own, linked from the card.
    fn to_entries(&self) -> CardEntries {
        let mut terms = Vec::new();
        let mut titles = Vec::new();
        for line in &self.headwords {
            let headword = headword::parse(line);
            let term = headword.term.trim();
            let alt = headword.alt.trim();
            let title = headword.title.trim();
            terms.push(term.to_string());
            if alt != term {
                terms.push(alt.to_string());
            }
            if title != term {
                titles.push(format!("<b>{title}</b>"));
            }
        }

        let mut main = String::new();
        let mut subentries = Vec::new();
        let mut subentry: Option<(String, String)> = None;
        for line in &self.body {
            let trimmed = line.trim();
            if trimmed == "@" || trimmed.starts_with("@ ") {
                subentries.extend(subentry.take());
                let term = trimmed[1..].trim();
                if !term.is_empty() {
                    // Writing to a String cannot fail
                    let _ = writeln!(main, "\t[m2][ref]{term}[/ref]");
                    subentry = Some((term.to_string(), String::new()));
                }
            } else if let Some((_, body)) = &mut subentry {
                body.push_str(line);
            } else {
                main.push_str(line);
            }
        }
        subentries.extend(subentry);

        let Some(first) = terms.first() else {
            tracing::warn!("Skipping a card without headwords: {main:?}");
            return CardEntries::default();
        };
        let mut out = CardEntries::default();
        let mut html = to_html(&main, first, &mut out);
        if !titles.is_empty() {
            titles.push(html);
            html = titles.join("<br/>");
        }

        out.entries.extend(new_entry(terms, html));
        for (term, body) in subentries {
            let html = to_html(&body, &term, &mut out);
            out.entries.extend(new_entry(vec![term], html));
        }
        out
    }
}

fn to_html(body: &str, headword: &str, out: &mut CardEntries) -> String {
    let html = markup::to_html(body, headword);
    out.resources.extend(html.resources);
    out.warnings.extend(html.warnings);
    // Trimmed in place, as cards can be long
    let mut text = html.text;
    text.truncate(text.trim_end().len());
    text.drain(..text.len() - text.trim_start().len());
    text
}

/// An entry for the first of `terms`, the rest being its alts.
/// Like pyglossary, which drops empty entries and repeated terms.
fn new_entry(terms: Vec<String>, html: String) -> Option<Entry> {
    if html.is_empty() {
        return None;
    }
    let mut unique: Vec<String> = Vec::new();
    for term in terms {
        if !term.is_empty() && !unique.contains(&term) {
            unique.push(term);
        }
    }
    let mut terms = unique.into_iter();
    let term = terms.next()?;
    let alts = terms.map(AltEntry::only_term).collect();
    Some(Entry::new(term, Definition::Html(html)).with_alts(alts))
}

/// Everything in `foo.dsl.files.zip`, and the `referenced` media found beside the .dsl.
fn read_resources(
    source: &mut Source,
    files: &DslFiles,
    mut referenced: HashSet<String>,
) -> Result<Vec<DataEntry>> {
    let mut data_entries = Vec::new();

    if let Some(resources) = &files.resources {
        let bytes = source.read(resources)?;
        let mut archive = ZipArchive::new(Cursor::new(bytes))
            .with_context(|| format!("Failed to open {resources}"))?;
        let mut names = zip_file_names(&archive)?;
        names.sort();
        for name in names {
            let mut file = archive.by_name(&name)?;
            let mut bytes = Vec::new();
            let _ = bytes.try_reserve_exact(usize::try_from(file.size()).unwrap_or(0));
            file.read_to_end(&mut bytes)?;
            referenced.remove(&name);
            data_entries.push(DataEntry::new(name, bytes));
        }
    }

    // The source names files relative to where it starts, not to the .dsl
    let dir = files.dsl.rsplit_once('/').map_or("", |(dir, _)| dir);
    let mut referenced: Vec<_> = referenced.into_iter().collect();
    referenced.sort();
    let mut missing = 0;
    for fname in referenced {
        let name = if dir.is_empty() {
            fname.clone()
        } else {
            format!("{dir}/{fname}")
        };
        match source.read(&name) {
            Ok(bytes) => data_entries.push(DataEntry::new(fname, bytes)),
            Err(err) => {
                tracing::debug!("Media not found: {fname}: {err}");
                missing += 1;
            }
        }
    }
    if missing > 0 {
        tracing::warn!("{missing} media files that cards refer to were not found");
    }

    Ok(data_entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entries(text: &str) -> Vec<(String, Vec<String>, String)> {
        split_cards(text)
            .iter()
            .flat_map(|card| card.to_entries().entries)
            .map(|entry| {
                let alts = entry.alts().iter().map(|a| a.term().to_string()).collect();
                (entry.term().to_string(), alts, entry.definition().to_text())
            })
            .collect()
    }

    #[test]
    fn header() {
        let (info, rest) = read_header(
            "#NAME\t\"My Dict\"\n#INDEX_LANGUAGE \"Greek\"\n#CONTENTS_LANGUAGE\t\"English\"\n\nword\n\tdef\n",
            "file",
        );
        assert_eq!(info.name(), "My Dict");
        assert_eq!(info.get("sourceLang"), Some("Greek"));
        assert_eq!(info.get("targetLang"), Some("English"));
        assert_eq!(rest, "word\n\tdef\n");
        assert_eq!(read_header("word\n\tdef\n", "file").0.name(), "file");
    }

    #[test]
    fn cards_with_several_headwords() {
        let got = entries("one\nuno\n\t[b]1[/b]\n\n\ntwo\n\t2\n\tmore\n");
        assert_eq!(
            got,
            [
                ("one".into(), vec!["uno".into()], "<b>1</b>".into()),
                ("two".into(), vec![], "2<br/>more".into()),
            ]
        );
    }

    #[test]
    fn optional_parts_and_titles() {
        let got = entries("colo(u)r\n\tx\n{to }go\n\ty\n");
        assert_eq!(
            got,
            [
                ("colour".into(), vec!["color".into()], "x".into()),
                ("go".into(), vec![], "<b>to go</b><br/>y".into()),
            ]
        );
    }

    #[test]
    fn subentries() {
        let got = entries("main\n\tbody\n\t@ sub\n\tsub body ~\n\t@\n\tafter\n");
        assert_eq!(
            got,
            [
                (
                    "main".into(),
                    vec![],
                    // The link goes where the subentry was, as an unclosed paragraph
                    r#"body<p style="padding-left:2em;margin:0"><a href="bword://sub">sub</a><br/>after"#
                        .into()
                ),
                ("sub".into(), vec![], "sub body sub".into()),
            ]
        );
    }

    #[test]
    fn decodes_by_bom() {
        let utf16: Vec<u8> = [0xFF, 0xFE]
            .into_iter()
            .chain("#NAME".encode_utf16().flat_map(u16::to_le_bytes))
            .collect();
        assert_eq!(decode(utf16).unwrap(), "#NAME");
        assert_eq!(decode(b"\xEF\xBB\xBF#NAME".to_vec()).unwrap(), "#NAME");
        assert_eq!(decode(b"#NAME".to_vec()).unwrap(), "#NAME");
    }
}
