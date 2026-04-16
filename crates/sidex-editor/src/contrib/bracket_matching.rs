//! Bracket matching — mirrors VS Code's bracket-matching contribution.
//!
//! Tracks the matching bracket pair at the current cursor position and
//! exposes ranges for the renderer to highlight.

use sidex_text::{Buffer, Position, Range};

/// A matched pair of brackets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BracketPair {
    pub open: Range,
    pub close: Range,
}

/// Full state for the bracket-matching feature.
#[derive(Debug, Clone, Default)]
pub struct BracketMatchState {
    /// The current matching bracket pair (if cursor is adjacent to a bracket).
    pub current_pair: Option<BracketPair>,
}

const OPEN_BRACKETS: &[char] = &['(', '[', '{'];
const CLOSE_BRACKETS: &[char] = &[')', ']', '}'];

fn matching_close(ch: char) -> Option<char> {
    OPEN_BRACKETS.iter().zip(CLOSE_BRACKETS.iter())
        .find(|(&o, _)| o == ch)
        .map(|(_, &c)| c)
}

fn matching_open(ch: char) -> Option<char> {
    CLOSE_BRACKETS.iter().zip(OPEN_BRACKETS.iter())
        .find(|(&c, _)| c == ch)
        .map(|(_, &o)| o)
}

impl BracketMatchState {
    /// Updates the bracket match for the given cursor position.  Should be
    /// called whenever the cursor moves.
    pub fn update(&mut self, buffer: &Buffer, pos: Position) {
        self.current_pair = Self::find_match(buffer, pos);
    }

    /// Clears the current bracket match.
    pub fn clear(&mut self) {
        self.current_pair = None;
    }

    /// Returns the ranges to highlight (the two bracket characters).
    #[must_use]
    pub fn highlight_ranges(&self) -> Option<(Range, Range)> {
        self.current_pair.as_ref().map(|p| (p.open, p.close))
    }

    fn find_match(buffer: &Buffer, pos: Position) -> Option<BracketPair> {
        let line_count = buffer.len_lines();
        if pos.line as usize >= line_count {
            return None;
        }
        let line = buffer.line_content(pos.line as usize);
        let col = pos.column as usize;

        // Check character at cursor position
        if let Some(ch) = line.chars().nth(col) {
            if let Some(result) = Self::try_match_at(buffer, pos, ch) {
                return Some(result);
            }
        }

        // Check character before cursor
        if col > 0 {
            if let Some(ch) = line.chars().nth(col - 1) {
                let before_pos = Position::new(pos.line, pos.column - 1);
                if let Some(result) = Self::try_match_at(buffer, before_pos, ch) {
                    return Some(result);
                }
            }
        }

        None
    }

    fn try_match_at(buffer: &Buffer, pos: Position, ch: char) -> Option<BracketPair> {
        if let Some(close_ch) = matching_close(ch) {
            let open_range = Range::new(pos, Position::new(pos.line, pos.column + 1));
            if let Some(close_pos) = Self::scan_forward(buffer, pos, ch, close_ch) {
                let close_range = Range::new(close_pos, Position::new(close_pos.line, close_pos.column + 1));
                return Some(BracketPair { open: open_range, close: close_range });
            }
        } else if let Some(open_ch) = matching_open(ch) {
            let close_range = Range::new(pos, Position::new(pos.line, pos.column + 1));
            if let Some(open_pos) = Self::scan_backward(buffer, pos, open_ch, ch) {
                let open_range = Range::new(open_pos, Position::new(open_pos.line, open_pos.column + 1));
                return Some(BracketPair { open: open_range, close: close_range });
            }
        }
        None
    }

    fn scan_forward(buffer: &Buffer, start: Position, open: char, close: char) -> Option<Position> {
        let mut depth = 0i32;
        let line_count = buffer.len_lines();

        for line_idx in (start.line as usize)..line_count {
            let content = buffer.line_content(line_idx);
            let start_col = if line_idx == start.line as usize { start.column as usize } else { 0 };

            for (ci, ch) in content.chars().enumerate().skip(start_col) {
                if ch == open {
                    depth += 1;
                } else if ch == close {
                    depth -= 1;
                    if depth == 0 {
                        return Some(Position::new(line_idx as u32, ci as u32));
                    }
                }
            }
        }
        None
    }

    fn scan_backward(buffer: &Buffer, start: Position, open: char, close: char) -> Option<Position> {
        let mut depth = 0i32;

        for line_idx in (0..=start.line as usize).rev() {
            let content = buffer.line_content(line_idx);
            let chars: Vec<char> = content.chars().collect();
            let end_col = if line_idx == start.line as usize {
                start.column as usize
            } else {
                chars.len().saturating_sub(1)
            };

            for ci in (0..=end_col).rev() {
                if ci >= chars.len() { continue; }
                let ch = chars[ci];
                if ch == close {
                    depth += 1;
                } else if ch == open {
                    depth -= 1;
                    if depth == 0 {
                        return Some(Position::new(line_idx as u32, ci as u32));
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(text: &str) -> Buffer {
        Buffer::from_str(text)
    }

    #[test]
    fn matches_parens() {
        let buffer = buf("(hello)");
        let mut state = BracketMatchState::default();
        state.update(&buffer, Position::new(0, 0));
        let pair = state.current_pair.as_ref().unwrap();
        assert_eq!(pair.open.start.column, 0);
        assert_eq!(pair.close.start.column, 6);
    }

    #[test]
    fn no_match_without_bracket() {
        let buffer = buf("hello");
        let mut state = BracketMatchState::default();
        state.update(&buffer, Position::new(0, 2));
        assert!(state.current_pair.is_none());
    }
}
