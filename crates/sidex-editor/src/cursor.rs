use sidex_text::{Buffer, Position};

use crate::selection::Selection;
use crate::word::{find_word_end, find_word_start};

/// Manages a single cursor's state and movement logic.
///
/// Each cursor has a [`Selection`] (which may be collapsed to a caret) and an
/// optional preferred column for vertical movement. When moving up/down, the
/// cursor tries to stay in the same column even if intermediate lines are
/// shorter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorState {
    /// The current selection (or collapsed caret).
    pub selection: Selection,
    /// Preferred column for vertical movement, maintained across up/down moves.
    pub preferred_column: Option<u32>,
}

impl CursorState {
    /// Creates a new cursor at the given position with no selection.
    #[must_use]
    pub fn new(pos: Position) -> Self {
        Self {
            selection: Selection::caret(pos),
            preferred_column: None,
        }
    }

    /// Creates a cursor from an existing selection.
    #[must_use]
    pub fn from_selection(selection: Selection) -> Self {
        Self {
            selection,
            preferred_column: None,
        }
    }

    /// Returns the active (head) position of the cursor.
    #[must_use]
    pub fn position(&self) -> Position {
        self.selection.active
    }

    /// Moves or extends the cursor to `new_active`. If `select` is false, the
    /// selection collapses to the new position.
    fn set_active(&mut self, new_active: Position, select: bool) {
        if select {
            self.selection.active = new_active;
        } else {
            self.selection = Selection::caret(new_active);
        }
    }

    /// Clears the preferred column (should be called on horizontal movement).
    fn clear_preferred_column(&mut self) {
        self.preferred_column = None;
    }

    /// Returns the preferred column or the current column.
    fn get_preferred_column(&self) -> u32 {
        self.preferred_column
            .unwrap_or(self.selection.active.column)
    }

    /// Move cursor left one character.
    pub fn move_left(&mut self, buffer: &Buffer, select: bool) {
        self.clear_preferred_column();
        let pos = self.selection.active;

        if !select && !self.selection.is_empty() {
            self.selection = Selection::caret(self.selection.start());
            return;
        }

        let new_pos = if pos.column > 0 {
            Position::new(pos.line, pos.column - 1)
        } else if pos.line > 0 {
            let prev_line = pos.line - 1;
            let prev_len = buffer.line_content_len(prev_line as usize) as u32;
            Position::new(prev_line, prev_len)
        } else {
            pos
        };

        self.set_active(new_pos, select);
    }

    /// Move cursor right one character.
    pub fn move_right(&mut self, buffer: &Buffer, select: bool) {
        self.clear_preferred_column();
        let pos = self.selection.active;

        if !select && !self.selection.is_empty() {
            self.selection = Selection::caret(self.selection.end());
            return;
        }

        let line_len = buffer.line_content_len(pos.line as usize) as u32;
        let new_pos = if pos.column < line_len {
            Position::new(pos.line, pos.column + 1)
        } else if (pos.line as usize) + 1 < buffer.len_lines() {
            Position::new(pos.line + 1, 0)
        } else {
            pos
        };

        self.set_active(new_pos, select);
    }

    /// Move cursor up one line, preserving the preferred column.
    pub fn move_up(&mut self, buffer: &Buffer, select: bool) {
        let pos = self.selection.active;
        if pos.line == 0 {
            let new_pos = Position::new(0, 0);
            self.set_active(new_pos, select);
            return;
        }

        let target_col = self.get_preferred_column();
        if self.preferred_column.is_none() {
            self.preferred_column = Some(pos.column);
        }

        let new_line = pos.line - 1;
        let new_line_len = buffer.line_content_len(new_line as usize) as u32;
        let new_col = target_col.min(new_line_len);
        self.set_active(Position::new(new_line, new_col), select);
    }

    /// Move cursor down one line, preserving the preferred column.
    pub fn move_down(&mut self, buffer: &Buffer, select: bool) {
        let pos = self.selection.active;
        let last_line = (buffer.len_lines() - 1) as u32;
        if pos.line >= last_line {
            let line_len = buffer.line_content_len(last_line as usize) as u32;
            self.set_active(Position::new(last_line, line_len), select);
            return;
        }

        let target_col = self.get_preferred_column();
        if self.preferred_column.is_none() {
            self.preferred_column = Some(pos.column);
        }

        let new_line = pos.line + 1;
        let new_line_len = buffer.line_content_len(new_line as usize) as u32;
        let new_col = target_col.min(new_line_len);
        self.set_active(Position::new(new_line, new_col), select);
    }

