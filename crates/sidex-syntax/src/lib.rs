//! # sidex-syntax
//!
//! Syntax highlighting and parsing for the `SideX` editor, powered by
//! [tree-sitter](https://tree-sitter.github.io/) with `TextMate` grammar
//! fallback and LSP semantic token support.
//!
//! This crate provides:
//!
//! - **Highlighting** — run tree-sitter queries to produce a stream of
//!   [`HighlightEvent`]s for rendering.
//! - **Incremental parsing** — maintain a parse tree per document and cheaply
//!   re-parse after edits.
//! - **Language registry** — map file extensions to tree-sitter grammars.
//! - **Scope mapping** — resolve tree-sitter capture names to semantic
//!   highlight categories.
//! - **Bracket matching** — AST-aware matching of bracket pairs.
//! - **Code folding** — derive foldable regions from the parse tree.
//! - **`TextMate` grammars** — regex-based fallback tokenizer for languages
//!   without tree-sitter support.
//! - **Semantic tokens** — merge LSP semantic token overlays with syntax
//!   highlighting.
//! - **Auto-indentation** — rule-based indent/outdent computation.

pub mod bracket;
pub mod folding;
pub mod highlight;
pub mod indent;
pub mod language;
pub mod parser;
pub mod scope;
pub mod semantic_tokens;
pub mod textmate;

pub use bracket::find_matching_bracket;
pub use folding::{compute_folding_ranges, FoldingKind, FoldingRange};
pub use highlight::{Highlight, HighlightConfig, HighlightError, HighlightEvent, Highlighter};
pub use indent::{compute_indent, default_indent_rules, IndentAction, IndentRule};
pub use language::{Language, LanguageRegistry};
pub use parser::{to_input_edit, DocumentParser};
pub use scope::{resolve_highlight_name, HighlightName};
pub use semantic_tokens::{
    decode_semantic_tokens, encode_semantic_tokens, merge_semantic_tokens, SemanticToken,
    SemanticTokenLegend, StyledSpan,
};
pub use textmate::{TextMateGrammar, TextMateTokenizer, TokenInfo, TokenizerState};
