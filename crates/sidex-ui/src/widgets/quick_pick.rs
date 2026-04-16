//! Command palette / quick-pick widget with fuzzy filtering.

use sidex_gpu::color::Color;
use sidex_gpu::GpuRenderer;

use crate::layout::{Edges, LayoutNode, Rect, Size};
use crate::widget::{EventResult, Key, MouseButton, UiEvent, Widget};

/// A single item in the quick-pick list.
#[derive(Clone, Debug)]
pub struct QuickPickItem {
    pub label: String,
    pub description: Option<String>,
    pub detail: Option<String>,
    pub group: Option<String>,
    /// Whether this item is picked in multi-select mode.
    pub picked: bool,
}

impl QuickPickItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: None,
            detail: None,
            group: None,
            picked: false,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }
}

/// A command-palette style picker with fuzzy filter and optional multi-select.
#[allow(dead_code)]
pub struct QuickPick<F: FnMut(usize)> {
    pub items: Vec<QuickPickItem>,
    pub placeholder: String,
    pub on_select: F,
    pub multi_select: bool,

    filter_text: String,
    filtered_indices: Vec<usize>,
    selected_index: usize,
    scroll_offset: f32,
    visible: bool,

    row_height: f32,
    max_visible_items: usize,
    width: f32,

    background: Color,
    border_color: Color,
    input_bg: Color,
    foreground: Color,
    highlight_fg: Color,
    selected_bg: Color,
    description_fg: Color,
}

