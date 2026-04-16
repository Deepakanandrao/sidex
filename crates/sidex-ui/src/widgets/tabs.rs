//! Tab bar widget with close buttons, dirty indicators, and overflow.

use sidex_gpu::color::Color;
use sidex_gpu::GpuRenderer;

use crate::layout::{LayoutNode, Rect, Size};
use crate::widget::{EventResult, MouseButton, UiEvent, Widget};

/// A single tab descriptor.
#[derive(Clone, Debug)]
pub struct Tab {
    pub id: String,
    pub label: String,
    pub is_dirty: bool,
    pub is_preview: bool,
}

/// A tab bar with selection, close, and overflow scrolling.
#[allow(dead_code)]
pub struct TabBar<S, C>
where
    S: FnMut(usize),
    C: FnMut(usize),
{
    pub tabs: Vec<Tab>,
    pub active: usize,
    pub on_select: S,
    pub on_close: C,

    tab_height: f32,
    tab_min_width: f32,
    tab_max_width: f32,
    scroll_offset: f32,

    active_bg: Color,
    active_fg: Color,
    inactive_bg: Color,
    inactive_fg: Color,
    border_color: Color,
    dirty_dot_color: Color,
    close_hover_bg: Color,

    hovered_tab: Option<usize>,
    hovered_close: Option<usize>,
}

impl<S, C> TabBar<S, C>
where
    S: FnMut(usize),
    C: FnMut(usize),
{
    pub fn new(tabs: Vec<Tab>, active: usize, on_select: S, on_close: C) -> Self {
        Self {
            tabs,
            active,
            on_select,
            on_close,
            tab_height: 35.0,
            tab_min_width: 80.0,
            tab_max_width: 200.0,
            scroll_offset: 0.0,
            active_bg: Color::from_hex("#1e1e1e").unwrap_or(Color::BLACK),
            active_fg: Color::WHITE,
            inactive_bg: Color::from_hex("#2d2d2d").unwrap_or(Color::BLACK),
            inactive_fg: Color::from_hex("#ffffff80").unwrap_or(Color::WHITE),
            border_color: Color::from_hex("#252526").unwrap_or(Color::BLACK),
            dirty_dot_color: Color::from_hex("#e8e8e8").unwrap_or(Color::WHITE),
            close_hover_bg: Color::from_hex("#404040").unwrap_or(Color::BLACK),
            hovered_tab: None,
            hovered_close: None,
        }
    }

    #[allow(clippy::cast_precision_loss)]
    fn tab_width(&self) -> f32 {
        self.tab_min_width
            .max(self.tab_max_width.min(160.0))
    }

    fn tab_rect_at(&self, index: usize, container: Rect) -> Rect {
        let w = self.tab_width();
        Rect::new(
            container.x + index as f32 * w - self.scroll_offset,
            container.y,
            w,
            self.tab_height,
        )
    }

    fn close_button_rect(&self, tab_rect: Rect) -> Rect {
        let size = 16.0;
        Rect::new(
            tab_rect.x + tab_rect.width - size - 8.0,
            tab_rect.y + (tab_rect.height - size) / 2.0,
            size,
            size,
        )
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn tab_index_at(&self, x: f32, container: Rect) -> Option<usize> {
        let w = self.tab_width();
        let rel = x - container.x + self.scroll_offset;
        if rel < 0.0 {
            return None;
        }
        let idx = (rel / w) as usize;
        if idx < self.tabs.len() { Some(idx) } else { None }
    }
}

impl<S, C> Widget for TabBar<S, C>
where
    S: FnMut(usize),
    C: FnMut(usize),
{
    fn layout(&self) -> LayoutNode {
        LayoutNode {
            size: Size::Fixed(self.tab_height),
            ..LayoutNode::default()
        }
    }

    fn render(&self, rect: Rect, renderer: &mut GpuRenderer) {
        let mut rr = sidex_gpu::RectRenderer::new();

        for (i, tab) in self.tabs.iter().enumerate() {
            let tr = self.tab_rect_at(i, rect);
            if tr.right() < rect.x || tr.x > rect.right() {
                continue;
            }

            let is_active = i == self.active;
            let bg = if is_active { self.active_bg } else { self.inactive_bg };
            rr.draw_rect(tr.x, tr.y, tr.width, tr.height, bg, 0.0);

            rr.draw_rect(tr.right() - 1.0, tr.y, 1.0, tr.height, self.border_color, 0.0);

            if tab.is_dirty {
                let dot_r = 4.0;
                let close_r = self.close_button_rect(tr);
                rr.draw_rect(
                    close_r.x + close_r.width / 2.0 - dot_r,
                    close_r.y + close_r.height / 2.0 - dot_r,
                    dot_r * 2.0,
                    dot_r * 2.0,
                    self.dirty_dot_color,
                    dot_r,
                );
            }

            if self.hovered_close == Some(i) {
                let cr = self.close_button_rect(tr);
                rr.draw_rect(cr.x, cr.y, cr.width, cr.height, self.close_hover_bg, 2.0);
            }
        }
        let _ = renderer;
    }

    fn handle_event(&mut self, event: &UiEvent, rect: Rect) -> EventResult {
        match event {
            UiEvent::MouseMove { x, y } => {
                if rect.contains(*x, *y) {
                    self.hovered_tab = self.tab_index_at(*x, rect);
                    self.hovered_close = self.hovered_tab.filter(|&i| {
                        let tr = self.tab_rect_at(i, rect);
                        let cr = self.close_button_rect(tr);
                        cr.contains(*x, *y)
                    });
                } else {
                    self.hovered_tab = None;
                    self.hovered_close = None;
                }
                EventResult::Ignored
            }
            UiEvent::MouseDown {
                x,
                y,
                button: MouseButton::Left,
            } if rect.contains(*x, *y) => {
                if let Some(idx) = self.tab_index_at(*x, rect) {
                    let tr = self.tab_rect_at(idx, rect);
                    let cr = self.close_button_rect(tr);
                    if cr.contains(*x, *y) {
                        (self.on_close)(idx);
                    } else {
                        self.active = idx;
                        (self.on_select)(idx);
                    }
                    EventResult::Handled
                } else {
                    EventResult::Ignored
                }
            }
            UiEvent::MouseScroll { dx, .. } if rect.contains(0.0, rect.y) => {
                let total_w = self.tabs.len() as f32 * self.tab_width();
                let max = (total_w - rect.width).max(0.0);
                self.scroll_offset = (self.scroll_offset - dx * 30.0).clamp(0.0, max);
                EventResult::Handled
            }
            _ => EventResult::Ignored,
        }
    }
}
