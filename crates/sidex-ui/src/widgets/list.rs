//! Virtual-scrolling list widget with single and multi-select.

use sidex_gpu::color::Color;
use sidex_gpu::GpuRenderer;

use crate::layout::{LayoutNode, Rect, Size};
use crate::widget::{EventResult, Key, MouseButton, UiEvent, Widget};

/// Selection mode for the list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectionMode {
    #[default]
    Single,
    Multi,
}

/// A virtual-scrolling list that only renders visible items.
#[allow(dead_code)]
pub struct List<T, R, S>
where
    R: Fn(&T, usize, bool) -> ListRow,
    S: FnMut(usize),
{
    pub items: Vec<T>,
    pub render_item: R,
    pub on_select: S,
    pub selected: Vec<usize>,
    pub selection_mode: SelectionMode,

    row_height: f32,
    scroll_offset: f32,
    focused: bool,

    hover_bg: Color,
    selected_bg: Color,
    selected_fg: Color,
}

/// Pre-rendered description of a single list row.
pub struct ListRow {
    pub text: String,
    pub icon: Option<String>,
    pub description: Option<String>,
}

impl<T, R, S> List<T, R, S>
where
    R: Fn(&T, usize, bool) -> ListRow,
    S: FnMut(usize),
{
    pub fn new(items: Vec<T>, render_item: R, on_select: S) -> Self {
        Self {
            items,
            render_item,
            on_select,
            selected: Vec::new(),
            selection_mode: SelectionMode::Single,
            row_height: 22.0,
            scroll_offset: 0.0,
            focused: false,
            hover_bg: Color::from_hex("#2a2d2e").unwrap_or(Color::BLACK),
            selected_bg: Color::from_hex("#04395e").unwrap_or(Color::BLACK),
            selected_fg: Color::WHITE,
        }
    }

    pub fn with_selection_mode(mut self, mode: SelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    /// Range of item indices currently visible in the viewport.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn visible_range(&self, rect: Rect) -> (usize, usize) {
        let first = (self.scroll_offset / self.row_height).floor() as usize;
        let count = (rect.height / self.row_height).ceil() as usize + 1;
        let last = (first + count).min(self.items.len());
        (first, last)
    }

    fn total_height(&self) -> f32 {
        self.items.len() as f32 * self.row_height
    }

    fn ensure_visible(&mut self, index: usize, rect: Rect) {
        let top = index as f32 * self.row_height;
        let bottom = top + self.row_height;
        if top < self.scroll_offset {
            self.scroll_offset = top;
        } else if bottom > self.scroll_offset + rect.height {
            self.scroll_offset = bottom - rect.height;
        }
    }

    fn primary_selected(&self) -> Option<usize> {
        self.selected.last().copied()
    }

    fn select_index(&mut self, index: usize, toggle: bool) {
        match self.selection_mode {
            SelectionMode::Single => {
                self.selected = vec![index];
            }
            SelectionMode::Multi if toggle => {
                if let Some(pos) = self.selected.iter().position(|&i| i == index) {
                    self.selected.remove(pos);
                } else {
                    self.selected.push(index);
                }
            }
            SelectionMode::Multi => {
                self.selected = vec![index];
            }
        }
        (self.on_select)(index);
    }
}

impl<T, R, S> Widget for List<T, R, S>
where
    R: Fn(&T, usize, bool) -> ListRow,
    S: FnMut(usize),
{
    fn layout(&self) -> LayoutNode {
        LayoutNode {
            size: Size::Flex(1.0),
            ..LayoutNode::default()
        }
    }

    fn render(&self, rect: Rect, renderer: &mut GpuRenderer) {
        let mut rr = sidex_gpu::RectRenderer::new();
        let (first, last) = self.visible_range(rect);

        for i in first..last {
            let is_selected = self.selected.contains(&i);
            let y = rect.y + i as f32 * self.row_height - self.scroll_offset;

            if is_selected {
                rr.draw_rect(rect.x, y, rect.width, self.row_height, self.selected_bg, 0.0);
            }

            let _row = (self.render_item)(&self.items[i], i, is_selected);
        }
        let _ = renderer;
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
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
                let index = ((y - rect.y + self.scroll_offset) / self.row_height).floor() as usize;
                if index < self.items.len() {
                    self.select_index(index, false);
                }
                EventResult::Handled
            }
            UiEvent::MouseScroll { dy, .. } => {
                let max = (self.total_height() - rect.height).max(0.0);
                self.scroll_offset = (self.scroll_offset - dy * 40.0).clamp(0.0, max);
                EventResult::Handled
            }
            UiEvent::KeyPress { key, modifiers } if self.focused => {
                let current = self.primary_selected().unwrap_or(0);
                match key {
                    Key::ArrowDown => {
                        let next = (current + 1).min(self.items.len().saturating_sub(1));
                        self.select_index(next, modifiers.shift);
                        self.ensure_visible(next, rect);
                        EventResult::Handled
                    }
                    Key::ArrowUp => {
                        let next = current.saturating_sub(1);
                        self.select_index(next, modifiers.shift);
                        self.ensure_visible(next, rect);
                        EventResult::Handled
                    }
                    Key::Home => {
                        self.select_index(0, false);
                        self.ensure_visible(0, rect);
                        EventResult::Handled
                    }
                    Key::End => {
                        let last = self.items.len().saturating_sub(1);
                        self.select_index(last, false);
                        self.ensure_visible(last, rect);
                        EventResult::Handled
                    }
                    Key::PageDown => {
                        let page = (rect.height / self.row_height) as usize;
                        let next = (current + page).min(self.items.len().saturating_sub(1));
                        self.select_index(next, false);
                        self.ensure_visible(next, rect);
                        EventResult::Handled
                    }
                    Key::PageUp => {
                        let page = (rect.height / self.row_height) as usize;
                        let next = current.saturating_sub(page);
                        self.select_index(next, false);
                        self.ensure_visible(next, rect);
                        EventResult::Handled
                    }
                    _ => EventResult::Ignored,
                }
            }
            _ => EventResult::Ignored,
        }
    }
}
