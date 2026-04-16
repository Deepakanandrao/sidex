//! Language registry mapping file extensions to tree-sitter grammars.
//!
//! Each [`Language`] bundles a tree-sitter grammar with its highlight queries,
//! file extension associations, comment syntax, and editor behaviours like
//! auto-closing pairs and indentation rules. The [`LanguageRegistry`] provides
//! fast lookup by extension or name.

use std::collections::HashMap;

use regex::Regex;

use crate::indent::{FoldingRules, IndentRule, OnEnterRule};

/// A language definition that pairs a tree-sitter grammar with metadata.
#[derive(Clone)]
pub struct Language {
    /// Canonical language name (e.g. `"rust"`, `"typescript"`).
    pub name: String,
    /// The compiled tree-sitter grammar.
    pub ts_language: tree_sitter::Language,
    /// Optional `highlights.scm` query source.
    pub highlight_query: Option<String>,
    /// Optional injection query for embedded languages.
    pub injection_query: Option<String>,
    /// File extensions this language handles (including the dot, e.g. `".rs"`).
    pub file_extensions: Vec<String>,
    /// Single-line comment prefix (e.g. `"//"`).
    pub line_comment: Option<String>,
    /// Block comment delimiters (e.g. `("/*", "*/")`).
    pub block_comment: Option<(String, String)>,
    /// Pairs that the editor auto-closes (e.g. `("(", ")")`, `("{", "}")`).
    pub auto_closing_pairs: Vec<(String, String)>,
    /// Pairs used for surrounding selections (e.g. `("(", ")")`, `("\"", "\"")`).
    pub surrounding_pairs: Vec<(String, String)>,
    /// Indentation rules for automatic indent/outdent.
    pub indent_rules: Vec<IndentRule>,
    /// Regex defining what constitutes a "word" for double-click selection and
    /// word-based navigation.
    pub word_pattern: Option<Regex>,
    /// Rules evaluated when the user presses Enter.
    pub on_enter_rules: Vec<OnEnterRule>,
    /// Marker-based folding rules (e.g. `#region` / `#endregion`).
    pub folding_rules: Option<FoldingRules>,
}

impl std::fmt::Debug for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Language")
            .field("name", &self.name)
            .field("has_highlight_query", &self.highlight_query.is_some())
            .field("has_injection_query", &self.injection_query.is_some())
            .field("file_extensions", &self.file_extensions)
            .field("line_comment", &self.line_comment)
            .field("block_comment", &self.block_comment)
            .field("auto_closing_pairs", &self.auto_closing_pairs.len())
            .field("surrounding_pairs", &self.surrounding_pairs.len())
            .field("indent_rules", &self.indent_rules.len())
            .field("has_word_pattern", &self.word_pattern.is_some())
            .field("on_enter_rules", &self.on_enter_rules.len())
            .field("has_folding_rules", &self.folding_rules.is_some())
            .finish_non_exhaustive()
    }
}

/// Registry that maps file extensions and names to [`Language`] definitions.
#[derive(Debug, Default)]
pub struct LanguageRegistry {
    by_name: HashMap<String, usize>,
    by_extension: HashMap<String, usize>,
    languages: Vec<Language>,
}

impl LanguageRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a language, indexing it by name and all its file extensions.
    pub fn register(&mut self, language: Language) {
        let idx = self.languages.len();
        self.by_name.insert(language.name.clone(), idx);
        for ext in &language.file_extensions {
            self.by_extension.insert(ext.clone(), idx);
        }
        self.languages.push(language);
    }

    /// Looks up a language by file extension (e.g. `".rs"`).
    #[must_use]
    pub fn language_for_extension(&self, ext: &str) -> Option<&Language> {
        self.by_extension.get(ext).map(|&idx| &self.languages[idx])
    }

    /// Looks up a language by canonical name (e.g. `"rust"`).
    #[must_use]
    pub fn language_for_name(&self, name: &str) -> Option<&Language> {
        self.by_name.get(name).map(|&idx| &self.languages[idx])
    }

    /// Returns the number of registered languages.
    #[must_use]
    pub fn len(&self) -> usize {
        self.languages.len()
    }

    /// Returns `true` if no languages have been registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.languages.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_rust_language() -> Language {
        Language {
            name: "rust".into(),
            ts_language: tree_sitter_rust::LANGUAGE.into(),
            highlight_query: None,
            file_extensions: vec![".rs".into()],
            line_comment: Some("//".into()),
            block_comment: Some(("/*".into(), "*/".into())),
            injection_query: None,
            auto_closing_pairs: vec![
                ("(".into(), ")".into()),
                ("{".into(), "}".into()),
                ("[".into(), "]".into()),
                ("\"".into(), "\"".into()),
            ],
            surrounding_pairs: vec![
                ("(".into(), ")".into()),
                ("{".into(), "}".into()),
                ("[".into(), "]".into()),
                ("\"".into(), "\"".into()),
            ],
            indent_rules: crate::indent::default_indent_rules(),
            word_pattern: None,
            on_enter_rules: crate::indent::default_on_enter_rules(),
            folding_rules: None,
        }
    }

    #[test]
    fn register_and_lookup_by_name() {
        let mut registry = LanguageRegistry::new();
        registry.register(make_rust_language());

        let lang = registry.language_for_name("rust").unwrap();
        assert_eq!(lang.name, "rust");
    }

    #[test]
    fn lookup_by_extension() {
        let mut registry = LanguageRegistry::new();
        registry.register(make_rust_language());

        let lang = registry.language_for_extension(".rs").unwrap();
        assert_eq!(lang.name, "rust");
    }

    #[test]
    fn lookup_missing_returns_none() {
        let registry = LanguageRegistry::new();
        assert!(registry.language_for_name("rust").is_none());
        assert!(registry.language_for_extension(".rs").is_none());
    }

    #[test]
    fn len_and_is_empty() {
        let mut registry = LanguageRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);

        registry.register(make_rust_language());
        assert!(!registry.is_empty());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn multiple_extensions() {
        let mut registry = LanguageRegistry::new();
        let lang = Language {
            name: "typescript".into(),
            ts_language: tree_sitter_rust::LANGUAGE.into(),
            highlight_query: None,
            injection_query: None,
            file_extensions: vec![".ts".into(), ".tsx".into()],
            line_comment: Some("//".into()),
            block_comment: Some(("/*".into(), "*/".into())),
            auto_closing_pairs: vec![],
            surrounding_pairs: vec![],
            indent_rules: vec![],
            word_pattern: None,
            on_enter_rules: vec![],
            folding_rules: None,
        };
        registry.register(lang);

        assert!(registry.language_for_extension(".ts").is_some());
        assert!(registry.language_for_extension(".tsx").is_some());
        assert_eq!(
            registry.language_for_extension(".ts").unwrap().name,
            "typescript"
        );
    }

    #[test]
    fn debug_impl() {
        let lang = make_rust_language();
        let dbg = format!("{lang:?}");
        assert!(dbg.contains("rust"));
    }
}
