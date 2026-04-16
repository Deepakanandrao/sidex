//! Full completion handling engine wrapping LSP `textDocument/completion`.
//!
//! [`CompletionSession`] manages an active completion session — triggering,
//! filtering, sorting, resolving, and accepting completion items.

use std::cmp::Ordering;

use anyhow::{Context, Result};
use lsp_types::{
    CompletionItem, CompletionResponse, CompletionTextEdit, CompletionTriggerKind, InsertTextFormat,
};
use serde::{Deserialize, Serialize};

use crate::client::LspClient;
use crate::conversion::{lsp_to_range, position_to_lsp};

/// What triggered a completion request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompletionTrigger {
    /// Manually invoked (e.g. Ctrl+Space).
    Invoked,
    /// Triggered by a character (e.g. `.`, `:`).
    Character(char),
    /// Re-triggered while a completion session is already active.
    TriggerForIncomplete,
}

impl CompletionTrigger {
    fn to_lsp(&self) -> (CompletionTriggerKind, Option<String>) {
        match self {
            Self::Invoked => (CompletionTriggerKind::INVOKED, None),
            Self::Character(c) => (
                CompletionTriggerKind::TRIGGER_CHARACTER,
                Some(c.to_string()),
            ),
            Self::TriggerForIncomplete => (
                CompletionTriggerKind::TRIGGER_FOR_INCOMPLETE_COMPLETIONS,
                None,
            ),
        }
    }
}

/// A completion list with items and metadata.
#[derive(Debug, Clone)]
pub struct CompletionList {
    /// Whether the list is incomplete and should be re-fetched on further typing.
    pub is_incomplete: bool,
    /// The completion items.
    pub items: Vec<CompletionItem>,
}

/// An edit operation produced when a completion item is accepted.
#[derive(Debug, Clone)]
pub struct EditOperation {
    pub range: sidex_text::Range,
    pub new_text: String,
}

/// Manages an active completion session.
pub struct CompletionSession {
    items: Vec<CompletionItem>,
    is_incomplete: bool,
}

impl CompletionSession {
    /// Creates an empty session.
    #[must_use]
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            is_incomplete: false,
        }
    }

    /// Triggers a completion request against the language server.
    pub async fn trigger(
        &mut self,
        client: &LspClient,
        uri: &str,
        position: sidex_text::Position,
        trigger: CompletionTrigger,
    ) -> Result<CompletionList> {
        let lsp_pos = position_to_lsp(position);
        let (kind, character) = trigger.to_lsp();

        let params = lsp_types::CompletionParams {
            text_document_position: lsp_types::TextDocumentPositionParams {
                text_document: lsp_types::TextDocumentIdentifier::new(
                    uri.parse().context("invalid URI")?,
                ),
                position: lsp_pos,
            },
            work_done_progress_params: lsp_types::WorkDoneProgressParams::default(),
            partial_result_params: lsp_types::PartialResultParams::default(),
            context: Some(lsp_types::CompletionContext {
                trigger_kind: kind,
                trigger_character: character,
            }),
        };

        let result = serde_json::to_value(params)?;
        let response_val = client
            .raw_request("textDocument/completion", Some(result))
            .await?;

        let response: CompletionResponse =
            serde_json::from_value(response_val).context("failed to parse CompletionResponse")?;

        match response {
            CompletionResponse::Array(items) => {
                self.items = items;
                self.is_incomplete = false;
            }
            CompletionResponse::List(list) => {
                self.items = list.items;
                self.is_incomplete = list.is_incomplete;
            }
        }

        Ok(CompletionList {
            is_incomplete: self.is_incomplete,
            items: self.items.clone(),
        })
    }

    /// Resolves a completion item to get full details (documentation, etc.).
    pub async fn resolve(client: &LspClient, item: &CompletionItem) -> Result<CompletionItem> {
        let val = serde_json::to_value(item)?;
        let result = client
            .raw_request("completionItem/resolve", Some(val))
            .await?;
        serde_json::from_value(result).context("failed to parse resolved CompletionItem")
    }

    /// Converts a completion item into edit operations for the editor.
    pub fn accept(item: &CompletionItem) -> Vec<EditOperation> {
        let mut ops = Vec::new();

        if let Some(ref text_edit) = item.text_edit {
            match text_edit {
                CompletionTextEdit::Edit(edit) => {
                    ops.push(EditOperation {
                        range: lsp_to_range(edit.range),
                        new_text: edit.new_text.clone(),
                    });
                }
                CompletionTextEdit::InsertAndReplace(edit) => {
                    ops.push(EditOperation {
                        range: lsp_to_range(edit.replace),
                        new_text: edit.new_text.clone(),
                    });
                }
            }
        } else if let Some(ref insert_text) = item.insert_text {
            ops.push(EditOperation {
                range: sidex_text::Range::new(
                    sidex_text::Position::ZERO,
                    sidex_text::Position::ZERO,
                ),
                new_text: insert_text.clone(),
            });
        } else {
            ops.push(EditOperation {
                range: sidex_text::Range::new(
                    sidex_text::Position::ZERO,
                    sidex_text::Position::ZERO,
                ),
                new_text: item.label.clone(),
            });
        }

        if let Some(ref additional) = item.additional_text_edits {
            for edit in additional {
                ops.push(EditOperation {
                    range: lsp_to_range(edit.range),
                    new_text: edit.new_text.clone(),
                });
            }
        }

        ops
    }

    /// Returns the current items.
    #[must_use]
    pub fn items(&self) -> &[CompletionItem] {
        &self.items
    }

    /// Whether the completion list is incomplete.
    #[must_use]
    pub fn is_incomplete(&self) -> bool {
        self.is_incomplete
    }
}

