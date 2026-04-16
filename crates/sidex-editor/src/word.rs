use sidex_text::{Buffer, Position};

/// Character classification for word boundary detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CharClass {
    /// Whitespace characters (space, tab, etc.).
    Whitespace,
    /// Punctuation / symbol characters.
    Punctuation,
    /// Word characters: letters, digits, underscore.
    Word,
    /// Uppercase letter (used for camelCase boundary detection).
    Upper,
}

fn classify(ch: char) -> CharClass {
    if ch.is_whitespace() {
        CharClass::Whitespace
    } else if ch.is_alphanumeric() || ch == '_' {
        if ch.is_uppercase() {
            CharClass::Upper
        } else {
            CharClass::Word
        }
    } else {
        CharClass::Punctuation
    }
}

/// Broad classification: word-like (Word/Upper) vs whitespace vs punctuation.
#[derive(PartialEq, Eq)]
enum BroadClass {
    WordLike,
    Whitespace,
    Punctuation,
}

fn broad_class(c: CharClass) -> BroadClass {
    match c {
        CharClass::Word | CharClass::Upper => BroadClass::WordLike,
        CharClass::Whitespace => BroadClass::Whitespace,
        CharClass::Punctuation => BroadClass::Punctuation,
    }
}

/// Returns `true` if there is a word boundary between `prev` and `curr`
/// character classes, including camelCase transitions.
fn is_boundary(prev: CharClass, curr: CharClass) -> bool {
    if prev == curr {
        return false;
    }
    // Same broad word-like class doesn't break (Word and Upper are both "word-like")
    // but camelCase boundary: lower->Upper is a break
    if prev == CharClass::Word && curr == CharClass::Upper {
        return true;
    }
    // Upper->Word is NOT a break (e.g., "XMLParser" — the "P" starts a new word
    // but we only break before the last uppercase in a run). We handle this in
    // the traversal functions rather than here.
    if prev == CharClass::Upper && curr == CharClass::Word {
        return false;
    }
    if prev == CharClass::Upper && curr == CharClass::Upper {
        return false;
    }
    if prev == CharClass::Word && curr == CharClass::Word {
        return false;
    }
    true
}

/// Finds the start of the word at or before the given position.
///
/// Handles camelCase boundaries: `myFunctionName` has word starts at `m`, `F`, `N`.
#[must_use]
pub fn find_word_start(buffer: &Buffer, pos: Position) -> Position {
    let line = pos.line as usize;
    let col = pos.column as usize;
    let content = buffer.line_content(line);
    let chars: Vec<char> = content.chars().collect();

    if col == 0 {
        if line == 0 {
            return Position::new(0, 0);
        }
        let prev_line = line - 1;
        let prev_len = buffer.line_content_len(prev_line);
        return Position::new(prev_line as u32, prev_len as u32);
    }

    let mut i = col.min(chars.len());
    if i == 0 {
        return pos;
    }
    i -= 1;

    // Skip whitespace first
    while i > 0 && chars[i].is_whitespace() {
        i -= 1;
    }

    if i == 0 {
        return Position::new(line as u32, 0);
    }

    let target_class = classify(chars[i]);
    while i > 0 {
        let prev_class = classify(chars[i - 1]);
        if is_boundary(prev_class, target_class) {
            break;
        }
        // For runs of uppercase followed by a word char, break before the last
        // uppercase: "XMLParser" → break before "P", so back up to where
        // uppercase run starts before a word char.
        if target_class == CharClass::Upper && prev_class == CharClass::Upper && i + 1 < chars.len()
        {
            let next_class = classify(chars[i + 1]);
            if next_class == CharClass::Word {
                break;
            }
        }
        i -= 1;
    }

    Position::new(line as u32, i as u32)
}

