//! Character encoding detection, decoding, and encoding.
//!
//! Mirrors the encoding support in Monaco / VS Code, which auto-detects BOM
//! markers and falls back to heuristics for common encodings.

use serde::{Deserialize, Serialize};

/// Supported character encodings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Encoding {
    /// UTF-8 (default).
    Utf8,
    /// UTF-8 with BOM.
    Utf8Bom,
    /// UTF-16 Little Endian.
    Utf16Le,
    /// UTF-16 Big Endian.
    Utf16Be,
    /// ISO 8859-1 / Latin-1.
    Latin1,
    /// Shift JIS (Japanese).
    ShiftJis,
    /// GBK / GB2312 (Chinese).
    Gbk,
    /// ASCII (7-bit subset of UTF-8).
    Ascii,
}

impl Encoding {
    /// Human-readable label for display in status bars / pickers.
    pub fn label(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Utf8Bom => "UTF-8 with BOM",
            Self::Utf16Le => "UTF-16 LE",
            Self::Utf16Be => "UTF-16 BE",
            Self::Latin1 => "ISO 8859-1",
            Self::ShiftJis => "Shift JIS",
            Self::Gbk => "GBK",
            Self::Ascii => "ASCII",
        }
    }
}

impl std::fmt::Display for Encoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

// ── BOM constants ────────────────────────────────────────────────────

const UTF8_BOM: &[u8] = &[0xEF, 0xBB, 0xBF];
const UTF16_LE_BOM: &[u8] = &[0xFF, 0xFE];
const UTF16_BE_BOM: &[u8] = &[0xFE, 0xFF];

// ── Detection ────────────────────────────────────────────────────────

/// Detect the encoding of `bytes` using BOM detection and heuristics.
///
/// Checks for a Byte Order Mark first, then applies statistical
/// heuristics for common encodings. Falls back to `Utf8` when unsure.
pub fn detect_encoding(bytes: &[u8]) -> Encoding {
    if bytes.starts_with(UTF8_BOM) {
        return Encoding::Utf8Bom;
    }
    if bytes.starts_with(UTF16_BE_BOM) {
        return Encoding::Utf16Be;
    }
    if bytes.starts_with(UTF16_LE_BOM) {
        return Encoding::Utf16Le;
    }

    // Check for null bytes which suggest UTF-16 without BOM
    if bytes.len() >= 2 {
        let null_even = bytes.iter().step_by(2).filter(|&&b| b == 0).count();
        let null_odd = bytes.iter().skip(1).step_by(2).filter(|&&b| b == 0).count();
        let total_pairs = bytes.len() / 2;
        if total_pairs > 0 {
            // Many nulls in even positions → likely UTF-16 BE
            if null_even > total_pairs / 3 && null_odd == 0 {
                return Encoding::Utf16Be;
            }
            // Many nulls in odd positions → likely UTF-16 LE
            if null_odd > total_pairs / 3 && null_even == 0 {
                return Encoding::Utf16Le;
            }
        }
    }

    // Pure ASCII check
    if bytes.iter().all(|&b| b < 0x80) {
        return Encoding::Ascii;
    }

    // Valid UTF-8 check
    if std::str::from_utf8(bytes).is_ok() {
        return Encoding::Utf8;
    }

    // Heuristic: Shift JIS detection
    if looks_like_shift_jis(bytes) {
        return Encoding::ShiftJis;
    }

    // Heuristic: GBK detection
    if looks_like_gbk(bytes) {
        return Encoding::Gbk;
    }

    // Fallback: Latin-1 accepts all single-byte values.
    Encoding::Latin1
}

fn looks_like_shift_jis(bytes: &[u8]) -> bool {
    let mut i = 0;
    let mut multi_count = 0;
    let mut invalid = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b < 0x80 {
            i += 1;
        } else if (0x81..=0x9F).contains(&b) || (0xE0..=0xEF).contains(&b) {
            if i + 1 >= bytes.len() {
                invalid += 1;
                break;
            }
            let b2 = bytes[i + 1];
            if (0x40..=0x7E).contains(&b2) || (0x80..=0xFC).contains(&b2) {
                multi_count += 1;
                i += 2;
            } else {
                invalid += 1;
                i += 1;
            }
        } else if (0xA1..=0xDF).contains(&b) {
            // Half-width katakana
            multi_count += 1;
            i += 1;
        } else {
            invalid += 1;
            i += 1;
        }
    }
    multi_count > 0 && invalid <= multi_count / 4
}

