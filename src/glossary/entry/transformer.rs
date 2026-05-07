use crate::{
    Definition,
    formats::mdict::StyleSheet,
    glossary::{AltMap, Glossary, entry::Entry},
};
use regex::Regex;
use std::{cell::RefCell, collections::HashSet, sync::LazyLock};

pub trait EntryTransform {
    fn apply(&self, entry: &mut Entry);
}

/// A collection of transforms to apply to some Entry iterator.
/// Sort of a builder class.
pub struct EntryTransformer<'a> {
    transforms: Vec<Box<dyn EntryTransform + 'a>>,
}

impl<'a> EntryTransformer<'a> {
    pub fn new() -> Self {
        Self {
            transforms: Vec::new(),
        }
    }

    pub fn add(mut self, transform: impl EntryTransform + 'a) -> Self {
        self.transforms.push(Box::new(transform));
        self
    }

    pub fn default() -> Self {
        Self::new().add(TrimWhiteSpace)
    }

    pub fn transform(&self, entry: &mut Entry) {
        for t in &self.transforms {
            t.apply(entry);
        }
    }

    pub fn transform_glossary(&self, glossary: &mut Glossary) {
        for entry in &mut glossary.entries {
            self.transform(entry);
        }
    }
}

impl std::fmt::Display for EntryTransformer<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EntryTransformer: {} transforms", self.transforms.len())
    }
}

pub struct PreventDuplicateTerms<'a> {
    alt_map: &'a AltMap,
    seen: RefCell<HashSet<String>>,
}

impl<'a> PreventDuplicateTerms<'a> {
    pub fn new(alt_map: &'a AltMap) -> Self {
        Self {
            alt_map,
            seen: RefCell::new(HashSet::new()),
        }
    }
}

impl EntryTransform for PreventDuplicateTerms<'_> {
    fn apply(&self, entry: &mut Entry) {
        let term = entry.s_terms(self.alt_map);
        let mut seen = self.seen.borrow_mut();
        if seen.insert(term.clone()) {
            return;
        }
        let mut n = 2;
        while !seen.insert(format!("{term} ({n})")) {
            n += 1;
        }
        entry.term = format!("{term} ({n})");
    }
}

// https://github.com/ilius/pyglossary/blob/master/pyglossary/entry_filters.py#L79
pub struct TrimWhiteSpace;

impl EntryTransform for TrimWhiteSpace {
    fn apply(&self, entry: &mut Entry) {
        entry.term = entry.term().trim().replace('\r', "");
    }
}

// https://github.com/xiaoyifang/goldendict-ng/blob/master/src/dict/mdictparser.cc#L600
pub struct ResolveMdictStyles {
    style_sheet: StyleSheet,
}

static STYLE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"`(\d+)`").unwrap());

impl ResolveMdictStyles {
    pub fn new(style_sheet: StyleSheet) -> Self {
        Self { style_sheet }
    }

    fn repl(&self, s: &str) -> String {
        let mut out = String::new();
        let mut pending_suffix = "";
        let mut last = 0;
        for cap in STYLE_RE.captures_iter(s) {
            let m = cap.get(0).unwrap();
            out.push_str(&s[last..m.start()]);
            last = m.end();
            let id: u32 = cap[1].parse().unwrap_or(0);
            if let Some((prefix, suffix)) = self.style_sheet.get(&id) {
                out.push_str(pending_suffix);
                out.push_str(prefix);
                pending_suffix = suffix;
            } else {
                out.push_str(pending_suffix);
                pending_suffix = "";
            }
        }
        if last > 0 {
            out.push_str(&s[last..]);
            out.push_str(pending_suffix);
        }
        out
    }
}

impl EntryTransform for ResolveMdictStyles {
    fn apply(&self, entry: &mut Entry) {
        let definition = self.repl(&entry.definition().to_text());
        entry.definition = Definition::Html(definition);
    }
}
