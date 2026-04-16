//! Diagnostic collection for tracking LSP diagnostics per file.
//!
//! Stores [`lsp_types::Diagnostic`] instances keyed by document URI,
//! typically updated from `textDocument/publishDiagnostics` notifications.

use std::collections::HashMap;

use lsp_types::Diagnostic;

/// Stores diagnostics grouped by document URI.
#[derive(Debug, Default, Clone)]
pub struct DiagnosticCollection {
    inner: HashMap<String, Vec<Diagnostic>>,
}

impl DiagnosticCollection {
    /// Creates an empty diagnostic collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces all diagnostics for the given URI.
    pub fn set(&mut self, uri: &str, diagnostics: Vec<Diagnostic>) {
        if diagnostics.is_empty() {
            self.inner.remove(uri);
        } else {
            self.inner.insert(uri.to_owned(), diagnostics);
        }
    }

    /// Returns the diagnostics for a given URI, or an empty slice if none.
    pub fn get(&self, uri: &str) -> &[Diagnostic] {
        self.inner.get(uri).map_or(&[], Vec::as_slice)
    }

    /// Iterates over all `(uri, diagnostics)` pairs.
    pub fn all(&self) -> impl Iterator<Item = (&str, &[Diagnostic])> {
        self.inner
            .iter()
            .map(|(uri, diags)| (uri.as_str(), diags.as_slice()))
    }

    /// Clears diagnostics for the given URI.
    pub fn clear(&mut self, uri: &str) {
        self.inner.remove(uri);
    }

    /// Clears all stored diagnostics.
    pub fn clear_all(&mut self) {
        self.inner.clear();
    }

    /// Returns the total number of diagnostics across all files.
    pub fn total_count(&self) -> usize {
        self.inner.values().map(Vec::len).sum()
    }

    /// Returns `true` if no diagnostics are stored.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use lsp_types::{DiagnosticSeverity, Position, Range};

    use super::*;

    fn make_diagnostic(message: &str, line: u32) -> Diagnostic {
        Diagnostic {
            range: Range::new(Position::new(line, 0), Position::new(line, 10)),
            severity: Some(DiagnosticSeverity::ERROR),
            message: message.to_owned(),
            ..Diagnostic::default()
        }
    }

    #[test]
    fn set_and_get() {
        let mut coll = DiagnosticCollection::new();
        let diags = vec![make_diagnostic("unused variable", 5)];
        coll.set("file:///main.rs", diags.clone());
        assert_eq!(coll.get("file:///main.rs").len(), 1);
        assert_eq!(coll.get("file:///main.rs")[0].message, "unused variable");
    }

    #[test]
    fn get_missing_uri_returns_empty() {
        let coll = DiagnosticCollection::new();
        assert!(coll.get("file:///nonexistent.rs").is_empty());
    }

    #[test]
    fn set_empty_removes_entry() {
        let mut coll = DiagnosticCollection::new();
        coll.set("file:///a.rs", vec![make_diagnostic("err", 0)]);
        assert_eq!(coll.total_count(), 1);

        coll.set("file:///a.rs", vec![]);
        assert!(coll.get("file:///a.rs").is_empty());
        assert!(coll.is_empty());
    }

    #[test]
    fn clear_uri() {
        let mut coll = DiagnosticCollection::new();
        coll.set("file:///a.rs", vec![make_diagnostic("err", 0)]);
        coll.set("file:///b.rs", vec![make_diagnostic("warn", 1)]);

        coll.clear("file:///a.rs");
        assert!(coll.get("file:///a.rs").is_empty());
        assert_eq!(coll.get("file:///b.rs").len(), 1);
    }

    #[test]
    fn clear_all() {
        let mut coll = DiagnosticCollection::new();
        coll.set("file:///a.rs", vec![make_diagnostic("e1", 0)]);
        coll.set("file:///b.rs", vec![make_diagnostic("e2", 1)]);
        coll.clear_all();
        assert!(coll.is_empty());
    }

    #[test]
    fn all_iterates_entries() {
        let mut coll = DiagnosticCollection::new();
        coll.set("file:///a.rs", vec![make_diagnostic("e1", 0)]);
        coll.set(
            "file:///b.rs",
            vec![make_diagnostic("e2", 1), make_diagnostic("e3", 2)],
        );

        let entries: Vec<_> = coll.all().collect();
        assert_eq!(entries.len(), 2);
        assert_eq!(coll.total_count(), 3);
    }

    #[test]
    fn total_count() {
        let mut coll = DiagnosticCollection::new();
        assert_eq!(coll.total_count(), 0);

        coll.set("file:///a.rs", vec![make_diagnostic("e1", 0)]);
        assert_eq!(coll.total_count(), 1);

        coll.set(
            "file:///b.rs",
            vec![make_diagnostic("e2", 1), make_diagnostic("e3", 2)],
        );
        assert_eq!(coll.total_count(), 3);
    }

    #[test]
    fn overwrite_diagnostics() {
        let mut coll = DiagnosticCollection::new();
        coll.set("file:///a.rs", vec![make_diagnostic("old", 0)]);
        coll.set("file:///a.rs", vec![make_diagnostic("new", 1)]);
        assert_eq!(coll.get("file:///a.rs").len(), 1);
        assert_eq!(coll.get("file:///a.rs")[0].message, "new");
    }
}
