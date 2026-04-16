//! Static text label widget.

use sidex_gpu::color::Color;
use sidex_gpu::GpuRenderer;

use crate::layout::{LayoutNode, Rect, Size};
use crate::widget::{EventResult, UiEvent, Widget};

/// A non-interactive text label.
#[derive(Clone, Debug)]
pub struct Label {
    pub text: String,
    pub color: Color,
    pub font_size: f32,
    pub bold: bool,
    pub italic: bool,
}

impl Label {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            color: Color::WHITE,
            font_size: 13.0,
            bold: false,
            italic: false,
        }
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_font_size(mut self, size: f32) -> Self {
        self.font_size = size;
        self
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub fn italic(mut self) -> Self {
        self.italic = true;
        self
    }
}

impl Widget for Label {
    #[allow(clippy::cast_precision_loss)]
    fn layout(&self) -> LayoutNode {
        let estimated_width = self.text.len() as f32 * self.font_size * 0.6;
        LayoutNode {
            size: Size::Fixed(estimated_width),
            ..LayoutNode::default()
        }
    }

    fn render(&self, _rect: Rect, renderer: &mut GpuRenderer) {
        // Actual text rendering requires a TextDrawContext with font system
        // and atlas, which are provided during the frame render pass.
        let _ = renderer;
    }

    fn handle_event(&mut self, _event: &UiEvent, _rect: Rect) -> EventResult {
        EventResult::Ignored
    }
}
