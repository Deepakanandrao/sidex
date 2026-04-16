//! Application state — owns all subsystems and wires them together.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use winit::window::Window;

use sidex_db::Database;
use sidex_extension_api::CommandRegistry as ExtCommandRegistry;
use sidex_extensions::ExtensionRegistry;
use sidex_gpu::GpuRenderer;
use sidex_keymap::{ContextKeys, KeybindingResolver};
use sidex_lsp::{DiagnosticCollection, LspClient, ServerRegistry};
use sidex_remote::RemoteManager;
use sidex_settings::Settings;
use sidex_syntax::LanguageRegistry;
use sidex_terminal::TerminalManager;
use sidex_theme::Theme;
use sidex_ui::Workbench;
use sidex_workspace::Workspace;

use crate::clipboard;
use crate::commands::{CommandRegistry, NavigationEntry};
use crate::document_state::DocumentState;
use crate::file_dialog;
use crate::layout::{Layout, LayoutRects};

/// The central application struct holding every subsystem.
pub struct App {
    // ── Rendering ────────────────────────────────────────────────
    pub renderer: GpuRenderer,

    // ── Documents (open files) ───────────────────────────────────
    pub documents: Vec<DocumentState>,
    pub active_document: usize,

    // ── Language ─────────────────────────────────────────────────
    pub language_registry: LanguageRegistry,
    pub lsp_clients: HashMap<String, LspClientEntry>,
    pub server_registry: ServerRegistry,
    pub diagnostics: DiagnosticCollection,

    // ── Workspace ────────────────────────────────────────────────
    pub workspace: Option<Workspace>,

    // ── Terminal ─────────────────────────────────────────────────
    pub terminal_manager: TerminalManager,

    // ── Extensions ───────────────────────────────────────────────
    pub extension_registry: ExtensionRegistry,
    pub ext_command_registry: ExtCommandRegistry,

    // ── Configuration ────────────────────────────────────────────
    pub settings: Settings,
    pub theme: Theme,
    pub keymap: KeybindingResolver,
    pub context_keys: ContextKeys,

    // ── State persistence ────────────────────────────────────────
    pub db: Database,

    // ── Remote ───────────────────────────────────────────────────
    pub remote_manager: RemoteManager,

    // ── UI ───────────────────────────────────────────────────────
    pub workbench: Workbench,
    pub commands: CommandRegistry,
    pub layout: Layout,
    pub layout_rects: LayoutRects,

    // ── Navigation ───────────────────────────────────────────────
    pub navigation_stack_back: Vec<NavigationEntry>,
    pub navigation_stack_forward: Vec<NavigationEntry>,

    // ── UI state flags ───────────────────────────────────────────
    pub show_quick_open: bool,
    pub show_command_palette: bool,
    pub show_goto_line: bool,
    pub show_find_widget: bool,
    pub find_replace_mode: bool,
    pub show_search_panel: bool,
    pub zoom_level: i32,
    pub needs_relayout: bool,
    pub needs_render: bool,

    // ── Internal ─────────────────────────────────────────────────
    window: Arc<Window>,
    cursor_visible: bool,
    cursor_blink_timer: std::time::Instant,

    /// Partial chord state: if the user pressed the first key of a
    /// two-key chord, this holds that combo so the resolver can
    /// match the second key.
    pub pending_chord: Option<sidex_keymap::KeyCombo>,
}

/// Wrapper for an LSP client associated with a language.
pub struct LspClientEntry {
    pub client: LspClient,
    pub language_id: String,
}

