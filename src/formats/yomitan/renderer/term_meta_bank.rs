use crate::formats::yomitan::{model::*, renderer::Renderer};

impl TermMetaBankEntry {
    pub fn to_html(&self) -> String {
        self.render()
    }
}

impl Renderer for TermMetaBankEntry {
    fn render(&self) -> String {
        match self {
            Self::Frequency(..) => todo!(),
            Self::Pitch(..) => todo!(),
            Self::Ipa(term, _, ipa_data) => {
                format!(r#"<div class="entry">{term}{}</div>"#, ipa_data.render())
            }
        }
    }
}

impl Renderer for IpaData {
    fn render(&self) -> String {
        let transcriptions = self
            .transcriptions
            .iter()
            .map(Renderer::render)
            .collect::<String>();

        format!(
            r#"<div class="ipa-block"><div class="reading">{}</div>{}</div>"#,
            self.reading, transcriptions
        )
    }
}

impl Renderer for IpaTranscription {
    fn render(&self) -> String {
        let tags = self
            .tags
            .as_ref()
            .map(|tags| format!(" <span class=\"tags\">{}</span>", tags.join(", ")))
            .unwrap_or_default();

        format!(
            r#"<div class="ipa-item"><span class="ipa">/{}/</span>{}</div>"#,
            self.ipa, tags
        )
    }
}
