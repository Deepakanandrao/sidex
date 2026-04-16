//! LSP semantic tokens overlay for merging semantic token data with syntax
//! highlighting events.
//!
//! Semantic tokens provide richer type information from the language server
//! that overrides the coarser tree-sitter/TextMate tokens where they exist.

use serde::{Deserialize, Serialize};

use crate::highlight::HighlightEvent;

/// A single semantic token as received from an LSP server (absolute coordinates).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticToken {
    pub line: u32,
    pub start: u32,
    pub length: u32,
    pub token_type: u32,
    pub modifiers: u32,
}

/// Maps token type/modifier indices to their string names, as negotiated
/// during LSP initialization.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SemanticTokenLegend {
    pub token_types: Vec<String>,
    pub token_modifiers: Vec<String>,
}

impl SemanticTokenLegend {
    #[must_use]
    pub fn new(token_types: Vec<String>, token_modifiers: Vec<String>) -> Self {
        Self {
            token_types,
            token_modifiers,
        }
    }

    /// Resolves a token type index to its name.
    #[must_use]
    pub fn type_name(&self, idx: u32) -> Option<&str> {
        self.token_types.get(idx as usize).map(String::as_str)
    }

    /// Returns the modifier names for the given bitmask.
    #[must_use]
    pub fn modifier_names(&self, mask: u32) -> Vec<&str> {
        let mut names = Vec::new();
        for (i, name) in self.token_modifiers.iter().enumerate() {
            if mask & (1 << i) != 0 {
                names.push(name.as_str());
            }
        }
        names
    }
}

/// A styled span in the final merged output, carrying both position and
/// semantic information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledSpan {
    /// Start byte offset in the source.
    pub start: usize,
    /// End byte offset in the source.
    pub end: usize,
    /// The token type name (e.g. `"function"`, `"variable"`), or `None` for
    /// unstyled source text.
    pub token_type: Option<String>,
    /// Modifier names (e.g. `"declaration"`, `"readonly"`).
    pub modifiers: Vec<String>,
}

/// Merges syntax highlight events with LSP semantic tokens.
///
/// Where a semantic token overlaps a syntax span, the semantic information
/// takes priority. Gaps between semantic tokens fall back to the syntax
/// highlighting.
pub fn merge_semantic_tokens(
    syntax_tokens: &[HighlightEvent],
    semantic: &[SemanticToken],
    legend: &SemanticTokenLegend,
    source: &str,
) -> Vec<StyledSpan> {
    let mut spans = Vec::new();

    let sem_ranges: Vec<(usize, usize, Option<String>, Vec<String>)> = semantic
        .iter()
        .filter_map(|tok| {
            let byte_start = line_col_to_byte(source, tok.line, tok.start)?;
            let byte_end = byte_start + tok.length as usize;
            let type_name = legend.type_name(tok.token_type).map(String::from);
            let mods = legend
                .modifier_names(tok.modifiers)
                .iter()
                .map(|s| (*s).to_owned())
                .collect();
            Some((byte_start, byte_end, type_name, mods))
        })
        .collect();

    let mut syntax_spans: Vec<(usize, usize)> = Vec::new();
    for event in syntax_tokens {
        if let HighlightEvent::Source { start, end } = event {
            syntax_spans.push((*start, *end));
        }
    }

    if syntax_spans.is_empty() && sem_ranges.is_empty() {
        return spans;
    }

    let max_byte = syntax_spans
        .iter()
        .map(|(_, e)| *e)
        .chain(sem_ranges.iter().map(|(_, e, _, _)| *e))
        .max()
        .unwrap_or(0);

    let mut pos = 0;
    while pos < max_byte {
        if let Some((s, e, ref ty, ref mods)) = sem_ranges.iter().find(|(s, _, _, _)| *s == pos) {
            spans.push(StyledSpan {
                start: *s,
                end: *e,
                token_type: ty.clone(),
                modifiers: mods.clone(),
            });
            pos = *e;
            continue;
        }

        let next_sem_start = sem_ranges
            .iter()
            .filter(|(s, _, _, _)| *s > pos)
            .map(|(s, _, _, _)| *s)
            .min()
            .unwrap_or(max_byte);

        let gap_end = next_sem_start.min(max_byte);
        if pos < gap_end {
            spans.push(StyledSpan {
                start: pos,
                end: gap_end,
                token_type: None,
                modifiers: Vec::new(),
            });
        }
        pos = gap_end;
    }

    spans
}

