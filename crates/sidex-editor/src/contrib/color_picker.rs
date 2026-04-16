//! Color picker — mirrors VS Code's color-picker contribution.
//!
//! Detects color literals in the document and tracks the state for an
//! inline color picker popup.

use sidex_text::{Position, Range};

/// A color in RGBA (0.0–1.0 per channel).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorRGBA {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl ColorRGBA {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Converts to a CSS hex string (e.g. `#ff00aaff`).
    #[must_use]
    pub fn to_hex(&self) -> String {
        format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            (self.r * 255.0) as u8,
            (self.g * 255.0) as u8,
            (self.b * 255.0) as u8,
            (self.a * 255.0) as u8,
        )
    }

    /// Parses a hex color string (`#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`).
    #[must_use]
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.strip_prefix('#')?;
        let (r, g, b, a) = match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
                (r, g, b, 255u8)
            }
            4 => {
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
                let a = u8::from_str_radix(&hex[3..4].repeat(2), 16).ok()?;
                (r, g, b, a)
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                (r, g, b, 255u8)
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                (r, g, b, a)
            }
            _ => return None,
        };
        Some(Self {
            r: f32::from(r) / 255.0,
            g: f32::from(g) / 255.0,
            b: f32::from(b) / 255.0,
            a: f32::from(a) / 255.0,
        })
    }
}

/// A detected color in the document.
#[derive(Debug, Clone)]
pub struct DocumentColor {
    /// The range of the color literal in the source.
    pub range: Range,
    /// The parsed colour value.
    pub color: ColorRGBA,
}

/// Full state for the color-picker feature.
#[derive(Debug, Clone, Default)]
pub struct ColorPickerState {
    /// All detected colors in the document.
    pub colors: Vec<DocumentColor>,
    /// Index of the color whose picker is currently open, if any.
    pub active_picker: Option<usize>,
    /// Whether the color provider is loading.
    pub is_loading: bool,
}

impl ColorPickerState {
    /// Sets the detected document colors from the language server.
    pub fn set_colors(&mut self, colors: Vec<DocumentColor>) {
        self.colors = colors;
        self.is_loading = false;
    }

    /// Opens the color picker for the color at `index`.
    pub fn open_picker(&mut self, index: usize) {
        if index < self.colors.len() {
            self.active_picker = Some(index);
        }
    }

    /// Closes the active picker.
    pub fn close_picker(&mut self) {
        self.active_picker = None;
    }

    /// Returns the document color being edited, if any.
    #[must_use]
    pub fn active_color(&self) -> Option<&DocumentColor> {
        self.active_picker.and_then(|i| self.colors.get(i))
    }

    /// Updates the color value for the active picker.
    pub fn update_active_color(&mut self, new_color: ColorRGBA) {
        if let Some(idx) = self.active_picker {
            if let Some(dc) = self.colors.get_mut(idx) {
                dc.color = new_color;
            }
        }
    }

    /// Clears all colors.
    pub fn clear(&mut self) {
        self.colors.clear();
        self.active_picker = None;
        self.is_loading = false;
    }

    /// Returns the color at the given position, if any.
    #[must_use]
    pub fn color_at(&self, pos: Position) -> Option<(usize, &DocumentColor)> {
        self.colors
            .iter()
            .enumerate()
            .find(|(_, dc)| dc.range.contains(pos))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        let c = ColorRGBA::from_hex("#ff8040").unwrap();
        assert!((c.r - 1.0).abs() < 0.01);
        assert!((c.g - 0.502).abs() < 0.01);
        assert!((c.b - 0.251).abs() < 0.01);

        let hex = c.to_hex();
        assert!(hex.starts_with("#ff"));
    }

    #[test]
    fn short_hex() {
        let c = ColorRGBA::from_hex("#f00").unwrap();
        assert!((c.r - 1.0).abs() < 0.01);
        assert!(c.g.abs() < 0.01);
        assert!(c.b.abs() < 0.01);
    }

    #[test]
    fn picker_lifecycle() {
        let mut state = ColorPickerState::default();
        state.set_colors(vec![DocumentColor {
            range: Range::new(Position::new(0, 0), Position::new(0, 7)),
            color: ColorRGBA::new(1.0, 0.0, 0.0, 1.0),
        }]);
        state.open_picker(0);
        assert!(state.active_color().is_some());
        state.close_picker();
        assert!(state.active_color().is_none());
    }
}
