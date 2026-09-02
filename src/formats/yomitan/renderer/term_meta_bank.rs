use std::fmt::Write;

use crate::formats::yomitan::{model::*, renderer::Renderer};

impl TermMetaBankEntry {
    pub fn to_html(&self) -> String {
        self.render()
    }
}

impl Renderer for TermMetaBankEntry {
    fn render_into(&self, out: &mut String) {
        match self {
            Self::Frequency(..) => todo!(),
            Self::Pitch(..) => todo!(),
            Self::Ipa(term, _, ipa_data) => {
                let _ = write!(out, r#"<div class="entry">{term}"#);
                ipa_data.render_into(out);
                out.push_str("</div>");
            }
        }
    }
}

impl Renderer for IpaData {
    fn render_into(&self, out: &mut String) {
        let _ = write!(
            out,
            r#"<div class="ipa-block"><div class="reading">{}</div>"#,
            self.reading
        );
        for transcription in &self.transcriptions {
            transcription.render_into(out);
        }
        out.push_str("</div>");
    }
}

impl Renderer for IpaTranscription {
    fn render_into(&self, out: &mut String) {
        let _ = write!(
            out,
            r#"<div class="ipa-item"><span class="ipa">/{}/</span>"#,
            self.ipa
        );
        if let Some(tags) = &self.tags {
            out.push_str(r#" <span class="tags">"#);
            for (i, tag) in tags.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(tag);
            }
            out.push_str("</span>");
        }
        out.push_str("</div>");
    }
}
