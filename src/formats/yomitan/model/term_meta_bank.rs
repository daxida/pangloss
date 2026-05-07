//! Yomitan term bank meta data model.
//!
//! Ported from the typescript [yomitan-dict-builder] library.
//!
//! There are some fundamental changes to the schema based on the fixtures
//! that Yomitan provide, since the typescript model seems convoluted.
//!
//! [yomitan-dict-builder]: https://github.com/MarvNC/yomichan-dict-builder/blob/master/src/types/yomitan/termbankmeta.ts

use serde::{Deserialize, Serialize};

pub type TermMetaBank = Vec<TermMetaBankEntry>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TermMetaBankEntry {
    Frequency(String, FreqKind, FrequencyData),
    Pitch(String, PitchKind, PitchData),
    Ipa(String, IpaKind, IpaData),
}

impl TermMetaBankEntry {
    pub const fn term(&self) -> &String {
        match self {
            Self::Frequency(t, _, _) | Self::Pitch(t, _, _) | Self::Ipa(t, _, _) => t,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FreqKind {
    Freq,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PitchKind {
    Pitch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IpaKind {
    Ipa,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FrequencyData {
    Plain(FrequencyTerm),
    WithReading {
        reading: String,
        frequency: FrequencyTerm,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FrequencyTerm {
    Term(String),
    Frequency(f64),
    WithDisplay {
        value: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        #[serde(rename = "displayValue")]
        display_value: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitchData {
    pub reading: String,
    pub pitches: Vec<PitchAccentInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitchAccentInfo {
    pub position: NumberOrString,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nasal: Option<NumberOrList>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub devoice: Option<NumberOrList>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NumberOrString {
    Number(u32),
    String(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NumberOrList {
    Single(u32),
    List(Vec<u32>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpaData {
    pub reading: String,
    pub transcriptions: Vec<IpaTranscription>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpaTranscription {
    pub ipa: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}
