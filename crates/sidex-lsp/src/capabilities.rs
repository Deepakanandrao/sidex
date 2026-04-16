//! Server capability negotiation helpers.
//!
//! Wraps [`lsp_types::ServerCapabilities`] with convenience methods for
//! checking which features a language server supports.

use lsp_types::ServerCapabilities;

/// Wrapper around [`ServerCapabilities`] providing ergonomic feature checks.
#[derive(Debug, Clone)]
pub struct ServerCaps {
    inner: ServerCapabilities,
}

impl ServerCaps {
    /// Creates a new wrapper from raw server capabilities.
    pub fn new(caps: ServerCapabilities) -> Self {
        Self { inner: caps }
    }

    /// Returns a reference to the underlying [`ServerCapabilities`].
    pub fn raw(&self) -> &ServerCapabilities {
        &self.inner
    }

    /// Whether the server supports `textDocument/completion`.
    pub fn supports_completion(&self) -> bool {
        self.inner.completion_provider.is_some()
    }

    /// Whether the server supports `textDocument/hover`.
    pub fn supports_hover(&self) -> bool {
        self.inner.hover_provider.is_some()
    }

    /// Whether the server supports `textDocument/definition`.
    pub fn supports_goto_definition(&self) -> bool {
        self.inner.definition_provider.is_some()
    }

    /// Whether the server supports `textDocument/references`.
    pub fn supports_references(&self) -> bool {
        self.inner.references_provider.is_some()
    }

    /// Whether the server supports `textDocument/rename`.
    pub fn supports_rename(&self) -> bool {
        self.inner.rename_provider.is_some()
    }

    /// Whether the server supports `textDocument/formatting`.
    pub fn supports_formatting(&self) -> bool {
        self.inner.document_formatting_provider.is_some()
    }

    /// Whether the server supports `textDocument/codeAction`.
    pub fn supports_code_action(&self) -> bool {
        self.inner.code_action_provider.is_some()
    }

    /// Whether the server supports `textDocument/signatureHelp`.
    pub fn supports_signature_help(&self) -> bool {
        self.inner.signature_help_provider.is_some()
    }

    /// Whether the server supports `textDocument/documentSymbol`.
    pub fn supports_document_symbols(&self) -> bool {
        self.inner.document_symbol_provider.is_some()
    }

    /// Whether the server supports `workspace/symbol`.
    pub fn supports_workspace_symbols(&self) -> bool {
        self.inner.workspace_symbol_provider.is_some()
    }

    /// Whether the server supports `textDocument/inlayHint`.
    pub fn supports_inlay_hints(&self) -> bool {
        self.inner.inlay_hint_provider.is_some()
    }

    /// Whether the server supports `textDocument/declaration`.
    pub fn supports_declaration(&self) -> bool {
        self.inner.declaration_provider.is_some()
    }

    /// Whether the server supports `textDocument/typeDefinition`.
    pub fn supports_type_definition(&self) -> bool {
        self.inner.type_definition_provider.is_some()
    }

    /// Whether the server supports `textDocument/implementation`.
    pub fn supports_implementation(&self) -> bool {
        self.inner.implementation_provider.is_some()
    }
}

impl From<ServerCapabilities> for ServerCaps {
    fn from(caps: ServerCapabilities) -> Self {
        Self::new(caps)
    }
}
