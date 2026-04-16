//! Completion / suggestion types and fuzzy matching for the editor.
//!
//! Provides [`CompletionItem`], [`CompletionList`], trigger handling, and a
//! fuzzy scoring algorithm that returns match positions for highlighting.

use serde::{Deserialize, Serialize};

use sidex_text::Position;

/// The kind of a completion item, matching Monaco / LSP completion item kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompletionItemKind {
    Text,
    Method,
    Function,
    Constructor,
    Field,
    Variable,
    Class,
    Interface,
    Module,
    Property,
    Unit,
    Value,
    Enum,
    Keyword,
    Snippet,
    Color,
    File,
    Reference,
    Folder,
    EnumMember,
    Constant,
    Struct,
    Event,
    Operator,
    TypeParameter,
}

/// A text edit to apply when a completion is accepted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompletionTextEdit {
    /// The range to replace.
    pub range: sidex_text::Range,
    /// The new text.
    pub new_text: String,
}

/// A single completion suggestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    /// The label shown in the completion list.
    pub label: String,
    /// The kind of completion.
    pub kind: CompletionItemKind,
    /// A short detail string (e.g. type signature).
    pub detail: Option<String>,
    /// Longer documentation (markdown or plain text).
    pub documentation: Option<String>,
    /// The text to insert. If `None`, `label` is used.
    pub insert_text: Option<String>,
    /// String used for sorting. If `None`, `label` is used.
    pub sort_text: Option<String>,
    /// String used for filtering. If `None`, `label` is used.
    pub filter_text: Option<String>,
    /// A text edit to apply instead of simple insertion.
    pub text_edit: Option<CompletionTextEdit>,
    /// Additional edits (e.g. auto-import).
    pub additional_edits: Vec<CompletionTextEdit>,
    /// An optional command to execute after accepting.
    pub command: Option<String>,
    /// Whether this item was pre-selected.
    pub preselect: bool,
}

impl CompletionItem {
    /// Returns the text used for filtering (falls back to label).
    pub fn effective_filter_text(&self) -> &str {
        self.filter_text.as_deref().unwrap_or(&self.label)
    }

    /// Returns the text used for sorting (falls back to label).
    pub fn effective_sort_text(&self) -> &str {
        self.sort_text.as_deref().unwrap_or(&self.label)
    }

    /// Returns the text that would be inserted (falls back to label).
    pub fn effective_insert_text(&self) -> &str {
        self.insert_text.as_deref().unwrap_or(&self.label)
    }
}

/// A list of completion items, possibly incomplete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionList {
    /// The completion items.
    pub items: Vec<CompletionItem>,
    /// If `true`, re-typing should re-request completions.
    pub is_incomplete: bool,
}

impl CompletionList {
    /// Creates a complete list from items.
    pub fn new(items: Vec<CompletionItem>) -> Self {
        Self {
            items,
            is_incomplete: false,
        }
    }

    /// Creates an incomplete list (the server has more results).
    pub fn incomplete(items: Vec<CompletionItem>) -> Self {
        Self {
            items,
            is_incomplete: true,
        }
    }
}

/// How a completion was triggered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompletionTriggerKind {
    /// Explicitly invoked (e.g. Ctrl+Space).
    Invoked,
    /// Triggered by a specific character (e.g. `.`).
    TriggerCharacter,
    /// Re-triggered because the previous result was incomplete.
    TriggerForIncompleteCompletions,
}

/// Context about how a completion request was triggered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionTrigger {
    /// The kind of trigger.
    pub kind: CompletionTriggerKind,
    /// The trigger character, if applicable.
    pub character: Option<char>,
    /// The cursor position when the completion was triggered.
    pub position: Position,
}

// ── Fuzzy matching ────────────────────────────────────────────────

/// Computes a fuzzy match score for `pattern` against `word`.
///
/// Returns `Some((score, match_positions))` where `match_positions` contains
/// the indices in `word` that matched (for highlighting), or `None` if the
/// pattern does not match.
///
/// Higher scores indicate better matches. Consecutive matches, word-boundary
/// matches, and camelCase matches receive bonuses.
pub fn fuzzy_score(pattern: &str, word: &str) -> Option<(i32, Vec<usize>)> {
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let word_chars: Vec<char> = word.chars().collect();
    let p_len = pattern_chars.len();
    let w_len = word_chars.len();

    if p_len == 0 {
        return Some((0, Vec::new()));
    }
    if p_len > w_len {
        return None;
    }

    let mut positions = Vec::with_capacity(p_len);
    let mut pi = 0;
    let mut score: i32 = 0;
    let mut prev_match: Option<usize> = None;

    for (wi, &wc) in word_chars.iter().enumerate() {
        if pi < p_len && wc.to_lowercase().eq(pattern_chars[pi].to_lowercase()) {
            positions.push(wi);

            score += 1;

            // Bonus for consecutive matches.
            if prev_match == Some(wi.wrapping_sub(1)) {
                score += 5;
            }

            // Bonus for matching at word boundaries.
            if wi == 0 || !word_chars[wi - 1].is_alphanumeric() {
                score += 10;
            }

            // Bonus for camelCase match.
            if wi > 0 && wc.is_uppercase() && word_chars[wi - 1].is_lowercase() {
                score += 8;
            }

            // Exact case match bonus.
            if wc == pattern_chars[pi] {
                score += 1;
            }

            prev_match = Some(wi);
            pi += 1;
        }
    }

    if pi == p_len {
        // Penalise longer words slightly so shorter matches rank higher.
        score -= (w_len as i32 - p_len as i32) / 4;
        Some((score, positions))
    } else {
        None
    }
}

