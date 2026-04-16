//! Signature help / parameter hints engine wrapping LSP
//! `textDocument/signatureHelp`.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::client::LspClient;
use crate::conversion::position_to_lsp;

/// Information about a single function/method signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignatureInfo {
    /// The full signature label (e.g. `"fn foo(x: i32, y: &str) -> bool"`).
    pub label: String,
    /// Optional documentation for the signature.
    pub documentation: Option<String>,
    /// Parameter information.
    pub parameters: Vec<ParameterInfo>,
    /// Index of the currently active parameter.
    pub active_parameter: usize,
}

/// Information about a single parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterInfo {
    /// The parameter label (e.g. `"x: i32"` or just `"x"`).
    pub label: String,
    /// Optional documentation for the parameter.
    pub documentation: Option<String>,
}

/// Requests signature help from the language server.
pub async fn request_signature(
    client: &LspClient,
    uri: &str,
    pos: sidex_text::Position,
) -> Result<Option<Vec<SignatureInfo>>> {
    let lsp_pos = position_to_lsp(pos);
    let response = client.signature_help(uri, lsp_pos).await?;

    let Some(sig_help) = response else {
        return Ok(None);
    };

    if sig_help.signatures.is_empty() {
        return Ok(None);
    }

    let active_sig = sig_help.active_signature.unwrap_or(0) as usize;
    let active_param = sig_help.active_parameter.unwrap_or(0) as usize;

    let signatures: Vec<SignatureInfo> = sig_help
        .signatures
        .into_iter()
        .enumerate()
        .map(|(i, sig)| {
            let documentation = sig.documentation.map(|doc| match doc {
                lsp_types::Documentation::String(s) => s,
                lsp_types::Documentation::MarkupContent(mc) => mc.value,
            });

            let parameters: Vec<ParameterInfo> = sig
                .parameters
                .unwrap_or_default()
                .into_iter()
                .map(|p| {
                    let label = match p.label {
                        lsp_types::ParameterLabel::Simple(s) => s,
                        lsp_types::ParameterLabel::LabelOffsets([start, end]) => {
                            if (start as usize) < sig.label.len()
                                && (end as usize) <= sig.label.len()
                            {
                                sig.label[start as usize..end as usize].to_owned()
                            } else {
                                format!("[{start}..{end}]")
                            }
                        }
                    };
                    let documentation = p.documentation.map(|doc| match doc {
                        lsp_types::Documentation::String(s) => s,
                        lsp_types::Documentation::MarkupContent(mc) => mc.value,
                    });
                    ParameterInfo {
                        label,
                        documentation,
                    }
                })
                .collect();

            let effective_active = if i == active_sig { active_param } else { 0 };

            SignatureInfo {
                label: sig.label,
                documentation,
                parameters,
                active_parameter: effective_active,
            }
        })
        .collect();

    Ok(Some(signatures))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_info_fields() {
        let sig = SignatureInfo {
            label: "fn foo(x: i32, y: &str) -> bool".into(),
            documentation: Some("Does foo things.".into()),
            parameters: vec![
                ParameterInfo {
                    label: "x: i32".into(),
                    documentation: Some("The x value.".into()),
                },
                ParameterInfo {
                    label: "y: &str".into(),
                    documentation: None,
                },
            ],
            active_parameter: 0,
        };
        assert_eq!(sig.parameters.len(), 2);
        assert_eq!(sig.active_parameter, 0);
        assert!(sig.documentation.is_some());
    }

    #[test]
    fn parameter_info_serialize() {
        let param = ParameterInfo {
            label: "x: i32".into(),
            documentation: Some("the x param".into()),
        };
        let json = serde_json::to_string(&param).unwrap();
        let back: ParameterInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.label, "x: i32");
        assert_eq!(back.documentation, Some("the x param".into()));
    }

    #[test]
    fn signature_info_serialize() {
        let sig = SignatureInfo {
            label: "fn test()".into(),
            documentation: None,
            parameters: vec![],
            active_parameter: 0,
        };
        let json = serde_json::to_string(&sig).unwrap();
        let back: SignatureInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.label, "fn test()");
    }
}
