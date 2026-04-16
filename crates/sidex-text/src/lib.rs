//! # sidex-text
//!
//! Rope-based text buffer for the `SideX` editor.
//!
//! This crate provides the foundational text storage layer, replacing Monaco's
//! piece table with a Rust [`ropey::Rope`]-backed buffer. It supports efficient
//! editing of large files, position/offset conversions, UTF-16 interop for LSP,
//! and line-ending detection/normalization.

mod buffer;
pub mod diff;
mod edit;
pub mod encoding;
mod line_ending;
mod position;
mod range;
pub mod search;
mod utf16;

pub use buffer::{Buffer, BufferSnapshot, IndentInfo, WordAtPosition, WordInfo, WordType};
pub use edit::{ChangeEvent, EditOperation};
pub use line_ending::{detect_line_ending, normalize_line_endings, LineEnding};
pub use position::Position;
pub use range::Range;
pub use search::{FindMatch, FindMatchesOptions, LIMIT_FIND_COUNT};
pub use utf16::{
    char_col_to_utf16_col, lsp_position_to_position, position_to_lsp_position,
    utf16_col_to_char_col, Utf16Position,
};
