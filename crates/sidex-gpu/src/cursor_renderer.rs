//! Cursor rendering with style variants, smooth blink animation, and smooth
//! position transitions.
//!
//! Supports multiple cursors for multi-cursor editing. Each cursor is drawn
//! using the [`RectRenderer`] with per-frame animation state updates.

use crate::color::Color;
use crate::rect_renderer::RectRenderer;

// ---------------------------------------------------------------------------
// Cursor style
// ---------------------------------------------------------------------------

/// Visual style of the editor cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CursorStyle {
    /// Thin vertical line (default).
    #[default]
    Line,
    /// Solid block covering the character cell.
    Block,
    /// Horizontal underline below the character.
    Underline,
    /// Extra-thin vertical line (1 px).
    LineThin,
    /// Block drawn as an outline only.
    BlockOutline,
    /// Extra-thin underline (1 px).
    UnderlineThin,
}

// ---------------------------------------------------------------------------
// Animation parameters
// ---------------------------------------------------------------------------

/// Configuration for cursor animation.
#[derive(Debug, Clone)]
pub struct CursorAnimConfig {
    /// Duration of a full blink cycle (on + off) in seconds.
    pub blink_period: f32,
    /// Time in seconds to fade between visible and hidden states.
    pub blink_fade_time: f32,
    /// Time in seconds for the cursor to glide to a new position.
    pub move_duration: f32,
    /// Whether blinking is enabled.
    pub blink_enabled: bool,
}

impl Default for CursorAnimConfig {
    fn default() -> Self {
        Self {
            blink_period: 1.0,
            blink_fade_time: 0.15,
            move_duration: 0.12,
            blink_enabled: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Cursor position
// ---------------------------------------------------------------------------

/// A cursor position in screen-space pixel coordinates.
#[derive(Debug, Clone, Copy)]
pub struct CursorPosition {
    /// X coordinate (left edge).
    pub x: f32,
    /// Y coordinate (top edge).
    pub y: f32,
    /// Character cell width.
    pub cell_width: f32,
    /// Character cell height (line height).
    pub cell_height: f32,
}

// ---------------------------------------------------------------------------
// Per-cursor animation state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct AnimatedPosition {
    current_x: f32,
    current_y: f32,
    target_x: f32,
    target_y: f32,
    move_t: f32,
}

impl AnimatedPosition {
    fn new(x: f32, y: f32) -> Self {
        Self {
            current_x: x,
            current_y: y,
            target_x: x,
            target_y: y,
            move_t: 1.0,
        }
    }

    fn set_target(&mut self, x: f32, y: f32) {
        if (self.target_x - x).abs() > 0.5 || (self.target_y - y).abs() > 0.5 {
            self.target_x = x;
            self.target_y = y;
            self.move_t = 0.0;
        }
    }

    fn advance(&mut self, dt: f32, duration: f32) {
        if self.move_t >= 1.0 {
            return;
        }
        self.move_t = (self.move_t + dt / duration.max(0.001)).min(1.0);
        let ease = ease_out_cubic(self.move_t);
        self.current_x += (self.target_x - self.current_x) * ease;
        self.current_y += (self.target_y - self.current_y) * ease;
    }
}

fn ease_out_cubic(t: f32) -> f32 {
    let inv = 1.0 - t;
    1.0 - inv * inv * inv
}

// ---------------------------------------------------------------------------
// CursorRenderer
// ---------------------------------------------------------------------------

/// Draws one or more blinking cursors with smooth animation.
pub struct CursorRenderer {
    style: CursorStyle,
    color: Color,
    config: CursorAnimConfig,
    /// Accumulated blink time (wraps around at `blink_period`).
    blink_clock: f32,
    /// Whether the blink timer should be reset on the next update (e.g. after
    /// a keypress the cursor should stay fully visible for a moment).
    reset_blink: bool,
    /// Per-cursor animated positions, indexed the same as the positions slice
    /// passed to [`render`](Self::render).
    anim_positions: Vec<AnimatedPosition>,
}

impl CursorRenderer {
    pub fn new(style: CursorStyle, color: Color) -> Self {
        Self {
            style,
            color,
            config: CursorAnimConfig::default(),
            blink_clock: 0.0,
            reset_blink: false,
            anim_positions: Vec::new(),
        }
    }

    pub fn set_style(&mut self, style: CursorStyle) {
        self.style = style;
    }

    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    pub fn config_mut(&mut self) -> &mut CursorAnimConfig {
        &mut self.config
    }

    /// Signal that the cursor moved (resets blink to fully visible).
    pub fn signal_activity(&mut self) {
        self.reset_blink = true;
    }

    /// Advances animation timers. Call once per frame with delta time.
    pub fn update(&mut self, dt: f32) {
        if self.reset_blink {
            self.blink_clock = 0.0;
            self.reset_blink = false;
        } else if self.config.blink_enabled {
            self.blink_clock = (self.blink_clock + dt) % self.config.blink_period;
        }

        for anim in &mut self.anim_positions {
            anim.advance(dt, self.config.move_duration);
        }
    }

    /// Renders all cursors into the given [`RectRenderer`].
    ///
    /// `positions` should contain one entry per active cursor.
    pub fn render(&mut self, rects: &mut RectRenderer, positions: &[CursorPosition]) {
        // Resize the animated positions buffer to match the number of cursors.
        while self.anim_positions.len() < positions.len() {
            let p = &positions[self.anim_positions.len()];
            self.anim_positions.push(AnimatedPosition::new(p.x, p.y));
        }
        self.anim_positions.truncate(positions.len());

        for (anim, pos) in self.anim_positions.iter_mut().zip(positions.iter()) {
            anim.set_target(pos.x, pos.y);
        }

        let alpha = self.blink_alpha();
        if alpha < 0.01 {
            return;
        }

        let mut draw_color = self.color;
        draw_color.a *= alpha;

        for (anim, pos) in self.anim_positions.iter().zip(positions.iter()) {
            let x = anim.current_x;
            let y = anim.current_y;
            let cw = pos.cell_width;
            let ch = pos.cell_height;

            match self.style {
                CursorStyle::Line => {
                    rects.draw_rect(x, y, 2.0, ch, draw_color, 0.0);
                }
                CursorStyle::LineThin => {
                    rects.draw_rect(x, y, 1.0, ch, draw_color, 0.0);
                }
                CursorStyle::Block => {
                    rects.draw_rect(x, y, cw, ch, draw_color, 0.0);
                }
                CursorStyle::BlockOutline => {
                    rects.draw_border(x, y, cw, ch, draw_color, 1.0);
                }
                CursorStyle::Underline => {
                    rects.draw_rect(x, y + ch - 2.0, cw, 2.0, draw_color, 0.0);
                }
                CursorStyle::UnderlineThin => {
                    rects.draw_rect(x, y + ch - 1.0, cw, 1.0, draw_color, 0.0);
                }
            }
        }
    }

    /// Computes the current blink opacity (0.0–1.0) using a smooth fade.
    fn blink_alpha(&self) -> f32 {
        if !self.config.blink_enabled {
            return 1.0;
        }
        let half = self.config.blink_period / 2.0;
        let fade = self.config.blink_fade_time.max(0.001);
        if self.blink_clock < half {
            // Visible phase — fade out at the end
            let remaining = half - self.blink_clock;
            if remaining < fade {
                remaining / fade
            } else {
                1.0
            }
        } else {
            // Hidden phase — fade in at the end
            let into_hidden = self.blink_clock - half;
            let remaining = half - into_hidden;
            if remaining < fade {
                1.0 - remaining / fade
            } else {
                0.0
            }
        }
    }
}
