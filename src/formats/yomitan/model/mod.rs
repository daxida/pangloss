//! Yomitan data model.
//!
//! Ported from the typescript [yomitan-dict-builder] library. See also the [spec].
//!
//! [yomitan-dict-builder]: https://github.com/MarvNC/yomichan-dict-builder/tree/master/src/types/yomitan
//! [spec]: https://github.com/yomidevs/yomitan/tree/master/ext/data/schemas

mod term_bank;
pub use term_bank::*;

mod term_meta_bank;
pub use term_meta_bank::*;

mod tag_bank;
pub use tag_bank::*;

#[derive(Debug, Clone)]
pub enum YomitanDefinition {
    TermBankEntry(TermBankEntry),
    TermMetaBankEntry(TermMetaBankEntry),
}
