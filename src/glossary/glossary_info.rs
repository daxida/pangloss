use indexmap::IndexMap;

/// INVARIANT: a `GlossaryInfo` always contains the "name" key in the first place.
#[derive(Debug, Default)]
pub struct GlossaryInfo {
    inner: IndexMap<String, String>,
}

impl GlossaryInfo {
    pub fn new() -> Self {
        let mut info = Self {
            inner: IndexMap::new(),
        };
        // Put name always at the beginning
        info.inner.insert("name".to_string(), "name".to_string());
        info
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    // https://github.com/ilius/pyglossary/blob/master/pyglossary/glossary_info.py#L60
    //
    /// If the normalized key is present, replace the value. Otherwise add the pair
    /// with the given (not normalized) key.
    pub fn insert(&mut self, key: &str, value: String) -> Option<String> {
        self.inner.insert(info_keys::normalize_key(key), value)
    }

    // Modify a value only if the key exists
    pub fn update(&mut self, key: &str, value: String) -> bool {
        self.inner.get_mut(key).is_some_and(|v| {
            *v = value;
            true
        })
    }

    /// By invariant, this never panics
    pub fn name(&self) -> &str {
        self.inner.get("name").unwrap()
    }

    /// Try to get the given key, then fallback to normalized.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.inner
            .get(&info_keys::normalize_key(key))
            .map(String::as_str)
    }
}

impl<'a> IntoIterator for &'a GlossaryInfo {
    type Item = (&'a String, &'a String);
    type IntoIter = indexmap::map::Iter<'a, String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl From<IndexMap<String, String>> for GlossaryInfo {
    fn from(map: IndexMap<String, String>) -> Self {
        let mut info = Self::new();
        for (key, value) in map {
            info.insert(&key, value);
        }
        info
    }
}

mod info_keys {
    use heck::ToLowerCamelCase;

    pub const NAME: &str = "name";
    pub const SOURCE_LANG: &str = "sourceLang";
    pub const TARGET_LANG: &str = "targetLang";
    // pub const COPYRIGHT: &str = "copyright";
    // pub const AUTHOR: &str = "author";
    // pub const PUBLISHER: &str = "publisher";

    // INVARIANT: the input string (key) is lowercase.
    // INVARIANT: the output string (normalized key) is camelCase.
    fn resolve_raw_key(key: &str) -> Option<&str> {
        debug_assert_eq!(key, key.to_lowercase());
        let key = &key.replace([' ', '-', '_'], "");
        match key.as_str() {
            "title" | "bookname" | "dbname" => Some(NAME),
            "sourcelang" | "inputlang" | "origlang" => Some(SOURCE_LANG),
            "targetlang" | "outputlang" | "destlang" => Some(TARGET_LANG),
            "date" => Some("creationTime"),
            _ => None,
        }
    }

    // Convert to camelCase.
    pub fn normalize_key(key: &str) -> String {
        match resolve_raw_key(&key.to_lowercase()) {
            Some(resolved) => resolved.to_string(),
            None => key.to_lower_camel_case(),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_normalize_key_resolve() {
            assert_eq!(normalize_key("Title"), "name");
            assert_eq!(normalize_key("BookName"), "name");
            assert_eq!(normalize_key("book name"), "name");
            assert_eq!(normalize_key("book   Name"), "name");
            assert_eq!(normalize_key("Date"), "creationTime");

            assert_eq!(normalize_key("sourcelang"), "sourceLang");
            assert_eq!(normalize_key("SourceLang"), "sourceLang");
            assert_eq!(normalize_key("inputlang"), "sourceLang");
            assert_eq!(normalize_key("targetlang"), "targetLang");
            assert_eq!(normalize_key("date"), "creationTime");
        }

        #[test]
        fn test_normalize_key() {
            // PascalCase -> camelCase
            assert_eq!(normalize_key("CreationDate"), "creationDate");

            // already camelCase -> unchanged
            assert_eq!(normalize_key("camelCase"), "camelCase");
            assert_eq!(normalize_key("sourceLang"), "sourceLang");

            // separators -> camelCase
            assert_eq!(normalize_key("creation_date"), "creationDate");
            assert_eq!(normalize_key("creation-date"), "creationDate");

            // lowercase -> unchanged
            assert_eq!(normalize_key("name"), "name");
            assert_eq!(normalize_key("description"), "description");
        }
    }
}