/// Decodes delta-encoded semantic tokens into absolute-position tokens.
///
/// The LSP protocol sends tokens as a flat `[deltaLine, deltaStartChar,
/// length, tokenType, tokenModifiers]` array. This function converts them
/// to absolute [`SemanticToken`] values.
pub fn decode_semantic_tokens(data: &[u32]) -> Vec<SemanticToken> {
    let mut tokens = Vec::with_capacity(data.len() / 5);
    let mut line = 0u32;
    let mut start = 0u32;

    for chunk in data.chunks_exact(5) {
        let delta_line = chunk[0];
        let delta_start = chunk[1];
        let length = chunk[2];
        let token_type = chunk[3];
        let modifiers = chunk[4];

        line += delta_line;
        if delta_line > 0 {
            start = delta_start;
        } else {
            start += delta_start;
        }

        tokens.push(SemanticToken {
            line,
            start,
            length,
            token_type,
            modifiers,
        });
    }

    tokens
}

/// Encodes absolute semantic tokens back into delta-encoded format.
pub fn encode_semantic_tokens(tokens: &[SemanticToken]) -> Vec<u32> {
    let mut data = Vec::with_capacity(tokens.len() * 5);
    let mut prev_line = 0u32;
    let mut prev_start = 0u32;

    for tok in tokens {
        let delta_line = tok.line - prev_line;
        let delta_start = if delta_line > 0 {
            tok.start
        } else {
            tok.start - prev_start
        };

        data.push(delta_line);
        data.push(delta_start);
        data.push(tok.length);
        data.push(tok.token_type);
        data.push(tok.modifiers);

        prev_line = tok.line;
        prev_start = tok.start;
    }

    data
}

/// Applies an incremental semantic token edit (delta) to existing data.
pub fn apply_semantic_token_edits(data: &mut Vec<u32>, edits: &[SemanticTokenEdit]) {
    for edit in edits.iter().rev() {
        let start = edit.start as usize;
        let delete_count = edit.delete_count as usize;
        let end = (start + delete_count).min(data.len());
        data.splice(start..end, edit.data.iter().copied());
    }
}

/// A single incremental edit to a semantic token data array.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticTokenEdit {
    pub start: u32,
    pub delete_count: u32,
    pub data: Vec<u32>,
}