impl App {
    /// Initialise every subsystem and optionally open a workspace path.
    pub async fn new(window: Arc<Window>, open_path: Option<&Path>) -> Result<Self> {
        let renderer = GpuRenderer::new(window.clone())
            .await
            .context("GPU initialisation failed")?;

        let mut settings = Settings::new();
        if let Some(user_settings) = user_settings_path() {
            if user_settings.exists() {
                if let Err(e) = settings.load_user(&user_settings) {
                    log::warn!("failed to load user settings: {e}");
                }
            }
        }

        let theme = Theme::default_dark();
        let mut keymap = KeybindingResolver::new();
        if let Some(kb_path) = user_keybindings_path() {
            if kb_path.exists() {
                if let Err(e) = keymap.load_user(&kb_path) {
                    log::warn!("failed to load user keybindings: {e}");
                }
            }
        }

        let mut context_keys = ContextKeys::new();
        context_keys.set_bool("editorTextFocus", true);
        context_keys.set_bool("editorHasSelection", false);
        context_keys.set_bool("editorReadonly", false);
        context_keys.set_bool("terminalFocus", false);
        context_keys.set_bool("sideBarVisible", true);
        context_keys.set_bool("panelVisible", true);
        context_keys.set_string("editorLangId", "plaintext");

        let workspace = open_path.map(Workspace::open);

        let db = Database::open_default().unwrap_or_else(|e| {
            log::warn!("failed to open state db, using temp: {e}");
            let tmp = std::env::temp_dir().join("sidex-fallback.db");
            Database::open(&tmp).expect("fallback db must open")
        });

        if let Some(state) = sidex_db::load_window_state(&db).ok().flatten() {
            log::debug!(
                "restored window state: {}x{} at ({}, {})",
                state.width,
                state.height,
                state.x,
                state.y,
            );
        }

        let terminal_manager = TerminalManager::new();
        let commands = CommandRegistry::new();
        let language_registry = LanguageRegistry::new();
        let server_registry = ServerRegistry::new();
        let extension_registry = ExtensionRegistry::new();
        let ext_command_registry = ExtCommandRegistry::new();
        let remote_manager = RemoteManager::new();
        let diagnostics = DiagnosticCollection::new();
        let workbench = Workbench::new(&theme);

        let (w, h) = renderer.surface_size();
        let layout = Layout::default();
        let layout_rects = layout.compute(w, h);

        let mut app = Self {
            renderer,
            documents: Vec::new(),
            active_document: 0,
            language_registry,
            lsp_clients: HashMap::new(),
            server_registry,
            diagnostics,
            workspace,
            terminal_manager,
            extension_registry,
            ext_command_registry,
            settings,
            theme,
            keymap,
            context_keys,
            db,
            remote_manager,
            workbench,
            commands,
            layout,
            layout_rects,
            navigation_stack_back: Vec::new(),
            navigation_stack_forward: Vec::new(),
            show_quick_open: false,
            show_command_palette: false,
            show_goto_line: false,
            show_find_widget: false,
            find_replace_mode: false,
            show_search_panel: false,
            zoom_level: 0,
            needs_relayout: false,
            needs_render: true,
            window,
            cursor_visible: true,
            cursor_blink_timer: std::time::Instant::now(),
            pending_chord: None,
        };

        if app.documents.is_empty() {
            app.documents.push(DocumentState::new_untitled());
        }

        Ok(app)
    }

    // ── Document access helpers ──────────────────────────────────

    /// Mutable reference to the active document state, if any.
    pub fn active_document_mut(&mut self) -> Option<&mut DocumentState> {
        self.documents.get_mut(self.active_document)
    }

    /// Immutable reference to the active document state, if any.
    pub fn active_document_ref(&self) -> Option<&DocumentState> {
        self.documents.get(self.active_document)
    }

    /// Returns the line comment prefix for the active document's language.
    pub fn active_comment_prefix(&self) -> String {
        self.active_document_ref()
            .and_then(|doc| {
                self.language_registry
                    .language_for_name(&doc.language_id)
                    .and_then(|lang| lang.line_comment.clone())
            })
            .unwrap_or_else(|| "//".to_owned())
    }

    /// Returns the block comment delimiters for the active document's language.
    pub fn active_block_comment(&self) -> (String, String) {
        self.active_document_ref()
            .and_then(|doc| {
                self.language_registry
                    .language_for_name(&doc.language_id)
                    .and_then(|lang| lang.block_comment.clone())
            })
            .unwrap_or_else(|| ("/*".to_owned(), "*/".to_owned()))
    }

    // ── File operations ──────────────────────────────────────────

    /// Create a new untitled document tab.
    pub fn new_untitled_file(&mut self) {
        self.documents.push(DocumentState::new_untitled());
        self.active_document = self.documents.len() - 1;
        self.update_context_keys();
        self.needs_render = true;
    }

