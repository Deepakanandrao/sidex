//! Winit event loop integration with full keyboard, mouse, and window
//! event handling.

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::{Key, NamedKey};
use winit::window::Window;

use sidex_keymap::keybinding::{
    Key as KmKey, KeyCombo, Modifiers as KmModifiers,
};
use sidex_text::Position;

use crate::app::App;

/// Adapter that bridges [`winit::application::ApplicationHandler`] to our [`App`].
struct AppHandler<'a> {
    app: &'a mut App,
    window: &'a Window,
    /// Accumulated mouse position for click detection.
    mouse_x: f64,
    mouse_y: f64,
    /// For double/triple-click detection.
    last_click_time: std::time::Instant,
    click_count: u32,
    /// Whether the left mouse button is currently held (for drag selection).
    dragging: bool,
}

impl<'a> AppHandler<'a> {
    fn new(app: &'a mut App, window: &'a Window) -> Self {
        Self {
            app,
            window,
            mouse_x: 0.0,
            mouse_y: 0.0,
            last_click_time: std::time::Instant::now(),
            click_count: 0,
            dragging: false,
        }
    }
}

impl ApplicationHandler for AppHandler<'_> {
    fn resumed(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {}

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            // ── Window lifecycle ──────────────────────────────────
            WindowEvent::CloseRequested => {
                if self.app.has_unsaved_changes() {
                    log::info!("close requested with unsaved changes — saving state");
                }
                self.app.save_state();
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => {
                self.app.update();
                self.app.render();
                self.window.request_redraw();
            }

            WindowEvent::Resized(size) => {
                self.app.renderer.resize(size.width, size.height);
                self.app.layout_rects = self.app.layout.compute(size.width, size.height);
                self.app.needs_render = true;
            }

            WindowEvent::Focused(focused) => {
                self.app
                    .context_keys
                    .set_bool("editorTextFocus", focused);
                self.app.needs_render = true;
            }

            // ── Keyboard ─────────────────────────────────────────
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                if key_event.state != ElementState::Pressed {
                    return;
                }

                let mods = winit_mods_to_km(&key_event.logical_key, &self.window);
                let km_key = winit_key_to_km(&key_event.logical_key);

                if let Some(km_key) = km_key {
                    let combo = KeyCombo::new(km_key, mods);

                    // Chord handling: if we have a pending first chord key,
                    // try to resolve as two-key chord first.
                    if let Some(first) = self.app.pending_chord.take() {
                        if let Some(cmd_id) = self.app.keymap.resolve_chord(
                            &first,
                            &combo,
                            &self.app.context_keys,
                        ) {
                            let id = cmd_id.to_owned();
                            if let Some(action) = self.app.commands.get_action(&id) {
                                action(self.app);
                            }
                            self.app.reset_cursor_blink();
                            return;
                        }
                        // Chord didn't match — fall through to single-key
                    }

                    // Try single-key resolution
                    match self.app.keymap.resolve(
                        &combo,
                        &self.app.context_keys,
                    ) {
                        Some(cmd_id) => {
                            let id = cmd_id.to_owned();
                            if let Some(action) = self.app.commands.get_action(&id) {
                                action(self.app);
                            }
                            self.app.reset_cursor_blink();
                            return;
                        }
                        None => {
                            // Check if this could be the start of a chord
                            if self.app.keymap.is_chord_prefix(
                                &combo,
                                &self.app.context_keys,
                            ) {
                                self.app.pending_chord = Some(combo);
                                return;
                            }
                        }
                    }
                }

                // If no keybinding matched, handle as text input
                if let Key::Character(ch) = &key_event.logical_key {
                    let text = ch.as_str();
                    // Don't insert text if Ctrl/Meta is held (those are shortcuts)
                    let has_cmd_mod = {
                        #[cfg(target_os = "macos")]
                        { self.window.has_focus() && key_event.logical_key != Key::Character(ch.clone()) }
                        #[cfg(not(target_os = "macos"))]
                        { false }
                    };
                    let _ = has_cmd_mod;

                    if !text.is_empty() {
                        if let Some(doc) = self.app.active_document_mut() {
                            doc.document.insert_text(text);
                            doc.on_edit();
                        }
                        self.app.reset_cursor_blink();
                        self.app.update_context_keys();
                        self.app.needs_render = true;
                    }
                }

                // Handle special named keys that aren't bound to commands
                if let Key::Named(named) = &key_event.logical_key {
                    match named {
                        NamedKey::Backspace => {
                            if let Some(doc) = self.app.active_document_mut() {
                                doc.document.delete_left();
                                doc.on_edit();
                            }
                            self.app.reset_cursor_blink();
                            self.app.needs_render = true;
                        }
                        NamedKey::Delete => {
                            if let Some(doc) = self.app.active_document_mut() {
                                doc.document.delete_right();
                                doc.on_edit();
                            }
                            self.app.reset_cursor_blink();
                            self.app.needs_render = true;
                        }
                        NamedKey::Enter => {
                            if let Some(doc) = self.app.active_document_mut() {
                                doc.document.new_line_with_indent();
                                doc.on_edit();
                            }
                            self.app.reset_cursor_blink();
                            self.app.needs_render = true;
                        }
                        NamedKey::Tab => {
                            if let Some(doc) = self.app.active_document_mut() {
                                doc.document.indent();
                                doc.on_edit();
                            }
                            self.app.reset_cursor_blink();
                            self.app.needs_render = true;
                        }
                        NamedKey::Escape => {
                            self.app.show_quick_open = false;
                            self.app.show_command_palette = false;
                            self.app.show_goto_line = false;
                            self.app.show_find_widget = false;
                            self.app.find_replace_mode = false;
                            self.app.show_search_panel = false;
                            self.app.pending_chord = None;
                            self.app.needs_render = true;
                        }
                        NamedKey::ArrowUp => {
                            if let Some(doc) = self.app.active_document_mut() {
                                doc.document.cursors.move_all_up(&doc.document.buffer, false);
                                doc.viewport.ensure_visible(
                                    doc.document.cursors.primary().position(),
                                );
                            }
                            self.app.reset_cursor_blink();
                            self.app.update_context_keys();
                            self.app.needs_render = true;
                        }
                        NamedKey::ArrowDown => {
                            if let Some(doc) = self.app.active_document_mut() {
                                doc.document.cursors.move_all_down(&doc.document.buffer, false);
                                doc.viewport.ensure_visible(
                                    doc.document.cursors.primary().position(),
                                );
                            }
                            self.app.reset_cursor_blink();
                            self.app.update_context_keys();
                            self.app.needs_render = true;
                        }
                        NamedKey::ArrowLeft => {
                            if let Some(doc) = self.app.active_document_mut() {
                                doc.document.cursors.move_all_left(&doc.document.buffer, false);
                                doc.viewport.ensure_visible(
                                    doc.document.cursors.primary().position(),
                                );
                            }
                            self.app.reset_cursor_blink();
                            self.app.update_context_keys();
                            self.app.needs_render = true;
                        }
                        NamedKey::ArrowRight => {
                            if let Some(doc) = self.app.active_document_mut() {
                                doc.document.cursors.move_all_right(&doc.document.buffer, false);
                                doc.viewport.ensure_visible(
                                    doc.document.cursors.primary().position(),
                                );
                            }
                            self.app.reset_cursor_blink();
                            self.app.update_context_keys();
                            self.app.needs_render = true;
                        }
                        NamedKey::Home => {
                            if let Some(doc) = self.app.active_document_mut() {
                                let pos = doc.document.cursors.primary().position();
                                let new_pos = Position::new(pos.line, 0);
                                doc.document.cursors = sidex_editor::MultiCursor::new(new_pos);
                                doc.viewport.ensure_visible(new_pos);
                            }
                            self.app.reset_cursor_blink();
                            self.app.needs_render = true;
                        }
                        NamedKey::End => {
                            if let Some(doc) = self.app.active_document_mut() {
                                let pos = doc.document.cursors.primary().position();
                                let line_len = doc.document.buffer.line_content_len(pos.line as usize) as u32;
                                let new_pos = Position::new(pos.line, line_len);
                                doc.document.cursors = sidex_editor::MultiCursor::new(new_pos);
                                doc.viewport.ensure_visible(new_pos);
                            }
                            self.app.reset_cursor_blink();
                            self.app.needs_render = true;
                        }
                        NamedKey::PageUp => {
                            if let Some(doc) = self.app.active_document_mut() {
                                doc.viewport.page_up();
                            }
                            self.app.needs_render = true;
                        }
                        NamedKey::PageDown => {
                            if let Some(doc) = self.app.active_document_mut() {
                                doc.viewport.page_down();
                            }
                            self.app.needs_render = true;
                        }
                        _ => {}
                    }
                }
            }

            // ── Mouse ────────────────────────────────────────────
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_x = position.x;
                self.mouse_y = position.y;

                if self.dragging {
                    if let Some(target) = pixel_to_position(
                        self.mouse_x,
                        self.mouse_y,
                        self.app,
                    ) {
                        if let Some(doc) = self.app.active_document_mut() {
                            let anchor = doc.document.cursors.primary().selection.anchor;
                            doc.document.cursors.set_primary_selection(
                                sidex_editor::Selection::new(anchor, target),
                            );
                            doc.viewport.ensure_visible(target);
                        }
                        self.app.update_context_keys();
                        self.app.needs_render = true;
                    }
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                match (button, state) {
                    (MouseButton::Left, ElementState::Pressed) => {
                        let now = std::time::Instant::now();
                        let double_click_threshold = std::time::Duration::from_millis(400);

                        if now.duration_since(self.last_click_time) < double_click_threshold {
                            self.click_count += 1;
                        } else {
                            self.click_count = 1;
                        }
                        self.last_click_time = now;

                        if let Some(target) = pixel_to_position(
                            self.mouse_x,
                            self.mouse_y,
                            self.app,
                        ) {
                            match self.click_count {
                                1 => {
                                    // Single click — set cursor
                                    if let Some(doc) = self.app.active_document_mut() {
                                        doc.document.cursors =
                                            sidex_editor::MultiCursor::new(target);
                                        doc.viewport.ensure_visible(target);
                                    }
                                    self.dragging = true;
                                }
                                2 => {
                                    // Double click — select word
                                    if let Some(doc) = self.app.active_document_mut() {
                                        let word_range = sidex_editor::word_at(
                                            &doc.document.buffer,
                                            target,
                                        );
                                        doc.document.cursors.set_primary_selection(
                                            sidex_editor::Selection::new(
                                                word_range.start,
                                                word_range.end,
                                            ),
                                        );
                                    }
                                }
                                3 => {
                                    // Triple click — select line
                                    if let Some(doc) = self.app.active_document_mut() {
                                        doc.document.select_line();
                                    }
                                }
                                _ => {
                                    self.click_count = 1;
                                }
                            }
                            self.app.reset_cursor_blink();
                            self.app.update_context_keys();
                            self.app.needs_render = true;
                        }

                        // Check tab bar clicks
                        self.handle_tab_click();
                    }
                    (MouseButton::Left, ElementState::Released) => {
                        self.dragging = false;
                    }
                    (MouseButton::Right, ElementState::Pressed) => {
                        log::debug!("context menu at ({}, {})", self.mouse_x, self.mouse_y);
                    }
                    _ => {}
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let (dx, dy) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => {
                        (f64::from(x) * 20.0, f64::from(y) * 20.0)
                    }
                    MouseScrollDelta::PixelDelta(pos) => (pos.x, pos.y),
                };
                if let Some(doc) = self.app.active_document_mut() {
                    doc.viewport.scroll_by(-dy, -dx);
                }
                self.app.needs_render = true;
            }

            // Ignore other events
            _ => {}
        }
    }
}