impl Default for CompletionSession {
    fn default() -> Self {
        Self::new()
    }
}

/// Sorts completion items by `sort_text` first, then by `label`.
pub fn sort_completion_items(items: &mut [CompletionItem]) {
    items.sort_by(|a, b| {
        let a_sort = a.sort_text.as_deref().unwrap_or(&a.label);
        let b_sort = b.sort_text.as_deref().unwrap_or(&b.label);
        a_sort.cmp(b_sort).then_with(|| a.label.cmp(&b.label))
    });
}

/// Filters completion items by matching the typed prefix against
/// `filter_text` (falling back to `label`).
pub fn filter_completion_items<'a>(
    items: &'a [CompletionItem],
    prefix: &str,
) -> Vec<&'a CompletionItem> {
    if prefix.is_empty() {
        return items.iter().collect();
    }
    let lower_prefix = prefix.to_lowercase();
    items
        .iter()
        .filter(|item| {
            let filter = item
                .filter_text
                .as_deref()
                .unwrap_or(&item.label)
                .to_lowercase();
            filter.starts_with(&lower_prefix)
        })
        .collect()
}

/// Returns items with `preselect` set to `true` first, preserving order
/// within each group.
pub fn preselect_first(items: &mut [CompletionItem]) {
    items.sort_by(|a, b| {
        let a_pre = a.preselect.unwrap_or(false);
        let b_pre = b.preselect.unwrap_or(false);
        match (a_pre, b_pre) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            _ => Ordering::Equal,
        }
    });
}

/// Returns `true` if the item's `insert_text_format` indicates a snippet.
#[must_use]
pub fn is_snippet(item: &CompletionItem) -> bool {
    item.insert_text_format == Some(InsertTextFormat::SNIPPET)
}

#[cfg(test)]
mod tests {
    use lsp_types::TextEdit;

    use super::*;

    fn make_item(label: &str) -> CompletionItem {
        CompletionItem {
            label: label.to_owned(),
            ..CompletionItem::default()
        }
    }

    fn make_item_with_sort(label: &str, sort_text: &str) -> CompletionItem {
        CompletionItem {
            label: label.to_owned(),
            sort_text: Some(sort_text.to_owned()),
            ..CompletionItem::default()
        }
    }

    #[test]
    fn sort_by_sort_text() {
        let mut items = vec![
            make_item_with_sort("beta", "2"),
            make_item_with_sort("alpha", "1"),
            make_item_with_sort("gamma", "3"),
        ];
        sort_completion_items(&mut items);
        assert_eq!(items[0].label, "alpha");
        assert_eq!(items[1].label, "beta");
        assert_eq!(items[2].label, "gamma");
    }

    #[test]
    fn sort_by_label_fallback() {
        let mut items = vec![make_item("zebra"), make_item("apple"), make_item("mango")];
        sort_completion_items(&mut items);
        assert_eq!(items[0].label, "apple");
        assert_eq!(items[1].label, "mango");
        assert_eq!(items[2].label, "zebra");
    }