    /// Open a file by path.
    pub fn open_file(&mut self, path: &Path) {
        for (i, doc) in self.documents.iter().enumerate() {
            if doc.file_path.as_deref() == Some(path) {
                self.active_document = i;
                self.update_context_keys();
                self.needs_render = true;
                return;
            }
        }

        match DocumentState::open_file(path, &self.language_registry) {
            Ok(doc_state) => {
                self.documents.push(doc_state);
                self.active_document = self.documents.len() - 1;
                if let Some(ws) = &mut self.workspace {
                    ws.add_recent(path);
                }
                self.update_context_keys();
                self.needs_render = true;
            }
            Err(e) => {
                log::error!("failed to open file {}: {e}", path.display());
            }
        }
    }

    /// Show a native file open dialog and open the selected file.
    pub fn open_file_dialog(&mut self) {
        if let Some(path) = file_dialog::open_file_dialog() {
            self.open_file(&path);
        }
    }

    /// Save the active file.
    pub fn save_active_file(&mut self) {
        let needs_save_as = self
            .active_document_ref()
            .map_or(false, |d| d.file_path.is_none());

        if needs_save_as {
            self.save_active_file_as();
            return;
        }

        if let Some(doc) = self.active_document_mut() {
            if let Err(e) = doc.save() {
                log::error!("save failed: {e}");
            }
            self.needs_render = true;
        }
    }

    /// Save the active file with a "Save As" dialog.
    pub fn save_active_file_as(&mut self) {
        let suggested = self
            .active_document_ref()
            .map(|d| d.display_name())
            .unwrap_or_default();

        if let Some(path) = file_dialog::save_file_dialog(&suggested) {
            if let Some(doc) = self.active_document_mut() {
                if let Err(e) = doc.save_as(&path) {
                    log::error!("save as failed: {e}");
                }
            }
            self.needs_render = true;
        }
    }

    /// Save all open files that have a file path.
    pub fn save_all_files(&mut self) {
        for doc in &mut self.documents {
            if doc.file_path.is_some() && doc.is_dirty() {
                if let Err(e) = doc.save() {
                    log::error!("save failed: {e}");
                }
            }
        }
        self.needs_render = true;
    }

    /// Close the active editor tab.
    pub fn close_active_editor(&mut self) {
        if self.documents.is_empty() {
            return;
        }

        let idx = self.active_document;

        if let Some(path) = self.documents[idx].file_path.as_ref() {
            self.commands
                .recently_closed
                .push(path.display().to_string());
        }

        self.documents.remove(idx);

        if self.documents.is_empty() {
            self.documents.push(DocumentState::new_untitled());
            self.active_document = 0;
        } else if self.active_document >= self.documents.len() {
            self.active_document = self.documents.len() - 1;
        }

        self.update_context_keys();
        self.needs_render = true;
    }

    /// Close all open editor tabs.
    pub fn close_all_editors(&mut self) {
        for doc in &self.documents {
            if let Some(path) = doc.file_path.as_ref() {
                self.commands
                    .recently_closed
                    .push(path.display().to_string());
            }
        }
        self.documents.clear();
        self.documents.push(DocumentState::new_untitled());
        self.active_document = 0;
        self.update_context_keys();
        self.needs_render = true;
    }

    /// Reopen the most recently closed editor.
    pub fn reopen_closed_editor(&mut self) {
        if let Some(path_str) = self.commands.recently_closed.pop() {
            let path = PathBuf::from(&path_str);
            if path.exists() {
                self.open_file(&path);
            }
        }
    }

    /// Switch to a document tab by index.
    pub fn switch_to_document(&mut self, index: usize) {
        if index < self.documents.len() {
            self.active_document = index;
            self.update_context_keys();
            self.needs_render = true;
        }
    }

    // ── Clipboard operations ─────────────────────────────────────

    /// Copy the current selection to clipboard.
    pub fn clipboard_copy(&mut self) {
        if let Some(doc) = self.active_document_ref() {
            let sel = doc.document.cursors.primary().selection;
            if !sel.is_empty() {
                let start = doc.document.buffer.position_to_offset(sel.start());
                let end = doc.document.buffer.position_to_offset(sel.end());
                let text = doc.document.buffer.slice(start..end);
                if let Err(e) = clipboard::copy_to_clipboard(&text) {
                    log::warn!("clipboard copy failed: {e}");
                }
            }
        }
    }

