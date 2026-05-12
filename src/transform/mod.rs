//! Transformations over a [`Glossary`](crate::Glossary).

mod entry_transform;
pub use entry_transform::*;

mod css_transform;
pub use css_transform::rewrite_css_classes;
