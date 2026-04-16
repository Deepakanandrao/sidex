//! Window layout computation.
//!
//! Divides the window into non-overlapping rectangles: title bar, activity
//! bar, sidebar, editor area, panel, and status bar.

/// A rectangle within the window, in physical pixels.
#[derive(Debug, Clone, Copy, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    /// Create a new rectangle.
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Whether a point falls within this rectangle.
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.width && py >= self.y && py < self.y + self.height
    }
}

/// Configurable layout dimensions.
#[derive(Debug, Clone)]
pub struct Layout {
    pub title_bar_height: f32,
    pub activity_bar_width: f32,
    pub sidebar_width: f32,
    pub status_bar_height: f32,
    pub panel_height: f32,
    pub sidebar_visible: bool,
    pub panel_visible: bool,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            title_bar_height: 30.0,
            activity_bar_width: 48.0,
            sidebar_width: 260.0,
            status_bar_height: 22.0,
            panel_height: 200.0,
            sidebar_visible: true,
            panel_visible: true,
        }
    }
}

/// Computed rectangles for each area of the window.
#[derive(Debug, Clone, Default)]
pub struct LayoutRects {
    pub title_bar: Rect,
    pub activity_bar: Rect,
    pub sidebar: Rect,
    pub editor_area: Rect,
    pub panel: Rect,
    pub status_bar: Rect,
}

impl Layout {
    /// Compute the layout rectangles for a given window size.
    #[allow(clippy::cast_precision_loss)]
    pub fn compute(&self, window_width: u32, window_height: u32) -> LayoutRects {
        let w = window_width as f32;
        let h = window_height as f32;

        let title = Rect::new(0.0, 0.0, w, self.title_bar_height);
        let status = Rect::new(0.0, h - self.status_bar_height, w, self.status_bar_height);

        let content_top = title.y + title.height;
        let content_height = h - title.height - status.height;

        let activity = Rect::new(
            0.0,
            content_top,
            self.activity_bar_width,
            content_height,
        );

        let sidebar_w = if self.sidebar_visible {
            self.sidebar_width
        } else {
            0.0
        };
        let sidebar = Rect::new(
            activity.x + activity.width,
            content_top,
            sidebar_w,
            content_height,
        );

        let editor_x = sidebar.x + sidebar.width;
        let editor_w = (w - editor_x).max(0.0);

        let panel_h = if self.panel_visible {
            self.panel_height.min(content_height * 0.5)
        } else {
            0.0
        };

        let editor_h = (content_height - panel_h).max(0.0);
        let editor = Rect::new(editor_x, content_top, editor_w, editor_h);

        let panel = Rect::new(
            editor_x,
            content_top + editor_h,
            editor_w,
            panel_h,
        );

        LayoutRects {
            title_bar: title,
            activity_bar: activity,
            sidebar,
            editor_area: editor,
            panel,
            status_bar: status,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_layout_covers_window() {
        let layout = Layout::default();
        let rects = layout.compute(1280, 720);

        assert!(rects.title_bar.width > 0.0);
        assert!(rects.activity_bar.height > 0.0);
        assert!(rects.editor_area.width > 0.0);
        assert!(rects.editor_area.height > 0.0);
        assert!(rects.status_bar.width > 0.0);
    }

    #[test]
    fn sidebar_hidden_gives_more_editor_space() {
        let mut layout = Layout::default();
        let with_sidebar = layout.compute(1280, 720);

        layout.sidebar_visible = false;
        let without_sidebar = layout.compute(1280, 720);

        assert!(without_sidebar.editor_area.width > with_sidebar.editor_area.width);
        assert!((without_sidebar.sidebar.width - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn panel_hidden_gives_more_editor_space() {
        let mut layout = Layout::default();
        let with_panel = layout.compute(1280, 720);

        layout.panel_visible = false;
        let without_panel = layout.compute(1280, 720);

        assert!(without_panel.editor_area.height > with_panel.editor_area.height);
    }

    #[test]
    fn rect_contains() {
        let r = Rect::new(10.0, 20.0, 100.0, 50.0);
        assert!(r.contains(10.0, 20.0));
        assert!(r.contains(50.0, 40.0));
        assert!(!r.contains(9.0, 20.0));
        assert!(!r.contains(10.0, 70.0));
    }

    #[test]
    fn status_bar_at_bottom() {
        let layout = Layout::default();
        let rects = layout.compute(800, 600);
        let expected_y = 600.0 - layout.status_bar_height;
        assert!((rects.status_bar.y - expected_y).abs() < f32::EPSILON);
    }
}
