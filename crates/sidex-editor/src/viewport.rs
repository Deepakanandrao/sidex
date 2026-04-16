//! Viewport management — tracks what portion of the document is visible
//! on screen and provides scrolling / visibility helpers.

use sidex_text::Position;

/// Represents the visible area of the document on screen.
#[derive(Debug, Clone, PartialEq)]
pub struct Viewport {
    /// First visible line (zero-based).
    pub first_visible_line: u32,
    /// Last visible line (zero-based, inclusive).
    pub last_visible_line: u32,
    /// Vertical scroll offset in pixels from the top of the document.
    pub scroll_top: f64,
    /// Horizontal scroll offset in pixels.
    pub scroll_left: f64,
    /// Number of fully visible lines.
    pub visible_line_count: u32,
    /// Total content width in pixels.
    pub content_width: f64,
    /// Total content height in pixels.
    pub content_height: f64,
    /// Height of a single line in pixels (used for calculations).
    pub line_height: f64,
    /// Height of the viewport in pixels.
    pub viewport_height: f64,
}

impl Viewport {
    /// Creates a new viewport with the given dimensions.
    pub fn new(line_height: f64, viewport_height: f64, viewport_width: f64) -> Self {
        let visible = lines_per_page(line_height, viewport_height);
        Self {
            first_visible_line: 0,
            last_visible_line: visible.saturating_sub(1),
            scroll_top: 0.0,
            scroll_left: 0.0,
            visible_line_count: visible,
            content_width: viewport_width,
            content_height: 0.0,
            line_height,
            viewport_height,
        }
    }

    /// Scrolls so that the given line is the first visible line.
    pub fn scroll_to_line(&mut self, line: u32) {
        self.first_visible_line = line;
        self.last_visible_line = line + self.visible_line_count.saturating_sub(1);
        self.scroll_top = f64::from(line) * self.line_height;
    }

    /// Scrolls so that the given position is visible (centered if possible).
    pub fn scroll_to_position(&mut self, pos: Position) {
        let center_offset = self.visible_line_count / 2;
        let target = pos.line.saturating_sub(center_offset);
        self.scroll_to_line(target);
    }

    /// If the given position is outside the visible area, scrolls minimally
    /// to make it visible.
    pub fn ensure_visible(&mut self, pos: Position) {
        if pos.line < self.first_visible_line {
            self.scroll_to_line(pos.line);
        } else if pos.line > self.last_visible_line {
            let new_first = pos
                .line
                .saturating_sub(self.visible_line_count.saturating_sub(1));
            self.scroll_to_line(new_first);
        }
    }

    /// Returns `true` if the given line is within the visible area.
    pub fn is_line_visible(&self, line: u32) -> bool {
        line >= self.first_visible_line && line <= self.last_visible_line
    }

    /// Returns `true` if the given position is within the visible area.
    pub fn is_position_visible(&self, pos: Position) -> bool {
        self.is_line_visible(pos.line)
    }

    /// Updates the content dimensions (call when document size changes).
    pub fn set_content_size(&mut self, width: f64, height: f64) {
        self.content_width = width;
        self.content_height = height;
    }

    /// Scrolls by a pixel delta (positive = down / right).
    pub fn scroll_by(&mut self, delta_y: f64, delta_x: f64) {
        self.scroll_top = (self.scroll_top + delta_y).max(0.0);
        self.scroll_left = (self.scroll_left + delta_x).max(0.0);
        self.first_visible_line = (self.scroll_top / self.line_height) as u32;
        self.last_visible_line =
            self.first_visible_line + self.visible_line_count.saturating_sub(1);
    }

    /// Scrolls up by one page.
    pub fn page_up(&mut self) {
        let delta = -(self.viewport_height);
        self.scroll_by(delta, 0.0);
    }

    /// Scrolls down by one page.
    pub fn page_down(&mut self) {
        self.scroll_by(self.viewport_height, 0.0);
    }
}

