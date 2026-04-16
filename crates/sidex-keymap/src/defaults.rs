//! Platform-aware default keybindings matching VS Code conventions.

use crate::keybinding::{Key, KeyBinding, KeyChord, KeyCombo, Modifiers};

/// Returns whether the current platform is macOS.
fn is_macos() -> bool {
    cfg!(target_os = "macos")
}

/// The primary modifier: Cmd on macOS, Ctrl elsewhere.
fn primary() -> Modifiers {
    if is_macos() {
        Modifiers::META
    } else {
        Modifiers::CTRL
    }
}

/// Primary + Shift.
fn primary_shift() -> Modifiers {
    primary() | Modifiers::SHIFT
}

/// Convenience: single-combo keybinding.
fn bind(modifiers: Modifiers, key: Key, command: &str) -> KeyBinding {
    KeyBinding::new(
        KeyChord::single(KeyCombo::new(key, modifiers)),
        command,
    )
}

/// Convenience: single-combo keybinding with a "when" clause.
fn bind_when(modifiers: Modifiers, key: Key, command: &str, when: &str) -> KeyBinding {
    KeyBinding::new(
        KeyChord::single(KeyCombo::new(key, modifiers)),
        command,
    )
    .with_when(when)
}

/// Convenience: two-combo chord keybinding.
fn chord(
    m1: Modifiers,
    k1: Key,
    m2: Modifiers,
    k2: Key,
    command: &str,
) -> KeyBinding {
    KeyBinding::new(
        KeyChord::double(KeyCombo::new(k1, m1), KeyCombo::new(k2, m2)),
        command,
    )
}

