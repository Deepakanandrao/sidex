//! Editor contributions — feature modules ported from VS Code's
//! `src/vs/editor/contrib/`.
//!
//! Each submodule encapsulates the state and logic for a single editor feature
//! (find/replace, folding, hover, autocomplete, etc.) as a self-contained unit
//! that can be driven by the GPU renderer or a Tauri command layer.

pub mod bracket_matching;
pub mod clipboard_operations;
pub mod code_action;
pub mod codelens;
pub mod color_picker;
pub mod comment;
pub mod find;
pub mod folding;
pub mod hover;
pub mod indent_guide;
pub mod inlay_hints;
pub mod lines_operations;
pub mod linked_editing;
pub mod multicursor;
pub mod parameter_hints;
pub mod rename;
pub mod snippet_controller;
pub mod sticky_scroll;
pub mod suggest;
pub mod word_highlighter;
