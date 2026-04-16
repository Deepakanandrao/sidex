//! Terminal grid/screen buffer.
//!
//! Provides the character grid that represents the visible terminal screen,
//! along with a ring-buffer scrollback for historical lines.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Default scrollback capacity (number of lines).
const DEFAULT_SCROLLBACK_CAPACITY: usize = 10_000;

/// A single cell in the terminal grid.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct Cell {
    pub character: char,
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            character: ' ',
            fg: Color::Default,
            bg: Color::Default,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
        }
    }
}

/// Terminal color representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Color {
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

/// The terminal character grid and scrollback buffer.
pub struct TerminalGrid {
    rows: u16,
    cols: u16,
    cells: Vec<Vec<Cell>>,
    cursor_row: u16,
    cursor_col: u16,
    scroll_top: u16,
    scroll_bottom: u16,
    scrollback: VecDeque<Vec<Cell>>,
    scrollback_capacity: usize,
}

impl TerminalGrid {
    /// Creates a new grid with the given dimensions.
    pub fn new(rows: u16, cols: u16) -> Self {
        let cells = (0..rows)
            .map(|_| vec![Cell::default(); cols as usize])
            .collect();
        Self {
            rows,
            cols,
            cells,
            cursor_row: 0,
            cursor_col: 0,
            scroll_top: 0,
            scroll_bottom: rows.saturating_sub(1),
            scrollback: VecDeque::new(),
            scrollback_capacity: DEFAULT_SCROLLBACK_CAPACITY,
        }
    }

    /// Creates a grid with a custom scrollback capacity.
    pub fn with_scrollback_capacity(rows: u16, cols: u16, capacity: usize) -> Self {
        let mut grid = Self::new(rows, cols);
        grid.scrollback_capacity = capacity;
        grid
    }

    /// Returns the number of rows in the visible grid.
    pub fn rows(&self) -> u16 {
        self.rows
    }

    /// Returns the number of columns in the visible grid.
    pub fn cols(&self) -> u16 {
        self.cols
    }

    /// Returns a reference to the cell at `(row, col)`.
    ///
    /// # Panics
    ///
    /// Panics if `row >= self.rows` or `col >= self.cols`.
    pub fn cell(&self, row: u16, col: u16) -> &Cell {
        &self.cells[row as usize][col as usize]
    }

    /// Returns a mutable reference to the cell at `(row, col)`.
    ///
    /// # Panics
    ///
    /// Panics if `row >= self.rows` or `col >= self.cols`.
    pub fn cell_mut(&mut self, row: u16, col: u16) -> &mut Cell {
        &mut self.cells[row as usize][col as usize]
    }

    /// Returns the current cursor position as `(row, col)`.
    pub fn cursor_position(&self) -> (u16, u16) {
        (self.cursor_row, self.cursor_col)
    }

    /// Sets the cursor position, clamping to grid bounds.
    pub fn set_cursor(&mut self, row: u16, col: u16) {
        self.cursor_row = row.min(self.rows.saturating_sub(1));
        self.cursor_col = col.min(self.cols.saturating_sub(1));
    }

    /// Returns the scroll region as `(top, bottom)`.
    pub fn scroll_region(&self) -> (u16, u16) {
        (self.scroll_top, self.scroll_bottom)
    }

    /// Sets the scroll region. Both bounds are inclusive row indices.
    pub fn set_scroll_region(&mut self, top: u16, bottom: u16) {
        if top < bottom && bottom < self.rows {
            self.scroll_top = top;
            self.scroll_bottom = bottom;
        }
    }

    /// Clears the entire grid, resetting all cells to defaults.
    pub fn clear(&mut self) {
        for row in &mut self.cells {
            for cell in row.iter_mut() {
                *cell = Cell::default();
            }
        }
    }

    /// Clears a single line, resetting all cells in that row.
    pub fn clear_line(&mut self, row: u16) {
        if (row as usize) < self.cells.len() {
            for cell in &mut self.cells[row as usize] {
                *cell = Cell::default();
            }
        }
    }

    /// Clears from the cursor to the end of the current line.
    pub fn clear_line_from_cursor(&mut self) {
        let row = self.cursor_row as usize;
        let col = self.cursor_col as usize;
        if row < self.cells.len() {
            for cell in self.cells[row].iter_mut().skip(col) {
                *cell = Cell::default();
            }
        }
    }

    /// Clears from the start of the current line to the cursor.
    pub fn clear_line_to_cursor(&mut self) {
        let row = self.cursor_row as usize;
        let col = self.cursor_col as usize;
        if row < self.cells.len() {
            for cell in self.cells[row].iter_mut().take(col + 1) {
                *cell = Cell::default();
            }
        }
    }

