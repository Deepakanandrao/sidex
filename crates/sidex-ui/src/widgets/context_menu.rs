//! Context menu with nested submenus, separators, and keyboard navigation.

use sidex_gpu::color::Color;
use sidex_gpu::GpuRenderer;

use crate::layout::{LayoutNode, Rect, Size};
use crate::widget::{EventResult, Key, MouseButton, UiEvent, Widget};

/// A single item in a context menu.
#[derive(Clone, Debug)]
pub enum MenuItem {
    /// A clickable action.
    Action {
        label: String,
        shortcut: Option<String>,
        icon: Option<String>,
        enabled: bool,
        checked: bool,
    },
    /// A submenu that expands on hover.
    Submenu {
        label: String,
        icon: Option<String>,
        items: Vec<MenuItem>,
    },
    /// A horizontal separator line.
    Separator,
}

impl MenuItem {
    /// Creates a simple enabled action.
    pub fn action(label: impl Into<String>) -> Self {
        Self::Action {
            label: label.into(),
            shortcut: None,
            icon: None,
            enabled: true,
            checked: false,
        }
    }

    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        if let Self::Action {
            shortcut: ref mut s,
            ..
        } = self
        {
            *s = Some(shortcut.into());
        }
        self
    }

    pub fn disabled(mut self) -> Self {
        if let Self::Action {
            enabled: ref mut e, ..
        } = self
        {
            *e = false;
        }
        self
    }

    pub fn submenu(label: impl Into<String>, items: Vec<MenuItem>) -> Self {
        Self::Submenu {
            label: label.into(),
            icon: None,
            items,
        }
    }

    fn is_separator(&self) -> bool {
        matches!(self, Self::Separator)
    }

    fn is_enabled(&self) -> bool {
        match self {
            Self::Action { enabled, .. } => *enabled,
            Self::Submenu { .. } => true,
            Self::Separator => false,
        }
    }
}

/// A popup context menu displayed at a screen position.
#[allow(dead_code)]
pub struct ContextMenu<F: FnMut(usize)> {
    pub items: Vec<MenuItem>,
    pub position: (f32, f32),
    pub on_select: F,

    row_height: f32,
    menu_width: f32,
    hovered_index: Option<usize>,
    active_submenu: Option<usize>,
    keyboard_index: Option<usize>,
    visible: bool,

    background: Color,
    border_color: Color,
    hover_bg: Color,
    foreground: Color,
    disabled_fg: Color,
    separator_color: Color,
    shortcut_fg: Color,
}

