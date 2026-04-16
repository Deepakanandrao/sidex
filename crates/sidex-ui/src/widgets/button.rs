//! Push-button widget with hover and press states.

use sidex_gpu::color::Color;
use sidex_gpu::GpuRenderer;

use crate::layout::{Edges, LayoutNode, Rect, Size};
use crate::widget::{EventResult, MouseButton, UiEvent, Widget};

/// Visual style for a button.
#[derive(Clone, Debug)]
pub struct ButtonStyle {
    pub background: Color,
    pub foreground: Color,
    pub hover_background: Color,
    pub press_background: Color,
    pub border_radius: f32,
    pub padding: Edges,
    pub font_size: f32,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            background: Color::from_hex("#0e639c").unwrap_or(Color::BLACK),
            foreground: Color::WHITE,
            hover_background: Color::from_hex("#1177bb").unwrap_or(Color::BLACK),
            press_background: Color::from_hex("#0d5689").unwrap_or(Color::BLACK),
            border_radius: 2.0,
            padding: Edges::symmetric(14.0, 6.0),
            font_size: 13.0,
        }
    }
}

/// Interactive state of the button.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum InteractState {
    #[default]
    Normal,
    Hovered,
    Pressed,
}

/// A clickable button with a text label.
pub struct Button<F: FnMut()> {
    pub label: String,
    pub on_click: F,
    pub style: ButtonStyle,
    state: InteractState,
}

impl<F: FnMut()> Button<F> {
    pub fn new(label: impl Into<String>, on_click: F) -> Self {
        Self {
            label: label.into(),
            on_click,
            style: ButtonStyle::default(),
            state: InteractState::Normal,
        }
    }

    pub fn with_style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }
}

impl<F: FnMut()> Widget for Button<F> {
    fn layout(&self) -> LayoutNode {
        let text_width = self.label.len() as f32 * self.style.font_size * 0.6;
        LayoutNode {
            size: Size::Fixed(text_width + self.style.padding.horizontal()),
            padding: self.style.padding,
            ..LayoutNode::default()
        }
    }

    fn render(&self, rect: Rect, renderer: &mut GpuRenderer) {
        let bg = match self.state {
            InteractState::Normal => self.style.background,
            InteractState::Hovered => self.style.hover_background,
            InteractState::Pressed => self.style.press_background,
        };

        let mut rects = sidex_gpu::RectRenderer::new();
        rects.draw_rect(rect.x, rect.y, rect.width, rect.height, bg, self.style.border_radius);
        let _ = renderer;

        // Text would be drawn via TextRenderer in a real frame; we queue the
        // rect draw here to demonstrate the pattern.  Actual flushing happens
        // during the render pass which requires the full frame context.
    }

    fn handle_event(&mut self, event: &UiEvent, rect: Rect) -> EventResult {
        match event {
            UiEvent::MouseMove { x, y } => {
                if rect.contains(*x, *y) {
                    if self.state != InteractState::Pressed {
                        self.state = InteractState::Hovered;
                    }
                } else {
                    self.state = InteractState::Normal;
                }
                EventResult::Ignored
            }
            UiEvent::MouseDown {
                x,
                y,
                button: MouseButton::Left,
            } if rect.contains(*x, *y) => {
                self.state = InteractState::Pressed;
                EventResult::Handled
            }
            UiEvent::MouseUp {
                x,
                y,
                button: MouseButton::Left,
            } => {
                if self.state == InteractState::Pressed && rect.contains(*x, *y) {
                    (self.on_click)();
                }
                self.state = if rect.contains(*x, *y) {
                    InteractState::Hovered
                } else {
                    InteractState::Normal
                };
                EventResult::Handled
            }
            _ => EventResult::Ignored,
        }
    }
}