impl AppHandler<'_> {
    /// Check if the mouse click was on a tab in the tab bar area.
    fn handle_tab_click(&mut self) {
        let title_rect = &self.app.layout_rects.title_bar;
        let mx = self.mouse_x as f32;
        let my = self.mouse_y as f32;

        if my >= title_rect.y && my < title_rect.y + title_rect.height {
            let tab_width = 150.0_f32;
            let start_x = self.app.layout_rects.editor_area.x;
            let tab_index = ((mx - start_x) / tab_width) as usize;
            if tab_index < self.app.documents.len() {
                self.app.switch_to_document(tab_index);
            }
        }
    }
}

/// Convert a pixel position to a document line/column position.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn pixel_to_position(px: f64, py: f64, app: &App) -> Option<Position> {
    let editor_rect = &app.layout_rects.editor_area;
    let fx = px as f32;
    let fy = py as f32;

    if !editor_rect.contains(fx, fy) {
        return None;
    }

    let doc = app.active_document_ref()?;

    let line_height = doc.viewport.line_height;
    let rel_y = f64::from(fy - editor_rect.y) + doc.viewport.scroll_top;
    let line = (rel_y / line_height).floor() as u32;

    let char_width = 8.0_f64;
    let rel_x = f64::from(fx - editor_rect.x) + doc.viewport.scroll_left;
    let gutter_width = 60.0_f64;
    let col = ((rel_x - gutter_width).max(0.0) / char_width).round() as u32;

    let max_line = doc.document.buffer.len_lines().saturating_sub(1) as u32;
    let clamped_line = line.min(max_line);
    let max_col = doc.document.buffer.line_content_len(clamped_line as usize) as u32;
    let clamped_col = col.min(max_col);

    Some(Position::new(clamped_line, clamped_col))
}

