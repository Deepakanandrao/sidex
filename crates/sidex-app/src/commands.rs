//! Built-in command registry.
//!
//! Each command is identified by a VS Code-compatible string ID. Commands are
//! registered with a human-readable label and an action callback that receives
//! mutable access to the application.

use std::collections::HashMap;

use crate::app::App;

/// Callback type for command execution.
type CommandAction = fn(&mut App);

/// A registered command with its human-readable label and action.
struct Command {
    label: String,
    action: CommandAction,
}

/// Registry of all built-in editor commands.
pub struct CommandRegistry {
    commands: HashMap<String, Command>,
    /// Recently closed editors for reopen (file paths).
    pub recently_closed: Vec<String>,
}

impl CommandRegistry {
    /// Creates a registry populated with all built-in commands.
    pub fn new() -> Self {
        let mut registry = Self {
            commands: HashMap::new(),
            recently_closed: Vec::new(),
        };
        registry.register_builtins();
        registry
    }

    /// Returns `true` if a command with the given ID exists.
    pub fn has(&self, id: &str) -> bool {
        self.commands.contains_key(id)
    }

    /// Returns the human-readable label for a command.
    pub fn label(&self, id: &str) -> Option<&str> {
        self.commands.get(id).map(|c| c.label.as_str())
    }

    /// Returns all registered command IDs (unsorted).
    pub fn ids(&self) -> Vec<&str> {
        self.commands.keys().map(String::as_str).collect()
    }

    /// Execute a command by ID against the given app.
    pub fn execute(&self, id: &str, app: &mut App) -> bool {
        if let Some(cmd) = self.commands.get(id) {
            (cmd.action)(app);
            log::debug!("executed command: {id}");
            true
        } else {
            log::warn!("unknown command: {id}");
            false
        }
    }

    /// Look up the action function pointer for a command, so it can be
    /// called after releasing the borrow on the registry.
    pub fn get_action(&self, id: &str) -> Option<CommandAction> {
        self.commands.get(id).map(|c| c.action)
    }

    fn register(&mut self, id: &str, label: &str, action: fn(&mut App)) {
        self.commands.insert(
            id.to_owned(),
            Command {
                label: label.to_owned(),
                action,
            },
        );
    }

    fn register_noop(&mut self, id: &str, label: &str) {
        self.register(id, label, |_| {});
    }

    fn register_builtins(&mut self) {
        self.register_file_commands();
        self.register_edit_commands();
        self.register_navigation_commands();
        self.register_view_commands();
        self.register_find_commands();
        self.register_terminal_commands();
        self.register_debug_commands();
        self.register_selection_commands();
    }

    // ── File commands ────────────────────────────────────────────

    fn register_file_commands(&mut self) {
        self.register(
            "workbench.action.files.newUntitledFile",
            "New File",
            |app| {
                app.new_untitled_file();
            },
        );

        self.register(
            "workbench.action.files.openFile",
            "Open File...",
            |app| {
                app.open_file_dialog();
            },
        );

        self.register(
            "workbench.action.files.save",
            "Save",
            |app| {
                app.save_active_file();
            },
        );

        self.register(
            "workbench.action.files.saveAs",
            "Save As...",
            |app| {
                app.save_active_file_as();
            },
        );

        self.register(
            "workbench.action.files.saveAll",
            "Save All",
            |app| {
                app.save_all_files();
            },
        );

        self.register(
            "workbench.action.closeActiveEditor",
            "Close Editor",
            |app| {
                app.close_active_editor();
            },
        );

        self.register(
            "workbench.action.closeAllEditors",
            "Close All Editors",
            |app| {
                app.close_all_editors();
            },
        );

        self.register(
            "workbench.action.reopenClosedEditor",
            "Reopen Closed Editor",
            |app| {
                app.reopen_closed_editor();
            },
        );
    }

    // ── Edit commands ────────────────────────────────────────────

