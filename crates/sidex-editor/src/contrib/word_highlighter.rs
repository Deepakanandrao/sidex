//! Word highlighter — mirrors VS Code's `WordHighlighter` contribution.
//!
//! Highlights all occurrences of the word under the cursor (debounced).  Also
//! supports LSP `documentHighlight` results.

use sidex_text::{Buffer, Position, Range};

/// The kind of a document highlight (from LSP).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum DocumentHighlightKind {
    /// A textual occurrence.
    #[default]
    Text,
    /// A read-access to a symbol.
    Read,
    /// A write-access to a symbol.
    Write,
}

/// A single highlight range with its kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightRange {
    pub range: Range,
    pub kind: DocumentHighlightKind,
}

/// Full state for the word-highlighter feature.
#[derive(Debug, Clone, Default)]
pub struct WordHighlightState {
    /// The currently highlighted ranges.
    pub highlights: Vec<HighlightRange>,
    /// The word that is currently highlighted (for display/debugging).
    pub highlighted_word: Option<String>,
    /// Debounce delay in milliseconds (default 250ms).
    pub debounce_ms: u64,
    /// Whether a highlight request is in-flight.
    pub is_loading: bool,
}


impl WordHighlightState {
    pub fn new() -> Self {
        Self {
            debounce_ms: 250,
            ..Self::default()
        }
    }

    /// Computes textual word highlights by finding all occurrences of the word
    /// at the cursor position.  This is the fallback when no LSP provider is
    /// available.
    pub fn highlight_word_at_cursor(&mut self, buffer: &Buffer, pos: Position) {
        self.highlights.clear();
        self.highlighted_word = None;

        let line_count = buffer.len_lines();
        if pos.line as usize >= line_count {
            return;
        }

        let line = buffer.line_content(pos.line as usize);
        let col = pos.column as usize;

        // Find word boundaries at cursor
        let chars: Vec<char> = line.chars().collect();
        if col >= chars.len() || !chars[col].is_alphanumeric() && chars[col] != '_' {
            return;
        }

        let start = (0..col)
            .rev()
            .take_while(|&i| chars[i].is_alphanumeric() || chars[i] == '_')
            .last()
            .unwrap_or(col);
        let end = (col..chars.len())
            .take_while(|&i| chars[i].is_alphanumeric() || chars[i] == '_')
            .last()
            .map_or(col, |i| i + 1);

        let word: String = chars[start..end].iter().collect();
        if word.is_empty() {
            return;
        }

        self.highlighted_word = Some(word.clone());

        for line_idx in 0..line_count {
            let content = buffer.line_content(line_idx);
            let mut search_start = 0;
            while let Some(found) = content[search_start..].find(&word) {
                let abs_start = search_start + found;
                let abs_end = abs_start + word.len();

                // Ensure whole-word match
                let before_ok = abs_start == 0 || {
                    let ch = content.as_bytes()[abs_start - 1];
                    !ch.is_ascii_alphanumeric() && ch != b'_'
                };
                let after_ok = abs_end >= content.len() || {
                    let ch = content.as_bytes()[abs_end];
                    !ch.is_ascii_alphanumeric() && ch != b'_'
                };

                if before_ok && after_ok {
                    self.highlights.push(HighlightRange {
                        range: Range::new(
                            Position::new(line_idx as u32, abs_start as u32),
                            Position::new(line_idx as u32, abs_end as u32),
                        ),
                        kind: DocumentHighlightKind::Text,
                    });
                }

                search_start = abs_end;
            }
        }
    }

    /// Receives LSP document-highlight results.
    pub fn set_lsp_highlights(&mut self, highlights: Vec<HighlightRange>) {
        self.highlights = highlights;
        self.is_loading = false;
    }

    /// Clears all highlights.
    pub fn clear(&mut self) {
        self.highlights.clear();
        self.highlighted_word = None;
        self.is_loading = false;
    }

    /// Returns just the ranges for rendering.
    #[must_use]
    pub fn ranges(&self) -> Vec<Range> {
        self.highlights.iter().map(|h| h.range).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(text: &str) -> Buffer {
        Buffer::from_str(text)
    }

    #[test]
    fn highlights_word_occurrences() {
        let buffer = buf("let foo = foo + bar;");
        let mut state = WordHighlightState::new();
        state.highlight_word_at_cursor(&buffer, Position::new(0, 4));
        assert_eq!(state.highlighted_word.as_deref(), Some("foo"));
        assert_eq!(state.highlights.len(), 2);
    }

    #[test]
    fn no_highlight_on_whitespace() {
        let buffer = buf("hello world");
        let mut state = WordHighlightState::new();
        state.highlight_word_at_cursor(&buffer, Position::new(0, 5));
        assert!(state.highlights.is_empty());
    }
}
