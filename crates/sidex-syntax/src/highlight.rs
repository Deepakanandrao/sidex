//! Syntax highlighting engine powered by tree-sitter queries.
//!
//! The [`Highlighter`] runs compiled tree-sitter highlight queries against a
//! source string, producing a stream of [`HighlightEvent`]s that downstream
//! renderers consume to colorize text.

use std::ops::Range as StdRange;

use streaming_iterator::StreamingIterator;
use tree_sitter::{Language, Node, Parser, Query, QueryCursor};

use crate::scope::resolve_highlight_name;

/// An opaque highlight index into the capture-name list of a [`HighlightConfig`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Highlight(pub u32);

/// Events emitted during highlighting, consumed by the renderer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HighlightEvent {
    /// A span of un-highlighted source text from byte `start` to byte `end`.
    Source { start: usize, end: usize },
    /// Begin a highlighted region with the given capture index.
    HighlightStart(Highlight),
    /// End the most recently started highlighted region.
    HighlightEnd,
}

/// Compiled configuration for highlighting a single language.
///
/// Wraps a tree-sitter [`Query`] together with the list of capture names
/// so that highlight indices can be resolved to semantic categories.
pub struct HighlightConfig {
    pub(crate) query: Query,
    pub(crate) capture_names: Vec<String>,
    pub(crate) language: Language,
}

impl std::fmt::Debug for HighlightConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HighlightConfig")
            .field("capture_names", &self.capture_names)
            .finish_non_exhaustive()
    }
}

/// Errors that can occur when constructing a [`HighlightConfig`].
#[derive(Debug, thiserror::Error)]
pub enum HighlightError {
    /// The highlight query could not be compiled.
    #[error("invalid highlight query: {0}")]
    InvalidQuery(#[from] tree_sitter::QueryError),
    /// The parser failed to parse the source.
    #[error("tree-sitter parse failed")]
    ParseFailed,
}

impl HighlightConfig {
    /// Creates a new highlight configuration from a tree-sitter language and
    /// a `highlights.scm` query source string.
    pub fn new(language: Language, query_source: &str) -> Result<Self, HighlightError> {
        let query = Query::new(&language, query_source)?;
        let capture_names = query
            .capture_names()
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        Ok(Self {
            query,
            capture_names,
            language,
        })
    }

    /// Returns the list of capture names defined in the query.
    #[must_use]
    pub fn capture_names(&self) -> &[String] {
        &self.capture_names
    }

    /// Resolves a [`Highlight`] index to its capture name string.
    #[must_use]
    pub fn capture_name(&self, highlight: Highlight) -> Option<&str> {
        self.capture_names
            .get(highlight.0 as usize)
            .map(String::as_str)
    }
}

/// Reusable syntax highlighter.
///
/// Holds a tree-sitter [`Parser`] and [`QueryCursor`] to avoid repeated
/// allocation across highlight calls.
pub struct Highlighter {
    parser: Parser,
    cursor: QueryCursor,
}

impl std::fmt::Debug for Highlighter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Highlighter").finish_non_exhaustive()
    }
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl Highlighter {
    /// Creates a new reusable highlighter.
    #[must_use]
    pub fn new() -> Self {
        Self {
            parser: Parser::new(),
            cursor: QueryCursor::new(),
        }
    }

    /// Highlights `source` using the given [`HighlightConfig`].
    ///
    /// If `byte_ranges` is provided, only matches overlapping those byte ranges
    /// are emitted (useful for highlighting only the visible viewport).
    pub fn highlight(
        &mut self,
        config: &HighlightConfig,
        source: &str,
        byte_ranges: Option<&[StdRange<usize>]>,
    ) -> Result<Vec<HighlightEvent>, HighlightError> {
        self.parser
            .set_language(&config.language)
            .expect("language version mismatch");

        let tree = self
            .parser
            .parse(source, None)
            .ok_or(HighlightError::ParseFailed)?;

        if let Some(ranges) = byte_ranges {
            let ts_ranges: Vec<tree_sitter::Range> = ranges
                .iter()
                .map(|r| tree_sitter::Range {
                    start_byte: r.start,
                    end_byte: r.end,
                    start_point: byte_offset_to_point(source, r.start),
                    end_point: byte_offset_to_point(source, r.end),
                })
                .collect();
            self.cursor.set_byte_range(0..source.len());
            // Pre-filter: only iterate matches in the given ranges.
            if let Some(first) = ts_ranges.first() {
                let last = ts_ranges.last().unwrap_or(first);
                self.cursor.set_byte_range(first.start_byte..last.end_byte);
            }
        } else {
            self.cursor.set_byte_range(0..source.len());
        }

        let root = tree.root_node();
        let events = Self::collect_events(&mut self.cursor, config, root, source, byte_ranges);
        Ok(events)
    }

    /// Walk query matches and convert them into a flat event stream.
    fn collect_events(
        cursor: &mut QueryCursor,
        config: &HighlightConfig,
        root: Node<'_>,
        source: &str,
        byte_ranges: Option<&[StdRange<usize>]>,
    ) -> Vec<HighlightEvent> {
        let mut events: Vec<HighlightEvent> = Vec::new();

        // Collect all captured spans sorted by start byte, breaking ties by
        // longer spans first (so nesting works correctly).
        let mut spans: Vec<(usize, usize, u32)> = Vec::new();
        let mut matches = cursor.matches(&config.query, root, source.as_bytes());
        while let Some(m) = matches.next() {
            for capture in m.captures {
                let node = capture.node;
                let start = node.start_byte();
                let end = node.end_byte();

                if let Some(ranges) = byte_ranges {
                    let overlaps = ranges.iter().any(|r| start < r.end && end > r.start);
                    if !overlaps {
                        continue;
                    }
                }

                #[allow(clippy::cast_possible_truncation)]
                spans.push((start, end, capture.index));
            }
        }

        spans.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
        spans.dedup();

        let source_len = source.len();
        let mut pos = 0;

        for (start, end, capture_idx) in &spans {
            let start = *start;
            let end = *end;
            let capture_idx = *capture_idx;

            // Only emit captures that map to a known highlight name.
            let capture_name = &config.capture_names[capture_idx as usize];
            if resolve_highlight_name(capture_name).is_none() {
                continue;
            }

            if start > pos {
                events.push(HighlightEvent::Source {
                    start: pos,
                    end: start,
                });
            }

            events.push(HighlightEvent::HighlightStart(Highlight(capture_idx)));
            events.push(HighlightEvent::Source {
                start,
                end: end.min(source_len),
            });
            events.push(HighlightEvent::HighlightEnd);

            pos = end;
        }

        if pos < source_len {
            events.push(HighlightEvent::Source {
                start: pos,
                end: source_len,
            });
        }

        events
    }
}

