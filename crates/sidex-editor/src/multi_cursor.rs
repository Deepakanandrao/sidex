use sidex_text::{Buffer, Position};

use crate::cursor::CursorState;
use crate::selection::Selection;

/// Manages multiple cursors within a document.
///
/// All cursors are kept sorted by position and non-overlapping. The "primary"
/// cursor is the last one added (typically the one the user most recently
/// interacted with).
#[derive(Debug, Clone)]
pub struct MultiCursor {
    /// The cursors, always sorted by position and non-overlapping.
    cursors: Vec<CursorState>,
    /// Index of the primary cursor within `cursors`.
    primary_idx: usize,
}

impl MultiCursor {
    /// Creates a new `MultiCursor` with a single cursor at the given position.
    #[must_use]
    pub fn new(pos: Position) -> Self {
        Self {
            cursors: vec![CursorState::new(pos)],
            primary_idx: 0,
        }
    }

    /// Returns the primary cursor (the last added cursor).
    #[must_use]
    pub fn primary(&self) -> &CursorState {
        &self.cursors[self.primary_idx]
    }

    /// Returns a mutable reference to the primary cursor.
    pub fn primary_mut(&mut self) -> &mut CursorState {
        &mut self.cursors[self.primary_idx]
    }

    /// Returns a slice of all cursors.
    #[must_use]
    pub fn cursors(&self) -> &[CursorState] {
        &self.cursors
    }

    /// Returns a mutable slice of all cursors.
    pub fn cursors_mut(&mut self) -> &mut [CursorState] {
        &mut self.cursors
    }

    /// Returns the number of cursors.
    #[must_use]
    pub fn len(&self) -> usize {
        self.cursors.len()
    }