fn looks_like_gbk(bytes: &[u8]) -> bool {
    let mut i = 0;
    let mut multi_count = 0;
    let mut invalid = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b < 0x80 {
            i += 1;
        } else if (0x81..=0xFE).contains(&b) {
            if i + 1 >= bytes.len() {
                invalid += 1;
                break;
            }
            let b2 = bytes[i + 1];
            if (0x40..=0xFE).contains(&b2) && b2 != 0x7F {
                multi_count += 1;
                i += 2;
            } else {
                invalid += 1;
                i += 1;
            }
        } else {
            invalid += 1;
            i += 1;
        }
    }
    multi_count > 0 && invalid <= multi_count / 4
}

// ── Decoding ─────────────────────────────────────────────────────────

/// Decode `bytes` using the specified `encoding` into a Rust `String`.
///
/// # Errors
///
/// Returns an error if the bytes are not valid for the given encoding.
pub fn decode(bytes: &[u8], encoding: Encoding) -> Result<String, EncodingError> {
    match encoding {
        Encoding::Utf8 => {
            String::from_utf8(bytes.to_vec()).map_err(|_| EncodingError::InvalidData(encoding))
        }
        Encoding::Utf8Bom => {
            let data = if bytes.starts_with(UTF8_BOM) {
                &bytes[3..]
            } else {
                bytes
            };
            String::from_utf8(data.to_vec()).map_err(|_| EncodingError::InvalidData(encoding))
        }
        Encoding::Utf16Le => decode_utf16(bytes, true),
        Encoding::Utf16Be => decode_utf16(bytes, false),
        Encoding::Latin1 => Ok(bytes.iter().map(|&b| b as char).collect()),
        Encoding::Ascii => {
            if bytes.iter().all(|&b| b < 0x80) {
                Ok(bytes.iter().map(|&b| b as char).collect())
            } else {
                Err(EncodingError::InvalidData(encoding))
            }
        }
        Encoding::ShiftJis | Encoding::Gbk => Err(EncodingError::UnsupportedEncoding(encoding)),
    }
}

fn decode_utf16(bytes: &[u8], little_endian: bool) -> Result<String, EncodingError> {
    let encoding = if little_endian {
        Encoding::Utf16Le
    } else {
        Encoding::Utf16Be
    };

    // Strip BOM if present
    let bom = if little_endian {
        UTF16_LE_BOM
    } else {
        UTF16_BE_BOM
    };
    let data = if bytes.starts_with(bom) {
        &bytes[2..]
    } else {
        bytes
    };

    if data.len() % 2 != 0 {
        return Err(EncodingError::InvalidData(encoding));
    }

    let code_units: Vec<u16> = data
        .chunks_exact(2)
        .map(|pair| {
            if little_endian {
                u16::from_le_bytes([pair[0], pair[1]])
            } else {
                u16::from_be_bytes([pair[0], pair[1]])
            }
        })
        .collect();

    String::from_utf16(&code_units).map_err(|_| EncodingError::InvalidData(encoding))
}

// ── Encoding ─────────────────────────────────────────────────────────

/// Encode a `&str` into bytes using the specified `encoding`.
///
/// # Errors
///
/// Returns an error for unsupported encodings (Shift JIS, GBK) or if
/// the text contains characters outside the encoding's range.
pub fn encode(text: &str, encoding: Encoding) -> Result<Vec<u8>, EncodingError> {
    match encoding {
        Encoding::Utf8 => Ok(text.as_bytes().to_vec()),
        Encoding::Utf8Bom => {
            let mut out = Vec::with_capacity(3 + text.len());
            out.extend_from_slice(UTF8_BOM);
            out.extend_from_slice(text.as_bytes());
            Ok(out)
        }
        Encoding::Utf16Le => Ok(encode_utf16(text, true)),
        Encoding::Utf16Be => Ok(encode_utf16(text, false)),
        Encoding::Latin1 => {
            let mut out = Vec::with_capacity(text.len());
            for c in text.chars() {
                let cp = c as u32;
                if cp > 0xFF {
                    return Err(EncodingError::InvalidData(encoding));
                }
                #[allow(clippy::cast_possible_truncation)]
                out.push(cp as u8);
            }
            Ok(out)
        }
        Encoding::Ascii => {
            let mut out = Vec::with_capacity(text.len());
            for c in text.chars() {
                if !c.is_ascii() {
                    return Err(EncodingError::InvalidData(encoding));
                }
                out.push(c as u8);
            }
            Ok(out)
        }
        Encoding::ShiftJis | Encoding::Gbk => Err(EncodingError::UnsupportedEncoding(encoding)),
    }
}

