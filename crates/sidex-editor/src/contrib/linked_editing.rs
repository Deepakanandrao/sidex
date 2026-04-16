//! Linked editing ranges — mirrors VS Code's `LinkedEditingContribution`.
//!
//! When the cursor is inside a linked range (e.g. an HTML tag name), edits
//! to that range are mirrored in all other linked ranges simultaneously.

use sidex_text::{Position, Range};

/// Full state for the linked-editing feature.
#[derive(Debug, Clone, Default)]
pub struct LinkedEditingState {
    /// Whether linked editing is currently active.
    pub is_active: bool,
    /// The set of ranges that are linked together.
    pub ranges: Vec<Range>,
    /// A word pattern regex that constrains valid edits within linked ranges.
    pub word_pattern: Option<String>,
    /// The position that triggered the linked editing session.
    pub trigger_position: Option<Position>,
}

impl LinkedEditingState {
    /// Activates linked editing with the given ranges.
    pub fn activate(&mut self, pos: Position, ranges: Vec<Range>, word_pattern: Option<String>) {
        self.is_active = !ranges.is_empty();
        self.trigger_position = Some(pos);
        self.ranges = ranges;
        self.word_pattern = word_pattern;
    }

    /// Deactivates linked editing.
    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.ranges.clear();
        self.word_pattern = None;
        self.trigger_position = None;
    }

    /// Returns `true` if the given position falls inside one of the linked
    /// ranges.
    #[must_use]
    pub fn contains_position(&self, pos: Position) -> bool {
        self.ranges.iter().any(|r| r.contains(pos))
    }

    /// Returns the linked range that contains `pos`, if any.
    #[must_use]
    pub fn range_at(&self, pos: Position) -> Option<&Range> {
        self.ranges.iter().find(|r| r.contains(pos))
    }

    /// Returns all other ranges that should mirror an edit made at `pos`.
    #[must_use]
    pub fn mirror_ranges(&self, pos: Position) -> Vec<Range> {
        self.ranges
            .iter()
            .filter(|r| !r.contains(pos))
            .copied()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linked_editing_lifecycle() {
        let mut state = LinkedEditingState::default();
        let ranges = vec![
            Range::new(Position::new(0, 1), Position::new(0, 4)),
            Range::new(Position::new(0, 10), Position::new(0, 13)),
        ];
        state.activate(Position::new(0, 2), ranges, None);
        assert!(state.is_active);

        let mirrors = state.mirror_ranges(Position::new(0, 2));
        assert_eq!(mirrors.len(), 1);
        assert_eq!(mirrors[0].start.column, 10);

        state.deactivate();
        assert!(!state.is_active);
    }
}
