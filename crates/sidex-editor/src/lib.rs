//! Editor core — cursor, selection, undo/redo, and edit operations for `SideX`.
//!
//! This crate provides the editing logic layer built on top of [`sidex_text`].
//! It manages cursors, selections, multi-cursor editing, undo/redo history,
//! and document-level operations like line movement, commenting, and indentation.
//! Also includes a snippet engine, completion types with fuzzy matching,
//! viewport management, and text decoration support.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::similar_names
)]

pub mod completion;
pub mod contrib;
pub mod cursor;
pub mod decoration;
pub mod document;
pub mod multi_cursor;
pub mod selection;
pub mod snippet;
pub mod undo;
pub mod viewport;
pub mod word;

pub use completion::{
    fuzzy_filter, fuzzy_score, CompletionItem, CompletionItemKind, CompletionList,
    CompletionTrigger, CompletionTriggerKind,
};
pub use cursor::CursorState;
pub use decoration::{Color, Decoration, DecorationCollection, DecorationOptions, DecorationSetId};
pub use document::{
    AutoClosingEditStrategy, AutoClosingStrategy, AutoIndentStrategy, AutoSurroundStrategy,
    CompositionOutcome, Document, EditOperationType, EditorConfig,
};
pub use multi_cursor::MultiCursor;
pub use selection::Selection;
pub use snippet::{parse_snippet, Snippet, SnippetPart, SnippetSession};
pub use undo::{EditGroup, UndoRedoStack};
pub use viewport::{lines_per_page, Viewport};
pub use word::{find_word_end, find_word_start, word_at};
