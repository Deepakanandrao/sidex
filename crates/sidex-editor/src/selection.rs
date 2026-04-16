use serde::{Deserialize, Serialize};
use sidex_text::Position;

/// A selection in a text document, defined by an anchor and an active position.
///
/// The anchor is where the selection started, the active position is where the
/// cursor (caret) currently is. When `anchor == active`, the selection is
/// collapsed (i.e., just a cursor with no highlighted text).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Selection {
    /// Where the selection started.
    pub anchor: Position,
    /// Where the cursor currently is (the "head" of the selection).
    pub active: Position,
}

impl Selection {
    /// Creates a new selection with the given anchor and active positions.
    #[must_use]
    pub const fn new(anchor: Position, active: Position) -> Self {
        Self { anchor, active }
    }

    /// Creates a collapsed selection (just a cursor) at the given position.
    #[must_use]
    pub const fn caret(pos: Position) -> Self {
        Self {
            anchor: pos,
            active: pos,
        }
    }

    /// Returns `true` if the selection is empty (no text is selected).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.anchor == self.active
    }

    /// Returns `true` if the selection is reversed (anchor is after active).
    #[must_use]
    pub fn is_reversed(&self) -> bool {
        self.anchor > self.active
    }

    /// Returns the start position (whichever of anchor/active comes first).
    #[must_use]
    pub fn start(&self) -> Position {
        std::cmp::min(self.anchor, self.active)
    }

    /// Returns the end position (whichever of anchor/active comes last).
    #[must_use]
    pub fn end(&self) -> Position {
        std::cmp::max(self.anchor, self.active)
    }

    /// Returns the normalized range (start <= end regardless of direction).
    #[must_use]
    pub fn range(&self) -> sidex_text::Range {
        sidex_text::Range::new(self.anchor, self.active)
    }
}

impl std::fmt::Display for Selection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_empty() {
            write!(f, "Caret({})", self.active)
        } else {
            write!(f, "Selection({}..{})", self.anchor, self.active)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caret_is_empty() {
        let sel = Selection::caret(Position::new(5, 10));
        assert!(sel.is_empty());
        assert!(!sel.is_reversed());
    }

    #[test]
    fn forward_selection() {
        let sel = Selection::new(Position::new(1, 0), Position::new(1, 10));
        assert!(!sel.is_empty());
        assert!(!sel.is_reversed());
        assert_eq!(sel.start(), Position::new(1, 0));
        assert_eq!(sel.end(), Position::new(1, 10));
    }

    #[test]
    fn reversed_selection() {
        let sel = Selection::new(Position::new(3, 5), Position::new(1, 2));
        assert!(sel.is_reversed());
        assert_eq!(sel.start(), Position::new(1, 2));
        assert_eq!(sel.end(), Position::new(3, 5));
    }

    #[test]
    fn range_is_normalized() {
        let sel = Selection::new(Position::new(5, 0), Position::new(2, 0));
        let r = sel.range();
        assert_eq!(r.start, Position::new(2, 0));
        assert_eq!(r.end, Position::new(5, 0));
    }

    #[test]
    fn equality() {
        let a = Selection::new(Position::new(1, 2), Position::new(3, 4));
        let b = Selection::new(Position::new(1, 2), Position::new(3, 4));
        assert_eq!(a, b);
    }

    #[test]
    fn clone_works() {
        let sel = Selection::caret(Position::new(0, 0));
        let cloned = sel;
        assert_eq!(sel, cloned);
    }

    #[test]
    fn serde_roundtrip() {
        let sel = Selection::new(Position::new(1, 2), Position::new(3, 4));
        let json = serde_json::to_string(&sel).unwrap();
        let deserialized: Selection = serde_json::from_str(&json).unwrap();
        assert_eq!(sel, deserialized);
    }

    #[test]
    fn display_caret() {
        let sel = Selection::caret(Position::new(0, 5));
        assert_eq!(format!("{sel}"), "Caret(0:5)");
    }

    #[test]
    fn display_selection() {
        let sel = Selection::new(Position::new(1, 0), Position::new(1, 10));
        assert_eq!(format!("{sel}"), "Selection(1:0..1:10)");
    }
}
