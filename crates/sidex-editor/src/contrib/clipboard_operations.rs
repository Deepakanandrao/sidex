//! Clipboard operations — mirrors VS Code's clipboard contribution.
//!
//! Enhanced cut/copy/paste: copy with syntax highlighting data, paste with
//! auto-indentation, multi-cursor paste distribution.

use sidex_text::{Buffer, Position, Range};

/// Metadata attached to a clipboard entry for rich paste.
#[derive(Debug, Clone, Default)]
pub struct ClipboardMetadata {
    /// Whether the clipboard contains a full line (should paste as a new line).
    pub is_full_line: bool,
    /// Number of cursors that produced this clipboard content.
    pub cursor_count: usize,
    /// Per-cursor text segments (for distributing paste across cursors).
    pub segments: Vec<String>,
    /// Optional syntax-highlighted HTML for rich paste into other apps.
    pub html: Option<String>,
}

/// Copies the selected text and produces clipboard metadata.
#[must_use]
pub fn copy_selections(buffer: &Buffer, selections: &[Range]) -> (String, ClipboardMetadata) {
    let mut texts = Vec::with_capacity(selections.len());
    for sel in selections {
        let start = buffer.position_to_offset(sel.start);
        let end = buffer.position_to_offset(sel.end);
        texts.push(buffer.slice(start..end));
    }

    let full_text = texts.join("\n");
    let metadata = ClipboardMetadata {
        is_full_line: false,
        cursor_count: selections.len(),
        segments: texts,
        html: None,
    };
    (full_text, metadata)
}

/// Copies a full line (no selection) — the paste should insert a new line.
#[must_use]
pub fn copy_line(buffer: &Buffer, line: u32) -> (String, ClipboardMetadata) {
    let content = buffer.line_content(line as usize).clone();
    let metadata = ClipboardMetadata {
        is_full_line: true,
        cursor_count: 1,
        segments: vec![content.clone()],
        html: None,
    };
    (content, metadata)
}

/// Pastes text, auto-indenting each line to match the current cursor line's
/// indentation.
pub fn paste_and_auto_indent(
    buffer: &mut Buffer,
    pos: Position,
    text: &str,
    _tab_size: u32,
    _use_spaces: bool,
) {
    if text.is_empty() {
        return;
    }

    let current_line = buffer.line_content(pos.line as usize);
    let base_indent = leading_whitespace(&current_line);

    let lines: Vec<&str> = text.lines().collect();
    if lines.len() <= 1 {
        let offset = buffer.position_to_offset(pos);
        buffer.insert(offset, text);
        return;
    }

    let paste_indent = lines
        .iter()
        .skip(1)
        .filter(|l| !l.trim().is_empty())
        .map(|l| leading_whitespace(l))
        .min()
        .unwrap_or_default();

    let mut result = String::new();
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            result.push('\n');
            if !line.trim().is_empty() {
                result.push_str(&base_indent);
                let stripped = strip_indent(line, &paste_indent);
                result.push_str(stripped);
            }
        } else {
            result.push_str(line);
        }
    }
    if text.ends_with('\n') {
        result.push('\n');
    }

    let offset = buffer.position_to_offset(pos);
    buffer.insert(offset, &result);
}

/// Pastes with multi-cursor distribution.
pub fn paste_distributed(
    buffer: &mut Buffer,
    positions: &[Position],
    metadata: &ClipboardMetadata,
) -> bool {
    if metadata.segments.len() != positions.len() || positions.is_empty() {
        return false;
    }

    let mut pairs: Vec<_> = positions.iter().zip(metadata.segments.iter()).collect();
    pairs.sort_by(|a, b| b.0.cmp(a.0));

    for (pos, text) in pairs {
        let offset = buffer.position_to_offset(*pos);
        buffer.insert(offset, text);
    }
    true
}

fn leading_whitespace(line: &str) -> String {
    line.chars().take_while(|c| c.is_whitespace()).collect()
}

fn strip_indent<'a>(line: &'a str, indent: &str) -> &'a str {
    line.strip_prefix(indent).unwrap_or(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(text: &str) -> Buffer {
        Buffer::from_str(text)
    }

    #[test]
    fn copy_multiple_selections() {
        let buffer = buf("foo bar baz");
        let sels = vec![
            Range::new(Position::new(0, 0), Position::new(0, 3)),
            Range::new(Position::new(0, 8), Position::new(0, 11)),
        ];
        let (text, meta) = copy_selections(&buffer, &sels);
        assert_eq!(text, "foo\nbaz");
        assert_eq!(meta.segments.len(), 2);
    }

    #[test]
    fn copy_full_line() {
        let buffer = buf("hello\nworld");
        let (text, meta) = copy_line(&buffer, 0);
        assert_eq!(text, "hello");
        assert!(meta.is_full_line);
    }

    #[test]
    fn paste_distributed_works() {
        let mut buffer = buf("aa bb");
        let positions = vec![Position::new(0, 2), Position::new(0, 5)];
        let meta = ClipboardMetadata {
            is_full_line: false,
            cursor_count: 2,
            segments: vec!["X".into(), "Y".into()],
            html: None,
        };
        let ok = paste_distributed(&mut buffer, &positions, &meta);
        assert!(ok);
        let text = buffer.text();
        assert!(text.contains("aaX"));
        assert!(text.contains("bbY"));
    }
}
