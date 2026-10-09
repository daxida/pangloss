//! The [ABBYY Lingvo DSL][wiki] format, read only.
//!
//! ABBYY never published a specification apart from the DSL Compiler chapter of the
//! Lingvo help, the pages that matter being:
//! - [DSL dictionary structure][structure]: the `#` header, encodings, `#INCLUDE`.
//! - [DSL card structure][card]: headword lines, indented card bodies.
//! - [DSL tags][tags]: the card markup.
//!
//! The rest (`(optional)` headword parts, `@` subentries, `_abrv.dsl`, `.dsl.files.zip`,
//! dictzip) is convention, which we take from [pyglossary's reader][pyglossary]. Its
//! html is followed byte for byte, as many dictionaries in the wild were made with it.
//!
//! [wiki]: https://ru.wikipedia.org/wiki/ABBYY_Lingvo
//! [structure]: http://lingvo.helpmax.net/en/troubleshooting/dsl-compiler/dsl-dictionary-structure/
//! [card]: http://lingvo.helpmax.net/en/troubleshooting/dsl-compiler/dsl-card-structure/
//! [tags]: http://lingvo.helpmax.net/en/troubleshooting/dsl-compiler/dsl-tags/
//! [pyglossary]: https://github.com/ilius/pyglossary/tree/master/pyglossary/plugins/dsl

pub(crate) mod files;
mod reader;

pub struct DslFormat;
