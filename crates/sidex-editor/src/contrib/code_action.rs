//! Code actions (quick fixes, refactorings) — mirrors VS Code's
//! `CodeActionController` + `CodeActionModel` + light-bulb logic.

use sidex_text::Range;

/// The kind of a code action, following LSP's `CodeActionKind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeActionKind {
    QuickFix,
    Refactor,
    RefactorExtract,
    RefactorInline,
    RefactorRewrite,
    Source,
    SourceOrganizeImports,
    SourceFixAll,
    Other(String),
}

/// A single code action returned by the language server.
#[derive(Debug, Clone)]
pub struct CodeAction {
    /// Human-readable title.
    pub title: String,
    /// The kind of code action.
    pub kind: Option<CodeActionKind>,
    /// Whether this is the preferred action for a given diagnostic.
    pub is_preferred: bool,
    /// Whether this action is disabled (with a reason).
    pub disabled_reason: Option<String>,
    /// Opaque data passed back to the server on apply.
    pub data: Option<String>,
}

/// Light-bulb indicator state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub enum LightBulbVisibility {
    #[default]
    Hidden,
    QuickFix,
    Refactor,
}

/// Full state for the code-action feature.
#[derive(Debug, Clone, Default)]
pub struct CodeActionState {
    /// Available code actions for the current cursor position / selection.
    pub actions: Vec<CodeAction>,
    /// The range for which actions were computed.
    pub trigger_range: Option<Range>,
    /// Whether actions are being fetched.
    pub is_loading: bool,
    /// Light-bulb visibility in the gutter.
    pub light_bulb: LightBulbVisibility,
    /// The line where the light bulb is shown.
    pub light_bulb_line: Option<u32>,
}


impl CodeActionState {
    /// Starts fetching code actions for the given range.
    pub fn request_code_actions(&mut self, range: Range) {
        self.trigger_range = Some(range);
        self.is_loading = true;
        self.actions.clear();
        self.light_bulb = LightBulbVisibility::Hidden;
    }

    /// Receives code actions from the provider and updates light-bulb state.
    pub fn receive_actions(&mut self, actions: Vec<CodeAction>) {
        self.is_loading = false;
        self.actions = actions;
        self.update_light_bulb();
    }

    /// Clears current actions.
    pub fn clear(&mut self) {
        self.actions.clear();
        self.is_loading = false;
        self.light_bulb = LightBulbVisibility::Hidden;
        self.light_bulb_line = None;
        self.trigger_range = None;
    }

    /// Returns the preferred action, if one exists.
    #[must_use]
    pub fn preferred_action(&self) -> Option<&CodeAction> {
        self.actions.iter().find(|a| a.is_preferred)
    }

    /// Returns only quick-fix actions.
    #[must_use]
    pub fn quick_fixes(&self) -> Vec<&CodeAction> {
        self.actions
            .iter()
            .filter(|a| matches!(a.kind, Some(CodeActionKind::QuickFix)))
            .collect()
    }

    /// Returns only refactoring actions.
    #[must_use]
    pub fn refactorings(&self) -> Vec<&CodeAction> {
        self.actions
            .iter()
            .filter(|a| {
                matches!(
                    a.kind,
                    Some(CodeActionKind::Refactor
                        | CodeActionKind::RefactorExtract
                        | CodeActionKind::RefactorInline
                        | CodeActionKind::RefactorRewrite)
                )
            })
            .collect()
    }

    fn update_light_bulb(&mut self) {
        if self.actions.is_empty() {
            self.light_bulb = LightBulbVisibility::Hidden;
            self.light_bulb_line = None;
            return;
        }

        let has_quickfix = self.actions.iter().any(|a| matches!(a.kind, Some(CodeActionKind::QuickFix)));
        self.light_bulb = if has_quickfix {
            LightBulbVisibility::QuickFix
        } else {
            LightBulbVisibility::Refactor
        };
        self.light_bulb_line = self.trigger_range.map(|r| r.start.line);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sidex_text::Position;

    #[test]
    fn light_bulb_shows_for_quickfix() {
        let mut state = CodeActionState::default();
        let range = Range::new(Position::new(5, 0), Position::new(5, 10));
        state.request_code_actions(range);
        state.receive_actions(vec![CodeAction {
            title: "Fix import".into(),
            kind: Some(CodeActionKind::QuickFix),
            is_preferred: true,
            disabled_reason: None,
            data: None,
        }]);
        assert_eq!(state.light_bulb, LightBulbVisibility::QuickFix);
        assert_eq!(state.light_bulb_line, Some(5));
    }
}
