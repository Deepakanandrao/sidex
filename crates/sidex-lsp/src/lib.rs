//! # sidex-lsp
//!
//! Language Server Protocol 3.17 client for the `SideX` editor.
//!
//! This crate provides a full-featured LSP client that communicates with
//! language servers over stdio using JSON-RPC 2.0. It includes:
//!
//! - **Transport** — `Content-Length`-framed JSON-RPC over async stdio.
//! - **Client** — high-level async API for all common LSP requests and
//!   notifications.
//! - **Registry** — maps language IDs to server configurations with
//!   sensible built-in defaults.
//! - **Diagnostics** — stores and queries diagnostics per file URI.
//! - **Capabilities** — ergonomic wrappers for server capability
//!   negotiation.
//! - **Conversion** — lossless type mapping between `sidex_text` and
//!   `lsp_types`.
//! - **Completion** — full completion session management with sorting,
//!   filtering, and snippet support.
//! - **Hover** — hover information with plaintext and markdown content.
//! - **Signature help** — function signature / parameter hints.
//! - **Go-to** — definition, declaration, implementation, type definition,
//!   and references navigation.
//! - **Rename** — prepare-rename and execute-rename support.
//! - **Code actions** — quick fixes, refactorings, and source organizers.
//! - **Inlay hints** — inline type and parameter annotations.

pub mod capabilities;
pub mod client;
pub mod code_action_engine;
pub mod completion_engine;
pub mod conversion;
pub mod diagnostics;
pub mod go_to;
pub mod hover_engine;
pub mod inlay_hints;
pub mod registry;
pub mod rename_engine;
pub mod signature_help;
pub mod transport;

pub use capabilities::ServerCaps;
pub use client::LspClient;
pub use code_action_engine::{request_code_actions, CodeActionInfo, CodeActionKind};
pub use completion_engine::{
    filter_completion_items, sort_completion_items, CompletionSession, CompletionTrigger,
};
pub use conversion::{lsp_to_position, lsp_to_range, position_to_lsp, range_to_lsp};
pub use diagnostics::DiagnosticCollection;
pub use go_to::{
    find_references, goto_declaration, goto_definition, goto_implementation, goto_type_definition,
    Location,
};
pub use hover_engine::{request_hover, HoverInfo, MarkupContent};
pub use inlay_hints::{request_inlay_hints, InlayHintInfo, InlayHintKind};
pub use registry::{ServerConfig, ServerRegistry};
pub use rename_engine::{execute_rename, prepare_rename, RenameInfo, WorkspaceEdit};
pub use signature_help::{request_signature, ParameterInfo, SignatureInfo};
pub use transport::{JsonRpcError, JsonRpcMessage, LspTransport, RequestId};