fn encode_utf16(text: &str, little_endian: bool) -> Vec<u8> {
    let mut out = Vec::new();
    if little_endian {
        out.extend_from_slice(UTF16_LE_BOM);
    } else {
        out.extend_from_slice(UTF16_BE_BOM);
    }

    for code_unit in text.encode_utf16() {
        let bytes = if little_endian {
            code_unit.to_le_bytes()
        } else {
            code_unit.to_be_bytes()
        };
        out.extend_from_slice(&bytes);
    }
    out
}

// ── Error type ───────────────────────────────────────────────────────

/// Errors that can occur during encoding/decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncodingError {
    /// The data is not valid for the given encoding.
    InvalidData(Encoding),
    /// The encoding is recognized but full encode/decode is not implemented.
    UnsupportedEncoding(Encoding),
}

impl std::fmt::Display for EncodingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidData(enc) => write!(f, "invalid data for encoding {enc}"),
            Self::UnsupportedEncoding(enc) => write!(f, "unsupported encoding: {enc}"),
        }
    }
}

impl std::error::Error for EncodingError {}

#[cfg(test)]
mod tests {
    use super::*;

    // ── detect_encoding ──────────────────────────────────────────────

    #[test]
    fn detect_utf8_bom() {
        let data = [0xEF, 0xBB, 0xBF, b'h', b'i'];
        assert_eq!(detect_encoding(&data), Encoding::Utf8Bom);
    }

    #[test]
    fn detect_utf16_le_bom() {
        let data = [0xFF, 0xFE, b'h', 0x00];
        assert_eq!(detect_encoding(&data), Encoding::Utf16Le);
    }

    #[test]
    fn detect_utf16_be_bom() {
        let data = [0xFE, 0xFF, 0x00, b'h'];
        assert_eq!(detect_encoding(&data), Encoding::Utf16Be);
    }

    #[test]
    fn detect_pure_ascii() {
        let data = b"hello world\n";
        assert_eq!(detect_encoding(data), Encoding::Ascii);
    }

    #[test]
    fn detect_utf8_multibyte() {
        let data = "héllo wörld".as_bytes();
        assert_eq!(detect_encoding(data), Encoding::Utf8);
    }

    #[test]
    fn detect_empty() {
        assert_eq!(detect_encoding(&[]), Encoding::Ascii);
    }

    #[test]
    fn detect_latin1_fallback() {
        // Bytes 0x80-0xFF that are NOT valid UTF-8 sequences
        let data: Vec<u8> = vec![0x80, 0x81, 0x82, 0x83, 0x84];
        let enc = detect_encoding(&data);
        // Should be either Latin1, ShiftJis, or Gbk depending on heuristics
        assert!(
            enc == Encoding::Latin1 || enc == Encoding::ShiftJis || enc == Encoding::Gbk,
            "unexpected encoding: {enc:?}"
        );
    }

    // ── decode ───────────────────────────────────────────────────────

    #[test]
    fn decode_utf8() {
        let data = "hello".as_bytes();
        assert_eq!(decode(data, Encoding::Utf8).unwrap(), "hello");
    }

    #[test]
    fn decode_utf8_bom() {
        let mut data = vec![0xEF, 0xBB, 0xBF];
        data.extend_from_slice(b"hello");
        assert_eq!(decode(&data, Encoding::Utf8Bom).unwrap(), "hello");
    }

