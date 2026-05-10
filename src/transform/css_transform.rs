//! Css selector tranforms.
//!
//! This is needed to convert css files from Mdict/Stardict, and any format
//! that uses html, to Yomitan's structured content.
//!
//! I tried lightningcss library, but they don't support an easy way of mutating
//! stylesheets (they also pull an obscene amount of dependencies). This custom
//! implementation should be enough, assuming the css we are given is valid.

use std::fmt::Write;

pub fn rewrite_css_classes(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut chars = css.chars().peekable();
    let mut depth = 0usize; // track whether we're inside a { } block

    while let Some(c) = chars.next() {
        match c {
            '{' => {
                depth += 1;
                out.push(c);
            }
            '}' => {
                depth = depth.saturating_sub(1);
                out.push(c);
            }
            '.' if depth == 0 => {
                let mut ident = String::new();
                while let Some(&nc) = chars.peek() {
                    if nc.is_alphanumeric() || nc == '-' || nc == '_' {
                        ident.push(nc);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if ident.is_empty() {
                    out.push('.');
                } else {
                    let _ = write!(out, r#"[data-sc-class="{ident}"]"#);
                }
            }
            _ => out.push(c),
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::rewrite_css_classes;

    #[test]
    fn simple_span_class() {
        let input = "span.grammar { color: red; }";
        let output = rewrite_css_classes(input);
        assert!(output.contains(r#"span[data-sc-class="grammar"]"#));
    }

    #[test]
    fn simple_div_class() {
        let input = "div.definition { margin-left: 10pt; }";
        let output = rewrite_css_classes(input);
        assert!(output.contains(r#"div[data-sc-class="definition"]"#));
    }

    #[test]
    fn multiple_rules() {
        let input = r"
            span.headword { font-size: 14pt; font-weight: bold; }
            span.grammar { font-style: italic; }
        ";
        let output = rewrite_css_classes(input);
        assert!(output.contains(r#"span[data-sc-class="headword"]"#));
        assert!(output.contains(r#"span[data-sc-class="grammar"]"#));
    }

    #[test]
    fn unchanged() {
        let input = "span { color: black; }";
        let output = rewrite_css_classes(input);
        assert!(output.contains("span"));
        assert!(!output.contains("data-sc-class"));
    }

    #[test]
    fn property_values_are_unchanged() {
        let input = "span.grammar { margin: .5em; opacity: .8; }";
        let output = rewrite_css_classes(input);
        assert!(output.contains(r#"span[data-sc-class="grammar"]"#));
        assert!(output.contains(".5em"));
        assert!(output.contains(".8"));
    }

    #[test]
    fn real_dictionary() {
        let input = r"
            div.head { margin-top: 10pt; font-family: Verdana; }
            div.definition { margin-left: 10pt; line-height: 140%; }
            span.headword { font-size: 14pt; font-weight: bold; color: #2049a4; }
            span.grammar { font-weight: bold; font-style: italic; color: #778899; }
            span.translation { font-weight: normal; color: black; }
        ";
        let output = rewrite_css_classes(input);
        assert!(output.contains(r#"div[data-sc-class="head"]"#));
        assert!(output.contains(r#"div[data-sc-class="definition"]"#));
        assert!(output.contains(r#"span[data-sc-class="headword"]"#));
        assert!(output.contains(r#"span[data-sc-class="grammar"]"#));
        assert!(output.contains(r#"span[data-sc-class="translation"]"#));
    }
}
