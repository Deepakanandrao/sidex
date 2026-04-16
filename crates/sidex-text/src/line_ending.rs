use serde::{Deserialize, Serialize};

/// Represents the type of line ending used in a text document.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LineEnding {
    /// Unix-style line feed (`\n`).
    #[default]
    Lf,
    /// Windows-style carriage return + line feed (`\r\n`).
    CrLf,
    /// Classic Mac-style carriage return (`\r`).
    Cr,
}

impl LineEnding {
    /// Returns the string representation of this line ending.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::CrLf => "\r\n",
            Self::Cr => "\r",
        }
    }
}

impl std::fmt::Display for LineEnding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lf => write!(f, "LF"),
            Self::CrLf => write!(f, "CRLF"),
            Self::Cr => write!(f, "CR"),
        }
    }
}

/// Detects the predominant line ending in a string.
///
/// Scans the first 10,000 characters and counts occurrences of each
/// line ending type. Returns the most common one, defaulting to `Lf`.
#[must_use]
pub fn detect_line_ending(text: &str) -> LineEnding {
    let sample = if text.len() > 10_000 {
        &text[..10_000]
    } else {
        text
    };

    let mut lf_count = 0u32;
    let mut crlf_count = 0u32;
    let mut cr_count = 0u32;

    let bytes = sample.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                crlf_count += 1;
                i += 2;
            } else {
                cr_count += 1;
                i += 1;
            }
        } else if bytes[i] == b'\n' {
            lf_count += 1;
            i += 1;
        } else {
            i += 1;
        }
    }

    if crlf_count >= lf_count && crlf_count >= cr_count && crlf_count > 0 {
        LineEnding::CrLf
    } else if cr_count >= lf_count && cr_count > 0 {
        LineEnding::Cr
    } else {
        LineEnding::Lf
    }
}

/// Normalizes all line endings in the given text to the target type.
#[must_use]
pub fn normalize_line_endings(text: &str, target: LineEnding) -> String {
    let target_str = target.as_str();
    let mut result = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'\r' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                i += 2;
            } else {
                i += 1;
            }
            result.push_str(target_str);
        } else if bytes[i] == b'\n' {
            result.push_str(target_str);
            i += 1;
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_lf() {
        assert_eq!(detect_line_ending("hello\nworld\n"), LineEnding::Lf);
    }

    #[test]
    fn detect_crlf() {
        assert_eq!(detect_line_ending("hello\r\nworld\r\n"), LineEnding::CrLf);
    }

    #[test]
    fn detect_cr() {
        assert_eq!(detect_line_ending("hello\rworld\r"), LineEnding::Cr);
    }

    #[test]
    fn detect_empty_defaults_to_lf() {
        assert_eq!(detect_line_ending(""), LineEnding::Lf);
    }

    #[test]
    fn normalize_crlf_to_lf() {
        let result = normalize_line_endings("hello\r\nworld\r\n", LineEnding::Lf);
        assert_eq!(result, "hello\nworld\n");
    }

    #[test]
    fn normalize_lf_to_crlf() {
        let result = normalize_line_endings("hello\nworld\n", LineEnding::CrLf);
        assert_eq!(result, "hello\r\nworld\r\n");
    }

    #[test]
    fn normalize_mixed() {
        let result = normalize_line_endings("a\nb\r\nc\rd\n", LineEnding::Lf);
        assert_eq!(result, "a\nb\nc\nd\n");
    }

    #[test]
    fn line_ending_as_str() {
        assert_eq!(LineEnding::Lf.as_str(), "\n");
        assert_eq!(LineEnding::CrLf.as_str(), "\r\n");
        assert_eq!(LineEnding::Cr.as_str(), "\r");
    }
}