    #[test]
    fn decode_utf8_bom_without_bom() {
        assert_eq!(decode(b"hello", Encoding::Utf8Bom).unwrap(), "hello");
    }

    #[test]
    fn decode_latin1() {
        let data = vec![0xE9]; // é in Latin-1
        assert_eq!(decode(&data, Encoding::Latin1).unwrap(), "é");
    }

    #[test]
    fn decode_ascii() {
        assert_eq!(decode(b"hello", Encoding::Ascii).unwrap(), "hello");
    }

    #[test]
    fn decode_ascii_rejects_high_bytes() {
        assert!(decode(&[0x80], Encoding::Ascii).is_err());
    }

    #[test]
    fn decode_utf16_le() {
        let mut data = vec![0xFF, 0xFE]; // BOM
        for unit in "hi".encode_utf16() {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        assert_eq!(decode(&data, Encoding::Utf16Le).unwrap(), "hi");
    }

    #[test]
    fn decode_utf16_be() {
        let mut data = vec![0xFE, 0xFF]; // BOM
        for unit in "hi".encode_utf16() {
            data.extend_from_slice(&unit.to_be_bytes());
        }
        assert_eq!(decode(&data, Encoding::Utf16Be).unwrap(), "hi");
    }

    #[test]
    fn decode_utf16_odd_bytes() {
        assert!(decode(&[0xFF, 0xFE, 0x00], Encoding::Utf16Le).is_err());
    }

    // ── encode ───────────────────────────────────────────────────────

    #[test]
    fn encode_utf8() {
        assert_eq!(encode("hello", Encoding::Utf8).unwrap(), b"hello");
    }

    #[test]
    fn encode_utf8_bom() {
        let result = encode("hi", Encoding::Utf8Bom).unwrap();
        assert!(result.starts_with(&[0xEF, 0xBB, 0xBF]));
        assert_eq!(&result[3..], b"hi");
    }

    #[test]
    fn encode_latin1() {
        assert_eq!(encode("é", Encoding::Latin1).unwrap(), vec![0xE9]);
    }

    #[test]
    fn encode_latin1_rejects_out_of_range() {
        assert!(encode("你", Encoding::Latin1).is_err());
    }

    #[test]
    fn encode_ascii() {
        assert_eq!(encode("hi", Encoding::Ascii).unwrap(), b"hi");
    }

    #[test]
    fn encode_ascii_rejects_non_ascii() {
        assert!(encode("é", Encoding::Ascii).is_err());
    }

    #[test]
    fn encode_utf16_le_roundtrip() {
        let encoded = encode("hello", Encoding::Utf16Le).unwrap();
        let decoded = decode(&encoded, Encoding::Utf16Le).unwrap();
        assert_eq!(decoded, "hello");
    }

    #[test]
    fn encode_utf16_be_roundtrip() {
        let encoded = encode("hello", Encoding::Utf16Be).unwrap();
        let decoded = decode(&encoded, Encoding::Utf16Be).unwrap();
        assert_eq!(decoded, "hello");
    }

    #[test]
    fn encode_utf16_emoji_roundtrip() {
        let text = "hello 😀 world";
        let encoded = encode(text, Encoding::Utf16Le).unwrap();
        let decoded = decode(&encoded, Encoding::Utf16Le).unwrap();
        assert_eq!(decoded, text);
    }

    // ── Encoding label ───────────────────────────────────────────────

    #[test]
    fn encoding_labels() {
        assert_eq!(Encoding::Utf8.label(), "UTF-8");
        assert_eq!(Encoding::Utf16Le.label(), "UTF-16 LE");
        assert_eq!(Encoding::Latin1.label(), "ISO 8859-1");
    }

    #[test]
    fn encoding_display() {
        assert_eq!(format!("{}", Encoding::Utf8), "UTF-8");
    }

    // ── Error display ────────────────────────────────────────────────

    #[test]
    fn error_display() {
        let err = EncodingError::InvalidData(Encoding::Utf8);
        assert!(format!("{err}").contains("invalid data"));
    }

    #[test]
    fn error_unsupported() {
        let err = EncodingError::UnsupportedEncoding(Encoding::ShiftJis);
        assert!(format!("{err}").contains("unsupported"));
    }
}