    /// Move cursor to the start of the current line.
    pub fn move_to_line_start(&mut self, _buffer: &Buffer, select: bool) {
        self.clear_preferred_column();
        let pos = self.selection.active;
        self.set_active(Position::new(pos.line, 0), select);
    }

    /// Move cursor to the end of the current line.
    pub fn move_to_line_end(&mut self, buffer: &Buffer, select: bool) {
        self.clear_preferred_column();
        let pos = self.selection.active;
        let line_len = buffer.line_content_len(pos.line as usize) as u32;
        self.set_active(Position::new(pos.line, line_len), select);
    }

    /// Move cursor left by one word (Ctrl+Left).
    pub fn move_word_left(&mut self, buffer: &Buffer, select: bool) {
        self.clear_preferred_column();
        let new_pos = find_word_start(buffer, self.selection.active);
        self.set_active(new_pos, select);
    }

    /// Move cursor right by one word (Ctrl+Right).
    pub fn move_word_right(&mut self, buffer: &Buffer, select: bool) {
        self.clear_preferred_column();
        let new_pos = find_word_end(buffer, self.selection.active);
        self.set_active(new_pos, select);
    }

    /// Move cursor to the start of the buffer (Ctrl+Home).
    pub fn move_to_buffer_start(&mut self, _buffer: &Buffer, select: bool) {
        self.clear_preferred_column();
        self.set_active(Position::new(0, 0), select);
    }

    /// Move cursor to the end of the buffer (Ctrl+End).
    pub fn move_to_buffer_end(&mut self, buffer: &Buffer, select: bool) {
        self.clear_preferred_column();
        let last_line = (buffer.len_lines() - 1) as u32;
        let last_col = buffer.line_content_len(last_line as usize) as u32;
        self.set_active(Position::new(last_line, last_col), select);
    }

    /// Move cursor up by `viewport_lines` lines (Page Up).
    pub fn move_page_up(&mut self, buffer: &Buffer, viewport_lines: u32, select: bool) {
        let pos = self.selection.active;
        let target_col = self.get_preferred_column();
        if self.preferred_column.is_none() {
            self.preferred_column = Some(pos.column);
        }

        let new_line = pos.line.saturating_sub(viewport_lines);
        let new_line_len = buffer.line_content_len(new_line as usize) as u32;
        let new_col = target_col.min(new_line_len);
        self.set_active(Position::new(new_line, new_col), select);
    }

