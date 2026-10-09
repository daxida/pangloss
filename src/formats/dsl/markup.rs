//! DSL card body markup to html.
//!
//! The html is pyglossary's ([lex.py], [transform.py]) byte for byte, since many
//! dictionaries in the wild were made with it. Its handling of broken markup is not
//! followed: a `[` not closed on its line is kept as text rather than dropping the card.
//!
//! [lex.py]: https://github.com/ilius/pyglossary/blob/master/pyglossary/plugins/dsl/lex.py
//! [transform.py]: https://github.com/ilius/pyglossary/blob/master/pyglossary/plugins/dsl/transform.py

use std::sync::LazyLock;

use regex::Regex;

/// `{{...}}`: a comment.
static COMMENT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{\{[^}]*\}\}").unwrap());

/// What `[s]` renders as an `<img>`.
const IMAGE_EXTENSIONS: [&str; 10] = [
    "bmp", "gif", "ico", "jpeg", "jpg", "png", "svg", "tif", "tiff", "webp",
];

/// What `[s]` renders as an audio `<object>`. pyglossary stops at wav and mp3, which
/// would leave the Forvo pronunciation dictionaries (opus, ogg) silent.
const AUDIO_EXTENSIONS: [&str; 6] = ["wav", "mp3", "ogg", "oga", "opus", "flac"];

const LABEL_OPEN: &str = r#"<i class="p"><font color="green">"#;
const LABEL_CLOSE: &str = "</font></i>";

pub struct Html {
    pub text: String,
    /// The media files that `[s]` refers to, in order of appearance.
    pub resources: Vec<String>,
    /// What was not understood, left for the caller to tally: a dictionary can
    /// repeat the same unknown tag in every card.
    pub warnings: Vec<String>,
}

/// The `card` body as html, with `~` standing for `headword`.
pub fn to_html(card: &str, headword: &str) -> Html {
    let card = COMMENT_RE.replace_all(card, "");
    let mut scanner = Scanner {
        rest: &card,
        headword,
        html: Html {
            text: String::new(),
            resources: Vec::new(),
            warnings: Vec::new(),
        },
        label: None,
    };
    scanner.run();
    scanner.html
}

struct Scanner<'a> {
    rest: &'a str,
    headword: &'a str,
    html: Html,
    /// Where the open `[p]` label starts in the html: it is wrapped once closed.
    label: Option<usize>,
}

