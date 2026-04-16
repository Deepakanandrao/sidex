//! Hover information display — mirrors VS Code's `ContentHoverController` +
//! `ContentHoverWidget`.
//!
//! Manages the state for showing hover tooltips with markdown content, code
//! blocks, and diagnostic information at a document position.

use sidex_text::Position;

/// The content to display inside a hover tooltip.
#[derive(Debug, Clone)]
pub enum HoverContent {
    /// Markdown-formatted text.
    Markdown(String),
    /// A fenced code block with optional language tag.
    CodeBlock { language: Option<String>, code: String },
}

/// Full state for the hover feature.
#[derive(Debug, Clone)]
pub struct HoverState {
    /// Whether the hover popup is currently visible.
    pub is_visible: bool,
    /// The document position that triggered the hover.
    pub position: Option<Position>,
    /// Content sections to render (may include multiple markdown blocks).
    pub contents: Vec<HoverContent>,
    /// Delay in milliseconds before showing the hover (default 500ms).
    pub delay_ms: u64,
    /// Whether a hover request is currently in-flight.
    pub is_loading: bool,
    /// Pixel coordinates for positioning (set by the renderer).
    pub anchor_x: f32,
    pub anchor_y: f32,
}

impl Default for HoverState {
    fn default() -> Self {
        Self {
            is_visible: false,
            position: None,
            contents: Vec::new(),
            delay_ms: 500,
            is_loading: false,
            anchor_x: 0.0,
            anchor_y: 0.0,
        }
    }
}

impl HoverState {
    /// Initiates a hover request at the given position.  The caller is
    /// responsible for scheduling the actual LSP request after `delay_ms`.
    pub fn request_hover(&mut self, pos: Position) {
        self.position = Some(pos);
        self.is_loading = true;
        self.contents.clear();
    }

    /// Resolves a hover request with content and makes the popup visible.
    pub fn show_hover(&mut self, pos: Position, contents: Vec<HoverContent>) {
        self.position = Some(pos);
        self.contents = contents;
        self.is_loading = false;
        self.is_visible = !self.contents.is_empty();
    }

    /// Hides the hover popup and clears its content.
    pub fn hide_hover(&mut self) {
        self.is_visible = false;
        self.is_loading = false;
        self.contents.clear();
        self.position = None;
    }

    /// Returns `true` when the hover has content to render.
    #[must_use]
    pub fn has_content(&self) -> bool {
        !self.contents.is_empty()
    }

    /// Sets the pixel anchor for the tooltip.
    pub fn set_anchor(&mut self, x: f32, y: f32) {
        self.anchor_x = x;
        self.anchor_y = y;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn show_and_hide() {
        let mut state = HoverState::default();
        assert!(!state.is_visible);

        let pos = Position::new(5, 10);
        state.show_hover(pos, vec![HoverContent::Markdown("hello".into())]);
        assert!(state.is_visible);
        assert!(state.has_content());

        state.hide_hover();
        assert!(!state.is_visible);
        assert!(!state.has_content());
    }

    #[test]
    fn request_sets_loading() {
        let mut state = HoverState::default();
        state.request_hover(Position::new(1, 0));
        assert!(state.is_loading);
        assert!(!state.is_visible);
    }
}
