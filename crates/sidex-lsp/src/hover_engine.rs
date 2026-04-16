//! Hover information engine wrapping LSP `textDocument/hover`.
//!
//! Provides a simplified API over the raw LSP hover response, converting
//! markup content into editor-friendly types.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::client::LspClient;
use crate::conversion::{lsp_to_range, position_to_lsp};

/// Markup content for hover display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarkupContent {
    /// Plain text content.
    Plaintext(String),
    /// Markdown-formatted content.
    Markdown(String),
}

/// Hover information returned to the editor.
#[derive(Debug, Clone)]
pub struct HoverInfo {
    /// One or more content blocks to display.
    pub contents: Vec<MarkupContent>,
    /// Optional range of the symbol that was hovered.
    pub range: Option<sidex_text::Range>,
}

/// Requests hover information from the language server.
pub async fn request_hover(
    client: &LspClient,
    uri: &str,
    pos: sidex_text::Position,
) -> Result<Option<HoverInfo>> {
    let lsp_pos = position_to_lsp(pos);
    let response = client.hover(uri, lsp_pos).await?;

    let Some(hover) = response else {
        return Ok(None);
    };

    let contents = convert_hover_contents(hover.contents);
    let range = hover.range.map(lsp_to_range);

    Ok(Some(HoverInfo { contents, range }))
}

fn convert_hover_contents(contents: lsp_types::HoverContents) -> Vec<MarkupContent> {
    match contents {
        lsp_types::HoverContents::Scalar(value) => {
            vec![convert_marked_string(value)]
        }
        lsp_types::HoverContents::Array(values) => {
            values.into_iter().map(convert_marked_string).collect()
        }
        lsp_types::HoverContents::Markup(markup) => {
            vec![convert_markup_content(markup)]
        }
    }
}

fn convert_marked_string(ms: lsp_types::MarkedString) -> MarkupContent {
    match ms {
        lsp_types::MarkedString::String(s) => MarkupContent::Plaintext(s),
        lsp_types::MarkedString::LanguageString(ls) => {
            MarkupContent::Markdown(format!("```{}\n{}\n```", ls.language, ls.value))
        }
    }
}

fn convert_markup_content(mc: lsp_types::MarkupContent) -> MarkupContent {
    match mc.kind {
        lsp_types::MarkupKind::PlainText => MarkupContent::Plaintext(mc.value),
        lsp_types::MarkupKind::Markdown => MarkupContent::Markdown(mc.value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_scalar_string() {
        let contents =
            lsp_types::HoverContents::Scalar(lsp_types::MarkedString::String("hello".into()));
        let result = convert_hover_contents(contents);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], MarkupContent::Plaintext("hello".into()));
    }

    #[test]
    fn convert_scalar_language_string() {
        let contents = lsp_types::HoverContents::Scalar(lsp_types::MarkedString::LanguageString(
            lsp_types::LanguageString {
                language: "rust".into(),
                value: "fn main()".into(),
            },
        ));
        let result = convert_hover_contents(contents);
        assert_eq!(result.len(), 1);
        assert!(matches!(&result[0], MarkupContent::Markdown(s) if s.contains("rust")));
    }

    #[test]
    fn convert_array() {
        let contents = lsp_types::HoverContents::Array(vec![
            lsp_types::MarkedString::String("first".into()),
            lsp_types::MarkedString::String("second".into()),
        ]);
        let result = convert_hover_contents(contents);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn convert_markup_plaintext() {
        let contents = lsp_types::HoverContents::Markup(lsp_types::MarkupContent {
            kind: lsp_types::MarkupKind::PlainText,
            value: "plain text".into(),
        });
        let result = convert_hover_contents(contents);
        assert_eq!(result[0], MarkupContent::Plaintext("plain text".into()));
    }

    #[test]
    fn convert_markup_markdown() {
        let contents = lsp_types::HoverContents::Markup(lsp_types::MarkupContent {
            kind: lsp_types::MarkupKind::Markdown,
            value: "**bold**".into(),
        });
        let result = convert_hover_contents(contents);
        assert_eq!(result[0], MarkupContent::Markdown("**bold**".into()));
    }

    #[test]
    fn hover_info_fields() {
        let info = HoverInfo {
            contents: vec![MarkupContent::Plaintext("test".into())],
            range: Some(sidex_text::Range::new(
                sidex_text::Position::new(1, 0),
                sidex_text::Position::new(1, 5),
            )),
        };
        assert_eq!(info.contents.len(), 1);
        assert!(info.range.is_some());
    }

    #[test]
    fn markup_content_serialize() {
        let mc = MarkupContent::Markdown("# Title".into());
        let json = serde_json::to_string(&mc).unwrap();
        let back: MarkupContent = serde_json::from_str(&json).unwrap();
        assert_eq!(mc, back);
    }
}