    fn register_edit_commands(&mut self) {
        self.register("editor.action.undo", "Undo", |app| {
            if let Some(doc) = app.active_document_mut() {
                doc.document.undo();
                doc.on_edit();
            }
        });

        self.register("editor.action.redo", "Redo", |app| {
            if let Some(doc) = app.active_document_mut() {
                doc.document.redo();
                doc.on_edit();
            }
        });

        self.register(
            "editor.action.clipboardCutAction",
            "Cut",
            |app| {
                app.clipboard_cut();
            },
        );

        self.register(
            "editor.action.clipboardCopyAction",
            "Copy",
            |app| {
                app.clipboard_copy();
            },
        );

        self.register(
            "editor.action.clipboardPasteAction",
            "Paste",
            |app| {
                app.clipboard_paste();
            },
        );

        self.register("editor.action.selectAll", "Select All", |app| {
            if let Some(doc) = app.active_document_mut() {
                doc.document.select_all();
            }
        });

        self.register("editor.action.commentLine", "Toggle Line Comment", |app| {
            let comment_prefix = app.active_comment_prefix();
            if let Some(doc) = app.active_document_mut() {
                doc.document.toggle_line_comment(&comment_prefix);
                doc.on_edit();
            }
        });

        self.register("editor.action.blockComment", "Toggle Block Comment", |app| {
            let (open, close) = app.active_block_comment();
            if let Some(doc) = app.active_document_mut() {
                doc.document.toggle_block_comment(&open, &close);
                doc.on_edit();
            }
        });

        self.register("editor.action.indentLines", "Indent Lines", |app| {
            if let Some(doc) = app.active_document_mut() {
                doc.document.indent();
                doc.on_edit();
            }
        });

        self.register("editor.action.outdentLines", "Outdent Lines", |app| {
            if let Some(doc) = app.active_document_mut() {
                doc.document.outdent();
                doc.on_edit();
            }
        });

        self.register(
            "editor.action.moveLinesUpAction",
            "Move Lines Up",
            |app| {
                if let Some(doc) = app.active_document_mut() {
                    doc.document.move_line_up();
                    doc.on_edit();
                }
            },
        );

        self.register(
            "editor.action.moveLinesDownAction",
            "Move Lines Down",
            |app| {
                if let Some(doc) = app.active_document_mut() {
                    doc.document.move_line_down();
                    doc.on_edit();
                }
            },
        );

        self.register(
            "editor.action.copyLinesUpAction",
            "Copy Lines Up",
            |app| {
                if let Some(doc) = app.active_document_mut() {
                    doc.document.copy_line_up();
                    doc.on_edit();
                }
            },
        );

        self.register(
            "editor.action.copyLinesDownAction",
            "Copy Lines Down",
            |app| {
                if let Some(doc) = app.active_document_mut() {
                    doc.document.copy_line_down();
                    doc.on_edit();
                }
            },
        );

        self.register("editor.action.deleteLines", "Delete Lines", |app| {
            if let Some(doc) = app.active_document_mut() {
                doc.document.delete_line();
                doc.on_edit();
            }
        });

        self.register("editor.action.joinLines", "Join Lines", |app| {
            if let Some(doc) = app.active_document_mut() {
                doc.document.join_lines();
                doc.on_edit();
            }
        });

        self.register(
            "editor.action.sortLinesAscending",
            "Sort Lines Ascending",
            |app| {
                if let Some(doc) = app.active_document_mut() {
                    doc.document.sort_lines_ascending();
                    doc.on_edit();
                }
            },
        );

        self.register(
            "editor.action.sortLinesDescending",
            "Sort Lines Descending",
            |app| {
                if let Some(doc) = app.active_document_mut() {
                    doc.document.sort_lines_descending();
                    doc.on_edit();
                }
            },
        );

        self.register(
            "editor.action.trimTrailingWhitespace",
            "Trim Trailing Whitespace",
            |app| {
                if let Some(doc) = app.active_document_mut() {
                    doc.document.trim_trailing_whitespace();
                    doc.on_edit();
                }
            },
        );

        self.register(
            "editor.action.transformToUppercase",
            "Transform to Uppercase",
            |app| {
                if let Some(doc) = app.active_document_mut() {
                    doc.document.transform_to_uppercase();
                    doc.on_edit();
                }
            },
        );

        self.register(
            "editor.action.transformToLowercase",
            "Transform to Lowercase",
            |app| {
                if let Some(doc) = app.active_document_mut() {
                    doc.document.transform_to_lowercase();
                    doc.on_edit();
                }
            },
        );

        self.register(
            "editor.action.insertLineAfter",
            "Insert Line Below",
            |app| {
                if let Some(doc) = app.active_document_mut() {
                    doc.document.insert_line_below();
                    doc.on_edit();
                }
            },
        );

        self.register(
            "editor.action.insertLineBefore",
            "Insert Line Above",
            |app| {
                if let Some(doc) = app.active_document_mut() {
                    doc.document.insert_line_above();
                    doc.on_edit();
                }
            },
        );

        self.register(
            "editor.action.transposeLetters",
            "Transpose Characters",
            |app| {
                if let Some(doc) = app.active_document_mut() {
                    doc.document.transpose_characters();
                    doc.on_edit();
                }
            },
        );
    }

