//! Notification toast widget with auto-dismiss and severity levels.

use sidex_gpu::color::Color;
use sidex_gpu::GpuRenderer;

use crate::layout::{Edges, LayoutNode, Rect, Size};
use crate::widget::{EventResult, MouseButton, UiEvent, Widget};

/// Severity level of a notification.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Severity {
    #[default]
    Info,
    Warning,
    Error,
}

/// An action button on a notification.
#[derive(Clone, Debug)]
pub struct NotificationAction {
    pub label: String,
}

impl NotificationAction {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
        }
    }
}

/// A notification toast that appears in a corner of the screen.
#[allow(dead_code)]
pub struct NotificationToast<F: FnMut()> {
    pub message: String,
    pub severity: Severity,
    pub actions: Vec<NotificationAction>,
    pub on_dismiss: F,

    /// Seconds until auto-dismiss (0 = no auto-dismiss).
    pub auto_dismiss_secs: f32,
    /// Elapsed time in seconds since the notification appeared.
    pub elapsed: f32,

    visible: bool,
    hovered: bool,

    toast_width: f32,
    toast_height: f32,
    background: Color,
    foreground: Color,
    border_color: Color,
    info_accent: Color,
    warning_accent: Color,
    error_accent: Color,
}

impl<F: FnMut()> NotificationToast<F> {
    pub fn new(message: impl Into<String>, severity: Severity, on_dismiss: F) -> Self {
        Self {
            message: message.into(),
            severity,
            actions: Vec::new(),
            on_dismiss,
            auto_dismiss_secs: 8.0,
            elapsed: 0.0,
            visible: true,
            hovered: false,
            toast_width: 400.0,
            toast_height: 64.0,
            background: Color::from_hex("#252526").unwrap_or(Color::BLACK),
            foreground: Color::from_hex("#cccccc").unwrap_or(Color::WHITE),
            border_color: Color::from_hex("#454545").unwrap_or(Color::BLACK),
            info_accent: Color::from_hex("#3794ff").unwrap_or(Color::WHITE),
            warning_accent: Color::from_hex("#cca700").unwrap_or(Color::WHITE),
            error_accent: Color::from_hex("#f14c4c").unwrap_or(Color::WHITE),
        }
    }

    pub fn with_actions(mut self, actions: Vec<NotificationAction>) -> Self {
        self.actions = actions;
        self
    }

    pub fn with_auto_dismiss(mut self, secs: f32) -> Self {
        self.auto_dismiss_secs = secs;
        self
    }

    /// Advances the auto-dismiss timer. Returns `true` if dismissed.
    pub fn tick(&mut self, dt: f32) -> bool {
        if !self.visible || self.hovered {
            return false;
        }
        self.elapsed += dt;
        if self.auto_dismiss_secs > 0.0 && self.elapsed >= self.auto_dismiss_secs {
            self.visible = false;
            (self.on_dismiss)();
            return true;
        }
        false
    }

    fn accent_color(&self) -> Color {
        match self.severity {
            Severity::Info => self.info_accent,
            Severity::Warning => self.warning_accent,
            Severity::Error => self.error_accent,
        }
    }

    /// Computes the toast rect positioned at the bottom-right of the viewport.
    fn toast_rect(&self, viewport: Rect, stack_index: usize) -> Rect {
        let margin = 12.0;
        let y_offset = stack_index as f32 * (self.toast_height + 8.0);
        Rect::new(
            viewport.x + viewport.width - self.toast_width - margin,
            viewport.y + viewport.height - self.toast_height - margin - y_offset,
            self.toast_width,
            self.toast_height,
        )
    }
}

impl<F: FnMut()> Widget for NotificationToast<F> {
    fn layout(&self) -> LayoutNode {
        LayoutNode {
            size: Size::Fixed(self.toast_height),
            padding: Edges::symmetric(12.0, 8.0),
            ..LayoutNode::default()
        }
    }

    fn render(&self, rect: Rect, renderer: &mut GpuRenderer) {
        if !self.visible {
            return;
        }
        let tr = self.toast_rect(rect, 0);
        let mut rr = sidex_gpu::RectRenderer::new();

        rr.draw_rect(tr.x, tr.y, tr.width, tr.height, self.background, 4.0);
        rr.draw_border(tr.x, tr.y, tr.width, tr.height, self.border_color, 1.0);

        let accent = self.accent_color();
        rr.draw_rect(tr.x, tr.y, 3.0, tr.height, accent, 0.0);

        let _ = renderer;
    }

    fn handle_event(&mut self, event: &UiEvent, rect: Rect) -> EventResult {
        if !self.visible {
            return EventResult::Ignored;
        }
        let tr = self.toast_rect(rect, 0);

        match event {
            UiEvent::MouseMove { x, y } => {
                self.hovered = tr.contains(*x, *y);
                EventResult::Ignored
            }
            UiEvent::MouseDown {
                x,
                y,
                button: MouseButton::Left,
            } if tr.contains(*x, *y) => {
                self.visible = false;
                (self.on_dismiss)();
                EventResult::Handled
            }
            _ => EventResult::Ignored,
        }
    }
}
