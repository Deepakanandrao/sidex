//! Autocomplete / suggestion widget state — mirrors VS Code's
//! `SuggestModel` + `SuggestWidget` + `CompletionModel`.
//!
//! Owns the active completion session, the filtered/sorted item list,
//! selection index, and trigger logic.

use crate::completion::{CompletionItem, CompletionTriggerKind};
use sidex_text::Position;

/// Trigger characters that automatically open the suggest widget.
pub const DEFAULT_TRIGGER_CHARS: &[char] = &['.', ':', '<', '"', '/', '@', '#'];

/// State of the suggestion widget's detail pane.
#[derive(Debug, Clone, Default)]
pub struct SuggestDetailPane {
    /// Whether the detail pane is expanded.
    pub is_visible: bool,
    /// Resolved documentation for the focused item.
    pub documentation: Option<String>,
    /// Type signature or detail string.
    pub detail: Option<String>,
}

/// The complete autocomplete session state.
#[derive(Debug, Clone)]
pub struct SuggestState {
    /// Whether the suggest widget is currently active.
    pub is_active: bool,
    /// How the session was triggered.
    pub trigger_kind: CompletionTriggerKind,
    /// The character that triggered the session (if trigger-character).
    pub trigger_character: Option<char>,
    /// The position where the completion was triggered.
    pub trigger_position: Option<Position>,
    /// The current filter/prefix text typed since the trigger.
    pub filter_text: String,
    /// All completion items received from the provider.
    pub all_items: Vec<CompletionItem>,
    /// Filtered + sorted items actually shown in the widget.
    pub visible_items: Vec<CompletionItem>,
    /// Zero-based index of the focused item in `visible_items`.
    pub selected_index: usize,
    /// Detail pane state.
    pub detail_pane: SuggestDetailPane,
    /// Whether a completion request is in-flight.
    pub is_loading: bool,
    /// Whether automatic suggestions on typing are enabled.
    pub auto_trigger_enabled: bool,
}

impl Default for SuggestState {
    fn default() -> Self {
        Self {
            is_active: false,
            trigger_kind: CompletionTriggerKind::Invoked,
            trigger_character: None,
            trigger_position: None,
            filter_text: String::new(),
            all_items: Vec::new(),
            visible_items: Vec::new(),
            selected_index: 0,
            detail_pane: SuggestDetailPane::default(),
            is_loading: false,
            auto_trigger_enabled: true,
        }
    }
}

impl SuggestState {
    /// Triggers a new completion session at the given position.
    pub fn trigger_suggest(
        &mut self,
        pos: Position,
        kind: CompletionTriggerKind,
        trigger_char: Option<char>,
    ) {
        self.is_active = true;
        self.trigger_kind = kind;
        self.trigger_character = trigger_char;
        self.trigger_position = Some(pos);
        self.filter_text.clear();
        self.all_items.clear();
        self.visible_items.clear();
        self.selected_index = 0;
        self.is_loading = true;
    }

    /// Receives items from the provider and applies filtering.
    pub fn receive_items(&mut self, items: Vec<CompletionItem>) {
        self.all_items = items;
        self.is_loading = false;
        self.refilter();
    }

    /// Re-filters `all_items` based on the current `filter_text`.
    pub fn refilter(&mut self) {
        if self.filter_text.is_empty() {
            self.visible_items = self.all_items.clone();
        } else {
            self.visible_items = self
                .all_items
                .iter()
                .filter(|item| {
                    let label_lower = item.label.to_lowercase();
                    let filter_lower = self.filter_text.to_lowercase();
                    label_lower.contains(&filter_lower)
                })
                .cloned()
                .collect();
        }
        if self.selected_index >= self.visible_items.len() {
            self.selected_index = 0;
        }
        if self.visible_items.is_empty() {
            self.cancel();
        }
    }

    /// Updates the filter text (called on each keystroke).
    pub fn update_filter(&mut self, text: String) {
        self.filter_text = text;
        self.refilter();
    }