/// Finds the end of the word at or after the given position.
///
/// Handles camelCase boundaries.
#[must_use]
pub fn find_word_end(buffer: &Buffer, pos: Position) -> Position {
    let line = pos.line as usize;
    let col = pos.column as usize;
    let content = buffer.line_content(line);
    let chars: Vec<char> = content.chars().collect();
    let len = chars.len();

    if col >= len {
        let total_lines = buffer.len_lines();
        if line + 1 >= total_lines {
            return Position::new(line as u32, len as u32);
        }
        return Position::new((line + 1) as u32, 0);
    }

    let mut i = col;

    // Skip whitespace first
    while i < len && chars[i].is_whitespace() {
        i += 1;
    }

    if i >= len {
        return Position::new(line as u32, len as u32);
    }

    let start_class = classify(chars[i]);
    i += 1;

    while i < len {
        let curr_class = classify(chars[i]);
        let broad_start = broad_class(start_class);
        let broad_curr = broad_class(curr_class);

        // Different broad class — always stop
        if broad_start != broad_curr {
            break;
        }

        // Within word-like characters, detect camelCase boundaries
        if start_class == CharClass::Word && curr_class == CharClass::Upper {
            break;
        }
        if start_class == CharClass::Upper
            && curr_class == CharClass::Upper
            && i + 1 < len
            && classify(chars[i + 1]) == CharClass::Word
        {
            break;
        }

        i += 1;
    }

    Position::new(line as u32, i as u32)
}

/// Returns the range of the word at the given position.
#[must_use]
pub fn word_at(buffer: &Buffer, pos: Position) -> sidex_text::Range {
    let start = find_word_start(buffer, pos);
    let end = find_word_end(buffer, pos);
    sidex_text::Range::new(start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buf(s: &str) -> Buffer {
        Buffer::from_str(s)
    }

    #[test]
    fn word_start_simple() {
        let b = buf("hello world");
        assert_eq!(
            find_word_start(&b, Position::new(0, 8)),
            Position::new(0, 6)
        );
        assert_eq!(
            find_word_start(&b, Position::new(0, 5)),
            Position::new(0, 0)
        );
    }

    #[test]
    fn word_end_simple() {
        let b = buf("hello world");
        assert_eq!(find_word_end(&b, Position::new(0, 0)), Position::new(0, 5));
        assert_eq!(find_word_end(&b, Position::new(0, 6)), Position::new(0, 11));
    }

    #[test]
    fn word_at_range() {
        let b = buf("hello world");
        let r = word_at(&b, Position::new(0, 7));
        assert_eq!(r.start, Position::new(0, 6));
        assert_eq!(r.end, Position::new(0, 11));
    }

    #[test]
    fn camel_case_boundary() {
        let b = buf("myFunctionName");
        assert_eq!(find_word_end(&b, Position::new(0, 0)), Position::new(0, 2));
        assert_eq!(find_word_end(&b, Position::new(0, 2)), Position::new(0, 10));
    }

    #[test]
    fn punctuation_boundary() {
        let b = buf("foo.bar");
        assert_eq!(find_word_end(&b, Position::new(0, 0)), Position::new(0, 3));
        assert_eq!(find_word_end(&b, Position::new(0, 3)), Position::new(0, 4));
        assert_eq!(find_word_end(&b, Position::new(0, 4)), Position::new(0, 7));
    }

    #[test]
    fn word_start_at_line_beginning() {
        let b = buf("hello\nworld");
        assert_eq!(
            find_word_start(&b, Position::new(1, 0)),
            Position::new(0, 5)
        );
    }

    #[test]
    fn word_end_at_line_end() {
        let b = buf("hello\nworld");
        assert_eq!(find_word_end(&b, Position::new(0, 5)), Position::new(1, 0));
    }

    #[test]
    fn whitespace_skip() {
        let b = buf("hello   world");
        assert_eq!(
            find_word_start(&b, Position::new(0, 8)),
            Position::new(0, 0)
        );
    }

    #[test]
    fn start_of_buffer() {
        let b = buf("hello");
        assert_eq!(
            find_word_start(&b, Position::new(0, 0)),
            Position::new(0, 0)
        );
    }

    #[test]
    fn end_of_buffer() {
        let b = buf("hello");
        assert_eq!(find_word_end(&b, Position::new(0, 5)), Position::new(0, 5));
    }

    #[test]
    fn empty_buffer() {
        let b = buf("");
        assert_eq!(
            find_word_start(&b, Position::new(0, 0)),
            Position::new(0, 0)
        );
        assert_eq!(find_word_end(&b, Position::new(0, 0)), Position::new(0, 0));
    }

    #[test]
    fn underscore_is_word_char() {
        let b = buf("my_var_name");
        assert_eq!(find_word_end(&b, Position::new(0, 0)), Position::new(0, 11));
    }
}
