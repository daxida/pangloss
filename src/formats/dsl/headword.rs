//! DSL headword lines, as pyglossary's [title.py] reads them:
//! - `(optional)` parts: indexed with them and without them,
//! - `{unsorted}` parts: shown in the card title, but not indexed,
//! - `\` escapes.
//!
//! [title.py]: https://github.com/ilius/pyglossary/blob/master/pyglossary/plugins/dsl/title.py

use std::str::Chars;

use crate::formats::dsl::markup;

#[derive(Default)]
pub struct Headword {
    /// With the optional parts.
    pub term: String,
    /// Without the optional parts.
    pub alt: String,
    /// What the card shows: everything, unsorted parts included.
    pub title: String,
}

pub fn parse(line: &str) -> Headword {
    let mut headword = Headword::default();
    let mut chars = line.chars();
    while let Some(c) = chars.next() {
        match c {
            '(' => {
                // As pyglossary, escapes in an optional part keep their backslash
                let optional = markup::escape(&raw_until(&mut chars, ')'));
                headword.term.push_str(&optional);
                headword.title.push_str(&optional);
            }
            '{' => {
                let unsorted = markup::to_html(&raw_until(&mut chars, '}'), "");
                headword.title.push_str(&unsorted.text);
            }
            c => {
                let c = if c == '\\' {
                    chars.next().unwrap_or(c)
                } else {
                    c
                };
                for part in [&mut headword.term, &mut headword.alt, &mut headword.title] {
                    markup::push_escaped_char(part, c);
                }
            }
        }
    }
    headword
}

/// The text up to `close`, which is consumed, with its escapes as they are.
fn raw_until(chars: &mut Chars, close: char) -> String {
    let mut text = String::new();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                text.push('\\');
                text.extend(chars.next());
            }
            c if c == close => break,
            c => text.push(c),
        }
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts(line: &str) -> (String, String, String) {
        let h = parse(line);
        (h.term, h.alt, h.title)
    }

    #[test]
    fn plain() {
        let (term, alt, title) = parts("word\n");
        assert_eq!(
            (term.trim(), alt.trim(), title.trim()),
            ("word", "word", "word")
        );
    }

    #[test]
    fn optional_parts_give_two_terms() {
        let (term, alt, title) = parts("colo(u)r\n");
        assert_eq!(term.trim(), "colour");
        assert_eq!(alt.trim(), "color");
        assert_eq!(title.trim(), "colour");
    }

    #[test]
    fn unsorted_parts_only_show_in_the_title() {
        let (term, alt, title) = parts("to {[i]}go{[/i]}\n");
        assert_eq!(term.trim(), "to go");
        assert_eq!(alt.trim(), "to go");
        assert_eq!(title.trim(), "to <i>go</i>");
    }

    #[test]
    fn escapes() {
        let (term, _, _) = parts("a\\(b\\) & c\n");
        assert_eq!(term.trim(), "a(b) &amp; c");
    }
}