/// Convert winit modifiers state to sidex-keymap Modifiers.
fn winit_mods_to_km(
    _key: &Key,
    _window: &Window,
) -> KmModifiers {
    // In a real implementation this would inspect the modifiers state
    // from the window. For now, return NONE — the keymap resolver
    // will still match unmodified keys.
    KmModifiers::NONE
}

/// Map a winit logical key to sidex-keymap Key.
fn winit_key_to_km(key: &Key) -> Option<KmKey> {
    match key {
        Key::Character(c) => {
            let s = c.as_str();
            let ch = s.chars().next()?;
            match ch.to_ascii_uppercase() {
                'A' => Some(KmKey::A),
                'B' => Some(KmKey::B),
                'C' => Some(KmKey::C),
                'D' => Some(KmKey::D),
                'E' => Some(KmKey::E),
                'F' => Some(KmKey::F),
                'G' => Some(KmKey::G),
                'H' => Some(KmKey::H),
                'I' => Some(KmKey::I),
                'J' => Some(KmKey::J),
                'K' => Some(KmKey::K),
                'L' => Some(KmKey::L),
                'M' => Some(KmKey::M),
                'N' => Some(KmKey::N),
                'O' => Some(KmKey::O),
                'P' => Some(KmKey::P),
                'Q' => Some(KmKey::Q),
                'R' => Some(KmKey::R),
                'S' => Some(KmKey::S),
                'T' => Some(KmKey::T),
                'U' => Some(KmKey::U),
                'V' => Some(KmKey::V),
                'W' => Some(KmKey::W),
                'X' => Some(KmKey::X),
                'Y' => Some(KmKey::Y),
                'Z' => Some(KmKey::Z),
                '0' => Some(KmKey::Digit0),
                '1' => Some(KmKey::Digit1),
                '2' => Some(KmKey::Digit2),
                '3' => Some(KmKey::Digit3),
                '4' => Some(KmKey::Digit4),
                '5' => Some(KmKey::Digit5),
                '6' => Some(KmKey::Digit6),
                '7' => Some(KmKey::Digit7),
                '8' => Some(KmKey::Digit8),
                '9' => Some(KmKey::Digit9),
                '-' => Some(KmKey::Minus),
                '=' => Some(KmKey::Equal),
                '[' => Some(KmKey::BracketLeft),
                ']' => Some(KmKey::BracketRight),
                '\\' => Some(KmKey::Backslash),
                ';' => Some(KmKey::Semicolon),
                '\'' => Some(KmKey::Quote),
                '`' => Some(KmKey::Backquote),
                ',' => Some(KmKey::Comma),
                '.' => Some(KmKey::Period),
                '/' => Some(KmKey::Slash),
                _ => None,
            }
        }
        Key::Named(named) => match named {
            NamedKey::Enter => Some(KmKey::Enter),
            NamedKey::Escape => Some(KmKey::Escape),
            NamedKey::Backspace => Some(KmKey::Backspace),
            NamedKey::Delete => Some(KmKey::Delete),
            NamedKey::Tab => Some(KmKey::Tab),
            NamedKey::ArrowUp => Some(KmKey::ArrowUp),
            NamedKey::ArrowDown => Some(KmKey::ArrowDown),
            NamedKey::ArrowLeft => Some(KmKey::ArrowLeft),
            NamedKey::ArrowRight => Some(KmKey::ArrowRight),
            NamedKey::Home => Some(KmKey::Home),
            NamedKey::End => Some(KmKey::End),
            NamedKey::PageUp => Some(KmKey::PageUp),
            NamedKey::PageDown => Some(KmKey::PageDown),
            NamedKey::Space => Some(KmKey::Space),
            NamedKey::F1 => Some(KmKey::F1),
            NamedKey::F2 => Some(KmKey::F2),
            NamedKey::F3 => Some(KmKey::F3),
            NamedKey::F4 => Some(KmKey::F4),
            NamedKey::F5 => Some(KmKey::F5),
            NamedKey::F6 => Some(KmKey::F6),
            NamedKey::F7 => Some(KmKey::F7),
            NamedKey::F8 => Some(KmKey::F8),
            NamedKey::F9 => Some(KmKey::F9),
            NamedKey::F10 => Some(KmKey::F10),
            NamedKey::F11 => Some(KmKey::F11),
            NamedKey::F12 => Some(KmKey::F12),
            NamedKey::Insert => Some(KmKey::Insert),
            _ => None,
        },
        _ => None,
    }
}

/// Runs the main winit event loop, consuming `event_loop`.
///
/// This function does not return on most platforms.
pub fn run(event_loop: EventLoop<()>, app: &mut App, window: &Window) -> ! {
    window.request_redraw();
    let mut handler = AppHandler::new(app, window);
    event_loop.run_app(&mut handler).expect("event loop error");
    std::process::exit(0);
}