    // ── Navigation commands ──────────────────────────────────────

    fn register_navigation_commands(&mut self) {
        self.register(
            "workbench.action.quickOpen",
            "Go to File...",
            |app| {
                app.show_quick_open = true;
            },
        );

        self.register(
            "workbench.action.showCommands",
            "Command Palette...",
            |app| {
                app.show_command_palette = true;
            },
        );

        self.register(
            "workbench.action.gotoLine",
            "Go to Line...",
            |app| {
                app.show_goto_line = true;
            },
        );

        self.register_noop("editor.action.goToDeclaration", "Go to Declaration");
        self.register_noop("editor.action.goToImplementation", "Go to Implementation");
        self.register_noop("editor.action.goToReferences", "Go to References");
        self.register_noop("editor.action.revealDefinition", "Go to Definition");
        self.register_noop("workbench.action.gotoSymbol", "Go to Symbol in Editor...");

        self.register(
            "workbench.action.navigateBack",
            "Go Back",
            |app| {
                if app.navigation_stack_back.is_empty() {
                    return;
                }
                let entry = app.navigation_stack_back.pop().unwrap();
                app.navigation_stack_forward.push(NavigationEntry {
                    doc_index: app.active_document,
                    line: app.active_document_ref().map_or(0, |d| {
                        d.document.cursors.primary().position().line
                    }),
                });
                app.active_document = entry.doc_index;
            },
        );

        self.register(
            "workbench.action.navigateForward",
            "Go Forward",
            |app| {
                if app.navigation_stack_forward.is_empty() {
                    return;
                }
                let entry = app.navigation_stack_forward.pop().unwrap();
                app.navigation_stack_back.push(NavigationEntry {
                    doc_index: app.active_document,
                    line: app.active_document_ref().map_or(0, |d| {
                        d.document.cursors.primary().position().line
                    }),
                });
                app.active_document = entry.doc_index;
            },
        );
    }

    // ── View commands ────────────────────────────────────────────

    fn register_view_commands(&mut self) {
        self.register(
            "workbench.action.toggleSidebarVisibility",
            "Toggle Sidebar",
            |app| {
                app.layout.sidebar_visible = !app.layout.sidebar_visible;
                app.needs_relayout = true;
            },
        );

        self.register(
            "workbench.action.togglePanel",
            "Toggle Panel",
            |app| {
                app.layout.panel_visible = !app.layout.panel_visible;
                app.needs_relayout = true;
            },
        );

        self.register(
            "workbench.action.terminal.toggleTerminal",
            "Toggle Terminal",
            |app| {
                app.layout.panel_visible = !app.layout.panel_visible;
                app.needs_relayout = true;
            },
        );

        self.register("workbench.action.zoomIn", "Zoom In", |app| {
            app.zoom_level = (app.zoom_level + 1).min(10);
        });

        self.register("workbench.action.zoomOut", "Zoom Out", |app| {
            app.zoom_level = (app.zoom_level - 1).max(-5);
        });

        self.register("workbench.action.zoomReset", "Reset Zoom", |app| {
            app.zoom_level = 0;
        });

        self.register_noop("workbench.action.toggleFullScreen", "Toggle Full Screen");

        self.register_noop("workbench.action.splitEditor", "Split Editor Right");
        self.register_noop("workbench.action.splitEditorDown", "Split Editor Down");
    }