    #[test]
    fn filter_by_prefix() {
        let items = vec![
            make_item("println"),
            make_item("print"),
            make_item("format"),
        ];
        let filtered = filter_completion_items(&items, "pri");
        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|i| i.label.starts_with("pri")));
    }

    #[test]
    fn filter_case_insensitive() {
        let items = vec![make_item("HashMap"), make_item("hashCode")];
        let filtered = filter_completion_items(&items, "hash");
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn filter_empty_prefix_returns_all() {
        let items = vec![make_item("a"), make_item("b")];
        let filtered = filter_completion_items(&items, "");
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn filter_no_match() {
        let items = vec![make_item("foo"), make_item("bar")];
        let filtered = filter_completion_items(&items, "xyz");
        assert!(filtered.is_empty());
    }

    #[test]
    fn preselect_items_first() {
        let mut items = vec![
            make_item("normal"),
            CompletionItem {
                label: "preselected".to_owned(),
                preselect: Some(true),
                ..CompletionItem::default()
            },
        ];
        preselect_first(&mut items);
        assert_eq!(items[0].label, "preselected");
    }

    #[test]
    fn is_snippet_check() {
        let mut item = make_item("test");
        assert!(!is_snippet(&item));
        item.insert_text_format = Some(InsertTextFormat::SNIPPET);
        assert!(is_snippet(&item));
    }

    #[test]
    fn accept_with_label_fallback() {
        let item = make_item("println!");
        let ops = CompletionSession::accept(&item);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].new_text, "println!");
    }

    #[test]
    fn accept_with_insert_text() {
        let item = CompletionItem {
            label: "println!".to_owned(),
            insert_text: Some("println!($0)".to_owned()),
            ..CompletionItem::default()
        };
        let ops = CompletionSession::accept(&item);
        assert_eq!(ops[0].new_text, "println!($0)");
    }

    #[test]
    fn accept_with_text_edit() {
        let item = CompletionItem {
            label: "println!".to_owned(),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                range: lsp_types::Range::new(
                    lsp_types::Position::new(0, 0),
                    lsp_types::Position::new(0, 5),
                ),
                new_text: "println!".to_owned(),
            })),
            ..CompletionItem::default()
        };
        let ops = CompletionSession::accept(&item);
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].new_text, "println!");
        assert_eq!(ops[0].range.start.column, 0);
        assert_eq!(ops[0].range.end.column, 5);
    }

    #[test]
    fn accept_with_additional_edits() {
        let item = CompletionItem {
            label: "HashMap".to_owned(),
            additional_text_edits: Some(vec![TextEdit {
                range: lsp_types::Range::new(
                    lsp_types::Position::new(0, 0),
                    lsp_types::Position::new(0, 0),
                ),
                new_text: "use std::collections::HashMap;\n".to_owned(),
            }]),
            ..CompletionItem::default()
        };
        let ops = CompletionSession::accept(&item);
        assert_eq!(ops.len(), 2);
    }

    #[test]
    fn session_default() {
        let session = CompletionSession::default();
        assert!(session.items().is_empty());
        assert!(!session.is_incomplete());
    }

    #[test]
    fn completion_trigger_variants() {
        let (kind, ch) = CompletionTrigger::Invoked.to_lsp();
        assert_eq!(kind, CompletionTriggerKind::INVOKED);
        assert!(ch.is_none());

        let (kind, ch) = CompletionTrigger::Character('.').to_lsp();
        assert_eq!(kind, CompletionTriggerKind::TRIGGER_CHARACTER);
        assert_eq!(ch, Some(".".to_owned()));

        let (kind, _) = CompletionTrigger::TriggerForIncomplete.to_lsp();
        assert_eq!(
            kind,
            CompletionTriggerKind::TRIGGER_FOR_INCOMPLETE_COMPLETIONS
        );
    }

    #[test]
    fn filter_with_filter_text_field() {
        let items = vec![CompletionItem {
            label: "Display Label".to_owned(),
            filter_text: Some("actual_filter".to_owned()),
            ..CompletionItem::default()
        }];
        let filtered = filter_completion_items(&items, "actual");
        assert_eq!(filtered.len(), 1);
        let filtered = filter_completion_items(&items, "Display");
        assert!(filtered.is_empty());
    }
}
