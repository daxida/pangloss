//! The [stardict] format.
//!
//! [stardict]: https://code.google.com/archive/p/babiloo/wikis/StarDict_format.wiki

pub(crate) mod files;
mod reader;
mod sts;
mod writer;

pub struct StardictFormat;