    // ── Find commands ────────────────────────────────────────────

    fn register_find_commands(&mut self) {
        self.register("actions.find", "Find", |app| {
            app.show_find_widget = true;
        });

        self.register(
            "editor.action.startFindReplaceAction",
            "Find and Replace",
            |app| {
                app.show_find_widget = true;
                app.find_replace_mode = true;
            },
        );

        self.register(
            "workbench.action.findInFiles",
            "Search in Files",
            |app| {
                app.show_search_panel = true;
            },
        );
    }

    // ── Terminal commands ────────────────────────────────────────

    fn register_terminal_commands(&mut self) {
        self.register("workbench.action.terminal.new", "New Terminal", |app| {
            if let Err(e) = app.terminal_manager.create(None, None) {
                log::error!("failed to create terminal: {e}");
            }
            app.layout.panel_visible = true;
            app.needs_relayout = true;
        });

        self.register("workbench.action.terminal.split", "Split Terminal", |app| {
            if let Err(e) = app.terminal_manager.create(None, None) {
                log::error!("failed to create terminal: {e}");
            }
        });

        self.register("workbench.action.terminal.kill", "Kill Terminal", |app| {
            let ids = app.terminal_manager.list();
            if let Some(last_id) = ids.last() {
                if let Err(e) = app.terminal_manager.remove(*last_id) {
                    log::error!("failed to kill terminal: {e}");
                }
            }
        });
    }

    // ── Debug commands ───────────────────────────────────────────

    fn register_debug_commands(&mut self) {
        self.register_noop("workbench.action.debug.start", "Start Debugging");
        self.register_noop("workbench.action.debug.stop", "Stop Debugging");
        self.register_noop("workbench.action.debug.restart", "Restart Debugging");
        self.register_noop("editor.debug.action.toggleBreakpoint", "Toggle Breakpoint");
        self.register_noop("workbench.action.debug.stepOver", "Step Over");
        self.register_noop("workbench.action.debug.stepInto", "Step Into");
        self.register_noop("workbench.action.debug.stepOut", "Step Out");
        self.register_noop("workbench.action.debug.continue", "Continue");
    }

    // ── Selection commands ───────────────────────────────────────

