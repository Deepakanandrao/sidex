//! Find/Replace widget state — mirrors VS Code's `FindReplaceState` +
//! `FindModel` + `FindDecorations`.
//!
//! This module owns the search query, match list, active match index, and
//! replacement logic. The renderer reads [`FindState`] to highlight matches
//! and position the find widget.

use sidex_text::search::{find_matches, FindMatch, FindMatchesOptions};
use sidex_text::{Buffer, Position, Range};

/// Maximum matches tracked before the engine stops counting.
pub const MATCHES_LIMIT: usize = 19_999;

/// Search option toggles (regex, case-sensitivity, whole word, etc.).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct FindOptions {
    pub is_regex: bool,
    pub match_case: bool,
    pub whole_word: bool,
    pub preserve_case: bool,
    /// When true, searching is restricted to the current selection.
    pub search_in_selection: bool,
    /// Whether the search should wrap around the document.
    pub wrap_around: bool,
}

impl Default for FindOptions {
    fn default() -> Self {
        Self {
            is_regex: false,
            match_case: false,
            whole_word: false,
            preserve_case: false,
            search_in_selection: false,
            wrap_around: true,
        }
    }
}

/// Full state of the find/replace widget.
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct FindState {
    /// The current search string entered by the user.
    pub search_string: String,
    /// The current replacement string.
    pub replace_string: String,
    /// Whether the find widget is visible.
    pub is_revealed: bool,
    /// Whether the replace row is revealed.
    pub is_replace_revealed: bool,
    /// Search option toggles.
    pub options: FindOptions,
    /// All matches in the document for the current query.
    pub matches: Vec<FindMatch>,
    /// Zero-based index of the currently active match, or `None`.
    pub active_match_idx: Option<usize>,
    /// Ranges to restrict the search to (when `search_in_selection` is true).
    pub search_scope: Option<Vec<Range>>,
    /// Search history (most-recent first).
    pub search_history: Vec<String>,
    /// Replace history (most-recent first).
    pub replace_history: Vec<String>,
}


impl FindState {
    /// Opens the find widget, optionally seeding the search string.
    pub fn reveal(&mut self, seed: Option<&str>) {
        self.is_revealed = true;
        if let Some(s) = seed {
            self.set_search_string(s.to_string());
        }
    }

    /// Closes the find widget and clears match highlights.
    pub fn dismiss(&mut self) {
        self.is_revealed = false;
        self.matches.clear();
        self.active_match_idx = None;
    }

    /// Updates the search string and pushes it into history.
    pub fn set_search_string(&mut self, s: String) {
        if !s.is_empty() && self.search_history.first() != Some(&s) {
            self.search_history.insert(0, s.clone());
            if self.search_history.len() > 50 {
                self.search_history.truncate(50);
            }
        }
        self.search_string = s;
    }

    /// Updates the replace string and pushes it into history.
    pub fn set_replace_string(&mut self, s: String) {
        if !s.is_empty() && self.replace_history.first() != Some(&s) {
            self.replace_history.insert(0, s.clone());
            if self.replace_history.len() > 50 {
                self.replace_history.truncate(50);
            }
        }
        self.replace_string = s;
    }

    // ── Toggle helpers ──────────────────────────────────────────────────

    pub fn toggle_regex(&mut self) {
        self.options.is_regex = !self.options.is_regex;
    }

    pub fn toggle_case_sensitive(&mut self) {
        self.options.match_case = !self.options.match_case;
    }

    pub fn toggle_whole_word(&mut self) {
        self.options.whole_word = !self.options.whole_word;
    }

    pub fn toggle_preserve_case(&mut self) {
        self.options.preserve_case = !self.options.preserve_case;
    }

    pub fn toggle_search_in_selection(&mut self) {
        self.options.search_in_selection = !self.options.search_in_selection;
    }

    // ── Core search operations ──────────────────────────────────────────

    /// Re-runs the search against `buffer`, populating `self.matches`.
    pub fn research(&mut self, buffer: &Buffer) {
        if self.search_string.is_empty() {
            self.matches.clear();
            self.active_match_idx = None;
            return;
        }

        let scope = if self.options.search_in_selection {
            self.search_scope.clone()
        } else {
            None
        };

        let opts = FindMatchesOptions {
            search_string: self.search_string.clone(),
            search_scope: scope,
            is_regex: self.options.is_regex,
            match_case: self.options.match_case,
            word_separators: if self.options.whole_word {
                Some(String::new())
            } else {
                None
            },
            capture_matches: false,
            limit_result_count: MATCHES_LIMIT,
        };

        self.matches = find_matches(buffer, &opts);

        if self.matches.is_empty() {
            self.active_match_idx = None;
        } else if let Some(idx) = self.active_match_idx {
            if idx >= self.matches.len() {
                self.active_match_idx = Some(0);
            }
        } else {
            self.active_match_idx = Some(0);
        }
    }

    /// Returns the currently active match range, if any.
    #[must_use]
    pub fn current_match(&self) -> Option<&FindMatch> {
        self.active_match_idx.and_then(|i| self.matches.get(i))
    }