impl Scanner<'_> {
    fn bump(&mut self) -> Option<char> {
        let c = self.rest.chars().next()?;
        self.rest = &self.rest[c.len_utf8()..];
        Some(c)
    }

    fn eat(&mut self, prefix: &str) -> bool {
        self.rest
            .strip_prefix(prefix)
            .map(|rest| self.rest = rest)
            .is_some()
    }

    fn push(&mut self, html: &str) {
        self.html.text.push_str(html);
    }

    fn run(&mut self) {
        while let Some(c) = self.bump() {
            match c {
                '\\' => match self.bump() {
                    None => self.push("\\"),
                    Some(' ') => self.push("&nbsp;"),
                    Some(c @ ('<' | '>')) if self.rest.starts_with(c) => {
                        self.bump();
                        self.push(if c == '<' { "&lt;&lt;" } else { "&gt;&gt;" });
                    }
                    Some(c) => push_escaped_char(&mut self.html.text, c),
                },
                '\n' => {
                    self.rest = self.rest.trim_start_matches([' ', '\t']);
                    // A line break, unless the next line opens a paragraph
                    if !self.rest.is_empty() && !self.rest.starts_with("[m") {
                        self.push("<br/>");
                    }
                }
                '~' => push_escaped(&mut self.html.text, self.headword),
                '<' if self.eat("<") => self.link(None),
                '[' => match tag_end(self.rest) {
                    Some(end) => {
                        let tag = &self.rest[..end];
                        self.rest = &self.rest[end + 1..];
                        self.tag(tag);
                    }
                    None => self.push("["),
                },
                c => push_escaped_char(&mut self.html.text, c),
            }
        }
        self.close_label();
    }

    fn tag(&mut self, tag: &str) {
        if let Some(closed) = tag.strip_prefix('/') {
            return self.close_tag(closed);
        }
        let (name, attr) = match tag.split_once([' ', '\t']) {
            Some((name, attr)) => (name, Some(parse_attr(attr))),
            None => (tag, None),
        };
        let target = match &attr {
            Some(("target", Some(target))) if !target.is_empty() => Some(target.as_str()),
            _ => None,
        };
        let html = match name {
            "ref" => return self.link(target),
            "url" => return self.url(target),
            "s" => return self.media(),
            "c" => {
                let color = match attr {
                    Some((color, None)) => color,
                    _ => "green",
                };
                &format!(r#"<font color="{color}">"#)
            }
            m if m.starts_with('m') && m[1..].bytes().all(|b| b.is_ascii_digit()) => {
                let padding = match &m[1..] {
                    "" | "0" => "0.3",
                    n => n,
                };
                &format!(r#"<p style="padding-left:{padding}em;margin:0">"#)
            }
            "p" => {
                self.label.get_or_insert(self.html.text.len());
                ""
            }
            "*" => r#"<span class="sec">"#,
            "ex" => r#"<span class="ex"><font color="steelblue">"#,
            "t" => r#"<font face="Helvetica" class="dsl_t">"#,
            "i" => "<i>",
            "b" => "<b>",
            "u" => "<u>",
            "'" => r#"<u class="accent">"#,
            "sup" => "<sup>",
            "sub" => "<sub>",
            // Not in the spec nor in pyglossary, but goldendict-ng breaks the line
            "br" => "<br/>",
            "trn" | "!trn" | "trs" | "!trs" | "lang" | "com" => "",
            _ => {
                self.html.warnings.push(format!("Unknown tag [{name}]"));
                ""
            }
        };
        self.push(html);
    }

    fn close_tag(&mut self, tag: &str) {
        let html = match tag {
            "m" => "</p>",
            "b" => "</b>",
            "u" | "'" => "</u>",
            "i" => "</i>",
            "sup" => "</sup>",
            "sub" => "</sub>",
            "c" | "t" => "</font>",
            "p" => {
                self.label.get_or_insert(self.html.text.len());
                return self.close_label();
            }
            "*" => "</span>",
            "ex" => "</font></span>",
            "ref" | "url" | "s" | "trn" | "!trn" | "trs" | "!trs" | "lang" | "com" => "",
            _ => {
                self.html
                    .warnings
                    .push(format!("Unknown closing tag [/{tag}]"));
                ""
            }
        };
        self.push(html);
    }

    // TODO: pyglossary shows what the _abrv.dsl says a label stands for as a tooltip.
    // The dictionaries we check against were made before it did.
    fn close_label(&mut self) {
        if let Some(start) = self.label.take() {
            self.html.text.insert_str(start, LABEL_OPEN);
            self.push(LABEL_CLOSE);
        }
    }

    /// The text up to the next tag, unescaped. `<<` links also end at `>>`.
    fn text(&mut self, until_angles: bool) -> String {
        let mut text = String::new();
        while !(self.rest.starts_with('[') || (until_angles && self.eat(">>"))) {
            match self.bump() {
                Some('\\') => text.extend(self.bump()),
                Some(c) => text.push(c),
                None => break,
            }
        }
        text
    }

    /// `[ref]headword[/ref]` or `<<headword>>`, a link to another card.
    fn link(&mut self, target: Option<&str>) {
        let text = self.text(true);
        let target = target.unwrap_or(&text);
        let html = format!(
            "<a href={}>{}</a>",
            quote_attr(&format!("bword://{target}")),
            escape(&text)
        );
        self.push(&html);
    }

    fn url(&mut self, target: Option<&str>) {
        let text = self.text(false);
        let mut href = target.unwrap_or(&text).to_string();
        if !href.contains("://") {
            href.insert_str(0, "http://");
        }
        let html = format!("<a href={}>{}</a>", quote_attr(&href), escape(&text));
        self.push(&html);
    }

    /// `[s]file[/s]`, a picture or a sound.
    fn media(&mut self) {
        // pyglossary takes escapes literally, but goldendict-ng does not: Forvo names
        // files like "\[...\] words.mp3", stored as "[...] words.mp3".
        let fname = self.text(false);
        let src = fname.replace('"', "&quot;");
        let ext = extension(&fname);
        if AUDIO_EXTENSIONS.contains(&ext) {
            self.push(&format!(
                r#"<object type="audio/x-wav" data="{src}" width="40" height="40"><param name="autoplay" value="false" /></object>"#
            ));
        } else if IMAGE_EXTENSIONS.contains(&ext) {
            self.push(&format!(r#"<img align="top" src="{src}" alt="{src}" />"#));
        } else {
            let warning = format!("Unknown file extension in [s]{fname:?}");
            self.html.warnings.push(warning);
        }
        self.html.resources.push(fname);
    }
}

/// Where the tag at the start of `rest` ends, at its `]`. None when there is no tag:
/// empty, or cut by a new line or another `[`. Escaped characters don't count.
fn tag_end(rest: &str) -> Option<usize> {
    let mut chars = rest.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            ']' if i > 0 => return Some(i),
            ']' | '[' | '\n' => return None,
            '\\' => {
                chars.next();
            }
            _ => (),
        }
    }
    None
}

/// The attribute of a tag, as pyglossary reads it: a name up to `=`, spaces included,
/// so that `[c dark green]` is one valueless attribute. A value ends at its closing
/// quote, or unquoted at a blank, and can escape characters with `\`.
fn parse_attr(attr: &str) -> (&str, Option<String>) {
    let attr = attr.trim_start_matches([' ', '\t']);
    let Some((name, value)) = attr.split_once('=') else {
        return (attr, None);
    };
    let value = value.trim_start_matches([' ', '\t']);
    let quote = value.chars().next().filter(|c| matches!(c, '"' | '\''));
    let mut chars = value[quote.map_or(0, char::len_utf8)..].chars();
    let mut unescaped = String::new();
    while let Some(c) = chars.next() {
        match c {
            '\\' => unescaped.extend(chars.next()),
            c if quote.map_or(matches!(c, ' ' | '\t'), |quote| c == quote) => break,
            c => unescaped.push(c),
        }
    }
    (name, Some(unescaped))
}

/// Python's `xml.sax.saxutils.escape`: quotes are left alone.
pub fn escape(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    push_escaped(&mut escaped, text);
    escaped
}

fn push_escaped(out: &mut String, text: &str) {
    for c in text.chars() {
        push_escaped_char(out, c);
    }
}

pub fn push_escaped_char(out: &mut String, c: char) {
    match c {
        '&' => out.push_str("&amp;"),
        '<' => out.push_str("&lt;"),
        '>' => out.push_str("&gt;"),
        c => out.push(c),
    }
}

/// Python's `xml.sax.saxutils.quoteattr`: the value quoted, with whichever quotes it lacks.
fn quote_attr(value: &str) -> String {
    let value = escape(value)
        .replace('\n', "&#10;")
        .replace('\r', "&#13;")
        .replace('\t', "&#9;");
    match (value.contains('"'), value.contains('\'')) {
        (false, _) => format!("\"{value}\""),
        (true, false) => format!("'{value}'"),
        (true, true) => format!("\"{}\"", value.replace('"', "&quot;")),
    }
}

/// Python's `os.path.splitext`, without the dot: leading dots of the name don't count.
fn extension(fname: &str) -> &str {
    let name = fname.rsplit('/').next().unwrap_or(fname);
    name.trim_start_matches('.')
        .rsplit_once('.')
        .map_or("", |(_, ext)| ext)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trimmed, as the reader does.
    fn html(card: &str) -> String {
        to_html(card, "word").text.trim().to_string()
    }

    #[test]
    fn formatting() {
        assert_eq!(html("[b]bold[/b] [i]it[/i]."), "<b>bold</b> <i>it</i>.");
        assert_eq!(
            html("[c]x[/c][c red]y[/c][c dark green]z[/c]."),
            r#"<font color="green">x</font><font color="red">y</font><font color="dark green">z</font>."#
        );
        assert_eq!(
            html("[m1]a[/m]\n\t[m]b[/m]\n"),
            r#"<p style="padding-left:1em;margin:0">a</p><p style="padding-left:0.3em;margin:0">b</p>"#
        );
    }

    #[test]
    fn labels_wrap_whole() {
        assert_eq!(
            html("[p]n.[/p] x [p]open"),
            concat!(
                r#"<i class="p"><font color="green">n.</font></i> x "#,
                r#"<i class="p"><font color="green">open</font></i>"#
            )
        );
    }

    #[test]
    fn newlines_break_unless_a_paragraph_follows() {
        assert_eq!(html("a\n\tb.\n"), "a<br/>b.");
        assert_eq!(
            html("a\n\t[m1]b[/m]."),
            r#"a<p style="padding-left:1em;margin:0">b</p>."#
        );
    }

    #[test]
    fn escapes_and_headword() {
        assert_eq!(html(r"\[x\] ~ a&b."), "[x] word a&amp;b.");
        assert_eq!(html(r"a\ b \<\< c."), "a&nbsp;b &lt;&lt; c.");
    }

    #[test]
    fn links() {
        let cases = [
            (
                "see [ref]other[/ref].",
                r#"see <a href="bword://other">other</a>."#,
            ),
            (
                "see <<other>>.",
                r#"see <a href="bword://other">other</a>."#,
            ),
            (
                r#"[ref target="x y"]shown[/ref]"#,
                r#"<a href="bword://x y">shown</a>"#,
            ),
            // Escapes in attribute values, as pyglossary reads them
            (
                r#"[ref target="a\]b"]x[/ref]"#,
                r#"<a href="bword://a]b">x</a>"#,
            ),
            (
                r#"[ref target="a\"b"]x[/ref]"#,
                r#"<a href='bword://a"b'>x</a>"#,
            ),
            (
                "[url]example.org[/url].",
                r#"<a href="http://example.org">example.org</a>."#,
            ),
        ];
        for (card, expected) in cases {
            assert_eq!(html(card), expected, "{card}");
        }
    }

    #[test]
    fn media() {
        let out = to_html(r"[s]pic.png[/s] [s]\[x\] a.opus[/s].", "");
        assert_eq!(
            out.text,
            concat!(
                r#"<img align="top" src="pic.png" alt="pic.png" /> "#,
                r#"<object type="audio/x-wav" data="[x] a.opus" width="40" height="40">"#,
                r#"<param name="autoplay" value="false" /></object>."#
            )
        );
        assert_eq!(out.resources, ["pic.png", "[x] a.opus"]);
    }

    #[test]
    fn comments_and_hidden_zones_are_dropped() {
        assert_eq!(
            html(r#"a{{note}}[com]b[/com][trn]c[/trn][lang name="Greek"]d[/lang]."#),
            "abcd."
        );
    }

    #[test]
    fn broken_markup_is_kept_as_text() {
        assert_eq!(html("a [b"), "a [b");
        // Not closed on its line, so not swallowing the next one
        assert_eq!(html("see [1\n\tnext [b]y[/b]"), "see [1<br/>next <b>y</b>");
        assert_eq!(html("a ] b []"), "a ] b []");
    }

    #[test]
    fn unknown_tags_are_dropped_and_reported() {
        let out = to_html("a[zz]b[/zz][br]c.", "");
        assert_eq!(out.text, "ab<br/>c.");
        assert_eq!(
            out.warnings,
            ["Unknown tag [zz]", "Unknown closing tag [/zz]"]
        );
    }

    #[test]
    fn quote_attr_picks_quotes() {
        assert_eq!(quote_attr("a"), "\"a\"");
        assert_eq!(quote_attr("a\"b"), "'a\"b'");
        assert_eq!(quote_attr("a\"b'c"), "\"a&quot;b'c\"");
    }

    #[test]
    fn extension_is_splitext() {
        assert_eq!(extension("a/b.png"), "png");
        assert_eq!(extension(".hidden"), "");
        assert_eq!(extension("x.tar.gz"), "gz");
    }
}
