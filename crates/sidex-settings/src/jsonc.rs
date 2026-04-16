//! JSON with Comments (JSONC) parser — strips `//`, `/* */` comments and
//! trailing commas before delegating to `serde_json`.

use anyhow::{Context, Result};
use serde_json::Value;

/// Parse a JSONC string into a `serde_json::Value`.
///
/// Removes single-line (`//`) and multi-line (`/* */`) comments as well as
/// trailing commas before closing `]` or `}`.
pub fn parse_jsonc(input: &str) -> Result<Value> {
    let stripped = strip_comments(input);
    let cleaned = remove_trailing_commas(&stripped);
    serde_json::from_str(&cleaned).context("failed to parse JSONC after stripping comments")
}

/// Strip `//` and `/* */` comments from a JSON-like string, respecting
/// quoted strings.
fn strip_comments(input: &str) -> String {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len);
    let mut i = 0;

    while i < len {
        match bytes[i] {
            b'"' => {
                out.push('"');
                i += 1;
                while i < len {
                    if bytes[i] == b'\\' && i + 1 < len {
                        out.push(bytes[i] as char);
                        out.push(bytes[i + 1] as char);
                        i += 2;
                    } else if bytes[i] == b'"' {
                        out.push('"');
                        i += 1;
                        break;
                    } else {
                        out.push(bytes[i] as char);
                        i += 1;
                    }
                }
            }
            b'/' if i + 1 < len && bytes[i + 1] == b'/' => {
                i += 2;
                while i < len && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if i + 1 < len && bytes[i + 1] == b'*' => {
                i += 2;
                while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                if i + 1 < len {
                    i += 2;
                }
            }
            _ => {
                out.push(bytes[i] as char);
                i += 1;
            }
        }
    }

    out
}

/// Remove trailing commas before `]` or `}`.
fn remove_trailing_commas(input: &str) -> String {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len);
    let mut i = 0;

    while i < len {
        if bytes[i] == b'"' {
            out.push('"');
            i += 1;
            while i < len {
                if bytes[i] == b'\\' && i + 1 < len {
                    out.push(bytes[i] as char);
                    out.push(bytes[i + 1] as char);
                    i += 2;
                } else if bytes[i] == b'"' {
                    out.push('"');
                    i += 1;
                    break;
                } else {
                    out.push(bytes[i] as char);
                    i += 1;
                }
            }
        } else if bytes[i] == b',' {
            let mut j = i + 1;
            while j < len && (bytes[j] == b' ' || bytes[j] == b'\t' || bytes[j] == b'\n' || bytes[j] == b'\r') {
                j += 1;
            }
            if j < len && (bytes[j] == b']' || bytes[j] == b'}') {
                i += 1;
            } else {
                out.push(',');
                i += 1;
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_line_comments() {
        let input = r#"{
            // this is a comment
            "key": "value"
        }"#;
        let v = parse_jsonc(input).unwrap();
        assert_eq!(v["key"], "value");
    }

    #[test]
    fn strip_block_comments() {
        let input = r#"{
            /* multi
               line */
            "key": "value"
        }"#;
        let v = parse_jsonc(input).unwrap();
        assert_eq!(v["key"], "value");
    }

    #[test]
    fn trailing_comma_object() {
        let input = r#"{ "a": 1, "b": 2, }"#;
        let v = parse_jsonc(input).unwrap();
        assert_eq!(v["a"], 1);
        assert_eq!(v["b"], 2);
    }

    #[test]
    fn trailing_comma_array() {
        let input = r#"[1, 2, 3, ]"#;
        let v = parse_jsonc(input).unwrap();
        assert_eq!(v.as_array().unwrap().len(), 3);
    }

    #[test]
    fn slash_in_string_preserved() {
        let input = r#"{ "url": "https://example.com" }"#;
        let v = parse_jsonc(input).unwrap();
        assert_eq!(v["url"], "https://example.com");
    }

    #[test]
    fn mixed_comments_and_trailing_commas() {
        let input = r#"{
            // editor settings
            "editor.fontSize": 14, // default size
            "editor.tabSize": 4,
            /* workspace settings */
        }"#;
        let v = parse_jsonc(input).unwrap();
        assert_eq!(v["editor.fontSize"], 14);
        assert_eq!(v["editor.tabSize"], 4);
    }

    #[test]
    fn empty_object() {
        let v = parse_jsonc("{}").unwrap();
        assert!(v.as_object().unwrap().is_empty());
    }

    #[test]
    fn escaped_quote_in_string() {
        let input = r#"{ "key": "val\"ue" }"#;
        let v = parse_jsonc(input).unwrap();
        assert_eq!(v["key"], "val\"ue");
    }
}
