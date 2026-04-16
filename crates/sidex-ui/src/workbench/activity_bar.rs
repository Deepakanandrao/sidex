//! Vertical activity bar (far-left icon column).

use sidex_gpu::color::Color;
use sidex_gpu::GpuRenderer;

use crate::layout::{LayoutNode, Rect, Size};
use crate::widget::{EventResult, MouseButton, UiEvent, Widget};

/// An entry in the activity bar.
#[derive(Clone, Debug)]
pub struct ActivityBarItem {
    pub id: String,
    pub icon: String,
    pub tooltip: String,
    pub badge_count: Option<u32>,
}

impl ActivityBarItem {
    pub fn new(id: impl Into<String>, icon: impl Into<String>, tooltip: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            icon: icon.into(),
            tooltip: tooltip.into(),
            badge_count: None,
        }
    }

    pub fn with_badge(mut self, count: u32) -> Self {
        self.badge_count = Some(count);
        self
    }
}

/// The vertical icon bar on the far left of the workbench.
#[allow(dead_code)]
pub struct ActivityBar<F: FnMut(usize)> {
    pub items: Vec<ActivityBarItem>,
    pub active_index: usize,
    pub on_select: F,

    width: f32,
    icon_size: f32,
    item_height: f32,
    hovered_index: Option<usize>,

    background: Color,
    foreground: Color,
    inactive_fg: Color,
    active_indicator: Color,
    badge_bg: Color,
    badge_fg: Color,
    hover_bg: Color,
}

impl<F: FnMut(usize)> ActivityBar<F> {
    pub fn new(items: Vec<ActivityBarItem>, on_select: F) -> Self {
        Self {
            items,
            active_index: 0,
            on_select,
            width: 48.0,
            icon_size: 24.0,
            item_height: 48.0,
            hovered_index: None,
            background: Color::from_hex("#333333").unwrap_or(Color::BLACK),
            foreground: Color::WHITE,
            inactive_fg: Color::from_hex("#ffffff66").unwrap_or(Color::WHITE),
            active_indicator: Color::WHITE,
            badge_bg: Color::from_hex("#007acc").unwrap_or(Color::BLACK),
            badge_fg: Color::WHITE,
            hover_bg: Color::from_hex("#2a2d2e").unwrap_or(Color::BLACK),
        }
    }

    fn item_rect(&self, index: usize, container: Rect) -> Rect {
        Rect::new(
            container.x,
            container.y + index as f32 * self.item_height,
            self.width,
            self.item_height,
        )
    }
}

impl<F: FnMut(usize)> Widget for ActivityBar<F> {
    fn layout(&self) -> LayoutNode {
        LayoutNode {
            size: Size::Fixed(self.width),
            ..LayoutNode::default()
        }
    }

    fn render(&self, rect: Rect, renderer: &mut GpuRenderer) {
        let mut rr = sidex_gpu::RectRenderer::new();

        rr.draw_rect(rect.x, rect.y, rect.width, rect.height, self.background, 0.0);

        for (i, _item) in self.items.iter().enumerate() {
            let ir = self.item_rect(i, rect);
            let is_active = i == self.active_index;

            if self.hovered_index == Some(i) && !is_active {
                rr.draw_rect(ir.x, ir.y, ir.width, ir.height, self.hover_bg, 0.0);
            }

            if is_active {
                rr.draw_rect(ir.x, ir.y, 2.0, ir.height, self.active_indicator, 0.0);
            }

            let icon_x = ir.x + (ir.width - self.icon_size) / 2.0;
            let icon_y = ir.y + (ir.height - self.icon_size) / 2.0;
            let _ = (icon_x, icon_y);

            if let Some(count) = _item.badge_count {
                if count > 0 {
                    let badge_size = 16.0;
                    let bx = ir.x + ir.width - badge_size - 6.0;
                    let by = ir.y + 6.0;
                    rr.draw_rect(bx, by, badge_size, badge_size, self.badge_bg, badge_size / 2.0);
                }
            }
        }
        let _ = renderer;
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn handle_event(&mut self, event: &UiEvent, rect: Rect) -> EventResult {
        match event {
            UiEvent::MouseMove { x, y } => {
                if rect.contains(*x, *y) {
                    let idx = ((y - rect.y) / self.item_height) as usize;
                    self.hovered_index = if idx < self.items.len() { Some(idx) } else { None };
                } else {
                    self.hovered_index = None;
                }
                EventResult::Ignored
            }
            UiEvent::MouseDown {
                x,
                y,
                button: MouseButton::Left,
            } if rect.contains(*x, *y) => {
                let idx = ((y - rect.y) / self.item_height) as usize;
                if idx < self.items.len() {
                    self.active_index = idx;
                    (self.on_select)(idx);
                    return EventResult::Handled;
                }
                EventResult::Ignored
            }
            _ => EventResult::Ignored,
        }
    }
}