/// Convert a byte offset into a tree-sitter `Point` (row/column).
fn byte_offset_to_point(source: &str, byte_offset: usize) -> tree_sitter::Point {
    let slice = &source[..byte_offset.min(source.len())];
    let row = slice.bytes().filter(|&b| b == b'\n').count();
    let last_newline = slice.rfind('\n').map_or(0, |i| i + 1);
    let column = byte_offset - last_newline;
    tree_sitter::Point { row, column }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rust_config_with_query(query_src: &str) -> HighlightConfig {
        let lang: Language = tree_sitter_rust::LANGUAGE.into();
        HighlightConfig::new(lang, query_src).expect("valid query")
    }

    #[test]
    fn highlight_config_capture_names() {
        let config = rust_config_with_query(
            r#"(line_comment) @comment
(string_literal) @string"#,
        );
        assert!(config.capture_names().contains(&"comment".to_string()));
        assert!(config.capture_names().contains(&"string".to_string()));
    }

    #[test]
    fn highlight_config_resolve_capture() {
        let config = rust_config_with_query("(line_comment) @comment");
        let name = config.capture_name(Highlight(0));
        assert_eq!(name, Some("comment"));
    }

    #[test]
    fn highlight_empty_source() {
        let config = rust_config_with_query("(line_comment) @comment");
        let mut hl = Highlighter::new();
        let events = hl.highlight(&config, "", None).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn highlight_produces_comment_events() {
        let config = rust_config_with_query("(line_comment) @comment");
        let source = "// hello\nlet x = 1;\n";
        let mut hl = Highlighter::new();
        let events = hl.highlight(&config, source, None).unwrap();

        let has_comment_start = events
            .iter()
            .any(|e| matches!(e, HighlightEvent::HighlightStart(Highlight(0))));
        assert!(has_comment_start, "expected a HighlightStart for comment");

        let has_end = events
            .iter()
            .any(|e| matches!(e, HighlightEvent::HighlightEnd));
        assert!(has_end, "expected a HighlightEnd");
    }

    #[test]
    fn highlight_with_byte_range() {
        let config = rust_config_with_query("(line_comment) @comment");
        let source = "let x = 1;\n// second\nlet y = 2;\n";
        let mut hl = Highlighter::new();
        let events = hl.highlight(&config, source, Some(&[11..21])).unwrap();

        let has_comment = events
            .iter()
            .any(|e| matches!(e, HighlightEvent::HighlightStart(Highlight(0))));
        assert!(
            has_comment,
            "comment in the visible range should be highlighted"
        );
    }

    #[test]
    fn byte_offset_to_point_basic() {
        let source = "abc\ndef\nghi";
        let p = byte_offset_to_point(source, 5);
        assert_eq!(p.row, 1);
        assert_eq!(p.column, 1);
    }
}
