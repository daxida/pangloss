//! Yomitan term bank data model.
//!
//! Ported from the typescript [yomitan-dict-builder] library.
//!
//! [yomitan-dict-builder]: https://github.com/MarvNC/yomichan-dict-builder/blob/master/src/types/yomitan/termbank.ts

#![allow(unused)]

use std::fmt;

use indexmap::IndexMap;
use serde::{
    Deserialize, Serialize, Serializer,
    ser::{SerializeStruct, SerializeTuple},
};
use serde_json::Value;

pub type TermBank = Vec<TermBankEntry>;

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
pub struct TermBankEntry {
    pub term: String,
    pub reading: String,
    pub definition_tags: String,
    pub rules: String, // space-separated rules
    pub frequency: i64,
    pub definitions: Vec<DetailedDefinition>,
    pub sequence: i64,
    pub term_tags: String,
}

impl TermBankEntry {
    pub fn raw(term: String, definition: String) -> Self {
        Self {
            term,
            definitions: vec![DetailedDefinition::String(definition)],
            ..Default::default()
        }
    }

    pub fn raw_inflection(term: String, alt: String) -> Self {
        Self {
            term: alt,
            definitions: vec![DetailedDefinition::Inflection(term, vec![])],
            ..Default::default()
        }
    }

    pub fn is_inflection(&self) -> bool {
        self.definitions
            .iter()
            .all(|def| matches!(def, DetailedDefinition::Inflection(..)))
    }
}

