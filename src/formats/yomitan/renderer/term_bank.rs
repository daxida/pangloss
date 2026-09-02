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
        let mut out = String::with_capacity(1024);
        self.write_html(tag_bank, &mut out);
        out
    }

    pub fn write_html(&self, tag_bank: &[TagBankEntry], out: &mut String) {
        let out = &mut *out;
        out.push_str(r#"<div class="entry">"#);
        render_headword(out, &self.term, &self.reading);
        out.push_str(r#"<div class="entry-body"><div class="definition-item-content">"#);
        render_definition_tags(out, &self.definition_tags, tag_bank);

        if let [definition] = self.definitions.as_slice() {
            definition.render_into(out);
        } else {
            out.push_str("<ol>");
            for definition in &self.definitions {
                out.push_str("<li>");
                definition.render_into(out);
                out.push_str("</li>");
            }
            out.push_str("</ol>");
        }

        out.push_str("</div></div></div>");
    }
}

fn render_headword(out: &mut String, term: &str, reading: &str) {
    let _ = write!(
        out,
        r#"<div class="headword"><span class="headword-term"><ruby>{term}<rt>{reading}</rt></ruby></span></div>"#
    );
}

// https://github.com/yomidevs/yomitan/blob/master/ext/js/display/display-generator.js#L736
fn render_definition_tags(out: &mut String, s: &str, tag_bank: &[TagBankEntry]) {
    if s.is_empty() {
        return;
    }
    // Do not split by whitespace: Yomitan splits by space and Jitendex uses
    // \u{a0} to circumvent the splitting logic.
    let mut tags: Vec<_> = s
        .split(' ')
        .map(|tag| (tag, tag_bank.iter().find(|t| t.short_tag == tag)))
        .collect();
    // sort them by tag_bank sort_order
    tags.sort_by_key(|(_, found)| found.map_or(i32::MAX, |t| t.sort_order));

    out.push_str(r#"<div class="definition-tag-list">"#);
    for (tag, found) in tags {
        out.push_str(r#"<span class="tag""#);
        if let Some(t) = found {
            if !t.category.is_empty() {
                let _ = write!(out, " data-category=\"{}\"", t.category);
            }
            let _ = write!(out, " title=\"{}\"", t.long_tag);
        }
        let _ = write!(
            out,
            r#"><span class="tag-label"><span class="tag-label-content">{tag}</span></span></span>"#
        );
    }
    out.push_str("</div>");
}

impl Renderer for DetailedDefinition {
    fn render_into(&self, out: &mut String) {
        match self {
            Self::String(s) => out.push_str(s),
            Self::Text(t) => out.push_str(&t.text),
            Self::StructuredContent(sc_content) => {
                out.push_str(r#"<span class="gloss-content structured-content">"#);
                sc_content.content.render_into(out);
                out.push_str("</span>");
            }
            Self::Image(_) => {
                // tracing::warn!("Skipping rendering for image definition");
            }
            Self::Inflection(term, rules) => {
                let _ = write!(out, "<b>{term}</b>: ");
                for (i, rule) in rules.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(rule);
                }
            }
        }
    }
}

impl Renderer for Node {
    fn render_into(&self, out: &mut String) {
        match self {
            Self::Text(t) => out.push_str(t),
            Self::Array(nodes) => {
                for node in nodes {
                    node.render_into(out);
                }
            }
            Self::LineBreak(node) => node.render_into(out),
            Self::Group(node) => node.render_into(out),
            Self::Generic(node) => node.render_into(out),
            Self::Table(node) => node.render_into(out),
            Self::Image(_) => {
                // tracing::warn!("Skipping rendering for image node");
            }
            Self::Backlink(node) => node.render_into(out),
        }
    }
}

impl Renderer for LineBreakNode {
    fn render_into(&self, out: &mut String) {
        // br is a void element, so we don't need to close it.
        out.push_str("<br>");
        self.content.render_into(out);
    }
}

impl Renderer for GroupNode {
    fn render_into(&self, out: &mut String) {
        let tag = self.tag.as_str();
        let table = matches!(self.tag, GroupNodeTag::Table);
        if table {
            out.push_str(r#"<div class="gloss-sc-table-container">"#);
        }

        let _ = write!(out, "<{tag} class=\"gloss-sc-{tag}\"");
        self.data.render_into(out);
        out.push('>');
        self.content.render_into(out);
        let _ = write!(out, "</{tag}>");

        if table {
            out.push_str("</div>");
        }
    }
}

impl Renderer for GenericNode {
    fn render_into(&self, out: &mut String) {
        let tag = self.tag.as_str();
        let _ = write!(out, "<{tag} class=\"gloss-sc-{tag}\"");
        self.data.render_into(out);

        if let Some(t) = &self.title {
            let _ = write!(out, " title=\"{t}\"");
        }
        if let Some(style) = &self.style {
            // Only worth an attribute if the style renders to anything. Write
            // it and take it back rather than rendering to a second buffer.
            //
            // Use ' to prevent style="list-style-type: "x""
            // I'm not sure if it's better to use ' here or in the
            // rendered style, but this seems easier.
            let opened = out.len();
            out.push_str(" style='");
            let empty = out.len();
            style.render_into(out);
            if out.len() == empty {
                out.truncate(opened);
            } else {
                out.push('\'');
            }
        }

        out.push('>');
        self.content.render_into(out);
        let _ = write!(out, "</{tag}>");
    }
}

impl Renderer for TableNode {
    fn render_into(&self, out: &mut String) {
        let tag = self.tag.as_str();
        let _ = write!(out, "<{tag} class=\"gloss-sc-{tag}\"");
        self.data.render_into(out);

        if let Some(col_span) = self.col_span {
            let _ = write!(out, " colspan=\"{col_span}\"");
        }
        if let Some(row_span) = self.row_span {
            let _ = write!(out, " rowspan=\"{row_span}\"");
        }

        out.push('>');
        self.content.render_into(out);
        let _ = write!(out, "</{tag}>");
    }
}

impl Renderer for BacklinkNode {
    // The external icon is on Yomitan side
    fn render_into(&self, out: &mut String) {
        let _ = write!(out, "<a class=gloss-link href=\"{}\">", self.href);
        self.content.render_into(out);
        out.push_str("</a>");
    }
}

impl Renderer for NodeData {
    fn render_into(&self, out: &mut String) {
        for (k, v) in &self.0 {
            let _ = write!(out, " data-sc-{k}=\"{v}\"");
        }
    }
}

// Note that this adds a trailing ; - the browser shouldn't care though
impl Renderer for NodeStyle {
    fn render_into(&self, out: &mut String) {
        if let Some(v) = &self.color {
            let _ = write!(out, "color:{v};");
        }
        if let Some(v) = &self.background_color {
            let _ = write!(out, "background-color:{v};");
        }
        if let Some(v) = &self.font_weight {
            let _ = write!(out, "font-weight:{v};");
        }
        if let Some(v) = &self.font_style {
            let _ = write!(out, "font-style:{v};");
        }
        if let Some(v) = &self.list_style_type {
            let _ = write!(out, "list-style-type:{v};");
        }
        if let Some(v) = &self.border_style {
            let _ = write!(out, "border-style:{v};");
        }
        if let Some(v) = &self.border_width {
            let _ = write!(out, "border-width:{v};");
        }
        if let Some(v) = &self.border_color {
            let _ = write!(out, "border-color:{v};");
        }
        if let Some(v) = &self.margin {
            let _ = write!(out, "margin:{v};");
        }
    }
}
