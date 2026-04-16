//! Keybinding resolution — maps key presses to commands using context-aware
//! matching with "when" clause evaluation.

use std::path::Path;

use anyhow::{Context, Result};

use crate::context::{evaluate, ContextKeys};
use crate::defaults::default_keybindings;
use crate::keybinding::{KeyBinding, KeyChord, KeyCombo};

/// Resolves key presses (and chords) to command identifiers by searching
/// through registered keybindings in reverse-priority order.
#[derive(Clone, Debug)]
pub struct KeybindingResolver {
    bindings: Vec<KeyBinding>,
}

impl Default for KeybindingResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl KeybindingResolver {
    /// Create a resolver pre-loaded with platform-appropriate defaults.
    pub fn new() -> Self {
        let mut resolver = Self {
            bindings: Vec::new(),
        };
        resolver.load_defaults();
        resolver
    }

    /// Load the built-in platform defaults (appends to existing bindings).
    pub fn load_defaults(&mut self) {
        self.bindings.extend(default_keybindings());
    }

    /// Load user keybindings from a JSON file. User bindings take priority
    /// over defaults (they are searched last → highest priority).
    pub fn load_user(&mut self, path: &Path) -> Result<()> {
        let contents = std::fs::read_to_string(path)
            .context("failed to read user keybindings file")?;
        let parsed: Vec<UserKeybinding> = serde_json::from_str(&contents)
            .context("failed to parse user keybindings JSON")?;

        for entry in parsed {
            let key = match KeyChord::parse(&entry.key) {
                Ok(k) => k,
                Err(e) => {
                    log::warn!("skipping invalid keybinding '{}': {e}", entry.key);
                    continue;
                }
            };
            let binding = KeyBinding {
                key,
                command: entry.command,
                when: entry.when,
                args: entry.args,
            };
            self.bindings.push(binding);
        }

        Ok(())
    }

    /// Add a single keybinding (highest priority).
    pub fn add(&mut self, binding: KeyBinding) {
        self.bindings.push(binding);
    }

    /// Resolve a single key combo to a command. Returns the command of the
    /// last matching keybinding whose "when" clause is satisfied.
    pub fn resolve<'a>(&'a self, key: &KeyCombo, context: &ContextKeys) -> Option<&'a str> {
        self.bindings
            .iter()
            .rev()
            .filter(|b| !b.key.is_chord())
            .filter(|b| b.key.parts.first() == Some(key))
            .filter(|b| Self::when_matches(b, context))
            .map(|b| b.command.as_str())
            .next()
    }

    /// Resolve a two-combo chord to a command.
    pub fn resolve_chord<'a>(
        &'a self,
        first: &KeyCombo,
        second: &KeyCombo,
        context: &ContextKeys,
    ) -> Option<&'a str> {
        self.bindings
            .iter()
            .rev()
            .filter(|b| b.key.parts.len() == 2)
            .filter(|b| b.key.parts[0] == *first && b.key.parts[1] == *second)
            .filter(|b| Self::when_matches(b, context))
            .map(|b| b.command.as_str())
            .next()
    }

    /// Check if any binding starts with this combo as the first part of a
    /// chord. Useful for knowing when to wait for a second key press.
    pub fn is_chord_prefix(&self, combo: &KeyCombo, context: &ContextKeys) -> bool {
        self.bindings
            .iter()
            .filter(|b| b.key.parts.len() >= 2)
            .filter(|b| b.key.parts[0] == *combo)
            .any(|b| Self::when_matches(b, context))
    }

    /// Return all registered bindings (read-only).
    pub fn bindings(&self) -> &[KeyBinding] {
        &self.bindings
    }

    fn when_matches(binding: &KeyBinding, context: &ContextKeys) -> bool {
        match &binding.when {
            None => true,
            Some(expr) => evaluate(expr, context),
        }
    }
}

/// Intermediate type for deserializing user `keybindings.json` entries.
#[derive(serde::Deserialize)]
struct UserKeybinding {
    key: String,
    command: String,
    #[serde(default)]
    when: Option<String>,
    #[serde(default)]
    args: Option<serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keybinding::{Key, Modifiers};

    fn primary() -> Modifiers {
        if cfg!(target_os = "macos") {
            Modifiers::META
        } else {
            Modifiers::CTRL
        }
    }

    #[test]
    fn resolve_ctrl_s() {
        let resolver = KeybindingResolver::new();
        let ctx = ContextKeys::new();
        let combo = KeyCombo::new(Key::S, primary());
        let cmd = resolver.resolve(&combo, &ctx);
        assert_eq!(cmd, Some("workbench.action.files.save"));
    }

    #[test]
    fn resolve_ctrl_c() {
        let resolver = KeybindingResolver::new();
        let ctx = ContextKeys::new();
        let combo = KeyCombo::new(Key::C, primary());
        let cmd = resolver.resolve(&combo, &ctx);
        assert_eq!(cmd, Some("editor.action.clipboardCopyAction"));
    }

    #[test]
    fn resolve_with_when_clause() {
        let resolver = KeybindingResolver::new();
        let mut ctx = ContextKeys::new();
        ctx.set_bool("editorTextFocus", true);
        let combo = KeyCombo::new(Key::Period, primary());
        let cmd = resolver.resolve(&combo, &ctx);
        assert_eq!(cmd, Some("editor.action.quickFix"));
    }

    #[test]
    fn resolve_when_clause_fails() {
        let resolver = KeybindingResolver::new();
        let ctx = ContextKeys::new();
        let combo = KeyCombo::new(Key::Period, primary());
        let cmd = resolver.resolve(&combo, &ctx);
        assert!(cmd.is_none());
    }

    #[test]
    fn resolve_chord() {
        let resolver = KeybindingResolver::new();
        let ctx = ContextKeys::new();
        let p = primary();
        let first = KeyCombo::new(Key::K, p);
        let second = KeyCombo::new(Key::C, p);
        let cmd = resolver.resolve_chord(&first, &second, &ctx);
        assert_eq!(cmd, Some("editor.action.addCommentLine"));
    }

    #[test]
    fn is_chord_prefix_true() {
        let resolver = KeybindingResolver::new();
        let ctx = ContextKeys::new();
        let combo = KeyCombo::new(Key::K, primary());
        assert!(resolver.is_chord_prefix(&combo, &ctx));
    }

    #[test]
    fn is_chord_prefix_false() {
        let resolver = KeybindingResolver::new();
        let ctx = ContextKeys::new();
        let combo = KeyCombo::new(Key::Q, Modifiers::ALT);
        assert!(!resolver.is_chord_prefix(&combo, &ctx));
    }

    #[test]
    fn user_binding_overrides_default() {
        let mut resolver = KeybindingResolver::new();
        let custom = KeyBinding::new(
            KeyChord::single(KeyCombo::new(Key::S, primary())),
            "custom.save",
        );
        resolver.add(custom);
        let ctx = ContextKeys::new();
        let combo = KeyCombo::new(Key::S, primary());
        assert_eq!(resolver.resolve(&combo, &ctx), Some("custom.save"));
    }

    #[test]
    fn no_match_returns_none() {
        let resolver = KeybindingResolver::new();
        let ctx = ContextKeys::new();
        let combo = KeyCombo::new(Key::Q, Modifiers::ALT | Modifiers::SHIFT);
        assert!(resolver.resolve(&combo, &ctx).is_none());
    }
}
