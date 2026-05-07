#![allow(clippy::match_same_arms)] // TODO: for now...

use crate::formats::yomitan::{
    TermBankEntry,
    model::{TagBankEntry, YomitanDefinition},
};

pub use conversion::HtmlConverter;

// Wrapping seems better than having definition as a different attrribute of Entry
// in order to force every caller to think about what they are adding/consuming to the glossary.
#[derive(Clone, Debug)]
pub enum Definition {
    // There should be another "Raw" variant, which is just text but we guarantee
    // that every conversion to it is trivial. This should fix the case where sometimes
    // we want html > text via tag removal, and sometimes html > text via identity
    // (just print the html) for debugging purposes etc.
    // At the moment Text *is* the "Raw" variant.
    Text(String),               // m (the default when we don't know)
    Html(String),               // h
    Yomitan(YomitanDefinition), // TODO: use box here: size too big
}

// TODO: better than cow would be to pass by value, which makes sense since they are
// "into" consuming versions. It does require some sort of consuming iteration over a Glossary.
impl Definition {
    // TODO: use cow, don't clone
    pub fn to_text(&self) -> String {
        match self {
            Self::Text(s) => s.clone(),
            // Intentionally lossy. We don't want html in a yomitan definition.
            // Self::Html(s) => strip_html(s),
            Self::Html(s) => s.clone(),
            Self::Yomitan(def) => match def {
                YomitanDefinition::TermBankEntry(term_bank_entry) => term_bank_entry
                    .definitions
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("\n"),
                YomitanDefinition::TermMetaBankEntry(_term_meta_bank_entry) => {
                    String::from("unimplemented to_text for TermMetaBankEntry")
                }
            },
        }
    }

    // TODO: use cow, don't clone
    // Only call this via HtmlConverter
    fn to_html(&self, tag_bank: Option<&[TagBankEntry]>) -> String {
        match self {
            Self::Text(s) => s.clone(),
            Self::Html(s) => s.clone(),
            Self::Yomitan(def) => match def {
                YomitanDefinition::TermBankEntry(term_bank_entry) => {
                    // Here we can get rid of the option since we know we are
                    // dealing with Yomitan.
                    term_bank_entry.to_html(tag_bank.unwrap_or_default())
                }
                YomitanDefinition::TermMetaBankEntry(term_meta_bank_entry) => {
                    term_meta_bank_entry.to_html() // TODO: use tags
                }
            },
        }
    }

    // We require access to the term to create a TermInformation
    pub fn to_yomitan(&self, term: &str) -> YomitanDefinition {
        debug_assert!(!term.is_empty());
        match self {
            Self::Text(s) => {
                YomitanDefinition::TermBankEntry(TermBankEntry::raw(term.to_string(), s.clone()))
            }
            Self::Html(s) => YomitanDefinition::TermBankEntry(TermBankEntry {
                term: term.to_string(),
                definitions: vec![conversion::html_to_structured_content(s)],
                ..Default::default()
            }),
            Self::Yomitan(defs) => defs.clone(),
        }
    }
}

// At some point, move this somewhere else
pub mod conversion {
    //! Conversion between definition kinds.
    //!
    //! This should be format-agnostic (even if the Yomitan kind is only used in the
    //! Yomitan format). For instance, html/xml is used across many dictionary formats.

    use std::{fmt::Write, path::PathBuf};

    use scraper::node::Element;
    use scraper::{ElementRef, Html, Node as ScraperNode};

    use crate::{Definition, Glossary, formats::yomitan::model::*};

    // A helper struct to prepend css links, while conversion methods over Definition
    // do not concern themselves about Glossary-level information.
    #[derive(Default)]
    pub struct HtmlConverter {
        pub css_files: Vec<PathBuf>,
        // Optional since in theory we could call this converter from readers
        // that are not Yomitan
        pub tag_bank: Option<TagBank>,
    }

    impl HtmlConverter {
        pub fn new(glossary: &Glossary) -> Self {
            Self {
                css_files: glossary.css_files().map(|d| d.fname.clone()).collect(),
                tag_bank: glossary.metadata.tag_bank.clone(),
            }
        }
    }

    impl HtmlConverter {
        pub fn convert(&self, def: &Definition) -> String {
            let mut out = self.leading_links();
            out.push_str(&def.to_html(self.tag_bank.as_deref()));
            out
        }

        fn leading_links(&self) -> String {
            self.css_files.iter().fold(String::new(), |mut acc, fname| {
                let _ = write!(
                    acc,
                    "<link rel='stylesheet' href='{}' type='text/css'>",
                    fname.display()
                );
                acc
            })
        }
    }

