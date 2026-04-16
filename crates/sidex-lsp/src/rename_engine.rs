//! Rename support wrapping LSP `textDocument/prepareRename` and
//! `textDocument/rename`.

use std::collections::HashMap;
use std::str::FromStr;

use anyhow::{Context, Result};
use lsp_types::{PrepareRenameResponse, TextDocumentIdentifier, TextDocumentPositionParams, Uri};
use serde::{Deserialize, Serialize};

use crate::client::LspClient;
use crate::conversion::{lsp_to_range, position_to_lsp};

/// Information returned by `prepareRename` — the valid range and a suggested
/// placeholder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameInfo {
    /// Range of the symbol to be renamed.
    pub range: sidex_text::Range,
    /// Suggested placeholder text (usually the current symbol name).
    pub placeholder: String,
}

/// A set of text edits grouped by file URI.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkspaceEdit {
    /// Map from file URI to the list of text edits for that file.
    pub changes: HashMap<String, Vec<TextEditInfo>>,
}

/// A single text edit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEditInfo {
    pub range: sidex_text::Range,
    pub new_text: String,
}

/// Checks whether a rename is valid at the given position and returns the
/// rename range and placeholder text.
pub async fn prepare_rename(
    client: &LspClient,
    uri: &str,
    pos: sidex_text::Position,
) -> Result<Option<RenameInfo>> {
    let lsp_pos = position_to_lsp(pos);
    let params = TextDocumentPositionParams {
        text_document: TextDocumentIdentifier::new(Uri::from_str(uri).context("invalid URI")?),
        position: lsp_pos,
    };
    let val = serde_json::to_value(params)?;
    let result = client
        .raw_request("textDocument/prepareRename", Some(val))
        .await?;

    if result.is_null() {
        return Ok(None);
    }

    let response: PrepareRenameResponse =
        serde_json::from_value(result).context("failed to parse prepareRename response")?;

    let info = match response {
        PrepareRenameResponse::Range(range) => RenameInfo {
            range: lsp_to_range(range),
            placeholder: String::new(),
        },
        PrepareRenameResponse::RangeWithPlaceholder { range, placeholder } => RenameInfo {
            range: lsp_to_range(range),
            placeholder,
        },
        PrepareRenameResponse::DefaultBehavior {
            default_behavior: _,
        } => {
            return Ok(None);
        }
    };

    Ok(Some(info))
}

/// Executes a rename at the given position with the new name, returning a
/// workspace edit.
pub async fn execute_rename(
    client: &LspClient,
    uri: &str,
    pos: sidex_text::Position,
    new_name: &str,
) -> Result<WorkspaceEdit> {
    let lsp_pos = position_to_lsp(pos);
    let response = client.rename(uri, lsp_pos, new_name).await?;

    match response {
        Some(edit) => Ok(convert_workspace_edit(edit)),
        None => Ok(WorkspaceEdit::default()),
    }
}

fn convert_workspace_edit(edit: lsp_types::WorkspaceEdit) -> WorkspaceEdit {
    let mut changes = HashMap::new();

    if let Some(raw_changes) = edit.changes {
        for (uri, edits) in raw_changes {
            let converted: Vec<TextEditInfo> = edits
                .into_iter()
                .map(|e| TextEditInfo {
                    range: lsp_to_range(e.range),
                    new_text: e.new_text,
                })
                .collect();
            changes.insert(uri.to_string(), converted);
        }
    }

    if let Some(document_changes) = edit.document_changes {
        use lsp_types::DocumentChanges;
        let operations = match document_changes {
            DocumentChanges::Edits(edits) => edits
                .into_iter()
                .map(lsp_types::DocumentChangeOperation::Edit)
                .collect::<Vec<_>>(),
            DocumentChanges::Operations(ops) => ops,
        };
        for change in operations {
            if let lsp_types::DocumentChangeOperation::Edit(text_doc_edit) = change {
                let uri_str = text_doc_edit.text_document.uri.to_string();
                let edits: Vec<TextEditInfo> = text_doc_edit
                    .edits
                    .into_iter()
                    .map(|e| match e {
                        lsp_types::OneOf::Left(edit) => TextEditInfo {
                            range: lsp_to_range(edit.range),
                            new_text: edit.new_text,
                        },
                        lsp_types::OneOf::Right(annotated) => TextEditInfo {
                            range: lsp_to_range(annotated.text_edit.range),
                            new_text: annotated.text_edit.new_text,
                        },
                    })
                    .collect();
                changes
                    .entry(uri_str)
                    .or_insert_with(Vec::new)
                    .extend(edits);
            }
        }
    }

    WorkspaceEdit { changes }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rename_info_serialize() {
        let info = RenameInfo {
            range: sidex_text::Range::new(
                sidex_text::Position::new(5, 10),
                sidex_text::Position::new(5, 15),
            ),
            placeholder: "old_name".into(),
        };
        let json = serde_json::to_string(&info).unwrap();
        let back: RenameInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(back.placeholder, "old_name");
    }

    #[test]
    fn workspace_edit_default_empty() {
        let edit = WorkspaceEdit::default();
        assert!(edit.changes.is_empty());
    }

    #[test]
    fn convert_workspace_edit_from_changes() {
        let mut raw_changes = HashMap::new();
        raw_changes.insert(
            "file:///test.rs".parse::<Uri>().unwrap(),
            vec![lsp_types::TextEdit {
                range: lsp_types::Range::new(
                    lsp_types::Position::new(1, 0),
                    lsp_types::Position::new(1, 5),
                ),
                new_text: "new_name".into(),
            }],
        );
        let lsp_edit = lsp_types::WorkspaceEdit {
            changes: Some(raw_changes),
            document_changes: None,
            change_annotations: None,
        };
        let result = convert_workspace_edit(lsp_edit);
        assert_eq!(result.changes.len(), 1);
        let edits = result.changes.values().next().unwrap();
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "new_name");
    }

    #[test]
    fn text_edit_info_fields() {
        let edit = TextEditInfo {
            range: sidex_text::Range::new(
                sidex_text::Position::new(0, 0),
                sidex_text::Position::new(0, 5),
            ),
            new_text: "replacement".into(),
        };
        assert_eq!(edit.new_text, "replacement");
        assert_eq!(edit.range.start.column, 0);
        assert_eq!(edit.range.end.column, 5);
    }

    #[test]
    fn workspace_edit_serialize() {
        let mut changes = HashMap::new();
        changes.insert(
            "file:///a.rs".into(),
            vec![TextEditInfo {
                range: sidex_text::Range::new(
                    sidex_text::Position::new(0, 0),
                    sidex_text::Position::new(0, 3),
                ),
                new_text: "bar".into(),
            }],
        );
        let edit = WorkspaceEdit { changes };
        let json = serde_json::to_string(&edit).unwrap();
        let back: WorkspaceEdit = serde_json::from_str(&json).unwrap();
        assert_eq!(back.changes.len(), 1);
    }
}
