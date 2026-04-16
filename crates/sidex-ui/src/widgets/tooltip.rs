//! Tooltip popup that appears on hover.

use sidex_gpu::color::Color;
use sidex_gpu::GpuRenderer;

use crate::layout::{Edges, LayoutNode, Rect, Size};
use crate::widget::{EventResult, UiEvent, Widget};

/// Preferred position of the tooltip relative to its target.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TooltipPosition {
    #[default]
    Below,
    Above,
    Left,
    Right,
}

/// A tooltip popup that displays text near a target rectangle.
#[allow(dead_code)]
pub struct Tooltip {
    pub content: String,
    pub target_rect: Rect,
    pub position: TooltipPosition,

    visible: bool,
    background: Color,
    foreground: Color,
    border_color: Color,
    font_size: f32,
    padding: Edges,
}

impl Tooltip {
    pub fn new(content: impl Into<String>, target_rect: Rect) -> Self {
        Self {
            content: content.into(),
            target_rect,
            position: TooltipPosition::Below,
            visible: false,
            background: Color::from_hex("#383838").unwrap_or(Color::BLACK),
            foreground: Color::from_hex("#cccccc").unwrap_or(Color::WHITE),
            border_color: Color::from_hex("#454545").unwrap_or(Color::BLACK),
            font_size: 12.0,
            padding: Edges::symmetric(8.0, 4.0),
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    #[allow(clippy::cast_precision_loss)]
    fn tooltip_size(&self) -> (f32, f32) {
        let text_w = self.content.len() as f32 * self.font_size * 0.6;
        let w = text_w + self.padding.horizontal();
        let h = self.font_size + self.padding.vertical();
        (w, h)
    }

    /// Computes the tooltip rect, clamped to stay within `viewport`.
    fn compute_rect(&self, viewport: Rect) -> Rect {
        let (w, h) = self.tooltip_size();
        let gap = 4.0;

        let (mut x, mut y) = match self.position {
            TooltipPosition::Below => (
                self.target_rect.x,
                self.target_rect.y + self.target_rect.height + gap,
            ),
            TooltipPosition::Above => (self.target_rect.x, self.target_rect.y - h - gap),
            TooltipPosition::Right => (
                self.target_rect.x + self.target_rect.width + gap,
                self.target_rect.y,
            ),
            TooltipPosition::Left => (self.target_rect.x - w - gap, self.target_rect.y),
        };

        if x + w > viewport.x + viewport.width {
            x = viewport.x + viewport.width - w;
        }
        if x < viewport.x {
            x = viewport.x;
        }
        if y + h > viewport.y + viewport.height {
            y = viewport.y + viewport.height - h;
        }
        if y < viewport.y {
            y = viewport.y;
        }

        Rect::new(x, y, w, h)
    }
}

impl Widget for Tooltip {
    fn layout(&self) -> LayoutNode {
        LayoutNode {
            size: Size::Auto,
            ..LayoutNode::default()
        }
    }

    fn render(&self, rect: Rect, renderer: &mut GpuRenderer) {
        if !self.visible {
            return;
        }
        let tr = self.compute_rect(rect);
        let mut rr = sidex_gpu::RectRenderer::new();

        rr.draw_rect(tr.x, tr.y, tr.width, tr.height, self.background, 3.0);
        rr.draw_border(tr.x, tr.y, tr.width, tr.height, self.border_color, 1.0);

        let _ = renderer;
    }

    fn handle_event(&mut self, event: &UiEvent, rect: Rect) -> EventResult {
        match event {
            UiEvent::MouseMove { x, y } => {
                if self.target_rect.contains(*x, *y) {
                    if !self.visible {
                        self.show();
                    }
                } else if self.visible {
                    let tr = self.compute_rect(rect);
                    if !tr.contains(*x, *y) {
                        self.hide();
                    }
                }
                EventResult::Ignored
            }
            _ => EventResult::Ignored,
        }
    }
}
