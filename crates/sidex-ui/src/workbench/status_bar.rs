//! Status bar at the bottom of the workbench.

use sidex_gpu::color::Color;
use sidex_gpu::GpuRenderer;

use crate::layout::{LayoutNode, Rect, Size};
use crate::widget::{EventResult, MouseButton, UiEvent, Widget};

/// Alignment of a status bar item.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatusBarAlignment {
    #[default]
    Left,
    Right,
}

/// A single status bar item.
#[derive(Clone, Debug)]
pub struct StatusBarItem {
    pub id: String,
    pub text: String,
    pub tooltip: Option<String>,
    pub alignment: StatusBarAlignment,
    pub priority: i32,
    pub color: Option<Color>,
    pub background_color: Option<Color>,
}

impl StatusBarItem {
    pub fn new(id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            tooltip: None,
            alignment: StatusBarAlignment::Left,
            priority: 0,
            color: None,
            background_color: None,
        }
    }

    pub fn right(mut self) -> Self {
        self.alignment = StatusBarAlignment::Right;
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}

/// The status bar at the bottom of the window.
#[allow(dead_code)]
pub struct StatusBar<F: FnMut(&str)> {
    pub items: Vec<StatusBarItem>,
    pub on_click: F,

    height: f32,
    font_size: f32,
    hovered_index: Option<usize>,

    background: Color,
    foreground: Color,
    hover_bg: Color,
    border_color: Color,
}

impl<F: FnMut(&str)> StatusBar<F> {
    pub fn new(items: Vec<StatusBarItem>, on_click: F) -> Self {
        Self {
            items,
            on_click,
            height: 22.0,
            font_size: 12.0,
            hovered_index: None,
            background: Color::from_hex("#007acc").unwrap_or(Color::BLACK),
            foreground: Color::WHITE,
            hover_bg: Color::from_hex("#ffffff1f").unwrap_or(Color::WHITE),
            border_color: Color::from_hex("#ffffff1f").unwrap_or(Color::WHITE),
        }
    }

    fn left_items(&self) -> Vec<(usize, &StatusBarItem)> {
        let mut items: Vec<_> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, i)| i.alignment == StatusBarAlignment::Left)
            .collect();
        items.sort_by_key(|(_, i)| std::cmp::Reverse(i.priority));
        items
    }

    fn right_items(&self) -> Vec<(usize, &StatusBarItem)> {
        let mut items: Vec<_> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, i)| i.alignment == StatusBarAlignment::Right)
            .collect();
        items.sort_by_key(|(_, i)| std::cmp::Reverse(i.priority));
        items
    }

    #[allow(clippy::cast_precision_loss)]
    fn item_rects(&self, rect: Rect) -> Vec<(usize, Rect)> {
        let padding_h = 8.0;
        let mut result = Vec::new();

        let mut x = rect.x;
        for (idx, item) in self.left_items() {
            let w = item.text.len() as f32 * self.font_size * 0.6 + padding_h * 2.0;
            result.push((idx, Rect::new(x, rect.y, w, rect.height)));
            x += w;
        }

        let mut x = rect.x + rect.width;
        for (idx, item) in self.right_items() {
            let w = item.text.len() as f32 * self.font_size * 0.6 + padding_h * 2.0;
            x -= w;
            result.push((idx, Rect::new(x, rect.y, w, rect.height)));
        }

        result
    }
}

impl<F: FnMut(&str)> Widget for StatusBar<F> {
    fn layout(&self) -> LayoutNode {
        LayoutNode {
            size: Size::Fixed(self.height),
            ..LayoutNode::default()
        }
    }

    fn render(&self, rect: Rect, renderer: &mut GpuRenderer) {
        let mut rr = sidex_gpu::RectRenderer::new();

        rr.draw_rect(rect.x, rect.y, rect.width, rect.height, self.background, 0.0);

        for (idx, ir) in self.item_rects(rect) {
            if let Some(bg) = self.items[idx].background_color {
                rr.draw_rect(ir.x, ir.y, ir.width, ir.height, bg, 0.0);
            }
            if self.hovered_index == Some(idx) {
                rr.draw_rect(ir.x, ir.y, ir.width, ir.height, self.hover_bg, 0.0);
            }
        }
        let _ = renderer;
    }

    fn handle_event(&mut self, event: &UiEvent, rect: Rect) -> EventResult {
        let item_rects = self.item_rects(rect);

        match event {
            UiEvent::MouseMove { x, y } => {
                self.hovered_index = item_rects
                    .iter()
                    .find(|(_, r)| r.contains(*x, *y))
                    .map(|(idx, _)| *idx);
                EventResult::Ignored
            }
            UiEvent::MouseDown {
                x,
                y,
                button: MouseButton::Left,
            } => {
                if let Some((idx, _)) = item_rects.iter().find(|(_, r)| r.contains(*x, *y)) {
                    let id = self.items[*idx].id.clone();
                    (self.on_click)(&id);
                    return EventResult::Handled;
                }
                EventResult::Ignored
            }
            _ => EventResult::Ignored,
        }
    }
}