/// Filters and sorts completion items using fuzzy matching.
///
/// Returns a `Vec<(item_index, score, match_positions)>` sorted by
/// descending score (best matches first).
pub fn fuzzy_filter(items: &[CompletionItem], pattern: &str) -> Vec<(usize, i32, Vec<usize>)> {
    let mut results: Vec<(usize, i32, Vec<usize>)> = items
        .iter()
        .enumerate()
        .filter_map(|(idx, item)| {
            let text = item.effective_filter_text();
            fuzzy_score(pattern, text).map(|(score, positions)| (idx, score, positions))
        })
        .collect();

    results.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(label: &str, kind: CompletionItemKind) -> CompletionItem {
        CompletionItem {
            label: label.to_string(),
            kind,
            detail: None,
            documentation: None,
            insert_text: None,
            sort_text: None,
            filter_text: None,
            text_edit: None,
            additional_edits: Vec::new(),
            command: None,
            preselect: false,
        }
    }

    #[test]
    fn fuzzy_score_exact_match() {
        let (score, positions) = fuzzy_score("hello", "hello").unwrap();
        assert!(score > 0);
        assert_eq!(positions, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn fuzzy_score_prefix_match() {
        let result = fuzzy_score("hel", "hello");
        assert!(result.is_some());
        let (score, positions) = result.unwrap();
        assert!(score > 0);
        assert_eq!(positions, vec![0, 1, 2]);
    }

    #[test]
    fn fuzzy_score_no_match() {
        assert!(fuzzy_score("xyz", "hello").is_none());
    }

    #[test]
    fn fuzzy_score_empty_pattern() {
        let (score, positions) = fuzzy_score("", "hello").unwrap();
        assert_eq!(score, 0);
        assert!(positions.is_empty());
    }

    #[test]
    fn fuzzy_score_case_insensitive() {
        let result = fuzzy_score("HEL", "hello");
        assert!(result.is_some());
    }

    #[test]
    fn fuzzy_score_camel_case_bonus() {
        let (camel_score, _) = fuzzy_score("gN", "getName").unwrap();
        let (flat_score, _) = fuzzy_score("gN", "getnothing").unwrap_or((0, vec![]));
        assert!(camel_score > flat_score);
    }

    #[test]
    fn fuzzy_score_pattern_longer_than_word() {
        assert!(fuzzy_score("longpattern", "short").is_none());
    }

    #[test]
    fn fuzzy_filter_sorts_by_score() {
        let items = vec![
            item("toString", CompletionItemKind::Method),
            item("toJSON", CompletionItemKind::Method),
            item("total", CompletionItemKind::Variable),
            item("map", CompletionItemKind::Method),
        ];
        let results = fuzzy_filter(&items, "to");
        assert!(!results.is_empty());
        assert!(results[0].0 != 3); // "map" shouldn't be first
    }

    #[test]
    fn fuzzy_filter_no_matches() {
        let items = vec![item("hello", CompletionItemKind::Text)];
        let results = fuzzy_filter(&items, "xyz");
        assert!(results.is_empty());
    }

    #[test]
    fn fuzzy_filter_empty_pattern() {
        let items = vec![
            item("a", CompletionItemKind::Text),
            item("b", CompletionItemKind::Text),
        ];
        let results = fuzzy_filter(&items, "");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn completion_item_effective_fields() {
        let mut ci = item("foo", CompletionItemKind::Function);
        assert_eq!(ci.effective_filter_text(), "foo");
        assert_eq!(ci.effective_sort_text(), "foo");
        assert_eq!(ci.effective_insert_text(), "foo");

        ci.filter_text = Some("bar".into());
        ci.sort_text = Some("001".into());
        ci.insert_text = Some("foo()".into());
        assert_eq!(ci.effective_filter_text(), "bar");
        assert_eq!(ci.effective_sort_text(), "001");
        assert_eq!(ci.effective_insert_text(), "foo()");
    }

    #[test]
    fn completion_list_complete() {
        let list = CompletionList::new(vec![item("a", CompletionItemKind::Text)]);
        assert!(!list.is_incomplete);
        assert_eq!(list.items.len(), 1);
    }

    #[test]
    fn completion_list_incomplete() {
        let list = CompletionList::incomplete(vec![]);
        assert!(list.is_incomplete);
    }

    #[test]
    fn completion_trigger_kinds() {
        let trigger = CompletionTrigger {
            kind: CompletionTriggerKind::TriggerCharacter,
            character: Some('.'),
            position: Position::new(0, 5),
        };
        assert_eq!(trigger.kind, CompletionTriggerKind::TriggerCharacter);
        assert_eq!(trigger.character, Some('.'));
    }
}