impl<F: FnMut(usize)> ContextMenu<F> {
    pub fn new(items: Vec<MenuItem>, position: (f32, f32), on_select: F) -> Self {
        Self {
            items,
            position,
            on_select,
            row_height: 26.0,
            menu_width: 220.0,
            hovered_index: None,
            active_submenu: None,
            keyboard_index: None,
            visible: true,
            background: Color::from_hex("#252526").unwrap_or(Color::BLACK),
            border_color: Color::from_hex("#454545").unwrap_or(Color::BLACK),
            hover_bg: Color::from_hex("#04395e").unwrap_or(Color::BLACK),
            foreground: Color::from_hex("#cccccc").unwrap_or(Color::WHITE),
            disabled_fg: Color::from_hex("#6b6b6b").unwrap_or(Color::WHITE),
            separator_color: Color::from_hex("#454545").unwrap_or(Color::BLACK),
            shortcut_fg: Color::from_hex("#aaaaaa").unwrap_or(Color::WHITE),
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.keyboard_index = None;
        self.hovered_index = None;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    fn menu_rect(&self) -> Rect {
        let h = self.items.iter().fold(0.0_f32, |acc, item| {
            acc + if item.is_separator() {
                9.0
            } else {
                self.row_height
            }
        });
        Rect::new(self.position.0, self.position.1, self.menu_width, h + 4.0)
    }

    fn item_rect_at(&self, index: usize) -> Rect {
        let base = self.menu_rect();
        let mut y = base.y + 2.0;
        for (i, item) in self.items.iter().enumerate() {
            let h = if item.is_separator() { 9.0 } else { self.row_height };
            if i == index {
                return Rect::new(base.x, y, base.width, h);
            }
            y += h;
        }
        Rect::ZERO
    }

    fn next_enabled(&self, from: usize, forward: bool) -> Option<usize> {
        let len = self.items.len();
        for offset in 1..=len {
            let idx = if forward {
                (from + offset) % len
            } else {
                (from + len - offset) % len
            };
            if self.items[idx].is_enabled() {
                return Some(idx);
            }
        }
        None
    }
}

impl<F: FnMut(usize)> Widget for ContextMenu<F> {
    fn layout(&self) -> LayoutNode {
        LayoutNode {
            size: Size::Auto,
            ..LayoutNode::default()
        }
    }

    fn render(&self, _rect: Rect, renderer: &mut GpuRenderer) {
        if !self.visible {
            return;
        }
        let mr = self.menu_rect();
        let mut rr = sidex_gpu::RectRenderer::new();

        rr.draw_rect(mr.x, mr.y, mr.width, mr.height, self.background, 4.0);
        rr.draw_border(mr.x, mr.y, mr.width, mr.height, self.border_color, 1.0);

        for (i, item) in self.items.iter().enumerate() {
            let ir = self.item_rect_at(i);
            if item.is_separator() {
                let cy = ir.y + ir.height / 2.0;
                rr.draw_rect(ir.x + 8.0, cy, ir.width - 16.0, 1.0, self.separator_color, 0.0);
                continue;
            }

            let is_hover = self.hovered_index == Some(i) || self.keyboard_index == Some(i);
            if is_hover && item.is_enabled() {
                rr.draw_rect(ir.x + 2.0, ir.y, ir.width - 4.0, ir.height, self.hover_bg, 2.0);
            }
        }
        let _ = renderer;
    }

    fn handle_event(&mut self, event: &UiEvent, _rect: Rect) -> EventResult {
        if !self.visible {
            return EventResult::Ignored;
        }
        let mr = self.menu_rect();

        match event {
            UiEvent::MouseMove { x, y } => {
                if mr.contains(*x, *y) {
                    self.hovered_index = None;
                    for (i, _) in self.items.iter().enumerate() {
                        let ir = self.item_rect_at(i);
                        if ir.contains(*x, *y) && !self.items[i].is_separator() {
                            self.hovered_index = Some(i);
                            if matches!(self.items[i], MenuItem::Submenu { .. }) {
                                self.active_submenu = Some(i);
                            }
                            break;
                        }
                    }
                    EventResult::Handled
                } else {
                    self.hovered_index = None;
                    EventResult::Ignored
                }
            }
            UiEvent::MouseDown {
                x,
                y,
                button: MouseButton::Left,
            } => {
                if mr.contains(*x, *y) {
                    if let Some(idx) = self.hovered_index {
                        if self.items[idx].is_enabled() && !matches!(self.items[idx], MenuItem::Submenu { .. }) {
                            (self.on_select)(idx);
                            self.hide();
                        }
                    }
                    EventResult::Handled
                } else {
                    self.hide();
                    EventResult::Handled
                }
            }
            UiEvent::KeyPress { key, .. } => match key {
                Key::ArrowDown => {
                    let from = self.keyboard_index.unwrap_or(self.items.len() - 1);
                    self.keyboard_index = self.next_enabled(from, true);
                    EventResult::Handled
                }
                Key::ArrowUp => {
                    let from = self.keyboard_index.unwrap_or(0);
                    self.keyboard_index = self.next_enabled(from, false);
                    EventResult::Handled
                }
                Key::Enter | Key::Space => {
                    if let Some(idx) = self.keyboard_index {
                        if self.items[idx].is_enabled()
                            && !matches!(self.items[idx], MenuItem::Submenu { .. })
                        {
                            (self.on_select)(idx);
                            self.hide();
                        }
                    }
                    EventResult::Handled
                }
                Key::Escape => {
                    self.hide();
                    EventResult::Handled
                }
                _ => EventResult::Ignored,
            },
            _ => EventResult::Ignored,
        }
    }
}