/// Return the full set of default keybindings, adapted for the current
/// platform.
pub fn default_keybindings() -> Vec<KeyBinding> {
    let p = primary();
    let ps = primary_shift();

    vec![
        // ── Clipboard ────────────────────────────────────────────────────
        bind(p, Key::C, "editor.action.clipboardCopyAction"),
        bind(p, Key::X, "editor.action.clipboardCutAction"),
        bind(p, Key::V, "editor.action.clipboardPasteAction"),

        // ── Undo / redo ──────────────────────────────────────────────────
        bind(p, Key::Z, "undo"),
        bind(ps, Key::Z, "redo"),

        // ── File operations ──────────────────────────────────────────────
        bind(p, Key::S, "workbench.action.files.save"),
        bind(ps, Key::S, "workbench.action.files.saveAs"),
        bind(p, Key::N, "workbench.action.files.newUntitledFile"),
        bind(p, Key::O, "workbench.action.files.openFile"),
        bind(p, Key::W, "workbench.action.closeActiveEditor"),

        // ── Quick open / command palette ─────────────────────────────────
        bind(p, Key::P, "workbench.action.quickOpen"),
        bind(ps, Key::P, "workbench.action.showCommands"),
        bind(ps, Key::N, "workbench.action.newWindow"),

        // ── Find / replace ───────────────────────────────────────────────
        bind(p, Key::F, "actions.find"),
        bind(p, Key::H, "editor.action.startFindReplaceAction"),
        bind(ps, Key::F, "workbench.action.findInFiles"),
        bind(ps, Key::H, "workbench.action.replaceInFiles"),

        // ── Editor navigation ────────────────────────────────────────────
        bind(p, Key::G, "workbench.action.gotoLine"),
        bind_when(p, Key::BracketLeft, "editor.action.outdentLines", "editorTextFocus"),
        bind_when(p, Key::BracketRight, "editor.action.indentLines", "editorTextFocus"),

        // ── Selection ────────────────────────────────────────────────────
        bind(p, Key::A, "editor.action.selectAll"),
        bind(p, Key::D, "editor.action.addSelectionToNextFindMatch"),
        bind(p, Key::L, "expandLineSelection"),
        bind(ps, Key::L, "editor.action.selectHighlights"),
        bind(ps, Key::K, "editor.action.deleteLines"),

        // ── Line manipulation ────────────────────────────────────────────
        bind(Modifiers::ALT | Modifiers::SHIFT, Key::ArrowDown, "editor.action.copyLinesDownAction"),
        bind(Modifiers::ALT | Modifiers::SHIFT, Key::ArrowUp, "editor.action.copyLinesUpAction"),
        bind(Modifiers::ALT, Key::ArrowDown, "editor.action.moveLinesDownAction"),
        bind(Modifiers::ALT, Key::ArrowUp, "editor.action.moveLinesUpAction"),

        // ── Multi-cursor ─────────────────────────────────────────────────
        bind(Modifiers::ALT | Modifiers::CTRL, Key::ArrowUp, "editor.action.insertCursorAbove"),
        bind(Modifiers::ALT | Modifiers::CTRL, Key::ArrowDown, "editor.action.insertCursorBelow"),

        // ── View ─────────────────────────────────────────────────────────
        bind(p, Key::Equal, "workbench.action.zoomIn"),
        bind(p, Key::Minus, "workbench.action.zoomOut"),
        bind(p | Modifiers::SHIFT, Key::Digit0, "workbench.action.zoomReset"),
        bind(p, Key::B, "workbench.action.toggleSidebarVisibility"),
        bind(p, Key::J, "workbench.action.togglePanel"),
        bind(p, Key::Backquote, "workbench.action.terminal.toggleTerminal"),

        // ── Tabs ─────────────────────────────────────────────────────────
        bind(p, Key::Tab, "workbench.action.nextEditor"),
        bind(ps, Key::Tab, "workbench.action.previousEditor"),
        bind(p, Key::Digit1, "workbench.action.openEditorAtIndex1"),
        bind(p, Key::Digit2, "workbench.action.openEditorAtIndex2"),
        bind(p, Key::Digit3, "workbench.action.openEditorAtIndex3"),
        bind(p, Key::Digit4, "workbench.action.openEditorAtIndex4"),
        bind(p, Key::Digit5, "workbench.action.openEditorAtIndex5"),
        bind(p, Key::Digit6, "workbench.action.openEditorAtIndex6"),
        bind(p, Key::Digit7, "workbench.action.openEditorAtIndex7"),
        bind(p, Key::Digit8, "workbench.action.openEditorAtIndex8"),
        bind(p, Key::Digit9, "workbench.action.openEditorAtIndex9"),

        // ── Code actions ─────────────────────────────────────────────────
        bind_when(p, Key::Period, "editor.action.quickFix", "editorTextFocus"),
        bind_when(Modifiers::NONE, Key::F2, "editor.action.rename", "editorTextFocus"),
        bind_when(Modifiers::NONE, Key::F12, "editor.action.revealDefinition", "editorTextFocus"),
        bind_when(Modifiers::ALT, Key::F12, "editor.action.peekDefinition", "editorTextFocus"),
        bind_when(Modifiers::SHIFT, Key::F12, "editor.action.goToReferences", "editorTextFocus"),
        bind_when(p, Key::Slash, "editor.action.commentLine", "editorTextFocus"),
        bind_when(ps, Key::Slash, "editor.action.blockComment", "editorTextFocus"),

        // ── Formatting ───────────────────────────────────────────────────
        bind_when(ps, Key::I, "editor.action.formatDocument", "editorTextFocus"),

        // ── Folding ──────────────────────────────────────────────────────
        chord(p, Key::K, p, Key::Digit0, "editor.unfoldAll"),
        chord(p, Key::K, p, Key::J, "editor.unfoldAll"),
        chord(p, Key::K, p, Key::C, "editor.action.addCommentLine"),
        chord(p, Key::K, p, Key::U, "editor.action.removeCommentLine"),

        // ── Splits ───────────────────────────────────────────────────────
        bind(p, Key::Backslash, "workbench.action.splitEditor"),

        // ── Debug ────────────────────────────────────────────────────────
        bind(Modifiers::NONE, Key::F5, "workbench.action.debug.start"),
        bind(Modifiers::SHIFT, Key::F5, "workbench.action.debug.stop"),
        bind(Modifiers::NONE, Key::F9, "editor.debug.action.toggleBreakpoint"),
        bind(Modifiers::NONE, Key::F10, "workbench.action.debug.stepOver"),
        bind(Modifiers::NONE, Key::F11, "workbench.action.debug.stepInto"),
        bind(Modifiers::SHIFT, Key::F11, "workbench.action.debug.stepOut"),

        // ── Terminal ─────────────────────────────────────────────────────
        bind(ps, Key::Backquote, "workbench.action.terminal.new"),

        // ── Misc ─────────────────────────────────────────────────────────
        bind(p, Key::Comma, "workbench.action.openSettings"),
        bind(p | Modifiers::SHIFT, Key::X, "workbench.view.extensions"),
        bind_when(Modifiers::NONE, Key::Escape, "workbench.action.closeQuickOpen", "inQuickOpen"),
        bind_when(Modifiers::NONE, Key::Escape, "cancelSelection", "editorHasSelection"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_many_defaults() {
        let bindings = default_keybindings();
        assert!(bindings.len() >= 50, "expected 50+ defaults, got {}", bindings.len());
    }

    #[test]
    fn ctrl_or_cmd_s_is_save() {
        let bindings = default_keybindings();
        let save = bindings
            .iter()
            .find(|b| b.command == "workbench.action.files.save")
            .unwrap();
        assert_eq!(save.key.parts[0].key, Key::S);
    }

    #[test]
    fn chord_bindings_present() {
        let bindings = default_keybindings();
        let chord_count = bindings.iter().filter(|b| b.key.is_chord()).count();
        assert!(chord_count >= 3, "expected chord bindings");
    }

    #[test]
    fn when_clauses_present() {
        let bindings = default_keybindings();
        let when_count = bindings.iter().filter(|b| b.when.is_some()).count();
        assert!(when_count >= 5, "expected when clauses");
    }

    #[test]
    fn primary_is_platform_appropriate() {
        let p = primary();
        if cfg!(target_os = "macos") {
            assert!(p.contains(Modifiers::META));
        } else {
            assert!(p.contains(Modifiers::CTRL));
        }
    }
}
