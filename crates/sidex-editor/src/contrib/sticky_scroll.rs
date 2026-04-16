//! Sticky scroll — mirrors VS Code's `StickyScrollController` +
//! `StickyScrollWidget`.
//!
//! Computes which scope headers (function, class, block) should be pinned at
//! the top of the editor viewport as the user scrolls.

/// A single sticky-scroll header line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StickyScrollLine {
    /// The original document line number (zero-based).
    pub line: u32,
    /// The indentation/nesting depth (0 = top-level).
    pub depth: u32,
    /// The text content to render.
    pub text: String,
}

/// Scope information used to compute sticky headers.
#[derive(Debug, Clone)]
pub struct ScopeRange {
    /// First line of the scope (zero-based).
    pub start_line: u32,
    /// Last line of the scope (zero-based, inclusive).
    pub end_line: u32,
    /// Nesting depth (0 = top-level).
    pub depth: u32,
    /// The text of the scope header line.
    pub header_text: String,
}

/// Full state for the sticky-scroll feature.
#[derive(Debug, Clone, Default)]
pub struct StickyScrollState {
    /// Maximum number of sticky lines to show.
    pub max_lines: u32,
    /// Whether sticky scroll is enabled.
    pub enabled: bool,
    /// The currently pinned header lines (top to bottom).
    pub pinned_lines: Vec<StickyScrollLine>,
    /// All known scopes in the document (from tree-sitter / LSP).
    pub scopes: Vec<ScopeRange>,
}

impl StickyScrollState {
    pub fn new(max_lines: u32) -> Self {
        Self {
            max_lines,
            enabled: true,
            pinned_lines: Vec::new(),
            scopes: Vec::new(),
        }
    }

    /// Sets the scopes (typically after a tree-sitter re-parse).
    pub fn set_scopes(&mut self, scopes: Vec<ScopeRange>) {
        self.scopes = scopes;
    }

    /// Recomputes which headers should be pinned based on the current first
    /// visible line in the viewport.
    pub fn update(&mut self, first_visible_line: u32) {
        if !self.enabled {
            self.pinned_lines.clear();
            return;
        }

        let mut active: Vec<&ScopeRange> = self
            .scopes
            .iter()
            .filter(|s| s.start_line < first_visible_line && s.end_line >= first_visible_line)
            .collect();

        active.sort_by_key(|s| s.depth);

        self.pinned_lines = active
            .into_iter()
            .take(self.max_lines as usize)
            .map(|s| StickyScrollLine {
                line: s.start_line,
                depth: s.depth,
                text: s.header_text.clone(),
            })
            .collect();
    }

    /// Returns the number of lines occupied by sticky headers.
    #[must_use]
    pub fn header_count(&self) -> usize {
        self.pinned_lines.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_pinned_headers() {
        let mut state = StickyScrollState::new(3);
        state.set_scopes(vec![
            ScopeRange { start_line: 0, end_line: 50, depth: 0, header_text: "fn main()".into() },
            ScopeRange { start_line: 5, end_line: 30, depth: 1, header_text: "for i in 0..10".into() },
            ScopeRange { start_line: 10, end_line: 20, depth: 2, header_text: "if x > 0".into() },
        ]);

        state.update(15);
        assert_eq!(state.pinned_lines.len(), 3);
        assert_eq!(state.pinned_lines[0].text, "fn main()");
        assert_eq!(state.pinned_lines[1].text, "for i in 0..10");
        assert_eq!(state.pinned_lines[2].text, "if x > 0");
    }

    #[test]
    fn respects_max_lines() {
        let mut state = StickyScrollState::new(1);
        state.set_scopes(vec![
            ScopeRange { start_line: 0, end_line: 50, depth: 0, header_text: "a".into() },
            ScopeRange { start_line: 5, end_line: 30, depth: 1, header_text: "b".into() },
        ]);
        state.update(10);
        assert_eq!(state.pinned_lines.len(), 1);
    }

    #[test]
    fn disabled_shows_nothing() {
        let mut state = StickyScrollState::new(3);
        state.enabled = false;
        state.set_scopes(vec![
            ScopeRange { start_line: 0, end_line: 50, depth: 0, header_text: "a".into() },
        ]);
        state.update(10);
        assert!(state.pinned_lines.is_empty());
    }
}