    /// Advances to the next match, wrapping if enabled.  Returns the new
    /// active match index.
    pub fn find_next(&mut self) -> Option<usize> {
        if self.matches.is_empty() {
            return None;
        }
        let next = match self.active_match_idx {
            Some(i) => {
                if i + 1 < self.matches.len() {
                    i + 1
                } else if self.options.wrap_around {
                    0
                } else {
                    return self.active_match_idx;
                }
            }
            None => 0,
        };
        self.active_match_idx = Some(next);
        self.active_match_idx
    }

    /// Moves to the previous match, wrapping if enabled.
    pub fn find_previous(&mut self) -> Option<usize> {
        if self.matches.is_empty() {
            return None;
        }
        let prev = match self.active_match_idx {
            Some(0) => {
                if self.options.wrap_around {
                    self.matches.len() - 1
                } else {
                    return self.active_match_idx;
                }
            }
            Some(i) => i - 1,
            None => self.matches.len() - 1,
        };
        self.active_match_idx = Some(prev);
        self.active_match_idx
    }

    /// Moves the active match to the one closest to `pos` (at or after).
    pub fn find_nearest(&mut self, pos: Position) {
        if self.matches.is_empty() {
            self.active_match_idx = None;
            return;
        }
        let idx = self
            .matches
            .iter()
            .position(|m| m.range.start >= pos)
            .unwrap_or(0);
        self.active_match_idx = Some(idx);
    }

    /// Replaces the current match with `self.replace_string` and advances.
    /// Returns the replacement text that was applied, if any.
    pub fn replace_current(&mut self, buffer: &mut Buffer) -> Option<String> {
        let idx = self.active_match_idx?;
        let m = self.matches.get(idx)?;
        let range = m.range;
        let replacement = self.replacement_text(&m.matches);
        let start = buffer.position_to_offset(range.start);
        let end = buffer.position_to_offset(range.end);
        buffer.replace(start..end, &replacement);
        self.research(buffer);
        Some(replacement)
    }

    /// Replaces all matches. Returns the number of replacements made.
    pub fn replace_all(&mut self, buffer: &mut Buffer) -> usize {
        if self.matches.is_empty() {
            return 0;
        }
        // Apply replacements in reverse order to preserve earlier offsets.
        let mut count = 0;
        let matches: Vec<_> = self.matches.iter().rev().cloned().collect();
        for m in &matches {
            let replacement = self.replacement_text(&m.matches);
            let start = buffer.position_to_offset(m.range.start);
            let end = buffer.position_to_offset(m.range.end);
            buffer.replace(start..end, &replacement);
            count += 1;
        }
        self.research(buffer);
        count
    }

    /// Returns all match ranges for decoration/highlighting.
    #[must_use]
    pub fn match_ranges(&self) -> Vec<Range> {
        self.matches.iter().map(|m| m.range).collect()
    }

    /// Returns `(current_1based, total)` for status display, e.g. "3 of 42".
    #[must_use]
    pub fn match_count_display(&self) -> (usize, usize) {
        let total = self.matches.len();
        let current = self.active_match_idx.map_or(0, |i| i + 1);
        (current, total)
    }

    // ── Internal helpers ────────────────────────────────────────────────

    fn replacement_text(&self, _captures: &[String]) -> String {
        // TODO: parse replace patterns ($1, $2, \n, case transforms)
        self.replace_string.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_buffer(text: &str) -> Buffer {
        Buffer::from_str(text)
    }

    #[test]
    fn basic_find_and_navigate() {
        let buf = make_buffer("foo bar foo baz foo");
        let mut state = FindState::default();
        state.set_search_string("foo".into());
        state.research(&buf);

        assert_eq!(state.matches.len(), 3);
        assert_eq!(state.active_match_idx, Some(0));

        state.find_next();
        assert_eq!(state.active_match_idx, Some(1));

        state.find_next();
        assert_eq!(state.active_match_idx, Some(2));

        // wrap around
        state.find_next();
        assert_eq!(state.active_match_idx, Some(0));
    }

    #[test]
    fn find_previous_wraps() {
        let buf = make_buffer("a a a");
        let mut state = FindState::default();
        state.set_search_string("a".into());
        state.research(&buf);

        assert_eq!(state.active_match_idx, Some(0));
        state.find_previous();
        assert_eq!(state.active_match_idx, Some(2));
    }

    #[test]
    fn replace_all_returns_count() {
        let mut buf = make_buffer("aaa");
        let mut state = FindState::default();
        state.set_search_string("a".into());
        state.set_replace_string("bb".into());
        state.research(&buf);

        let count = state.replace_all(&mut buf);
        assert_eq!(count, 3);
        assert_eq!(buf.text(), "bbbbbb");
    }

    #[test]
    fn dismiss_clears() {
        let buf = make_buffer("hello");
        let mut state = FindState::default();
        state.set_search_string("hello".into());
        state.research(&buf);
        assert!(!state.matches.is_empty());

        state.dismiss();
        assert!(state.matches.is_empty());
        assert!(!state.is_revealed);
    }
}