    /// Returns `true` if there are no cursors (should never happen in practice).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cursors.is_empty()
    }

    /// Adds a new cursor at the given position. The new cursor becomes the
    /// primary cursor.
    pub fn add_cursor(&mut self, pos: Position) {
        let new_cursor = CursorState::new(pos);
        self.cursors.push(new_cursor);
        self.primary_idx = self.cursors.len() - 1;
        self.sort_and_merge();
    }

    /// Adds a cursor on the line above the primary cursor (Ctrl+Alt+Up).
    pub fn add_cursor_above(&mut self, buffer: &Buffer) {
        let primary_pos = self.primary().position();
        if primary_pos.line == 0 {
            return;
        }
        let new_line = primary_pos.line - 1;
        let line_len = buffer.line_content_len(new_line as usize) as u32;
        let new_col = primary_pos.column.min(line_len);
        self.add_cursor(Position::new(new_line, new_col));
    }

    /// Adds a cursor on the line below the primary cursor (Ctrl+Alt+Down).
    pub fn add_cursor_below(&mut self, buffer: &Buffer) {
        let primary_pos = self.primary().position();
        let last_line = (buffer.len_lines() - 1) as u32;
        if primary_pos.line >= last_line {
            return;
        }
        let new_line = primary_pos.line + 1;
        let line_len = buffer.line_content_len(new_line as usize) as u32;
        let new_col = primary_pos.column.min(line_len);
        self.add_cursor(Position::new(new_line, new_col));
    }

    /// Removes all secondary cursors, keeping only the primary (Escape).
    pub fn collapse_to_primary(&mut self) {
        let primary = self.cursors[self.primary_idx].clone();
        self.cursors = vec![primary];
        self.primary_idx = 0;
    }

    /// Merges overlapping cursors after movement.
    pub fn merge_overlapping(&mut self) {
        self.sort_and_merge();
    }

    /// Creates cursors at all occurrences of `search` in the buffer
    /// (Ctrl+Shift+L).
    pub fn select_all_occurrences(&mut self, buffer: &Buffer, search: &str) {
        if search.is_empty() {
            return;
        }

        let text = buffer.text();
        let mut new_cursors = Vec::new();
        let search_len = search.chars().count() as u32;

        let mut start_idx = 0;
        while let Some(found) = text[start_idx..].find(search) {
            let char_offset = text[..start_idx + found].chars().count();
            let pos_start = buffer.offset_to_position(char_offset);
            let pos_end = Position::new(pos_start.line, pos_start.column + search_len);
            new_cursors.push(CursorState::from_selection(Selection::new(
                pos_start, pos_end,
            )));
            start_idx += found + search.len();
        }

        if !new_cursors.is_empty() {
            self.primary_idx = new_cursors.len() - 1;
            self.cursors = new_cursors;
            self.sort_and_merge();
        }
    }

    /// Iterates over all cursors immutably.
    pub fn for_each(&self, mut f: impl FnMut(&CursorState)) {
        for cursor in &self.cursors {
            f(cursor);
        }
    }

    /// Iterates over all cursors mutably.
    pub fn for_each_mut(&mut self, mut f: impl FnMut(&mut CursorState)) {
        for cursor in &mut self.cursors {
            f(cursor);
        }
    }

    // ── Movement methods that apply to all cursors ─────────────────

    /// Moves all cursors left and merges any that overlap.
    pub fn move_all_left(&mut self, buffer: &Buffer, select: bool) {
        self.for_each_mut(|c| c.move_left(buffer, select));
        self.sort_and_merge();
    }

    /// Moves all cursors right and merges any that overlap.
    pub fn move_all_right(&mut self, buffer: &Buffer, select: bool) {
        self.for_each_mut(|c| c.move_right(buffer, select));
        self.sort_and_merge();
    }

    /// Moves all cursors up and merges any that overlap.
    pub fn move_all_up(&mut self, buffer: &Buffer, select: bool) {
        self.for_each_mut(|c| c.move_up(buffer, select));
        self.sort_and_merge();
    }

    /// Moves all cursors down and merges any that overlap.
    pub fn move_all_down(&mut self, buffer: &Buffer, select: bool) {
        self.for_each_mut(|c| c.move_down(buffer, select));
        self.sort_and_merge();
    }

    /// Moves all cursors to line start.
    pub fn move_all_to_line_start(&mut self, buffer: &Buffer, select: bool) {
        self.for_each_mut(|c| c.move_to_line_start(buffer, select));
        self.sort_and_merge();
    }

    /// Moves all cursors to line end.
    pub fn move_all_to_line_end(&mut self, buffer: &Buffer, select: bool) {
        self.for_each_mut(|c| c.move_to_line_end(buffer, select));
        self.sort_and_merge();
    }

    /// Moves all cursors one word left.
    pub fn move_all_word_left(&mut self, buffer: &Buffer, select: bool) {
        self.for_each_mut(|c| c.move_word_left(buffer, select));
        self.sort_and_merge();
    }

    /// Moves all cursors one word right.
    pub fn move_all_word_right(&mut self, buffer: &Buffer, select: bool) {
        self.for_each_mut(|c| c.move_word_right(buffer, select));
        self.sort_and_merge();
    }

    /// Moves all cursors to buffer start.
    pub fn move_all_to_buffer_start(&mut self, buffer: &Buffer, select: bool) {
        self.for_each_mut(|c| c.move_to_buffer_start(buffer, select));
        self.sort_and_merge();
    }

    /// Moves all cursors to buffer end.
    pub fn move_all_to_buffer_end(&mut self, buffer: &Buffer, select: bool) {
        self.for_each_mut(|c| c.move_to_buffer_end(buffer, select));
        self.sort_and_merge();
    }

    /// Moves all cursors one page up.
    pub fn move_all_page_up(&mut self, buffer: &Buffer, viewport_lines: u32, select: bool) {
        self.for_each_mut(|c| c.move_page_up(buffer, viewport_lines, select));
        self.sort_and_merge();
    }

    /// Moves all cursors one page down.
    pub fn move_all_page_down(&mut self, buffer: &Buffer, viewport_lines: u32, select: bool) {
        self.for_each_mut(|c| c.move_page_down(buffer, viewport_lines, select));
        self.sort_and_merge();
    }

    /// Sets the selection of the primary cursor.
    pub fn set_primary_selection(&mut self, selection: Selection) {
        self.cursors[self.primary_idx].selection = selection;
        self.cursors[self.primary_idx].preferred_column = None;
    }

    // ── Internal helpers ───────────────────────────────────────────

    fn sort_and_merge(&mut self) {
        if self.cursors.len() <= 1 {
            return;
        }

        // Track the primary cursor's identity before sorting.
        let primary_active = self.cursors[self.primary_idx].selection.active;

        self.cursors
            .sort_by(|a, b| a.selection.start().cmp(&b.selection.start()));

        // Merge overlapping cursors.
        let mut merged: Vec<CursorState> = Vec::with_capacity(self.cursors.len());
        for cursor in self.cursors.drain(..) {
            if let Some(last) = merged.last() {
                if cursor.selection.start() <= last.selection.end() {
                    // Overlap — keep the one with a broader selection.
                    let last_range = last.selection.range();
                    let curr_range = cursor.selection.range();
                    if curr_range.end > last_range.end {
                        let len = merged.len();
                        merged[len - 1] = cursor;
                    }
                    continue;
                }
            }
            merged.push(cursor);
        }

        self.cursors = merged;

        // Restore primary index by finding the cursor closest to the old primary.
        self.primary_idx = self
            .cursors
            .iter()
            .enumerate()
            .min_by_key(|(_, c)| {
                let diff_line = (i64::from(c.selection.active.line)
                    - i64::from(primary_active.line))
                .unsigned_abs();
                let diff_col = (i64::from(c.selection.active.column)
                    - i64::from(primary_active.column))
                .unsigned_abs();
                (diff_line, diff_col)
            })
            .map_or(0, |(i, _)| i);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(s: &str) -> Buffer {
        Buffer::from_str(s)
    }

    #[test]
    fn single_cursor() {
        let mc = MultiCursor::new(Position::new(0, 0));
        assert_eq!(mc.len(), 1);
        assert_eq!(mc.primary().position(), Position::new(0, 0));
    }

    #[test]
    fn add_cursor() {
        let mut mc = MultiCursor::new(Position::new(0, 0));
        mc.add_cursor(Position::new(1, 0));
        assert_eq!(mc.len(), 2);
        assert_eq!(mc.primary().position(), Position::new(1, 0));
    }

    #[test]
    fn add_cursor_above() {
        let b = buf("line1\nline2\nline3");
        let mut mc = MultiCursor::new(Position::new(2, 3));
        mc.add_cursor_above(&b);
        assert_eq!(mc.len(), 2);
        // Should have cursors on lines 1 and 2
        let positions: Vec<_> = mc.cursors().iter().map(|c| c.position()).collect();
        assert!(positions.contains(&Position::new(1, 3)));
        assert!(positions.contains(&Position::new(2, 3)));
    }

    #[test]
    fn add_cursor_below() {
        let b = buf("line1\nline2\nline3");
        let mut mc = MultiCursor::new(Position::new(0, 3));
        mc.add_cursor_below(&b);
        assert_eq!(mc.len(), 2);
        let positions: Vec<_> = mc.cursors().iter().map(|c| c.position()).collect();
        assert!(positions.contains(&Position::new(0, 3)));
        assert!(positions.contains(&Position::new(1, 3)));
    }

    #[test]
    fn add_cursor_above_at_first_line() {
        let b = buf("hello");
        let mut mc = MultiCursor::new(Position::new(0, 0));
        mc.add_cursor_above(&b);
        assert_eq!(mc.len(), 1);
    }

    #[test]
    fn add_cursor_below_at_last_line() {
        let b = buf("hello");
        let mut mc = MultiCursor::new(Position::new(0, 0));
        mc.add_cursor_below(&b);
        assert_eq!(mc.len(), 1);
    }

    #[test]
    fn collapse_to_primary() {
        let mut mc = MultiCursor::new(Position::new(0, 0));
        mc.add_cursor(Position::new(1, 0));
        mc.add_cursor(Position::new(2, 0));
        assert_eq!(mc.len(), 3);
        mc.collapse_to_primary();
        assert_eq!(mc.len(), 1);
    }

    #[test]
    fn merge_overlapping() {
        let mut mc = MultiCursor::new(Position::new(0, 5));
        mc.add_cursor(Position::new(0, 5));
        assert_eq!(mc.len(), 1);
    }

    #[test]
    fn select_all_occurrences() {
        let b = buf("foo bar foo baz foo");
        let mut mc = MultiCursor::new(Position::new(0, 0));
        mc.select_all_occurrences(&b, "foo");
        assert_eq!(mc.len(), 3);
    }

    #[test]
    fn select_all_occurrences_empty_search() {
        let b = buf("foo bar");
        let mut mc = MultiCursor::new(Position::new(0, 0));
        mc.select_all_occurrences(&b, "");
        assert_eq!(mc.len(), 1);
    }

    #[test]
    fn select_all_occurrences_no_match() {
        let b = buf("hello world");
        let mut mc = MultiCursor::new(Position::new(0, 0));
        mc.select_all_occurrences(&b, "xyz");
        assert_eq!(mc.len(), 1);
    }

    #[test]
    fn move_all_down() {
        let b = buf("aaa\nbbb\nccc");
        let mut mc = MultiCursor::new(Position::new(0, 1));
        mc.add_cursor(Position::new(1, 1));
        mc.move_all_down(&b, false);
        let positions: Vec<_> = mc.cursors().iter().map(|c| c.position()).collect();
        assert_eq!(positions, vec![Position::new(1, 1), Position::new(2, 1)]);
    }

    #[test]
    fn move_all_merges_overlapping() {
        let b = buf("ab\ncd");
        let mut mc = MultiCursor::new(Position::new(0, 0));
        mc.add_cursor(Position::new(0, 1));
        // Move both to line start — they'll overlap
        mc.move_all_to_line_start(&b, false);
        assert_eq!(mc.len(), 1);
    }

    #[test]
    fn for_each_visits_all() {
        let mut mc = MultiCursor::new(Position::new(0, 0));
        mc.add_cursor(Position::new(1, 0));
        let mut count = 0;
        mc.for_each(|_| count += 1);
        assert_eq!(count, 2);
    }
}