impl<F: FnMut(usize)> QuickPick<F> {
    pub fn new(items: Vec<QuickPickItem>, on_select: F) -> Self {
        let count = items.len();
        let indices: Vec<usize> = (0..count).collect();
        Self {
            items,
            placeholder: "Type to search...".into(),
            on_select,
            multi_select: false,
            filter_text: String::new(),
            filtered_indices: indices,
            selected_index: 0,
            scroll_offset: 0.0,
            visible: true,
            row_height: 26.0,
            max_visible_items: 12,
            width: 600.0,
            background: Color::from_hex("#252526").unwrap_or(Color::BLACK),
            border_color: Color::from_hex("#454545").unwrap_or(Color::BLACK),
            input_bg: Color::from_hex("#3c3c3c").unwrap_or(Color::BLACK),
            foreground: Color::from_hex("#cccccc").unwrap_or(Color::WHITE),
            highlight_fg: Color::from_hex("#18a3ff").unwrap_or(Color::WHITE),
            selected_bg: Color::from_hex("#04395e").unwrap_or(Color::BLACK),
            description_fg: Color::from_hex("#aaaaaa").unwrap_or(Color::WHITE),
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.filter_text.clear();
        self.selected_index = 0;
        self.scroll_offset = 0.0;
        self.refilter();
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    fn refilter(&mut self) {
        if self.filter_text.is_empty() {
            self.filtered_indices = (0..self.items.len()).collect();
        } else {
            let query = self.filter_text.to_lowercase();
            self.filtered_indices = (0..self.items.len())
                .filter(|&i| fuzzy_match(&self.items[i].label, &query))
                .collect();
        }
        self.selected_index = 0;
        self.scroll_offset = 0.0;
    }

    fn visible_count(&self) -> usize {
        self.filtered_indices.len().min(self.max_visible_items)
    }

    fn list_height(&self) -> f32 {
        self.visible_count() as f32 * self.row_height
    }

    fn input_height(&self) -> f32 {
        32.0
    }

    fn panel_rect(&self, viewport_width: f32) -> Rect {
        let x = (viewport_width - self.width) / 2.0;
        let total_h = self.input_height() + self.list_height() + 8.0;
        Rect::new(x.max(0.0), 80.0, self.width, total_h)
    }

    fn ensure_selected_visible(&mut self) {
        let top = self.selected_index as f32 * self.row_height;
        let bottom = top + self.row_height;
        let vis = self.visible_count() as f32 * self.row_height;
        if top < self.scroll_offset {
            self.scroll_offset = top;
        } else if bottom > self.scroll_offset + vis {
            self.scroll_offset = bottom - vis;
        }
    }
}

impl<F: FnMut(usize)> Widget for QuickPick<F> {
    fn layout(&self) -> LayoutNode {
        LayoutNode {
            size: Size::Auto,
            padding: Edges::all(0.0),
            ..LayoutNode::default()
        }
    }

    fn render(&self, rect: Rect, renderer: &mut GpuRenderer) {
        if !self.visible {
            return;
        }
        let pr = self.panel_rect(rect.width);
        let mut rr = sidex_gpu::RectRenderer::new();

        rr.draw_rect(pr.x, pr.y, pr.width, pr.height, self.background, 6.0);
        rr.draw_border(pr.x, pr.y, pr.width, pr.height, self.border_color, 1.0);

        let input_r = Rect::new(pr.x + 8.0, pr.y + 4.0, pr.width - 16.0, self.input_height());
        rr.draw_rect(
            input_r.x,
            input_r.y,
            input_r.width,
            input_r.height,
            self.input_bg,
            2.0,
        );

        let list_y = pr.y + self.input_height() + 4.0;
        for (vi, &item_idx) in self.filtered_indices.iter().enumerate() {
            let y = list_y + vi as f32 * self.row_height - self.scroll_offset;
            if y + self.row_height < list_y || y > list_y + self.list_height() {
                continue;
            }
            if vi == self.selected_index {
                rr.draw_rect(
                    pr.x + 4.0,
                    y,
                    pr.width - 8.0,
                    self.row_height,
                    self.selected_bg,
                    2.0,
                );
            }
            let _ = &self.items[item_idx];
        }
        let _ = renderer;
    }

    fn handle_event(&mut self, event: &UiEvent, rect: Rect) -> EventResult {
        if !self.visible {
            return EventResult::Ignored;
        }

        match event {
            UiEvent::KeyPress { key, modifiers } => match key {
                Key::Escape => {
                    self.hide();
                    EventResult::Handled
                }
                Key::ArrowDown => {
                    if !self.filtered_indices.is_empty() {
                        self.selected_index =
                            (self.selected_index + 1).min(self.filtered_indices.len() - 1);
                        self.ensure_selected_visible();
                    }
                    EventResult::Handled
                }
                Key::ArrowUp => {
                    self.selected_index = self.selected_index.saturating_sub(1);
                    self.ensure_selected_visible();
                    EventResult::Handled
                }
                Key::Enter => {
                    if let Some(&item_idx) = self.filtered_indices.get(self.selected_index) {
                        if self.multi_select {
                            self.items[item_idx].picked = !self.items[item_idx].picked;
                        }
                        (self.on_select)(item_idx);
                        if !self.multi_select {
                            self.hide();
                        }
                    }
                    EventResult::Handled
                }
                Key::Backspace => {
                    self.filter_text.pop();
                    self.refilter();
                    EventResult::Handled
                }
                Key::Char(ch) if !modifiers.command() => {
                    self.filter_text.push(*ch);
                    self.refilter();
                    EventResult::Handled
                }
                _ => EventResult::Ignored,
            },
            UiEvent::MouseDown {
                x,
                y,
                button: MouseButton::Left,
            } => {
                let pr = self.panel_rect(rect.width);
                if !pr.contains(*x, *y) {
                    self.hide();
                    return EventResult::Handled;
                }

                let list_y = pr.y + self.input_height() + 4.0;
                if *y >= list_y {
                    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                    let vi = ((y - list_y + self.scroll_offset) / self.row_height) as usize;
                    if let Some(&item_idx) = self.filtered_indices.get(vi) {
                        self.selected_index = vi;
                        (self.on_select)(item_idx);
                        if !self.multi_select {
                            self.hide();
                        }
                    }
                }
                EventResult::Handled
            }
            _ => EventResult::Ignored,
        }
    }
}

/// Simple case-insensitive substring fuzzy match.
fn fuzzy_match(haystack: &str, query: &str) -> bool {
    let hay = haystack.to_lowercase();
    let mut hay_chars = hay.chars();
    for qc in query.chars() {
        loop {
            match hay_chars.next() {
                Some(hc) if hc == qc => break,
                Some(_) => continue,
                None => return false,
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_match_basic() {
        assert!(fuzzy_match("Open File", "opfi"));
        assert!(fuzzy_match("Toggle Sidebar", "tgsb"));
        assert!(!fuzzy_match("abc", "abdc"));
    }

    #[test]
    fn fuzzy_match_case_insensitive() {
        assert!(fuzzy_match("FooBar", "foob"));
        assert!(fuzzy_match("FooBar", "fb"));
    }
}
