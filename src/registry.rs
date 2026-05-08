use std::path::Path;

use anyhow::Result;
use clap::ValueEnum;

use crate::{
    Context, Reader, Writer,
    formats::{
        html::HtmlFormat, json::JsonFormat, mdict::MdictFormat, stardict::StardictFormat,
        text::TextFormat, yomitan::YomitanFormat,
    },
    glossary::Glossary,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ReaderFormat {
    Text,
    Mdict,
    Stardict,
    Json,
    Yomitan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum WriterFormat {
    Text,
    Mdict,
    Stardict,
    Json,
    Yomitan,
    Html,
}

impl PartialEq<WriterFormat> for ReaderFormat {
    fn eq(&self, other: &WriterFormat) -> bool {
        matches!(
            (self, other),
            (Self::Text, WriterFormat::Text)
                | (Self::Mdict, WriterFormat::Mdict)
                | (Self::Stardict, WriterFormat::Stardict)
                | (Self::Json, WriterFormat::Json)
                | (Self::Yomitan, WriterFormat::Yomitan)
        )
    }
}

impl Reader for ReaderFormat {
    fn read_with_context(&self, path: &Path, ctx: &Context) -> Result<Glossary> {
        match self {
            Self::Text => TextFormat.read_with_context(path, ctx),
            Self::Mdict => MdictFormat::default().read_with_context(path, ctx),
            Self::Stardict => StardictFormat.read_with_context(path, ctx),
            Self::Json => JsonFormat.read_with_context(path, ctx),
            Self::Yomitan => YomitanFormat.read_with_context(path, ctx),
        }
    }
}

impl Writer for WriterFormat {
    fn write_with_context(&self, path: &Path, glossary: &Glossary, ctx: &Context) -> Result<()> {
        match self {
            Self::Text => TextFormat.write_with_context(path, glossary, ctx),
            Self::Mdict => MdictFormat::default().write_with_context(path, glossary, ctx),
            Self::Stardict => StardictFormat.write_with_context(path, glossary, ctx),
            Self::Json => JsonFormat.write_with_context(path, glossary, ctx),
            Self::Yomitan => YomitanFormat.write_with_context(path, glossary, ctx),
            Self::Html => HtmlFormat.write_with_context(path, glossary, ctx),
        }
    }
}

impl ReaderFormat {
    pub fn try_from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "txt" => Some(Self::Text),
            "mdx" => Some(Self::Mdict),
            "ifo" => Some(Self::Stardict),
            "json" => Some(Self::Json),
            "zip" => Some(Self::Yomitan),
            _ => None,
        }
    }
}

impl WriterFormat {
    pub fn try_from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()? {
            "txt" => Some(Self::Text),
            "mdx" => Some(Self::Mdict),
            "ifo" => Some(Self::Stardict),
            "json" => Some(Self::Json),
            "zip" => Some(Self::Yomitan),
            "hdir" => Some(Self::Html),
            _ => None,
        }
    }
}