fn line_col_to_byte(source: &str, line: u32, col: u32) -> Option<usize> {
    let mut byte_offset = 0usize;

    for (current_line, src_line) in source.split('\n').enumerate() {
        #[allow(clippy::cast_possible_truncation)]
        if current_line == line as usize {
            let col_byte = src_line
                .char_indices()
                .nth(col as usize)
                .map_or(src_line.len(), |(i, _)| i);
            return Some(byte_offset + col_byte);
        }
        byte_offset += src_line.len() + 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_empty() {
        let tokens = decode_semantic_tokens(&[]);
        assert!(tokens.is_empty());
    }

    #[test]
    fn decode_single_token() {
        let data = vec![0, 5, 3, 1, 0];
        let tokens = decode_semantic_tokens(&data);
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].line, 0);
        assert_eq!(tokens[0].start, 5);
        assert_eq!(tokens[0].length, 3);
        assert_eq!(tokens[0].token_type, 1);
        assert_eq!(tokens[0].modifiers, 0);
    }

    #[test]
    fn decode_multiple_same_line() {
        let data = vec![0, 5, 3, 0, 0, 0, 10, 4, 1, 0];
        let tokens = decode_semantic_tokens(&data);
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].line, 0);
        assert_eq!(tokens[0].start, 5);
        assert_eq!(tokens[1].line, 0);
        assert_eq!(tokens[1].start, 15);
    }

    #[test]
    fn decode_different_lines() {
        let data = vec![0, 5, 3, 0, 0, 2, 3, 4, 1, 0];
        let tokens = decode_semantic_tokens(&data);
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0].line, 0);
        assert_eq!(tokens[1].line, 2);
        assert_eq!(tokens[1].start, 3);
    }

    #[test]
    fn encode_roundtrip() {
        let original = vec![0, 5, 3, 1, 0, 2, 3, 4, 2, 1, 0, 10, 2, 0, 3];
        let tokens = decode_semantic_tokens(&original);
        let re_encoded = encode_semantic_tokens(&tokens);
        assert_eq!(original, re_encoded);
    }

    #[test]
    fn legend_type_name() {
        let legend = SemanticTokenLegend::new(
            vec!["namespace".into(), "type".into(), "function".into()],
            vec!["declaration".into(), "readonly".into()],
        );
        assert_eq!(legend.type_name(0), Some("namespace"));
        assert_eq!(legend.type_name(2), Some("function"));
        assert_eq!(legend.type_name(99), None);
    }

    #[test]
    fn legend_modifier_names() {
        let legend = SemanticTokenLegend::new(
            vec![],
            vec!["declaration".into(), "readonly".into(), "static".into()],
        );
        let names = legend.modifier_names(0b101);
        assert_eq!(names, vec!["declaration", "static"]);
    }

    #[test]
    fn merge_with_no_semantic() {
        let syntax = vec![HighlightEvent::Source { start: 0, end: 10 }];
        let legend = SemanticTokenLegend::default();
        let spans = merge_semantic_tokens(&syntax, &[], &legend, "0123456789");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].start, 0);
        assert_eq!(spans[0].end, 10);
        assert!(spans[0].token_type.is_none());
    }

    #[test]
    fn merge_with_semantic_override() {
        let syntax = vec![HighlightEvent::Source { start: 0, end: 10 }];
        let legend = SemanticTokenLegend::new(vec!["variable".into()], vec![]);
        let semantic = vec![SemanticToken {
            line: 0,
            start: 0,
            length: 5,
            token_type: 0,
            modifiers: 0,
        }];
        let spans = merge_semantic_tokens(&syntax, &semantic, &legend, "hello world");
        let typed: Vec<_> = spans.iter().filter(|s| s.token_type.is_some()).collect();
        assert!(!typed.is_empty());
        assert_eq!(typed[0].token_type.as_deref(), Some("variable"));
    }

    #[test]
    fn apply_edit() {
        let mut data = vec![0, 5, 3, 1, 0, 0, 10, 4, 2, 0];
        let edit = SemanticTokenEdit {
            start: 5,
            delete_count: 5,
            data: vec![1, 3, 2, 3, 0],
        };
        apply_semantic_token_edits(&mut data, &[edit]);
        assert_eq!(data.len(), 10);
        assert_eq!(data[5..10], [1, 3, 2, 3, 0]);
    }

    #[test]
    fn styled_span_fields() {
        let span = StyledSpan {
            start: 0,
            end: 5,
            token_type: Some("function".into()),
            modifiers: vec!["declaration".into()],
        };
        assert_eq!(span.token_type.as_deref(), Some("function"));
        assert_eq!(span.modifiers, vec!["declaration"]);
    }

    #[test]
    fn line_col_to_byte_basic() {
        let source = "hello\nworld\nfoo";
        assert_eq!(line_col_to_byte(source, 0, 0), Some(0));
        assert_eq!(line_col_to_byte(source, 0, 3), Some(3));
        assert_eq!(line_col_to_byte(source, 1, 0), Some(6));
        assert_eq!(line_col_to_byte(source, 2, 0), Some(12));
    }
}
