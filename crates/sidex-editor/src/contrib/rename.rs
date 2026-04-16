//! Inline rename — mirrors VS Code's `RenameController` + `RenameWidget`.

use sidex_text::{Position, Range};

/// The phase of a rename operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenamePhase {
    /// No rename in progress.
    Idle,
    /// Resolving the rename range from the language server.
    Resolving,
    /// The rename input box is visible and the user is typing.
    Editing,
    /// The rename edits are being computed / applied.
    Applying,
}

/// Full state for the inline-rename feature.
#[derive(Debug, Clone)]
pub struct RenameState {
    /// Current phase.
    pub phase: RenamePhase,
    /// The position that triggered the rename.
    pub trigger_position: Option<Position>,
    /// The range of the symbol being renamed.
    pub rename_range: Option<Range>,
    /// The original symbol text before rename.
    pub original_text: String,
    /// The current text in the rename input box.
    pub new_name: String,
    /// Whether the provider supports rename at the trigger position.
    pub is_valid: bool,
    /// A placeholder hint from the provider (pre-fills the input).
    pub placeholder: Option<String>,
}

impl Default for RenameState {
    fn default() -> Self {
        Self {
            phase: RenamePhase::Idle,
            trigger_position: None,
            rename_range: None,
            original_text: String::new(),
            new_name: String::new(),
            is_valid: false,
            placeholder: None,
        }
    }
}

impl RenameState {
    /// Initiates a rename at the given position.
    pub fn start_rename(&mut self, pos: Position) {
        self.phase = RenamePhase::Resolving;
        self.trigger_position = Some(pos);
        self.original_text.clear();
        self.new_name.clear();
        self.rename_range = None;
        self.is_valid = false;
    }

    /// Called when the provider resolves the rename range and placeholder.
    pub fn resolve(&mut self, range: Range, text: String, placeholder: Option<String>) {
        self.rename_range = Some(range);
        self.original_text.clone_from(&text);
        self.new_name = placeholder.clone().unwrap_or(text);
        self.placeholder = placeholder;
        self.is_valid = true;
        self.phase = RenamePhase::Editing;
    }

    /// Called when resolution fails — cancels the rename.
    pub fn resolve_failed(&mut self) {
        self.cancel_rename();
    }

    /// Updates the new name as the user types.
    pub fn set_new_name(&mut self, name: String) {
        self.new_name = name;
    }

    /// Confirms the rename (transitions to Applying phase).
    /// Returns the new name if valid.
    pub fn apply_rename(&mut self) -> Option<String> {
        if self.phase != RenamePhase::Editing || self.new_name.is_empty() {
            return None;
        }
        if self.new_name == self.original_text {
            self.cancel_rename();
            return None;
        }
        self.phase = RenamePhase::Applying;
        Some(self.new_name.clone())
    }

    /// Cancels the rename and resets to idle.
    pub fn cancel_rename(&mut self) {
        self.phase = RenamePhase::Idle;
        self.trigger_position = None;
        self.rename_range = None;
        self.original_text.clear();
        self.new_name.clear();
        self.is_valid = false;
        self.placeholder = None;
    }

    /// Finalises after the rename edits have been applied.
    pub fn finish(&mut self) {
        self.cancel_rename();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rename_lifecycle() {
        let mut state = RenameState::default();
        state.start_rename(Position::new(3, 10));
        assert_eq!(state.phase, RenamePhase::Resolving);

        let range = Range::new(Position::new(3, 8), Position::new(3, 13));
        state.resolve(range, "hello".into(), None);
        assert_eq!(state.phase, RenamePhase::Editing);
        assert_eq!(state.new_name, "hello");

        state.set_new_name("world".into());
        let result = state.apply_rename();
        assert_eq!(result, Some("world".into()));
        assert_eq!(state.phase, RenamePhase::Applying);
    }

    #[test]
    fn rename_same_name_cancels() {
        let mut state = RenameState::default();
        state.start_rename(Position::new(0, 0));
        let range = Range::new(Position::new(0, 0), Position::new(0, 3));
        state.resolve(range, "foo".into(), None);
        let result = state.apply_rename();
        assert!(result.is_none());
        assert_eq!(state.phase, RenamePhase::Idle);
    }
}