    /// Clears from the cursor to the end of the screen.
    pub fn clear_below(&mut self) {
        self.clear_line_from_cursor();
        for r in (self.cursor_row + 1)..self.rows {
            self.clear_line(r);
        }
    }

    /// Clears from the start of the screen to the cursor.
    pub fn clear_above(&mut self) {
        self.clear_line_to_cursor();
        for r in 0..self.cursor_row {
            self.clear_line(r);
        }
    }

    /// Scrolls the scroll region up by one line, pushing the top line
    /// into the scrollback buffer.
    #[allow(clippy::assigning_clones)]
    pub fn scroll_up(&mut self) {
        let top = self.scroll_top as usize;
        let bottom = self.scroll_bottom as usize;
        if top > bottom || bottom >= self.cells.len() {
            return;
        }

        let evicted = self.cells[top].clone();

        // Only push to scrollback when the full screen is the scroll region.
        if self.scroll_top == 0 {
            self.scrollback.push_back(evicted);
            if self.scrollback.len() > self.scrollback_capacity {
                self.scrollback.pop_front();
            }
        }

        for r in top..bottom {
            self.cells[r] = self.cells[r + 1].clone();
        }
        self.cells[bottom] = vec![Cell::default(); self.cols as usize];
    }

    /// Scrolls the scroll region down by one line.
    #[allow(clippy::assigning_clones)]
    pub fn scroll_down(&mut self) {
        let top = self.scroll_top as usize;
        let bottom = self.scroll_bottom as usize;
        if top > bottom || bottom >= self.cells.len() {
            return;
        }

        for r in (top + 1..=bottom).rev() {
            self.cells[r] = self.cells[r - 1].clone();
        }
        self.cells[top] = vec![Cell::default(); self.cols as usize];
    }

    /// Resizes the grid, preserving content where possible.
    pub fn resize(&mut self, rows: u16, cols: u16) {
        let new_cols = cols as usize;

        // Adjust existing rows to new column count.
        for row in &mut self.cells {
            row.resize(new_cols, Cell::default());
        }

        let new_rows = rows as usize;
        let old_rows = self.cells.len();

        if new_rows > old_rows {
            // Pull lines back from scrollback if available.
            let extra = new_rows - old_rows;
            let from_scrollback = extra.min(self.scrollback.len());
            let mut restored: Vec<Vec<Cell>> = self
                .scrollback
                .drain(self.scrollback.len() - from_scrollback..)
                .collect();
            for row in &mut restored {
                row.resize(new_cols, Cell::default());
            }
            restored.append(&mut self.cells);
            self.cells = restored;

            // Fill remaining if scrollback wasn't enough.
            while self.cells.len() < new_rows {
                self.cells.push(vec![Cell::default(); new_cols]);
            }
        } else if new_rows < old_rows {
            // Push excess top lines into scrollback.
            let excess = old_rows - new_rows;
            for row in self.cells.drain(..excess) {
                self.scrollback.push_back(row);
                if self.scrollback.len() > self.scrollback_capacity {
                    self.scrollback.pop_front();
                }
            }
        }

        self.rows = rows;
        self.cols = cols;
        self.scroll_top = 0;
        self.scroll_bottom = rows.saturating_sub(1);
        self.cursor_row = self.cursor_row.min(rows.saturating_sub(1));
        self.cursor_col = self.cursor_col.min(cols.saturating_sub(1));
    }

    /// Returns the number of lines currently in the scrollback buffer.
    pub fn scrollback_len(&self) -> usize {
        self.scrollback.len()
    }

    /// Returns a reference to a scrollback line by index (0 = oldest).
    pub fn scrollback_line(&self, index: usize) -> Option<&[Cell]> {
        self.scrollback.get(index).map(Vec::as_slice)
    }

    /// Writes a character at the current cursor position with the given template cell
    /// attributes, then advances the cursor.
    pub fn write_char(&mut self, c: char, template: &Cell) {
        if self.cursor_col >= self.cols {
            self.cursor_col = 0;
            self.cursor_row += 1;
            if self.cursor_row > self.scroll_bottom {
                self.cursor_row = self.scroll_bottom;
                self.scroll_up();
            }
        }

        let row = self.cursor_row as usize;
        let col = self.cursor_col as usize;
        if row < self.cells.len() && col < self.cells[row].len() {
            self.cells[row][col] = Cell {
                character: c,
                fg: template.fg,
                bg: template.bg,
                bold: template.bold,
                italic: template.italic,
                underline: template.underline,
                strikethrough: template.strikethrough,
            };
        }
        self.cursor_col += 1;
    }

