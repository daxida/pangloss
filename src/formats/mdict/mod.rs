//! See <https://mdict4j.readthedocs.io/zh-cn/latest/reference/fileformat.html>

use anyhow::{Context, Result, ensure};

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
struct EncryptionKind(u8);

impl EncryptionKind {
    const fn encrypts_index(self) -> bool {
        self.0 & 2 != 0
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

        ensure!(
            level == 0 || level == 2,
            "Unsupported encryption detected. Kind: {level}"
        );

        Ok(Self(level))
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