    // Bad but bear with me for now
    // This is the prototype of html => text
    #[allow(unused)]
    fn strip_html(s: &str) -> String {
        let mut out = String::with_capacity(s.len());
        let mut in_tag = false;
        for c in s.chars() {
            match c {
                '<' => in_tag = true,
                '>' => {
                    in_tag = false;
                    if !out.ends_with(' ') {
                        out.push(' ');
                    }
                }
                _ if !in_tag => out.push(c),
                _ => {}
            }
        }
        out.trim().to_string()
    }

    pub fn html_to_structured_content(html: &str) -> DetailedDefinition {
        let fragment = Html::parse_fragment(html);
        let root = fragment.root_element();
        DetailedDefinition::StructuredContent(StructuredContent::new(element_to_node(root)))
    }

    fn element_to_node(el: ElementRef) -> Node {
        let children: Vec<Node> = el
            .children()
            .filter_map(|child| match child.value() {
                ScraperNode::Text(t) => Some(Node::Text(t.to_string())),
                ScraperNode::Element(_) => ElementRef::wrap(child).map(element_to_node),
                _ => None,
            })
            .collect();

        let content = if children.len() == 1 {
            children.into_iter().next().unwrap()
        } else {
            Node::Array(children)
        };

        let value = el.value();
        // let tag = value.name();
        let title = value.attr("title").map(str::to_string);

        // helper to remove duplication
        let make = |tag: NTag| {
            Node::Generic(Box::new(GenericNode {
                tag,
                content: Some(content.clone()),
                title: None,
                style: extract_styles(el.value()),
                data: None,
                lang: None,
            }))
        };

        match el.value().name() {
            // unwrap artificial root
            "span" if el.value().attr("data-root").is_some() => content,

            "span" => make(NTag::Span),
            "div" => make(NTag::Div),
            "ol" => make(NTag::Ol),
            "ul" => make(NTag::Ul),
            "li" => make(NTag::Li),
            "details" => make(NTag::Details),
            "summary" => make(NTag::Summary),

            // normalize deprecated <font>
            "font" => make(NTag::Span),

            // bold and italic
            "b" | "strong" => Node::Generic(Box::new(GenericNode {
                tag: NTag::Span,
                content: Some(content),
                title,
                style: Some(NodeStyle {
                    font_weight: Some("bold".to_string()),
                    ..extract_styles(el.value()).unwrap_or_default()
                }),
                data: None,
                lang: None,
            })),
            "i" | "em" => Node::Generic(Box::new(GenericNode {
                tag: NTag::Span,
                content: Some(content),
                title,
                style: Some(NodeStyle {
                    font_style: Some("italic".to_string()),
                    ..extract_styles(el.value()).unwrap_or_default()
                }),
                data: None,
                lang: None,
            })),

            "br" => Node::LineBreak(Box::new(LineBreakNode {
                tag: LineBreakNodeTag::Br,
                content: None,
            })),

            // fallback
            _ => content,
        }
    }

    // Support some inline css
    fn extract_styles(value: &Element) -> Option<NodeStyle> {
        // <font color="..."> or similar legacy attrs
        let mut style = NodeStyle::default();
        if let Some(color) = value.attr("color") {
            style.color = Some(color.to_string());
        }

        // inline CSS
        if let Some(style_attr) = value.attr("style") {
            for part in style_attr.split(';') {
                let mut kv = part.splitn(2, ':');
                let key = kv.next().map(str::trim);
                let val = kv.next().map(str::trim);

                match (key, val) {
                    (Some("color"), Some(v)) => style.color = Some(v.to_string()),
                    (Some("font-size"), Some(v)) => style.font_size = Some(v.to_string()),
                    (Some("font-weight"), Some(v)) => style.font_weight = Some(v.to_string()),
                    (Some("font-style"), Some(v)) => style.font_style = Some(v.to_string()),
                    // Does not appear in the Yomitan schema
                    (Some("font-family"), _) => (),
                    (Some("background" | "background-color"), Some(v)) => {
                        style.background_color = Some(v.to_string());
                    }
                    (Some(key), Some(v)) => {
                        tracing::warn!("Detected unsupported style: {key} | {v}");
                    }
                    _ => (),
                }
            }
        }

        if style == NodeStyle::default() {
            None
        } else {
            // tracing::warn!("Detected some style: {style:?}");
            Some(style)
        }
    }
}