/// Calculates how many full lines fit in the viewport.
pub fn lines_per_page(line_height: f64, viewport_height: f64) -> u32 {
    if line_height <= 0.0 {
        return 0;
    }
    (viewport_height / line_height).floor() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_per_page_basic() {
        assert_eq!(lines_per_page(20.0, 400.0), 20);
        assert_eq!(lines_per_page(18.0, 400.0), 22);
    }

    #[test]
    fn lines_per_page_zero_height() {
        assert_eq!(lines_per_page(0.0, 400.0), 0);
        assert_eq!(lines_per_page(-1.0, 400.0), 0);
    }

    #[test]
    fn new_viewport() {
        let vp = Viewport::new(20.0, 400.0, 800.0);
        assert_eq!(vp.first_visible_line, 0);
        assert_eq!(vp.visible_line_count, 20);
        assert_eq!(vp.last_visible_line, 19);
        assert!((vp.scroll_top - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn scroll_to_line() {
        let mut vp = Viewport::new(20.0, 400.0, 800.0);
        vp.scroll_to_line(10);
        assert_eq!(vp.first_visible_line, 10);
        assert_eq!(vp.last_visible_line, 29);
        assert!((vp.scroll_top - 200.0).abs() < f64::EPSILON);
    }

    #[test]
    fn scroll_to_position() {
        let mut vp = Viewport::new(20.0, 400.0, 800.0);
        vp.scroll_to_position(Position::new(50, 0));
        assert!(vp.first_visible_line <= 50);
        assert!(vp.last_visible_line >= 50);
    }

    #[test]
    fn ensure_visible_already_visible() {
        let mut vp = Viewport::new(20.0, 400.0, 800.0);
        vp.scroll_to_line(0);
        let old_first = vp.first_visible_line;
        vp.ensure_visible(Position::new(5, 0));
        assert_eq!(vp.first_visible_line, old_first);
    }

    #[test]
    fn ensure_visible_below() {
        let mut vp = Viewport::new(20.0, 400.0, 800.0);
        vp.scroll_to_line(0);
        vp.ensure_visible(Position::new(50, 0));
        assert!(vp.first_visible_line > 0);
        assert!(vp.last_visible_line >= 50);
    }

    #[test]
    fn ensure_visible_above() {
        let mut vp = Viewport::new(20.0, 400.0, 800.0);
        vp.scroll_to_line(100);
        vp.ensure_visible(Position::new(50, 0));
        assert_eq!(vp.first_visible_line, 50);
    }

    #[test]
    fn is_line_visible() {
        let mut vp = Viewport::new(20.0, 400.0, 800.0);
        vp.scroll_to_line(10);
        assert!(vp.is_line_visible(10));
        assert!(vp.is_line_visible(29));
        assert!(!vp.is_line_visible(9));
        assert!(!vp.is_line_visible(30));
    }

    #[test]
    fn scroll_by() {
        let mut vp = Viewport::new(20.0, 400.0, 800.0);
        vp.scroll_by(100.0, 50.0);
        assert_eq!(vp.first_visible_line, 5);
        assert!((vp.scroll_left - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn scroll_by_negative_clamped() {
        let mut vp = Viewport::new(20.0, 400.0, 800.0);
        vp.scroll_by(-999.0, -999.0);
        assert!((vp.scroll_top - 0.0).abs() < f64::EPSILON);
        assert!((vp.scroll_left - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn page_up_down() {
        let mut vp = Viewport::new(20.0, 400.0, 800.0);
        vp.page_down();
        assert_eq!(vp.first_visible_line, 20);
        vp.page_up();
        assert_eq!(vp.first_visible_line, 0);
    }

    #[test]
    fn set_content_size() {
        let mut vp = Viewport::new(20.0, 400.0, 800.0);
        vp.set_content_size(1200.0, 10000.0);
        assert!((vp.content_width - 1200.0).abs() < f64::EPSILON);
        assert!((vp.content_height - 10000.0).abs() < f64::EPSILON);
    }
}
