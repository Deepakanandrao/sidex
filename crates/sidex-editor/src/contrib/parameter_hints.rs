//! Signature help / parameter hints — mirrors VS Code's
//! `ParameterHintsModel` + `ParameterHintsWidget`.

use sidex_text::Position;

/// A single parameter in a signature.
#[derive(Debug, Clone)]
pub struct ParameterInfo {
    /// Display label for this parameter (e.g. `x: i32`).
    pub label: String,
    /// Optional documentation.
    pub documentation: Option<String>,
}

/// A function/method signature displayed in the parameter hints popup.
#[derive(Debug, Clone)]
pub struct SignatureInfo {
    /// The full signature label (e.g. `fn foo(x: i32, y: &str) -> bool`).
    pub label: String,
    /// Optional documentation for the function.
    pub documentation: Option<String>,
    /// The parameters of this signature.
    pub parameters: Vec<ParameterInfo>,
}

/// Full state for the parameter-hints feature.
#[derive(Debug, Clone, Default)]
pub struct ParameterHintState {
    /// Whether the hints popup is visible.
    pub is_visible: bool,
    /// Available overloaded signatures.
    pub signatures: Vec<SignatureInfo>,
    /// Index of the currently displayed signature.
    pub active_signature: usize,
    /// Index of the currently highlighted parameter.
    pub active_parameter: usize,
    /// The position at which hints were triggered.
    pub trigger_position: Option<Position>,
    /// Characters that trigger signature help (e.g. `(`, `,`).
    pub trigger_characters: Vec<char>,
    /// Characters that re-trigger after already visible (e.g. `,`).
    pub retrigger_characters: Vec<char>,
}

impl ParameterHintState {
    /// Shows parameter hints with the given signatures.
    pub fn show(&mut self, pos: Position, signatures: Vec<SignatureInfo>, active_param: usize) {
        self.is_visible = !signatures.is_empty();
        self.trigger_position = Some(pos);
        self.signatures = signatures;
        self.active_signature = 0;
        self.active_parameter = active_param;
    }

    /// Hides the parameter hints popup.
    pub fn hide(&mut self) {
        self.is_visible = false;
        self.signatures.clear();
        self.active_signature = 0;
        self.active_parameter = 0;
        self.trigger_position = None;
    }

    /// Cycles to the next overloaded signature.
    pub fn next_signature(&mut self) {
        if !self.signatures.is_empty() {
            self.active_signature = (self.active_signature + 1) % self.signatures.len();
        }
    }

    /// Cycles to the previous overloaded signature.
    pub fn prev_signature(&mut self) {
        if !self.signatures.is_empty() {
            self.active_signature = if self.active_signature == 0 {
                self.signatures.len() - 1
            } else {
                self.active_signature - 1
            };
        }
    }

    /// Updates the active parameter index (e.g. when the user types a comma).
    pub fn set_active_parameter(&mut self, idx: usize) {
        self.active_parameter = idx;
    }

    /// Returns the currently active signature, if any.
    #[must_use]
    pub fn current_signature(&self) -> Option<&SignatureInfo> {
        self.signatures.get(self.active_signature)
    }

    /// Returns the currently highlighted parameter info, if any.
    #[must_use]
    pub fn current_parameter(&self) -> Option<&ParameterInfo> {
        self.current_signature()
            .and_then(|sig| sig.parameters.get(self.active_parameter))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sig() -> SignatureInfo {
        SignatureInfo {
            label: "fn foo(a: i32, b: &str)".into(),
            documentation: None,
            parameters: vec![
                ParameterInfo { label: "a: i32".into(), documentation: None },
                ParameterInfo { label: "b: &str".into(), documentation: None },
            ],
        }
    }

    #[test]
    fn show_and_navigate() {
        let mut state = ParameterHintState::default();
        state.show(Position::new(1, 5), vec![make_sig(), make_sig()], 0);
        assert!(state.is_visible);
        assert_eq!(state.active_signature, 0);

        state.next_signature();
        assert_eq!(state.active_signature, 1);

        state.next_signature();
        assert_eq!(state.active_signature, 0); // wraps
    }

    #[test]
    fn current_parameter() {
        let mut state = ParameterHintState::default();
        state.show(Position::new(0, 0), vec![make_sig()], 1);
        let param = state.current_parameter().unwrap();
        assert_eq!(param.label, "b: &str");
    }
}
