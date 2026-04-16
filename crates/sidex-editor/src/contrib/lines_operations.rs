//! Line manipulation operations — mirrors VS Code's `linesOperations`
//! contribution.
//!
//! Sort, reverse, deduplicate, join, duplicate, and move lines.

use sidex_text::{Buffer, Range};

/// Sorts lines in the given range alphabetically.
pub fn sort_lines(buffer: &mut Buffer, start_line: u32, end_line: u32, descending: bool) {
    let (start, end) = clamp_lines(buffer, start_line, end_line);
    let mut lines: Vec<String> = (start..=end)
        .map(|l| buffer.line_content(l as usize).clone())
        .collect();

    if descending {
        lines.sort_by(|a, b| b.cmp(a));
    } else {
        lines.sort();
    }

    replace_lines(buffer, start, end, &lines);
}

/// Reverses the order of lines in the given range.
pub fn reverse_lines(buffer: &mut Buffer, start_line: u32, end_line: u32) {
    let (start, end) = clamp_lines(buffer, start_line, end_line);
    let mut lines: Vec<String> = (start..=end)
        .map(|l| buffer.line_content(l as usize).clone())
        .collect();
    lines.reverse();
    replace_lines(buffer, start, end, &lines);
}

/// Removes duplicate consecutive lines in the given range.
pub fn unique_lines(buffer: &mut Buffer, start_line: u32, end_line: u32) {
    let (start, end) = clamp_lines(buffer, start_line, end_line);
    let lines: Vec<String> = (start..=end)
        .map(|l| buffer.line_content(l as usize).clone())
        .collect();

    let mut unique = Vec::with_capacity(lines.len());
    for line in &lines {
        if unique.last() != Some(line) {
            unique.push(line.clone());
        }
    }

    replace_lines(buffer, start, end, &unique);
}

/// Joins lines in the given range into a single line, separated by a space.
pub fn join_lines(buffer: &mut Buffer, start_line: u32, end_line: u32) {
    let (start, end) = clamp_lines(buffer, start_line, end_line);
    if start == end {
        return;
    }

    let mut joined = String::new();
    for l in start..=end {
        let content = buffer.line_content(l as usize);
        let trimmed = if l == start {
            content.trim_end().to_string()
        } else {
            content.trim().to_string()
        };
        if !joined.is_empty() && !trimmed.is_empty() {
            joined.push(' ');
        }
        joined.push_str(&trimmed);
    }

    replace_lines(buffer, start, end, &[joined]);
}

/// Duplicates the given line, inserting a copy below it.
pub fn duplicate_line(buffer: &mut Buffer, line: u32) {
    let line_count = buffer.len_lines() as u32;
    if line >= line_count {
        return;
    }
    let content = buffer.line_content(line as usize).clone();
    let end_of_line = buffer.line_to_char(line as usize) + buffer.line_content_len(line as usize);
    buffer.insert(end_of_line, &format!("\n{content}"));
}

/// Moves a line up by one position.
pub fn move_line_up(buffer: &mut Buffer, line: u32) {
    if line == 0 || line as usize >= buffer.len_lines() {
        return;
    }
    let current = buffer.line_content(line as usize).clone();
    let above = buffer.line_content((line - 1) as usize).clone();
    replace_lines(buffer, line - 1, line, &[current, above]);
}

/// Moves a line down by one position.
pub fn move_line_down(buffer: &mut Buffer, line: u32) {
    let last = buffer.len_lines().saturating_sub(1) as u32;
    if line >= last {
        return;
    }
    let current = buffer.line_content(line as usize).clone();
    let below = buffer.line_content((line + 1) as usize).clone();
    replace_lines(buffer, line, line + 1, &[below, current]);
}

