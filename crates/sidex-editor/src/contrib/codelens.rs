//! Code lens — mirrors VS Code's `CodeLensController` + `CodeLensWidget`.
//!
//! Code lenses are actionable text rendered above lines (e.g. "3 references",
//! "Run test"). They are fetched lazily and resolved when scrolled into view.

use sidex_text::Range;

/// A single code lens item.
#[derive(Debug, Clone)]
pub struct CodeLensItem {
    /// The range in the document this lens applies to (typically the start
    /// line of a symbol).
    pub range: Range,
    /// The command title to display (e.g. "Run | Debug").
    pub command_title: Option<String>,
    /// An opaque command identifier to invoke on click.
    pub command_id: Option<String>,
    /// Whether this lens has been resolved (title populated).
    pub is_resolved: bool,
}

/// Full state for the code-lens feature.
#[derive(Debug, Clone, Default)]
pub struct CodeLensState {
    /// All code lenses for the current document.
    pub lenses: Vec<CodeLensItem>,
    /// Whether a fetch is in-flight.
    pub is_loading: bool,
    /// Lines currently visible in the viewport (for lazy resolution).
    pub visible_range: Option<(u32, u32)>,
}

impl CodeLensState {
    /// Sets new unresolved lenses (e.g. from an LSP `codeLens` request).
    pub fn set_lenses(&mut self, lenses: Vec<CodeLensItem>) {
        self.lenses = lenses;
        self.is_loading = false;
    }

    /// Marks a lens as resolved with the given title and command.
    pub fn resolve_lens(&mut self, index: usize, title: String, command_id: String) {
        if let Some(lens) = self.lenses.get_mut(index) {
            lens.command_title = Some(title);
            lens.command_id = Some(command_id);
            lens.is_resolved = true;
        }
    }

    /// Returns lenses that are within the visible range and still unresolved.
    #[must_use]
    pub fn unresolved_in_viewport(&self) -> Vec<usize> {
        let Some((start, end)) = self.visible_range else {
            return Vec::new();
        };
        self.lenses
            .iter()
            .enumerate()
            .filter(|(_, l)| !l.is_resolved && l.range.start.line >= start && l.range.start.line <= end)
            .map(|(i, _)| i)
            .collect()
    }

    /// Returns all resolved lenses sorted by line.
    #[must_use]
    pub fn resolved_lenses(&self) -> Vec<&CodeLensItem> {
        let mut lenses: Vec<_> = self.lenses.iter().filter(|l| l.is_resolved).collect();
        lenses.sort_by_key(|l| l.range.start.line);
        lenses
    }

    /// Clears all lenses.
    pub fn clear(&mut self) {
        self.lenses.clear();
        self.is_loading = false;
    }

    /// Updates the visible range for lazy resolution.
    pub fn set_visible_range(&mut self, start: u32, end: u32) {
        self.visible_range = Some((start, end));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sidex_text::Position;

    #[test]
    fn resolve_lens() {
        let mut state = CodeLensState::default();
        state.set_lenses(vec![CodeLensItem {
            range: Range::new(Position::new(5, 0), Position::new(5, 0)),
            command_title: None,
            command_id: None,
            is_resolved: false,
        }]);
        state.set_visible_range(0, 10);

        let unresolved = state.unresolved_in_viewport();
        assert_eq!(unresolved, vec![0]);

        state.resolve_lens(0, "2 references".into(), "showRefs".into());
        assert!(state.lenses[0].is_resolved);
        assert!(state.unresolved_in_viewport().is_empty());
    }
}
