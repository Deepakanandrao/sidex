//! Comment toggle — mirrors VS Code's `LineCommentCommand` +
//! `BlockCommentCommand`.
//!
//! Provides line-comment and block-comment toggling that operates on a buffer
//! and selection range.

use sidex_text::{Buffer, Range};

/// Toggles line comments for the given line range using `prefix` (e.g. `"//"`).
///
/// If all non-empty lines in the range already have the prefix, the prefix is
/// removed.  Otherwise, the prefix is added to every line.
pub fn toggle_line_comment(buffer: &mut Buffer, start_line: u32, end_line: u32, prefix: &str) {
    let line_count = buffer.len_lines() as u32;
    let end = end_line.min(line_count.saturating_sub(1));

    let all_commented = (start_line..=end).all(|l| {
        let content = buffer.line_content(l as usize);
        let trimmed = content.trim_start();
        trimmed.is_empty() || trimmed.starts_with(prefix)
    });

    if all_commented {
        remove_line_comments(buffer, start_line, end, prefix);
    } else {
        add_line_comments(buffer, start_line, end, prefix);
    }
}

/// Adds the comment prefix to each line.
fn add_line_comments(buffer: &mut Buffer, start: u32, end: u32, prefix: &str) {
    let prefix_with_space = format!("{prefix} ");
    for line in (start..=end).rev() {
        let content = buffer.line_content(line as usize);
        let indent_len = content.len() - content.trim_start().len();
        let insert_offset = buffer.line_to_char(line as usize) + indent_len;
        buffer.insert(insert_offset, &prefix_with_space);
    }
}

fn remove_line_comments(buffer: &mut Buffer, start: u32, end: u32, prefix: &str) {
    for line in (start..=end).rev() {
        let content = buffer.line_content(line as usize);
        let trimmed = content.trim_start();
        if !trimmed.starts_with(prefix) {
            continue;
        }
        let indent_len = content.len() - trimmed.len();
        let remove_len = if trimmed.len() > prefix.len()
            && trimmed.as_bytes().get(prefix.len()) == Some(&b' ')
        {
            prefix.len() + 1
        } else {
            prefix.len()
        };
        let char_start = buffer.line_to_char(line as usize) + indent_len;
        let char_end = char_start + remove_len;
        buffer.remove(char_start..char_end);
    }
}

/// Toggles a block comment around the given range using `open`/`close`
/// delimiters (e.g. `"/*"` and `"*/"`).
///
/// If the range is already wrapped in the delimiters, they are removed.
/// Otherwise, they are inserted.
pub fn toggle_block_comment(buffer: &mut Buffer, range: Range, open: &str, close: &str) {
    let start_offset = buffer.position_to_offset(range.start);
    let end_offset = buffer.position_to_offset(range.end);
    let text = buffer.slice(start_offset..end_offset);

    let trimmed = text.trim();
    if trimmed.starts_with(open) && trimmed.ends_with(close) {
        let inner = trimmed
            .strip_prefix(open)
            .and_then(|s| s.strip_suffix(close))
            .unwrap_or(trimmed);
        let inner = inner
            .strip_prefix(' ')
            .unwrap_or(inner);
        let inner = inner
            .strip_suffix(' ')
            .unwrap_or(inner);
        buffer.replace(start_offset..end_offset, inner);
    } else {
        let wrapped = format!("{open} {text} {close}");
        buffer.replace(start_offset..end_offset, &wrapped);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sidex_text::Position;

    fn buf(text: &str) -> Buffer {
        Buffer::from_str(text)
    }

    #[test]
    fn add_line_comments() {
        let mut buffer = buf("foo\nbar\nbaz");
        toggle_line_comment(&mut buffer, 0, 2, "//");
        let text = buffer.text();
        assert!(text.contains("// foo"));
        assert!(text.contains("// bar"));
        assert!(text.contains("// baz"));
    }

    #[test]
    fn remove_line_comments() {
        let mut buffer = buf("// foo\n// bar");
        toggle_line_comment(&mut buffer, 0, 1, "//");
        let text = buffer.text();
        assert_eq!(text, "foo\nbar");
    }

    #[test]
    fn block_comment_toggle() {
        let mut buffer = buf("hello world");
        let range = Range::new(Position::new(0, 0), Position::new(0, 11));
        toggle_block_comment(&mut buffer, range, "/*", "*/");
        assert_eq!(buffer.text(), "/* hello world */");
    }

    #[test]
    fn block_comment_remove() {
        let mut buffer = buf("/* hello */");
        let range = Range::new(Position::new(0, 0), Position::new(0, 11));
        toggle_block_comment(&mut buffer, range, "/*", "*/");
        assert_eq!(buffer.text(), "hello");
    }
}