    /// Move cursor down by `viewport_lines` lines (Page Down).
    pub fn move_page_down(&mut self, buffer: &Buffer, viewport_lines: u32, select: bool) {
        let pos = self.selection.active;
        let last_line = (buffer.len_lines() - 1) as u32;
        let target_col = self.get_preferred_column();
        if self.preferred_column.is_none() {
            self.preferred_column = Some(pos.column);
        }

        let new_line = (pos.line + viewport_lines).min(last_line);
        let new_line_len = buffer.line_content_len(new_line as usize) as u32;
        let new_col = target_col.min(new_line_len);
        self.set_active(Position::new(new_line, new_col), select);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(s: &str) -> Buffer {
        Buffer::from_str(s)
    }

    #[test]
    fn move_left_basic() {
        let b = buf("hello");
        let mut c = CursorState::new(Position::new(0, 3));
        c.move_left(&b, false);
        assert_eq!(c.position(), Position::new(0, 2));
        assert!(c.selection.is_empty());
    }

    #[test]
    fn move_left_wraps_to_prev_line() {
        let b = buf("hello\nworld");
        let mut c = CursorState::new(Position::new(1, 0));
        c.move_left(&b, false);
        assert_eq!(c.position(), Position::new(0, 5));
    }

    #[test]
    fn move_left_at_buffer_start() {
        let b = buf("hello");
        let mut c = CursorState::new(Position::new(0, 0));
        c.move_left(&b, false);
        assert_eq!(c.position(), Position::new(0, 0));
    }

    #[test]
    fn move_left_collapses_selection() {
        let b = buf("hello");
        let mut c =
            CursorState::from_selection(Selection::new(Position::new(0, 1), Position::new(0, 4)));
        c.move_left(&b, false);
        assert_eq!(c.position(), Position::new(0, 1));
        assert!(c.selection.is_empty());
    }

    #[test]
    fn move_left_with_select() {
        let b = buf("hello");
        let mut c = CursorState::new(Position::new(0, 3));
        c.move_left(&b, true);
        assert_eq!(c.selection.anchor, Position::new(0, 3));
        assert_eq!(c.selection.active, Position::new(0, 2));
    }

    #[test]
    fn move_right_basic() {
        let b = buf("hello");
        let mut c = CursorState::new(Position::new(0, 2));
        c.move_right(&b, false);
        assert_eq!(c.position(), Position::new(0, 3));
    }

    #[test]
    fn move_right_wraps_to_next_line() {
        let b = buf("hello\nworld");
        let mut c = CursorState::new(Position::new(0, 5));
        c.move_right(&b, false);
        assert_eq!(c.position(), Position::new(1, 0));
    }

    #[test]
    fn move_right_at_buffer_end() {
        let b = buf("hello");
        let mut c = CursorState::new(Position::new(0, 5));
        c.move_right(&b, false);
        assert_eq!(c.position(), Position::new(0, 5));
    }

    #[test]
    fn move_up_basic() {
        let b = buf("hello\nworld");
        let mut c = CursorState::new(Position::new(1, 3));
        c.move_up(&b, false);
        assert_eq!(c.position(), Position::new(0, 3));
    }

    #[test]
    fn move_up_clamps_column() {
        let b = buf("hi\nhello world");
        let mut c = CursorState::new(Position::new(1, 10));
        c.move_up(&b, false);
        assert_eq!(c.position(), Position::new(0, 2));
    }

    #[test]
    fn move_up_preserves_preferred_column() {
        let b = buf("hello world\nhi\nhello world");
        let mut c = CursorState::new(Position::new(2, 10));
        c.move_up(&b, false);
        assert_eq!(c.position(), Position::new(1, 2));
        c.move_up(&b, false);
        assert_eq!(c.position(), Position::new(0, 10));
    }

    #[test]
    fn move_up_at_first_line() {
        let b = buf("hello");
        let mut c = CursorState::new(Position::new(0, 3));
        c.move_up(&b, false);
        assert_eq!(c.position(), Position::new(0, 0));
    }

    #[test]
    fn move_down_basic() {
        let b = buf("hello\nworld");
        let mut c = CursorState::new(Position::new(0, 3));
        c.move_down(&b, false);
        assert_eq!(c.position(), Position::new(1, 3));
    }

    #[test]
    fn move_down_at_last_line() {
        let b = buf("hello\nworld");
        let mut c = CursorState::new(Position::new(1, 2));
        c.move_down(&b, false);
        assert_eq!(c.position(), Position::new(1, 5));
    }

    #[test]
    fn move_to_line_start() {
        let b = buf("hello");
        let mut c = CursorState::new(Position::new(0, 3));
        c.move_to_line_start(&b, false);
        assert_eq!(c.position(), Position::new(0, 0));
    }

    #[test]
    fn move_to_line_end() {
        let b = buf("hello");
        let mut c = CursorState::new(Position::new(0, 2));
        c.move_to_line_end(&b, false);
        assert_eq!(c.position(), Position::new(0, 5));
    }

    #[test]
    fn move_to_buffer_start() {
        let b = buf("hello\nworld");
        let mut c = CursorState::new(Position::new(1, 3));
        c.move_to_buffer_start(&b, false);
        assert_eq!(c.position(), Position::new(0, 0));
    }

    #[test]
    fn move_to_buffer_end() {
        let b = buf("hello\nworld");
        let mut c = CursorState::new(Position::new(0, 0));
        c.move_to_buffer_end(&b, false);
        assert_eq!(c.position(), Position::new(1, 5));
    }

    #[test]
    fn move_word_left() {
        let b = buf("hello world");
        let mut c = CursorState::new(Position::new(0, 8));
        c.move_word_left(&b, false);
        assert_eq!(c.position(), Position::new(0, 6));
    }

    #[test]
    fn move_word_right() {
        let b = buf("hello world");
        let mut c = CursorState::new(Position::new(0, 0));
        c.move_word_right(&b, false);
        assert_eq!(c.position(), Position::new(0, 5));
    }

    #[test]
    fn page_up() {
        let b = buf("a\nb\nc\nd\ne\nf\ng\nh\ni\nj");
        let mut c = CursorState::new(Position::new(8, 0));
        c.move_page_up(&b, 5, false);
        assert_eq!(c.position(), Position::new(3, 0));
    }

    #[test]
    fn page_down() {
        let b = buf("a\nb\nc\nd\ne\nf\ng\nh\ni\nj");
        let mut c = CursorState::new(Position::new(1, 0));
        c.move_page_down(&b, 5, false);
        assert_eq!(c.position(), Position::new(6, 0));
    }

    #[test]
    fn move_down_with_select() {
        let b = buf("hello\nworld");
        let mut c = CursorState::new(Position::new(0, 3));
        c.move_down(&b, true);
        assert_eq!(c.selection.anchor, Position::new(0, 3));
        assert_eq!(c.selection.active, Position::new(1, 3));
        assert!(!c.selection.is_empty());
    }
}
