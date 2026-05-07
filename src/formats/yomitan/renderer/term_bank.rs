//! Render [`TermBankEntry`] into html.
//!
//! Compare to Yomitan [display].
//!
//! [display]: https://github.com/yomidevs/yomitan/blob/master/ext/js/display/structured-content-generator.js

use std::fmt::Write;

use crate::formats::yomitan::{model::*, renderer::Renderer};

// We don't implement render directly for TermBankEntry because we need metadata
// to resolve tags.
impl TermBankEntry {
    pub fn to_html(&self, tag_bank: &[TagBankEntry]) -> String {
        let headword = render_headword(&self.term, &self.reading);
        let tags = render_definition_tags(&self.definition_tags, tag_bank);

        let defs = if self.definitions.len() == 1 {
            self.definitions[0].render()
        } else {
            let mut buf = String::new();
            buf += "<ol>";
            for definition in &self.definitions {
                buf += "<li>";
                buf += definition.render().as_str();
                buf += "</li>";
            }
            buf += "</ol>";
            buf
        };

        format!(
            r#"<div class="entry">{headword}<div class="entry-body"><div class="definition-item-content">{tags}{defs}</div></div>"#
        )
    }
}

fn render_headword(term: &str, reading: &str) -> String {
    format!(
        r#"<div class="headword"><span class="headword-term"><ruby>{term}<rt>{reading}</rt></ruby></span></div>"#
    )
}

// https://github.com/yomidevs/yomitan/blob/master/ext/js/display/display-generator.js#L736
fn render_definition_tags(s: &str, tag_bank: &[TagBankEntry]) -> String {
    if s.is_empty() {
        return String::new();
    }
    // Do not split by whitespace: Yomitan splits by space and Jitendex uses
    // \u{a0} to circumvent the spliting logic.
    let mut tags: Vec<_> = s.split(' ').collect();
    // sort them by tag_bank sort_order
    tags.sort_by_key(|tag| {
        tag_bank
            .iter()
            .find(|t| t.short_tag == *tag)
            .map_or(i32::MAX, |t| t.sort_order)
    });
    let mut buf = String::from("<div class=\"definition-tag-list\">");
    for tag in tags {
        let _ = write!(buf, "<span class=\"tag\"");
        if let Some(t) = tag_bank.iter().find(|t| t.short_tag == tag) {
            if !t.category.is_empty() {
                let _ = write!(buf, " data-category=\"{}\"", t.category);
            }
            let _ = write!(buf, " title=\"{}\"", t.long_tag);
        }
        buf.push('>');
        let _ = write!(
            buf,
            "<span class=\"tag-label\"><span class=\"tag-label-content\">{tag}</span></span></span>"
        );
    }
    buf.push_str("</div>");
    buf
}

impl Renderer for DetailedDefinition {
    fn render(&self) -> String {
        match self {
            Self::String(s) => s.clone(),
            Self::Text(t) => t.text.clone(),
            Self::StructuredContent(sc_content) => {
                let sc = sc_content.content.render();
                format!(r#"<span class="gloss-content structured-content">{sc}</span>"#)
            }
            Self::Image(_) => {
                // tracing::warn!("Skipping rendering for image definition");
                String::new()
            }
            Self::Inflection(term, rules) => {
                format!("<b>{term}</b>: {}", rules.join(", "))
            }
        }
    }
}

impl Renderer for Node {
    fn render(&self) -> String {
        match self {
            Self::Text(t) => t.clone(),
            Self::Array(nodes) => nodes.iter().map(Self::render).collect(),
            Self::LineBreak(node) => node.render(),
            Self::Group(node) => node.render(),
            Self::Generic(node) => node.render(),
            Self::Table(node) => node.render(),
            Self::Image(_) => {
                // tracing::warn!("Skipping rendering for image node");
                String::new()
            }
            Self::Backlink(node) => node.render(),
        }
    }
}

impl Renderer for LineBreakNode {
    fn render(&self) -> String {
        format!("<br>{}</br>", self.content.render())
    }
}

impl Renderer for GroupNode {
    fn render(&self) -> String {
        let content = self.content.render();
        let tag = self.tag.as_str();
        let attrs = self.data.render();
        let inner = format!("<{tag} class=\"gloss-sc-{tag}\"{attrs}>{content}</{tag}>");

        if matches!(self.tag, GroupNodeTag::Table) {
            format!("<div class=\"gloss-sc-table-container\">{inner}</div>")
        } else {
            inner
        }
    }
}

impl Renderer for GenericNode {
    fn render(&self) -> String {
        let content = self.content.render();
        let tag = self.tag.as_str();
        let mut attrs = self.data.render();

        if let Some(t) = &self.title {
            let _ = write!(attrs, " title=\"{t}\"");
        }
        if let Some(style) = &self.style {
            let rendered = style.render();
            if !rendered.is_empty() {
                // Use ' to prevent style="list-style-type: "x""
                // I'm not sure if it's better to use ' here or in the
                // rendered style, but this seems easier.
                let _ = write!(attrs, " style='{rendered}'");
            }
        }

        format!("<{tag} class=\"gloss-sc-{tag}\"{attrs}>{content}</{tag}>")
    }
}

impl Renderer for TableNode {
    fn render(&self) -> String {
        let content = self.content.render();
        let tag = self.tag.as_str();
        let mut attrs = self.data.render();

        if let Some(col_span) = self.col_span {
            let _ = write!(attrs, " colspan=\"{col_span}\"");
        }
        if let Some(row_span) = self.row_span {
            let _ = write!(attrs, " rowspan=\"{row_span}\"");
        }

        format!("<{tag} class=\"gloss-sc-{tag}\"{attrs}>{content}</{tag}>")
    }
}

impl Renderer for BacklinkNode {
    // The external icon is on Yomitan side
    fn render(&self) -> String {
        let content = self.content.render();
        format!(r#"<a class=gloss-link href="{}">{content}</a>"#, self.href)
    }
}

impl Renderer for NodeData {
    fn render(&self) -> String {
        let mut attrs = String::new();
        for (k, v) in &self.0 {
            let _ = write!(attrs, " data-sc-{k}=\"{v}\"");
        }
        attrs
    }
}

// Note that this adds a trailing ; - the browser shouldn't care though
impl Renderer for NodeStyle {
    fn render(&self) -> String {
        let mut buf = String::new();
        if let Some(v) = &self.color {
            let _ = write!(buf, "color:{v};");
        }
        if let Some(v) = &self.background_color {
            let _ = write!(buf, "background-color:{v};");
        }
        if let Some(v) = &self.font_weight {
            let _ = write!(buf, "font-weight:{v};");
        }
        if let Some(v) = &self.font_style {
            let _ = write!(buf, "font-style:{v};");
        }
        if let Some(v) = &self.list_style_type {
            let _ = write!(buf, "list-style-type:{v};");
        }
        buf
    }
}