    fn register_selection_commands(&mut self) {
        self.register(
            "editor.action.smartSelect.expand",
            "Expand Selection",
            |app| {
                if let Some(doc) = app.active_document_mut() {
                    doc.document.smart_select_grow();
                }
            },
        );

        self.register(
            "editor.action.smartSelect.shrink",
            "Shrink Selection",
            |app| {
                if let Some(doc) = app.active_document_mut() {
                    doc.document.smart_select_shrink();
                }
            },
        );

        self.register(
            "editor.action.selectHighlights",
            "Add Cursors to Line Selections",
            |app| {
                if let Some(doc) = app.active_document_mut() {
                    doc.document.add_cursor_at_each_selection_line();
                }
            },
        );

        self.register("editor.action.wordWrap", "Toggle Word Wrap", |app| {
            if let Some(doc) = app.active_document_mut() {
                doc.document.toggle_word_wrap();
            }
        });
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Entry in the navigation history stack for back/forward.
#[derive(Debug, Clone)]
pub struct NavigationEntry {
    pub doc_index: usize,
    pub line: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtins_are_registered() {
        let reg = CommandRegistry::new();
        assert!(reg.has("workbench.action.files.save"));
        assert!(reg.has("editor.action.undo"));
        assert!(reg.has("workbench.action.terminal.new"));
    }

    #[test]
    fn label_lookup() {
        let reg = CommandRegistry::new();
        assert_eq!(reg.label("workbench.action.files.save"), Some("Save"));
    }

    #[test]
    fn missing_command() {
        let reg = CommandRegistry::new();
        assert!(!reg.has("nonexistent.command"));
        assert!(reg.label("nonexistent.command").is_none());
    }

    #[test]
    fn all_ids_nonempty() {
        let reg = CommandRegistry::new();
        assert!(reg.ids().len() >= 40);
    }

    #[test]
    fn file_commands_registered() {
        let reg = CommandRegistry::new();
        assert!(reg.has("workbench.action.files.newUntitledFile"));
        assert!(reg.has("workbench.action.files.openFile"));
        assert!(reg.has("workbench.action.files.saveAs"));
        assert!(reg.has("workbench.action.files.saveAll"));
        assert!(reg.has("workbench.action.closeActiveEditor"));
        assert!(reg.has("workbench.action.closeAllEditors"));
        assert!(reg.has("workbench.action.reopenClosedEditor"));
    }

    #[test]
    fn edit_commands_registered() {
        let reg = CommandRegistry::new();
        assert!(reg.has("editor.action.clipboardCutAction"));
        assert!(reg.has("editor.action.clipboardCopyAction"));
        assert!(reg.has("editor.action.clipboardPasteAction"));
        assert!(reg.has("editor.action.commentLine"));
        assert!(reg.has("editor.action.blockComment"));
        assert!(reg.has("editor.action.indentLines"));
        assert!(reg.has("editor.action.outdentLines"));
        assert!(reg.has("editor.action.moveLinesUpAction"));
        assert!(reg.has("editor.action.moveLinesDownAction"));
        assert!(reg.has("editor.action.copyLinesUpAction"));
        assert!(reg.has("editor.action.copyLinesDownAction"));
        assert!(reg.has("editor.action.deleteLines"));
        assert!(reg.has("editor.action.joinLines"));
        assert!(reg.has("editor.action.sortLinesAscending"));
        assert!(reg.has("editor.action.sortLinesDescending"));
        assert!(reg.has("editor.action.trimTrailingWhitespace"));
        assert!(reg.has("editor.action.transformToUppercase"));
        assert!(reg.has("editor.action.transformToLowercase"));
    }

    #[test]
    fn navigation_commands_registered() {
        let reg = CommandRegistry::new();
        assert!(reg.has("workbench.action.quickOpen"));
        assert!(reg.has("workbench.action.showCommands"));
        assert!(reg.has("workbench.action.gotoLine"));
        assert!(reg.has("editor.action.goToDeclaration"));
        assert!(reg.has("editor.action.goToImplementation"));
        assert!(reg.has("editor.action.goToReferences"));
        assert!(reg.has("workbench.action.navigateBack"));
        assert!(reg.has("workbench.action.navigateForward"));
    }

    #[test]
    fn view_commands_registered() {
        let reg = CommandRegistry::new();
        assert!(reg.has("workbench.action.toggleSidebarVisibility"));
        assert!(reg.has("workbench.action.togglePanel"));
        assert!(reg.has("workbench.action.terminal.toggleTerminal"));
        assert!(reg.has("workbench.action.zoomIn"));
        assert!(reg.has("workbench.action.zoomOut"));
        assert!(reg.has("workbench.action.zoomReset"));
    }

    #[test]
    fn find_commands_registered() {
        let reg = CommandRegistry::new();
        assert!(reg.has("actions.find"));
        assert!(reg.has("editor.action.startFindReplaceAction"));
        assert!(reg.has("workbench.action.findInFiles"));
    }

    #[test]
    fn terminal_commands_registered() {
        let reg = CommandRegistry::new();
        assert!(reg.has("workbench.action.terminal.new"));
        assert!(reg.has("workbench.action.terminal.split"));
        assert!(reg.has("workbench.action.terminal.kill"));
    }
}
