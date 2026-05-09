//! See <https://mdict4j.readthedocs.io/zh-cn/latest/reference/fileformat.html>

use anyhow::{Context, Result};

mod reader;
mod writer;

pub use reader::StyleSheet;

#[derive(Default)]
pub struct MdictFormat {
    // Only used by the writing logic
    compression: CompressionKind,
}

impl MdictFormat {
    pub const fn new(compression: CompressionKind) -> Self {
        Self { compression }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    Utf16,
}

impl Encoding {
    pub fn char_size(&self) -> usize {
        match self {
            Encoding::Utf8 => 1,
            Encoding::Utf16 => 2,
        }
    }

    pub fn decode(&self, data: &[u8]) -> String {
        match self {
            Encoding::Utf8 => String::from_utf8_lossy(data).to_string(),
            Encoding::Utf16 => {
                let u16s: Vec<u16> = data
                    .chunks_exact(2)
                    .map(|c| u16::from_le_bytes([c[0], c[1]]))
                    .collect();
                String::from_utf16_lossy(&u16s)
            }
        }
    }
}

impl TryFrom<&str> for Encoding {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self> {
        match s {
            "UTF-8" => Ok(Encoding::Utf8),
            "UTF-16" | "UTF-16LE" => Ok(Encoding::Utf16),
            other => Err(anyhow::anyhow!("Unsupported encoding: {other}")),
        }
    }
}

/// No compression
const COMPRESSION_HEADER_0: u32 = 0x0000_0000u32;
/// Zip compression
const COMPRESSION_HEADER_2: u32 = 0x0200_0000u32;

#[derive(Default, Clone, Copy)]
pub enum CompressionKind {
    None,
    Lzo,
    #[default]
    Zip,
}

#[derive(Debug, Clone, Copy)]
pub enum EncryptionKind {
    Zero,
    Two,
}

impl EncryptionKind {
    pub const fn encrypts_index(self) -> bool {
        matches!(self, EncryptionKind::Two)
    }
}

impl TryFrom<&str> for EncryptionKind {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self> {
        let level = if s.eq_ignore_ascii_case("no") {
            0
        } else if s.eq_ignore_ascii_case("yes") {
            1
        } else {
            s.parse::<u8>().context("invalid encryption level")?
        };

        match level {
            0 => Ok(EncryptionKind::Zero),
            2 => Ok(EncryptionKind::Two),
            other => Err(anyhow::anyhow!("Unsupported encryption kind: {other}")),
        }
    }
}

const ATTR_ORDER: [&str; 16] = [
    "GeneratedByEngineVersion",
    "RequiredEngineVersion",
    "Encrypted",
    "Encoding",
    "Format",
    "Stripkey",
    "CreationDate",
    "Compact",
    "Compat",
    "KeyCaseSensitive",
    "Description",
    "Title",
    "DataSourceFormat",
    "StyleSheet",
    "Left2Right",
    "RegisterBy",
];

#[allow(clippy::match_same_arms)]
fn default_attr(key: &str) -> &'static str {
    match key {
        "GeneratedByEngineVersion" => "2.0",
        "RequiredEngineVersion" => "2.0",
        "Encrypted" => "No",
        "Encoding" => "UTF-8",
        "Format" => "Html",
        "Stripkey" => "Yes",
        "Compact" => "Yes",
        "Compat" => "Yes",
        "KeyCaseSensitive" => "No",
        "DataSourceFormat" => "106",
        "Left2Right" => "Yes",
        _ => "",
    }
}