impl Serialize for TermBankEntry {
    // serialize as tuple
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tup = serializer.serialize_tuple(8)?;
        tup.serialize_element(&self.term)?;
        tup.serialize_element(&self.reading)?;
        tup.serialize_element(&self.definition_tags)?;
        tup.serialize_element(&self.rules)?;
        tup.serialize_element(&self.frequency)?;
        tup.serialize_element(&self.definitions)?;
        tup.serialize_element(&self.sequence)?;
        tup.serialize_element(&self.term_tags)?;
        tup.end()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DetailedDefinition {
    String(String),
    Text(Text),
    StructuredContent(StructuredContent),
    Image(Image),
    Inflection(String, Vec<String>),
}

// Used in Definition convertions from Yomitan to text.
// WARN: temporary so we can work on it
impl fmt::Display for DetailedDefinition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::String(s) => write!(f, "{s}"),
            Self::Text(t) => write!(f, "{}", t.text),
            Self::StructuredContent(content) => {
                let json = serde_json::to_string(content).map_err(|_| fmt::Error)?;
                write!(f, "{json}")
            }
            Self::Image(image) => write!(f, "some image"),
            Self::Inflection(term, rules) => write!(f, "{term} ({})", rules.join(", ")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Text {
    pub r#type: TextTag,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextTag {
    Text,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Image {
    pub r#type: ImageTag,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageTag {
    Image,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StructuredContent {
    pub r#type: StructuredContentTag,
    pub content: Node,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StructuredContentTag {
    StructuredContent,
}

impl StructuredContent {
    pub const fn new(content: Node) -> Self {
        Self {
            r#type: StructuredContentTag::StructuredContent,
            content,
        }
    }
}

/// A structured content node. See [yomitan-dict-builder].
///
/// [yomitan-dict-builder]: https://github.com/MarvNC/yomichan-dict-builder/blob/master/src/types/yomitan/termbank.ts#L91
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Node {
    Text(String),     // 32
    Array(Vec<Self>), // 32
    LineBreak(Box<LineBreakNode>),
    Group(Box<GroupNode>),
    Table(Box<TableNode>),
    Generic(Box<GenericNode>),   // 16
    Image(Box<ImageNode>),       // 16
    Backlink(Box<BacklinkNode>), // 16
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineBreakNode {
    pub tag: LineBreakNodeTag,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Node>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineBreakNodeTag {
    Br,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupNode {
    pub tag: GroupNodeTag,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Node>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<NodeData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GroupNodeTag {
    Ruby,
    Rt,
    Rp,
    Table,
    Thead,
    Tbody,
    Tfoot,
    Tr,
}

impl GroupNodeTag {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Ruby => "ruby",
            Self::Rt => "rt",
            Self::Rp => "rp",
            Self::Table => "table",
            Self::Thead => "thead",
            Self::Tbody => "tbody",
            Self::Tfoot => "tfoot",
            Self::Tr => "tr",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableNode {
    pub tag: TableNodeTag,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Node>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<NodeData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub col_span: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row_span: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TableNodeTag {
    Td,
    Th,
}

impl TableNodeTag {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Td => "td",
            Self::Th => "th",
        }
    }
}

// For tests, not really needed. The default serializer should be fine.
#[allow(clippy::ref_option)]
fn serialize_f64_as_int<S: Serializer>(v: &Option<f64>, s: S) -> Result<S::Ok, S::Error> {
    match v {
        None => s.serialize_none(),
        Some(f) if f.fract() == 0.0 => s.serialize_i64(*f as i64),
        Some(f) => s.serialize_f64(*f),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageNode {
    pub tag: ImageNodeTag,
    pub path: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<NodeData>,
    #[serde(
        serialize_with = "serialize_f64_as_int",
        skip_serializing_if = "Option::is_none"
    )]
    pub width: Option<f64>,
    #[serde(
        serialize_with = "serialize_f64_as_int",
        skip_serializing_if = "Option::is_none"
    )]
    pub height: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pixelated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_rendering: Option<ImageRendering>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appearance: Option<ImageAppearance>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collapsed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collapsible: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertical_align: Option<VerticalAlign>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_radius: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_units: Option<SizeUnits>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageNodeTag {
    Img,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImageRendering {
    Auto,
    Pixelated,
    CrispEdges,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImageAppearance {
    Auto,
    Monochrome,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VerticalAlign {
    Baseline,
    Sub,
    Super,
    TextTop,
    TextBottom,
    Middle,
    Top,
    Bottom,
}

impl TryFrom<&str> for VerticalAlign {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "baseline" => Ok(Self::Baseline),
            "sub" => Ok(Self::Sub),
            "super" => Ok(Self::Super),
            "text-top" => Ok(Self::TextTop),
            "text-bottom" => Ok(Self::TextBottom),
            "middle" => Ok(Self::Middle),
            "top" => Ok(Self::Top),
            "bottom" => Ok(Self::Bottom),
            _ => Err("unsupported vertical-align value"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SizeUnits {
    Px,
    Em,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GenericNode {
    pub tag: NTag,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Node>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<NodeData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<NodeStyle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NTag {
    Span,
    Div,
    Ol,
    Ul,
    Li,
    Details,
    Summary,
}

impl NTag {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Span => "span",
            Self::Div => "div",
            Self::Ol => "ol",
            Self::Ul => "ul",
            Self::Li => "li",
            Self::Details => "details",
            Self::Summary => "summary",
        }
    }
}

/// Structured content data.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeData(pub IndexMap<String, String>);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BacklinkNode {
    pub tag: BacklinkTag,
    pub href: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Node>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BacklinkTag {
    A,
}

impl BacklinkNode {
    pub const fn new(href: String, content: String) -> Self {
        Self {
            tag: BacklinkTag::A,
            href,
            content: Some(Node::Text(content)),
            lang: None,
        }
    }
}

// https://github.com/MarvNC/yomichan-dict-builder/blob/master/src/types/yomitan/termbank.ts#L35
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NodeStyle {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_style: Option<FontStyle>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<FontWeight>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub vertical_align: Option<VerticalAlign>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub white_space: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_style_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontStyle {
    Normal,
    Italic,
}

impl fmt::Display for FontStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => write!(f, "normal"),
            Self::Italic => write!(f, "italic"),
        }
    }
}

impl TryFrom<&str> for FontStyle {
    type Error = &'static str;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // italic!important == italic
        match value.trim_end_matches("!important") {
            "normal" => Ok(Self::Normal),
            "italic" => Ok(Self::Italic),
            _ => Err("unsupported font-style value"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FontWeight {
    Normal,
    Bold,
}

impl fmt::Display for FontWeight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Normal => write!(f, "normal"),
            Self::Bold => write!(f, "bold"),
        }
    }
}

impl TryFrom<&str> for FontWeight {
    type Error = &'static str;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "normal" => Ok(Self::Normal),
            "bold" => Ok(Self::Bold),
            _ => Err("unsupported font-weight value"),
        }
    }
}
