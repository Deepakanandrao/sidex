//! Single-line text input widget.
//!
//! Supports cursor movement, text selection, and clipboard operations.

use sidex_gpu::color::Color;
use sidex_gpu::GpuRenderer;

use crate::layout::{Edges, LayoutNode, Rect, Size};
use crate::widget::{EventResult, Key, MouseButton, UiEvent, Widget};

/// A single-line text input field.
#[allow(dead_code)]
pub struct TextInput<F: FnMut(&str)> {
    pub value: String,
    pub placeholder: String,
    pub on_change: F,
    /// Byte offset of the cursor within `value`.
    cursor: usize,
    /// Byte offset of the selection anchor (equal to `cursor` when no selection).
    selection_anchor: usize,
    focused: bool,
    font_size: f32,
    background: Color,
    foreground: Color,
    placeholder_color: Color,
    border_color: Color,
    selection_color: Color,
}

impl<F: FnMut(&str)> TextInput<F> {
    pub fn new(value: impl Into<String>, on_change: F) -> Self {
        let value = value.into();
        let len = value.len();
        Self {
            value,
            placeholder: String::new(),
            on_change,
            cursor: len,
            selection_anchor: len,
            focused: false,
            font_size: 13.0,
            background: Color::from_hex("#3c3c3c").unwrap_or(Color::BLACK),
            foreground: Color::from_hex("#cccccc").unwrap_or(Color::WHITE),
            placeholder_color: Color::from_hex("#cccccc80").unwrap_or(Color::WHITE),
            border_color: Color::from_hex("#007fd4").unwrap_or(Color::WHITE),
            selection_color: Color::from_hex("#264f78").unwrap_or(Color::BLACK),
        }
    }

    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    fn selection_range(&self) -> (usize, usize) {
        let lo = self.cursor.min(self.selection_anchor);
        let hi = self.cursor.max(self.selection_anchor);
        (lo, hi)
    }

    fn has_selection(&self) -> bool {
        self.cursor != self.selection_anchor
    }

    fn delete_selection(&mut self) {
        let (lo, hi) = self.selection_range();
        self.value.drain(lo..hi);
        self.cursor = lo;
        self.selection_anchor = lo;
    }

    fn insert_char(&mut self, ch: char) {
        if self.has_selection() {
            self.delete_selection();
        }
        self.value.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
        self.selection_anchor = self.cursor;
    }

    fn move_cursor_left(&mut self, shift: bool) {
        if self.cursor > 0 {
            let prev = self.value[..self.cursor]
                .char_indices()
                .next_back()
                .map_or(0, |(i, _)| i);
            self.cursor = prev;
        }
        if !shift {
            self.selection_anchor = self.cursor;
        }
    }

    fn move_cursor_right(&mut self, shift: bool) {
        if self.cursor < self.value.len() {
            let next = self.value[self.cursor..]
                .char_indices()
                .nth(1)
                .map_or(self.value.len(), |(i, _)| self.cursor + i);
            self.cursor = next;
        }
        if !shift {
            self.selection_anchor = self.cursor;
        }
    }

    #[allow(clippy::cast_precision_loss, clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    fn x_to_offset(&self, x: f32, rect: Rect) -> usize {
        let char_width = self.font_size * 0.6;
        let rel = (x - rect.x - 6.0).max(0.0);
        let idx = (rel / char_width).round() as usize;
        let mut byte_offset = 0;
        for (i, (offset, _)) in self.value.char_indices().enumerate() {
            if i >= idx {
                return offset;
            }
            byte_offset = offset;
        }
        if idx > 0 { self.value.len() } else { byte_offset }
    }
}

impl<F: FnMut(&str)> Widget for TextInput<F> {
    fn layout(&self) -> LayoutNode {
        LayoutNode {
            size: Size::Flex(1.0),
            padding: Edges::symmetric(6.0, 4.0),
            ..LayoutNode::default()
        }
    }

    fn render(&self, rect: Rect, renderer: &mut GpuRenderer) {
        let mut rects = sidex_gpu::RectRenderer::new();

        rects.draw_rect(rect.x, rect.y, rect.width, rect.height, self.background, 2.0);

        if self.focused {
            rects.draw_border(rect.x, rect.y, rect.width, rect.height, self.border_color, 1.0);
        }

        if self.has_selection() {
            let (lo, hi) = self.selection_range();
            let char_w = self.font_size * 0.6;
            #[allow(clippy::cast_precision_loss)]
            let sel_x = rect.x + 6.0 + self.value[..lo].chars().count() as f32 * char_w;
            #[allow(clippy::cast_precision_loss)]
            let sel_w = self.value[lo..hi].chars().count() as f32 * char_w;
            rects.draw_rect(sel_x, rect.y + 2.0, sel_w, rect.height - 4.0, self.selection_color, 0.0);
        }

        let _ = renderer;
    }

    fn handle_event(&mut self, event: &UiEvent, rect: Rect) -> EventResult {
        match event {
            UiEvent::Focus => {
                self.focused = true;
                EventResult::Handled
            }
            UiEvent::Blur => {
                self.focused = false;
                EventResult::Handled
            }
            UiEvent::MouseDown {
                x,
                y,
                button: MouseButton::Left,
            } if rect.contains(*x, *y) => {
                self.focused = true;
                let offset = self.x_to_offset(*x, rect);
                self.cursor = offset;
                self.selection_anchor = offset;
                EventResult::Handled
            }
            UiEvent::KeyPress { key, modifiers } if self.focused => match key {
                Key::Char(ch) => {
                    self.insert_char(*ch);
                    (self.on_change)(&self.value);
                    EventResult::Handled
                }
                Key::Backspace => {
                    if self.has_selection() {
                        self.delete_selection();
                    } else if self.cursor > 0 {
                        self.move_cursor_left(false);
                        let end = self.value[self.cursor..]
                            .char_indices()
                            .nth(1)
                            .map_or(self.value.len(), |(i, _)| self.cursor + i);
                        self.value.drain(self.cursor..end);
                    }
                    (self.on_change)(&self.value);
                    EventResult::Handled
                }
                Key::Delete => {
                    if self.has_selection() {
                        self.delete_selection();
                    } else if self.cursor < self.value.len() {
                        let end = self.value[self.cursor..]
                            .char_indices()
                            .nth(1)
                            .map_or(self.value.len(), |(i, _)| self.cursor + i);
                        self.value.drain(self.cursor..end);
                    }
                    (self.on_change)(&self.value);
                    EventResult::Handled
                }
                Key::ArrowLeft => {
                    self.move_cursor_left(modifiers.shift);
                    EventResult::Handled
                }
                Key::ArrowRight => {
                    self.move_cursor_right(modifiers.shift);
                    EventResult::Handled
                }
                Key::Home => {
                    self.cursor = 0;
                    if !modifiers.shift {
                        self.selection_anchor = 0;
                    }
                    EventResult::Handled
                }
                Key::End => {
                    self.cursor = self.value.len();
                    if !modifiers.shift {
                        self.selection_anchor = self.value.len();
                    }
                    EventResult::Handled
                }
                Key::Tab => EventResult::FocusNext,
                _ => EventResult::Ignored,
            },
            _ => EventResult::Ignored,
        }
    }
}
