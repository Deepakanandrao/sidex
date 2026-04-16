//! Inlay hints — mirrors VS Code's inlay-hint contribution.
//!
//! Tracks inlay hints (type annotations, parameter names) returned by the
//! language server for rendering between text characters.

use sidex_text::Position;

/// The kind of an inlay hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlayHintKind {
    /// A type annotation hint (e.g. `: i32`).
    Type,
    /// A parameter name hint (e.g. `name:`).
    Parameter,
    /// An unclassified hint.
    Other,
}

/// A single inlay hint label part (hints can have clickable parts).
#[derive(Debug, Clone)]
pub struct InlayHintLabelPart {
    /// The display text.
    pub value: String,
    /// Optional tooltip (markdown).
    pub tooltip: Option<String>,
    /// Optional command to execute on click.
    pub command_id: Option<String>,
}

/// A single inlay hint to render in the editor.
#[derive(Debug, Clone)]
pub struct InlayHint {
    /// Position in the document where the hint should be rendered.
    pub position: Position,
    /// The label parts (concatenated for display).
    pub label: Vec<InlayHintLabelPart>,
    /// The kind of hint.
    pub kind: InlayHintKind,
    /// Whether the hint should be rendered with padding on the left.
    pub padding_left: bool,
    /// Whether the hint should be rendered with padding on the right.
    pub padding_right: bool,
}

impl InlayHint {
    /// Returns the full display text of this hint.
    #[must_use]
    pub fn display_text(&self) -> String {
        self.label.iter().map(|p| p.value.as_str()).collect()
    }
}

/// Full state for the inlay-hints feature.
#[derive(Debug, Clone, Default)]
pub struct InlayHintState {
    /// All inlay hints for the current document / viewport.
    pub hints: Vec<InlayHint>,
    /// Whether hints are currently enabled.
    pub enabled: bool,
    /// Whether a fetch is in-flight.
    pub is_loading: bool,
    /// The viewport range that hints were fetched for.
    pub fetched_range: Option<(u32, u32)>,
}

impl InlayHintState {
    pub fn new() -> Self {
        Self {
            enabled: true,
            ..Self::default()
        }
    }

    /// Sets hints received from the language server.
    pub fn set_hints(&mut self, hints: Vec<InlayHint>) {
        self.hints = hints;
        self.is_loading = false;
    }

    /// Returns hints for a specific line.
    #[must_use]
    pub fn hints_for_line(&self, line: u32) -> Vec<&InlayHint> {
        self.hints.iter().filter(|h| h.position.line == line).collect()
    }

    /// Clears all hints.
    pub fn clear(&mut self) {
        self.hints.clear();
        self.is_loading = false;
        self.fetched_range = None;
    }

    /// Requests a refresh for the given viewport range.
    pub fn request_refresh(&mut self, start_line: u32, end_line: u32) {
        self.fetched_range = Some((start_line, end_line));
        self.is_loading = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hints_for_line() {
        let mut state = InlayHintState::new();
        state.set_hints(vec![
            InlayHint {
                position: Position::new(5, 10),
                label: vec![InlayHintLabelPart { value: ": i32".into(), tooltip: None, command_id: None }],
                kind: InlayHintKind::Type,
                padding_left: true,
                padding_right: false,
            },
            InlayHint {
                position: Position::new(7, 3),
                label: vec![InlayHintLabelPart { value: "name:".into(), tooltip: None, command_id: None }],
                kind: InlayHintKind::Parameter,
                padding_left: false,
                padding_right: true,
            },
        ]);
        assert_eq!(state.hints_for_line(5).len(), 1);
        assert_eq!(state.hints_for_line(7).len(), 1);
        assert!(state.hints_for_line(0).is_empty());
    }
}
