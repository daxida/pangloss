//! The [yomitan] format.
//!
//! [yomitan]: https://github.com/yomidevs/yomitan

// Pub because the Yomitan model is used as one of the definition kinds.
pub(crate) mod model;

mod reader;
mod renderer;
mod writer;

pub use model::*;

pub struct YomitanFormat;
