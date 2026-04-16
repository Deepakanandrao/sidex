//! Editor area with split groups, tab bars, and drop zones.

use sidex_gpu::color::Color;
use sidex_gpu::GpuRenderer;

use crate::layout::{Direction, LayoutNode, Rect, Size};
use crate::widget::{EventResult, MouseButton, UiEvent, Widget};
use crate::widgets::tabs::Tab;

/// An editor group containing a tab bar and a content area.
pub struct EditorGroup {
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
}

impl EditorGroup {
    pub fn new(tabs: Vec<Tab>, active_tab: usize) -> Self {
        Self { tabs, active_tab }
    }
}

/// Drop zone locations for drag-and-drop in the editor area.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropZone {
    Left,
    Right,
    Top,
    Bottom,
    Center,
}

/// The editor area — holds one or more editor groups in a split layout.
pub struct EditorArea<S, C>
where
    S: FnMut(usize, usize),
    C: FnMut(usize, usize),
{
    pub groups: Vec<EditorGroup>,
    pub active_group: usize,
    pub split_direction: Direction,
    pub on_tab_select: S,
    pub on_tab_close: C,

    group_sizes: Vec<f32>,
    tab_height: f32,
    background: Color,
    border_color: Color,
    drop_highlight: Color,
    active_drop_zone: Option<DropZone>,
}

impl<S, C> EditorArea<S, C>
where
    S: FnMut(usize, usize),
    C: FnMut(usize, usize),
{
    pub fn new(groups: Vec<EditorGroup>, on_tab_select: S, on_tab_close: C) -> Self {
        let len = groups.len();
        Self {
            groups,
            active_group: 0,
            split_direction: Direction::Row,
            on_tab_select,
            on_tab_close,
            group_sizes: vec![1.0; len],
            tab_height: 35.0,
            background: Color::from_hex("#1e1e1e").unwrap_or(Color::BLACK),
            border_color: Color::from_hex("#252526").unwrap_or(Color::BLACK),
            drop_highlight: Color::from_hex("#007acc44").unwrap_or(Color::BLACK),
            active_drop_zone: None,
        }
    }

    fn group_rects(&self, rect: Rect) -> Vec<Rect> {
        let is_row = self.split_direction == Direction::Row;
        let total = if is_row { rect.width } else { rect.height };
        let weight_sum: f32 = self.group_sizes.iter().sum();
        let mut cursor = if is_row { rect.x } else { rect.y };
        let mut result = Vec::new();

        for &w in &self.group_sizes {
            let size = if weight_sum > 0.0 {
                total * w / weight_sum
            } else {
                0.0
            };
            let gr = if is_row {
                Rect::new(cursor, rect.y, size, rect.height)
            } else {
                Rect::new(rect.x, cursor, rect.width, size)
            };
            result.push(gr);
            cursor += size;
        }
        result
    }
}

impl<S, C> Widget for EditorArea<S, C>
where
    S: FnMut(usize, usize),
    C: FnMut(usize, usize),
{
    fn layout(&self) -> LayoutNode {
        LayoutNode {
            direction: self.split_direction,
            size: Size::Flex(1.0),
            children: self
                .group_sizes
                .iter()
                .map(|&w| LayoutNode {
                    size: Size::Flex(w),
                    ..LayoutNode::default()
                })
                .collect(),
            ..LayoutNode::default()
        }
    }

    fn render(&self, rect: Rect, renderer: &mut GpuRenderer) {
        let mut rr = sidex_gpu::RectRenderer::new();
        let group_rects = self.group_rects(rect);

        for (gi, gr) in group_rects.iter().enumerate() {
            rr.draw_rect(gr.x, gr.y, gr.width, gr.height, self.background, 0.0);

            let tab_rect = Rect::new(gr.x, gr.y, gr.width, self.tab_height);
            rr.draw_rect(
                tab_rect.x,
                tab_rect.y + tab_rect.height - 1.0,
                tab_rect.width,
                1.0,
                self.border_color,
                0.0,
            );

            if gi + 1 < group_rects.len() {
                let bx = gr.right() - 1.0;
                rr.draw_rect(bx, gr.y, 1.0, gr.height, self.border_color, 0.0);
            }
        }

        if let Some(zone) = self.active_drop_zone {
            let drop_rect = match zone {
                DropZone::Left => Rect::new(rect.x, rect.y, rect.width / 2.0, rect.height),
                DropZone::Right => Rect::new(
                    rect.x + rect.width / 2.0,
                    rect.y,
                    rect.width / 2.0,
                    rect.height,
                ),
                DropZone::Top => Rect::new(rect.x, rect.y, rect.width, rect.height / 2.0),
                DropZone::Bottom => Rect::new(
                    rect.x,
                    rect.y + rect.height / 2.0,
                    rect.width,
                    rect.height / 2.0,
                ),
                DropZone::Center => rect,
            };
            rr.draw_rect(
                drop_rect.x,
                drop_rect.y,
                drop_rect.width,
                drop_rect.height,
                self.drop_highlight,
                0.0,
            );
        }
        let _ = renderer;
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn handle_event(&mut self, event: &UiEvent, rect: Rect) -> EventResult {
        let group_rects = self.group_rects(rect);

        match event {
            UiEvent::MouseDown {
                x,
                y,
                button: MouseButton::Left,
            } => {
                for (gi, gr) in group_rects.iter().enumerate() {
                    let tab_rect = Rect::new(gr.x, gr.y, gr.width, self.tab_height);
                    if tab_rect.contains(*x, *y) && gi < self.groups.len() {
                        self.active_group = gi;
                        let group = &self.groups[gi];
                        let tab_w = 160.0_f32.min(gr.width / group.tabs.len().max(1) as f32);
                        let tab_idx = ((x - gr.x) / tab_w) as usize;
                        let tab_idx = tab_idx.min(group.tabs.len().saturating_sub(1));
                        (self.on_tab_select)(gi, tab_idx);
                        return EventResult::Handled;
                    }
                }
                EventResult::Ignored
            }
            _ => EventResult::Ignored,
        }
    }
}