/// Deletes the given line.
pub fn delete_line(buffer: &mut Buffer, line: u32) {
    let line_count = buffer.len_lines() as u32;
    if line >= line_count {
        return;
    }
    if line_count == 1 {
        let len = buffer.line_content_len(0);
        buffer.remove(0..len);
        return;
    }
    let start = buffer.line_to_char(line as usize);
    let end = if (line + 1) < line_count {
        buffer.line_to_char((line + 1) as usize)
    } else {
        // Last line: also remove the preceding newline
        let prev_end = buffer.line_to_char(line as usize);
        let line_end = prev_end + buffer.line_content_len(line as usize);
        // Remove from end of previous line (the newline char) to end of this line
        buffer.remove((prev_end.saturating_sub(1))..line_end);
        return;
    };
    buffer.remove(start..end);
}

/// Transforms text in the range to upper case.
pub fn to_upper_case(buffer: &mut Buffer, range: Range) {
    let start = buffer.position_to_offset(range.start);
    let end = buffer.position_to_offset(range.end);
    let text = buffer.slice(start..end);
    let upper = text.to_uppercase();
    buffer.replace(start..end, &upper);
}

/// Transforms text in the range to lower case.
pub fn to_lower_case(buffer: &mut Buffer, range: Range) {
    let start = buffer.position_to_offset(range.start);
    let end = buffer.position_to_offset(range.end);
    let text = buffer.slice(start..end);
    let lower = text.to_lowercase();
    buffer.replace(start..end, &lower);
}

// ── Internal helpers ────────────────────────────────────────────────────

fn clamp_lines(buffer: &Buffer, start: u32, end: u32) -> (u32, u32) {
    let last = buffer.len_lines().saturating_sub(1) as u32;
    (start.min(last), end.min(last))
}

fn replace_lines(buffer: &mut Buffer, start_line: u32, end_line: u32, new_lines: &[String]) {
    let char_start = buffer.line_to_char(start_line as usize);
    let char_end = buffer.line_to_char(end_line as usize) + buffer.line_content_len(end_line as usize);
    let replacement = new_lines.join("\n");
    buffer.replace(char_start..char_end, &replacement);
}

#[cfg(test)]
mod tests {
    use super::*;
    use sidex_text::Position;

    fn buf(text: &str) -> Buffer {
        Buffer::from_str(text)
    }

    #[test]
    fn sort_ascending() {
        let mut buffer = buf("cherry\napple\nbanana");
        sort_lines(&mut buffer, 0, 2, false);
        assert_eq!(buffer.text(), "apple\nbanana\ncherry");
    }

    #[test]
    fn sort_descending() {
        let mut buffer = buf("apple\nbanana\ncherry");
        sort_lines(&mut buffer, 0, 2, true);
        assert_eq!(buffer.text(), "cherry\nbanana\napple");
    }

    #[test]
    fn reverse() {
        let mut buffer = buf("a\nb\nc");
        reverse_lines(&mut buffer, 0, 2);
        assert_eq!(buffer.text(), "c\nb\na");
    }

    #[test]
    fn unique() {
        let mut buffer = buf("a\na\nb\nb\nc");
        unique_lines(&mut buffer, 0, 4);
        assert_eq!(buffer.text(), "a\nb\nc");
    }

    #[test]
    fn join() {
        let mut buffer = buf("hello\n  world\n  !");
        join_lines(&mut buffer, 0, 2);
        assert_eq!(buffer.text(), "hello world !");
    }

    #[test]
    fn duplicate() {
        let mut buffer = buf("foo\nbar");
        duplicate_line(&mut buffer, 0);
        assert_eq!(buffer.len_lines(), 3);
        assert_eq!(buffer.line_content(0), "foo");
        assert_eq!(buffer.line_content(1), "foo");
    }

    #[test]
    fn move_up() {
        let mut buffer = buf("a\nb\nc");
        move_line_up(&mut buffer, 1);
        assert_eq!(buffer.text(), "b\na\nc");
    }

    #[test]
    fn move_down() {
        let mut buffer = buf("a\nb\nc");
        move_line_down(&mut buffer, 0);
        assert_eq!(buffer.text(), "b\na\nc");
    }

    #[test]
    fn case_transform() {
        let mut buffer = buf("Hello World");
        let range = Range::new(Position::new(0, 0), Position::new(0, 11));
        to_upper_case(&mut buffer, range);
        assert_eq!(buffer.text(), "HELLO WORLD");
    }
}
