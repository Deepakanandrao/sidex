//! Scrollbar widget with thumb dragging and click-to-page.

use sidex_gpu::color::Color;
use sidex_gpu::GpuRenderer;

use crate::layout::{LayoutNode, Rect, Size};
use crate::widget::{EventResult, MouseButton, UiEvent, Widget};

/// Scrollbar orientation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Orientation {
    #[default]
    Vertical,
    Horizontal,
}

/// A scrollbar with proportional thumb sizing and drag support.
pub struct Scrollbar<F: FnMut(f32)> {
    pub orientation: Orientation,
    /// Total content size in pixels.
    pub total: f32,
    /// Visible viewport size in pixels.
    pub visible: f32,
    /// Current scroll offset in pixels.
    pub position: f32,
    pub on_scroll: F,

    thumb_color: Color,
    thumb_hover_color: Color,
    thumb_active_color: Color,
    track_color: Color,

    dragging: bool,
    hovered: bool,
    /// Position along the track where the drag started.
    drag_start_offset: f32,
}

impl<F: FnMut(f32)> Scrollbar<F> {
    pub fn new(total: f32, visible: f32, position: f32, on_scroll: F) -> Self {
        Self {
            orientation: Orientation::Vertical,
            total,
            visible,
            position,
            on_scroll,
            thumb_color: Color::from_hex("#79797966").unwrap_or(Color::WHITE),
            thumb_hover_color: Color::from_hex("#646464b3").unwrap_or(Color::WHITE),
            thumb_active_color: Color::from_hex("#bfbfbf66").unwrap_or(Color::WHITE),
            track_color: Color::TRANSPARENT,
            dragging: false,
            hovered: false,
            drag_start_offset: 0.0,
        }
    }

    pub fn horizontal(mut self) -> Self {
        self.orientation = Orientation::Horizontal;
        self
    }

    fn track_length(&self, rect: Rect) -> f32 {
        match self.orientation {
            Orientation::Vertical => rect.height,
            Orientation::Horizontal => rect.width,
        }
    }

    fn thumb_rect(&self, rect: Rect) -> Rect {
        if self.total <= 0.0 || self.visible >= self.total {
            return rect;
        }
        let track = self.track_length(rect);
        let thumb_size = (self.visible / self.total * track).max(20.0).min(track);
        let max_offset = self.total - self.visible;
        let ratio = if max_offset > 0.0 {
            self.position / max_offset
        } else {
            0.0
        };
        let thumb_pos = ratio * (track - thumb_size);

        match self.orientation {
            Orientation::Vertical => Rect::new(rect.x, rect.y + thumb_pos, rect.width, thumb_size),
            Orientation::Horizontal => {
                Rect::new(rect.x + thumb_pos, rect.y, thumb_size, rect.height)
            }
        }
    }

    fn position_from_track(&self, track_pos: f32, rect: Rect) -> f32 {
        let track = self.track_length(rect);
        let thumb_size = (self.visible / self.total * track).max(20.0).min(track);
        let usable = track - thumb_size;
        if usable <= 0.0 {
            return 0.0;
        }
        let ratio = (track_pos / usable).clamp(0.0, 1.0);
        ratio * (self.total - self.visible)
    }

    fn event_pos(&self, x: f32, y: f32, rect: Rect) -> f32 {
        match self.orientation {
            Orientation::Vertical => y - rect.y,
            Orientation::Horizontal => x - rect.x,
        }
    }
}

impl<F: FnMut(f32)> Widget for Scrollbar<F> {
    fn layout(&self) -> LayoutNode {
        let cross = 14.0;
        LayoutNode {
            size: Size::Fixed(cross),
            ..LayoutNode::default()
        }
    }

    fn render(&self, rect: Rect, renderer: &mut GpuRenderer) {
        let mut rr = sidex_gpu::RectRenderer::new();

        rr.draw_rect(
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            self.track_color,
            0.0,
        );

        let thumb = self.thumb_rect(rect);
        let thumb_color = if self.dragging {
            self.thumb_active_color
        } else if self.hovered {
            self.thumb_hover_color
        } else {
            self.thumb_color
        };
        rr.draw_rect(thumb.x, thumb.y, thumb.width, thumb.height, thumb_color, 3.0);

        let _ = renderer;
    }

    fn handle_event(&mut self, event: &UiEvent, rect: Rect) -> EventResult {
        match event {
            UiEvent::MouseDown {
                x,
                y,
                button: MouseButton::Left,
            } if rect.contains(*x, *y) => {
                let thumb = self.thumb_rect(rect);
                let pos = self.event_pos(*x, *y, rect);
                if thumb.contains(*x, *y) {
                    self.dragging = true;
                    let thumb_start = match self.orientation {
                        Orientation::Vertical => thumb.y - rect.y,
                        Orientation::Horizontal => thumb.x - rect.x,
                    };
                    self.drag_start_offset = pos - thumb_start;
                } else {
                    let new_pos = self.position_from_track(pos - self.track_length(rect) * self.visible / self.total / 2.0, rect);
                    self.position = new_pos;
                    (self.on_scroll)(self.position);
                }
                EventResult::Handled
            }
            UiEvent::MouseMove { x, y } => {
                if self.dragging {
                    let pos = self.event_pos(*x, *y, rect);
                    let new_pos =
                        self.position_from_track(pos - self.drag_start_offset, rect);
                    self.position = new_pos;
                    (self.on_scroll)(self.position);
                    EventResult::Handled
                } else {
                    let thumb = self.thumb_rect(rect);
                    self.hovered = thumb.contains(*x, *y);
                    EventResult::Ignored
                }
            }
            UiEvent::MouseUp { .. } if self.dragging => {
                self.dragging = false;
                EventResult::Handled
            }
            UiEvent::MouseScroll { dy, .. } if rect.contains(0.0, 0.0) => {
                let max = (self.total - self.visible).max(0.0);
                self.position = (self.position - dy * 40.0).clamp(0.0, max);
                (self.on_scroll)(self.position);
                EventResult::Handled
            }
            _ => EventResult::Ignored,
        }
    }
}