    /// Cut the current selection to clipboard.
    pub fn clipboard_cut(&mut self) {
        self.clipboard_copy();
        if let Some(doc) = self.active_document_mut() {
            let sel = doc.document.cursors.primary().selection;
            if !sel.is_empty() {
                doc.document.delete_right();
                doc.on_edit();
            }
        }
    }

    /// Paste from clipboard.
    pub fn clipboard_paste(&mut self) {
        if let Some(text) = clipboard::paste_from_clipboard() {
            if let Some(doc) = self.active_document_mut() {
                doc.document.insert_text(&text);
                doc.on_edit();
            }
        }
    }

    // ── Context key management ───────────────────────────────────

    /// Update context keys to reflect current editor state.
    pub fn update_context_keys(&mut self) {
        if let Some(doc) = self.documents.get(self.active_document) {
            let has_selection = !doc.document.cursors.primary().selection.is_empty();
            self.context_keys
                .set_bool("editorHasSelection", has_selection);
            self.context_keys
                .set_string("editorLangId", &doc.language_id);
            self.context_keys
                .set_bool("editorReadonly", false);
        }
        self.context_keys
            .set_bool("sideBarVisible", self.layout.sidebar_visible);
        self.context_keys
            .set_bool("panelVisible", self.layout.panel_visible);
    }

    // ── Tick / update ────────────────────────────────────────────

    /// Tick logic: cursor blink, auto-save timers, layout recomputation.
    pub fn update(&mut self) {
        const BLINK_INTERVAL: std::time::Duration = std::time::Duration::from_millis(530);
        if self.cursor_blink_timer.elapsed() >= BLINK_INTERVAL {
            self.cursor_visible = !self.cursor_visible;
            self.cursor_blink_timer = std::time::Instant::now();
            self.needs_render = true;
        }

        if self.needs_relayout {
            let (w, h) = self.renderer.surface_size();
            self.layout_rects = self.layout.compute(w, h);
            self.needs_relayout = false;
            self.needs_render = true;
        }
    }

    // ── Render ───────────────────────────────────────────────────

    /// Render the current frame.
    pub fn render(&mut self) {
        let frame = match self.renderer.begin_frame() {
            Ok(f) => f,
            Err(e) => {
                log::error!("begin_frame failed: {e}");
                return;
            }
        };

        self.renderer.end_frame(frame);
        self.needs_render = false;
    }

    // ── State persistence ────────────────────────────────────────

    /// Persist application state before exit.
    pub fn save_state(&self) {
        let size = self.window.inner_size();
        let pos = self.window.outer_position().unwrap_or_default();
        let active_path = self
            .active_document_ref()
            .and_then(|d| d.file_path.as_ref())
            .map(|p| p.display().to_string());

        let state = sidex_db::WindowState {
            x: pos.x,
            y: pos.y,
            width: size.width,
            height: size.height,
            is_maximized: self.window.is_maximized(),
            sidebar_width: f64::from(self.layout.sidebar_width),
            panel_height: f64::from(self.layout.panel_height),
            active_editor: active_path,
        };
        if let Err(e) = sidex_db::save_window_state(&self.db, &state) {
            log::warn!("failed to save window state: {e}");
        }
    }

    /// Check if any open documents have unsaved changes.
    pub fn has_unsaved_changes(&self) -> bool {
        self.documents.iter().any(DocumentState::is_dirty)
    }

    /// Get the window reference.
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// Whether the cursor should be drawn (blink state).
    pub fn cursor_visible(&self) -> bool {
        self.cursor_visible
    }

    /// Reset cursor blink so it's visible immediately (e.g. after typing).
    pub fn reset_cursor_blink(&mut self) {
        self.cursor_visible = true;
        self.cursor_blink_timer = std::time::Instant::now();
    }
}

/// Returns the user settings file path (`~/.config/sidex/settings.json`).
fn user_settings_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("sidex").join("settings.json"))
}

/// Returns the user keybindings file path (`~/.config/sidex/keybindings.json`).
fn user_keybindings_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("sidex").join("keybindings.json"))
}
