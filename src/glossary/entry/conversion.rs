//! Conversion between definition kinds.
//!
//! This should be format-agnostic (even if the Yomitan kind is only used in the
//! Yomitan format). For instance, html/xml is used across many dictionary formats.

// TODO: move this somewhere else

use std::{fmt::Write, path::PathBuf};

use indexmap::IndexMap;
use scraper::{ElementRef, Html, Node as ScraperNode, node::Element};

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
    let title = value.attr("title").map(str::to_string);

    // Yomitan's structured content supports a `data` field on nodes, which renders
    // as HTML data attributes: e.g. `data-sc-class="grammar"`.
    //
    // We use this to preserve the original CSS class from the source HTML.
    // For example, `<span class="grammar">` becomes a GenericNode with
    // `data: Some(NodeData({"class": "grammar"}))`, which Yomitan renders as:
    //
    //   <span class="gloss-sc-span" data-sc-class="grammar">...</span>
    //
    // This allows the dictionary's stylesheet to target these nodes via attribute
    // selectors like `span[data-sc-class="grammar"]` instead of `span.grammar`,
    // preserving the original visual styling in Yomitan's reader.
    let data = value
        .attr("class")
        .filter(|c| !c.is_empty())
        .map(|c| NodeData(IndexMap::from([("class".to_string(), c.to_string())])));

    // helper to remove duplication
    let make = |tag: NTag| {
        Node::Generic(Box::new(GenericNode {
            tag,
            content: Some(content.clone()),
            title: None,
            style: extract_styles(el.value()),
            data: data.clone(),
            lang: None,
        }))
    };

    let make_bold = || {
        Node::Generic(Box::new(GenericNode {
            tag: NTag::Div,
            content: Some(content.clone()),
            title: title.clone(),
            style: Some(NodeStyle {
                font_weight: Some(FontWeight::Bold),
                ..extract_styles(el.value()).unwrap_or_default()
            }),
            data: data.clone(),
            lang: None,
        }))
    };

    #[allow(clippy::match_same_arms)]
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

        "br" => Node::LineBreak(Box::new(LineBreakNode {
            tag: LineBreakNodeTag::Br,
            content: None,
        })),

        // From here below, these are tags not supported by Yomitan that we
        // try to match to their closest relative.

        // normalize deprecated <font>
        "font" => make(NTag::Span),

        // bold and italic
        "b" | "strong" => make_bold(),
        "i" | "em" => Node::Generic(Box::new(GenericNode {
            tag: NTag::Span,
            content: Some(content),
            title,
            style: Some(NodeStyle {
                font_style: Some(FontStyle::Italic),
                ..extract_styles(el.value()).unwrap_or_default()
            }),
            data: None,
            lang: None,
        })),

        // paragraph
        "p" => make(NTag::Div),
        // https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/section
        "section" => make(NTag::Div),
        // https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/dl
        "dl" => make(NTag::Ul),
        "dd" => make(NTag::Li),

        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => make_bold(),

        // fallback
        // _ => content,
        //
        // A more verbose fallback
        other => {
            if !["html", "a", "link"].contains(&other) {
                tracing::warn!("Skipping {other}");
            }
            content
        }
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
    // https://github.com/MarvNC/yomichan-dict-builder/blob/master/src/types/yomitan/termbank.ts#L35
    if let Some(style_attr) = value.attr("style") {
        for part in style_attr.split(';') {
            let mut kv = part.splitn(2, ':');
            let key = kv.next().map(str::trim);
            let val = kv.next().map(str::trim);

            match (key, val) {
                (Some("color"), Some(v)) => style.color = Some(v.to_string()),
                (Some("font-size"), Some(v)) => style.font_size = Some(v.to_string()),
                (Some("font-weight"), Some(v)) => {
                    if let Ok(font_weight) = FontWeight::try_from(v) {
                        style.font_weight = Some(font_weight);
                    } else {
                        tracing::warn!("Detected unsupported font_weight variant: {v}");
                    }
                }
                (Some("font-style"), Some(v)) => {
                    if let Ok(font_style) = FontStyle::try_from(v) {
                        style.font_style = Some(font_style);
                    } else {
                        tracing::warn!("Detected unsupported font_style variant: {v}");
                    }
                }
                (Some("background" | "background-color"), Some(v)) => {
                    style.background_color = Some(v.to_string());
                }
                (Some("white-space"), Some(v)) => style.white_space = Some(v.to_string()),
                (Some("vertical-align"), Some(v)) => {
                    if let Ok(vertical_align) = VerticalAlign::try_from(v) {
                        style.vertical_align = Some(vertical_align);
                    } else {
                        tracing::warn!("Detected unsupported vertical_align variant: {v}");
                    }
                }
                // Do not appear in the Yomitan schema
                (Some("width" | "font-family" | "display" | "text-indent" | "line-height"), _) => {}
                (Some(key), Some(v)) => {
                    tracing::warn!("Detected unsupported style: {key} | {v} @ {value:?}");
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
