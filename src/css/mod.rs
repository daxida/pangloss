//! Css handling: merging the css files of a [`Glossary`](crate::Glossary), and
//! rewriting css to match Yomitan's structured content.

mod merge;
pub use merge::{concat_css_files, merge_css_files};

mod rewrite;
pub use rewrite::rewrite_css_classes;
