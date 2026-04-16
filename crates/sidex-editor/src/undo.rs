use std::time::Instant;

use sidex_text::EditOperation;

use crate::selection::Selection;

/// How long between edits before we stop merging into the same undo group.
const AUTO_GROUP_TIMEOUT_MS: u128 = 500;

/// A group of edits that are undone/redone together as a single unit.
#[derive(Debug, Clone)]
pub struct EditGroup {
    /// Pairs of (forward edit, inverse edit) for each edit in this group.
    pub edits: Vec<(EditOperation, EditOperation)>,
    /// Cursor state before the edit was applied.
    pub cursor_before: Vec<Selection>,
    /// Cursor state after the edit was applied.
    pub cursor_after: Vec<Selection>,
    /// Timestamp of the last edit added to this group (for auto-grouping).
    pub timestamp: Instant,
}

impl EditGroup {
    /// Creates a new edit group with a single edit pair.
    #[must_use]
    pub fn new(
        forward: EditOperation,
        inverse: EditOperation,
        cursor_before: Vec<Selection>,
        cursor_after: Vec<Selection>,
    ) -> Self {
        Self {
            edits: vec![(forward, inverse)],
            cursor_before,
            cursor_after,
            timestamp: Instant::now(),
        }
    }

    /// Creates an empty edit group with the given cursor states.
    #[must_use]
    pub fn empty(cursor_before: Vec<Selection>, cursor_after: Vec<Selection>) -> Self {
        Self {
            edits: Vec::new(),
            cursor_before,
            cursor_after,
            timestamp: Instant::now(),
        }
    }

    /// Returns `true` if this group can be merged with a new edit based on
    /// timing (edits within 500ms are grouped).
    #[must_use]
    pub fn can_merge(&self, now: Instant) -> bool {
        now.duration_since(self.timestamp).as_millis() < AUTO_GROUP_TIMEOUT_MS
    }
}

/// Manages undo and redo stacks for the editor.
///
/// Edits are organized into [`EditGroup`]s. Consecutive character insertions
/// within 500ms are automatically merged into a single undo unit.
#[derive(Debug, Clone)]
pub struct UndoRedoStack {
    undo_stack: Vec<EditGroup>,
    redo_stack: Vec<EditGroup>,
}

impl UndoRedoStack {
    /// Creates a new, empty undo/redo stack.
    #[must_use]
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// Pushes an edit group onto the undo stack and clears the redo stack.
    ///
    /// If the last undo group can be merged with this edit (based on timing),
    /// the edits are combined into a single group.
    pub fn push(&mut self, group: EditGroup) {
        self.redo_stack.clear();

        if let Some(last) = self.undo_stack.last_mut() {
            if last.can_merge(group.timestamp) && group.edits.len() == 1 {
                last.edits.extend(group.edits);
                last.cursor_after = group.cursor_after;
                last.timestamp = group.timestamp;
                return;
            }
        }

        self.undo_stack.push(group);
    }

    /// Pushes an edit group that should NOT be merged with previous groups.
    pub fn push_barrier(&mut self, group: EditGroup) {
        self.redo_stack.clear();
        self.undo_stack.push(group);
    }

    /// Pops the top group from the undo stack and pushes it to redo.
    ///
    /// Returns `None` if the undo stack is empty.
    pub fn undo(&mut self) -> Option<EditGroup> {
        let group = self.undo_stack.pop()?;
        self.redo_stack.push(group.clone());
        Some(group)
    }

    /// Pops the top group from the redo stack and pushes it to undo.
    ///
    /// Returns `None` if the redo stack is empty.
    pub fn redo(&mut self) -> Option<EditGroup> {
        let group = self.redo_stack.pop()?;
        self.undo_stack.push(group.clone());
        Some(group)
    }

    /// Returns `true` if there are edits to undo.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Returns `true` if there are edits to redo.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Clears both undo and redo stacks.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Returns the depth of the undo stack.
    #[must_use]
    pub fn undo_depth(&self) -> usize {
        self.undo_stack.len()
    }

    /// Returns the depth of the redo stack.
    #[must_use]
    pub fn redo_depth(&self) -> usize {
        self.redo_stack.len()
    }
}

impl Default for UndoRedoStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use sidex_text::{Position, Range};

    use super::*;

    fn sel(line: u32, col: u32) -> Selection {
        Selection::caret(Position::new(line, col))
    }

    fn make_group() -> EditGroup {
        EditGroup::new(
            EditOperation::insert(Position::new(0, 0), "a".into()),
            EditOperation::delete(Range::new(Position::new(0, 0), Position::new(0, 1))),
            vec![sel(0, 0)],
            vec![sel(0, 1)],
        )
    }

    #[test]
    fn empty_stack() {
        let stack = UndoRedoStack::new();
        assert!(!stack.can_undo());
        assert!(!stack.can_redo());
    }

    #[test]
    fn push_and_undo() {
        let mut stack = UndoRedoStack::new();
        stack.push_barrier(make_group());
        assert!(stack.can_undo());
        assert!(!stack.can_redo());

        let group = stack.undo().unwrap();
        assert_eq!(group.edits.len(), 1);
        assert!(!stack.can_undo());
        assert!(stack.can_redo());
    }

    #[test]
    fn undo_and_redo() {
        let mut stack = UndoRedoStack::new();
        stack.push_barrier(make_group());
        stack.undo();
        let group = stack.redo().unwrap();
        assert_eq!(group.edits.len(), 1);
        assert!(stack.can_undo());
        assert!(!stack.can_redo());
    }

    #[test]
    fn push_clears_redo() {
        let mut stack = UndoRedoStack::new();
        stack.push_barrier(make_group());
        stack.undo();
        assert!(stack.can_redo());
        stack.push_barrier(make_group());
        assert!(!stack.can_redo());
    }

    #[test]
    fn auto_grouping() {
        let mut stack = UndoRedoStack::new();
        // Two pushes in rapid succession should merge
        stack.push(make_group());
        stack.push(make_group());
        assert_eq!(stack.undo_depth(), 1);

        let group = stack.undo().unwrap();
        assert_eq!(group.edits.len(), 2);
    }

    #[test]
    fn barrier_prevents_merge() {
        let mut stack = UndoRedoStack::new();
        stack.push_barrier(make_group());
        stack.push_barrier(make_group());
        assert_eq!(stack.undo_depth(), 2);
    }

    #[test]
    fn clear() {
        let mut stack = UndoRedoStack::new();
        stack.push_barrier(make_group());
        stack.push_barrier(make_group());
        stack.undo();
        stack.clear();
        assert!(!stack.can_undo());
        assert!(!stack.can_redo());
    }

    #[test]
    fn cursor_state_preserved() {
        let mut stack = UndoRedoStack::new();
        let group = EditGroup::new(
            EditOperation::insert(Position::new(0, 0), "x".into()),
            EditOperation::delete(Range::new(Position::new(0, 0), Position::new(0, 1))),
            vec![sel(0, 0)],
            vec![sel(0, 1)],
        );
        stack.push_barrier(group);

        let undone = stack.undo().unwrap();
        assert_eq!(undone.cursor_before, vec![sel(0, 0)]);
        assert_eq!(undone.cursor_after, vec![sel(0, 1)]);
    }

    #[test]
    fn undo_empty_returns_none() {
        let mut stack = UndoRedoStack::new();
        assert!(stack.undo().is_none());
    }

    #[test]
    fn redo_empty_returns_none() {
        let mut stack = UndoRedoStack::new();
        assert!(stack.redo().is_none());
    }
}