    /// Returns the text content of a given row as a string (trimming trailing spaces).
    pub fn row_text(&self, row: u16) -> String {
        if row as usize >= self.cells.len() {
            return String::new();
        }
        let text: String = self.cells[row as usize].iter().map(|c| c.character).collect();
        text.trim_end().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_grid_has_correct_dimensions() {
        let grid = TerminalGrid::new(24, 80);
        assert_eq!(grid.rows(), 24);
        assert_eq!(grid.cols(), 80);
        assert_eq!(grid.cursor_position(), (0, 0));
        assert_eq!(grid.scroll_region(), (0, 23));
    }

    #[test]
    fn cell_access_returns_default() {
        let grid = TerminalGrid::new(10, 10);
        let cell = grid.cell(0, 0);
        assert_eq!(cell.character, ' ');
        assert_eq!(cell.fg, Color::Default);
        assert_eq!(cell.bg, Color::Default);
        assert!(!cell.bold);
    }

    #[test]
    fn cell_mutation() {
        let mut grid = TerminalGrid::new(10, 10);
        grid.cell_mut(5, 5).character = 'X';
        grid.cell_mut(5, 5).bold = true;
        assert_eq!(grid.cell(5, 5).character, 'X');
        assert!(grid.cell(5, 5).bold);
    }

    #[test]
    fn clear_resets_all_cells() {
        let mut grid = TerminalGrid::new(4, 4);
        grid.cell_mut(0, 0).character = 'A';
        grid.cell_mut(3, 3).character = 'Z';
        grid.clear();
        assert_eq!(grid.cell(0, 0).character, ' ');
        assert_eq!(grid.cell(3, 3).character, ' ');
    }

    #[test]
    fn clear_line_resets_single_row() {
        let mut grid = TerminalGrid::new(4, 4);
        grid.cell_mut(1, 0).character = 'A';
        grid.cell_mut(1, 1).character = 'B';
        grid.clear_line(1);
        assert_eq!(grid.cell(1, 0).character, ' ');
        assert_eq!(grid.cell(1, 1).character, ' ');
    }

    #[test]
    fn resize_grow_preserves_content() {
        let mut grid = TerminalGrid::new(4, 4);
        grid.cell_mut(0, 0).character = 'A';
        grid.resize(6, 6);
        assert_eq!(grid.rows(), 6);
        assert_eq!(grid.cols(), 6);
        // Content from row 0 is pushed down by 2 (pulled from empty scrollback won't exist,
        // so new rows are added at bottom).
        // Since scrollback was empty, no rows restored — original content stays in place.
        assert_eq!(grid.cell(0, 0).character, 'A');
    }

    #[test]
    fn resize_shrink_pushes_to_scrollback() {
        let mut grid = TerminalGrid::new(4, 4);
        grid.cell_mut(0, 0).character = 'T';
        grid.resize(2, 4);
        assert_eq!(grid.rows(), 2);
        assert_eq!(grid.scrollback_len(), 2);
        // The first two rows were evicted; row with 'T' should be in scrollback.
        let line = grid.scrollback_line(0).unwrap();
        assert_eq!(line[0].character, 'T');
    }

    #[test]
    fn scroll_up_moves_lines_and_adds_to_scrollback() {
        let mut grid = TerminalGrid::new(3, 4);
        grid.cell_mut(0, 0).character = '0';
        grid.cell_mut(1, 0).character = '1';
        grid.cell_mut(2, 0).character = '2';
        grid.scroll_up();
        // Row 0's old content goes to scrollback.
        assert_eq!(grid.scrollback_len(), 1);
        assert_eq!(grid.scrollback_line(0).unwrap()[0].character, '0');
        // Rows shifted up.
        assert_eq!(grid.cell(0, 0).character, '1');
        assert_eq!(grid.cell(1, 0).character, '2');
        // Bottom row cleared.
        assert_eq!(grid.cell(2, 0).character, ' ');
    }

    #[test]
    fn scrollback_capacity_enforced() {
        let mut grid = TerminalGrid::with_scrollback_capacity(2, 4, 3);
        for i in 0..5 {
            grid.cell_mut(0, 0).character = char::from(b'A' + i);
            grid.scroll_up();
        }
        assert_eq!(grid.scrollback_len(), 3);
    }

    #[test]
    fn write_char_and_row_text() {
        let mut grid = TerminalGrid::new(4, 10);
        let template = Cell::default();
        for c in "Hello".chars() {
            grid.write_char(c, &template);
        }
        assert_eq!(grid.row_text(0), "Hello");
        assert_eq!(grid.cursor_position(), (0, 5));
    }

    #[test]
    fn write_char_wraps_at_end_of_line() {
        let mut grid = TerminalGrid::new(4, 3);
        let template = Cell::default();
        for c in "ABCDE".chars() {
            grid.write_char(c, &template);
        }
        assert_eq!(grid.row_text(0), "ABC");
        assert_eq!(grid.row_text(1), "DE");
    }
}