    /// Selects the next item in the list.
    pub fn next_suggestion(&mut self) {
        if !self.visible_items.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.visible_items.len();
        }
    }

    /// Selects the previous item in the list.
    pub fn prev_suggestion(&mut self) {
        if !self.visible_items.is_empty() {
            self.selected_index = if self.selected_index == 0 {
                self.visible_items.len() - 1
            } else {
                self.selected_index - 1
            };
        }
    }

    /// Returns the currently focused completion item, if any.
    #[must_use]
    pub fn focused_item(&self) -> Option<&CompletionItem> {
        self.visible_items.get(self.selected_index)
    }

    /// Accepts the currently focused suggestion.  Returns the item to insert.
    pub fn accept_suggestion(&mut self) -> Option<CompletionItem> {
        let item = self.visible_items.get(self.selected_index).cloned();
        self.cancel();
        item
    }

    /// Cancels the current completion session.
    pub fn cancel(&mut self) {
        self.is_active = false;
        self.is_loading = false;
        self.all_items.clear();
        self.visible_items.clear();
        self.selected_index = 0;
        self.detail_pane = SuggestDetailPane::default();
    }

    /// Returns `true` if the given character should auto-trigger completions.
    #[must_use]
    pub fn is_trigger_character(ch: char) -> bool {
        DEFAULT_TRIGGER_CHARS.contains(&ch)
    }

    /// Returns `true` if the given character should auto-trigger based on a
    /// custom set of trigger characters (from a language server).
    #[must_use]
    pub fn is_custom_trigger(ch: char, triggers: &[char]) -> bool {
        triggers.contains(&ch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::completion::CompletionItemKind;

    fn make_item(label: &str) -> CompletionItem {
        CompletionItem {
            label: label.to_string(),
            kind: CompletionItemKind::Function,
            detail: None,
            documentation: None,
            insert_text: None,
            filter_text: None,
            sort_text: None,
            text_edit: None,
            additional_edits: Vec::new(),
            command: None,
            preselect: false,
        }
    }

    #[test]
    fn trigger_and_receive() {
        let mut state = SuggestState::default();
        state.trigger_suggest(Position::new(0, 5), CompletionTriggerKind::Invoked, None);
        assert!(state.is_active);
        assert!(state.is_loading);

        state.receive_items(vec![make_item("foo"), make_item("bar")]);
        assert!(!state.is_loading);
        assert_eq!(state.visible_items.len(), 2);
    }

    #[test]
    fn filter_narrows_items() {
        let mut state = SuggestState::default();
        state.trigger_suggest(Position::new(0, 0), CompletionTriggerKind::Invoked, None);
        state.receive_items(vec![
            make_item("forEach"),
            make_item("filter"),
            make_item("map"),
        ]);

        state.update_filter("f".into());
        assert_eq!(state.visible_items.len(), 2);

        state.update_filter("fil".into());
        assert_eq!(state.visible_items.len(), 1);
        assert_eq!(state.visible_items[0].label, "filter");
    }

    #[test]
    fn navigate_suggestions() {
        let mut state = SuggestState::default();
        state.trigger_suggest(Position::new(0, 0), CompletionTriggerKind::Invoked, None);
        state.receive_items(vec![make_item("a"), make_item("b"), make_item("c")]);

        assert_eq!(state.selected_index, 0);
        state.next_suggestion();
        assert_eq!(state.selected_index, 1);
        state.next_suggestion();
        assert_eq!(state.selected_index, 2);
        state.next_suggestion();
        assert_eq!(state.selected_index, 0); // wraps
    }

    #[test]
    fn accept_clears_session() {
        let mut state = SuggestState::default();
        state.trigger_suggest(Position::new(0, 0), CompletionTriggerKind::Invoked, None);
        state.receive_items(vec![make_item("hello")]);

        let accepted = state.accept_suggestion();
        assert!(accepted.is_some());
        assert!(!state.is_active);
    }
}
