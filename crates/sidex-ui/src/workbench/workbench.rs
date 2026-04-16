//! Top-level workbench layout composing all VS Code chrome components.

use sidex_gpu::GpuRenderer;
use sidex_theme::Theme;

use crate::layout::{compute_layout, Direction, LayoutNode, Rect, Size};
use crate::widget::{EventResult, UiEvent};

/// Position of the sidebar relative to the editor area.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SidebarPosition {
    #[default]
    Left,
    Right,
}

/// Position of the bottom panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PanelPosition {
    #[default]
    Bottom,
    Right,
}

/// Pre-computed rectangles for each region of the workbench.
#[derive(Clone, Debug, Default)]
pub struct WorkbenchLayout {
    pub title_bar: Rect,
    pub activity_bar: Rect,
    pub sidebar: Rect,
    pub editor_area: Rect,
    pub panel: Rect,
    pub status_bar: Rect,
}

/// The top-level workbench that composes all UI regions.
pub struct Workbench {
    pub sidebar_visible: bool,
    pub sidebar_position: SidebarPosition,
    pub sidebar_width: f32,

    pub panel_visible: bool,
    pub panel_position: PanelPosition,
    pub panel_height: f32,

    pub title_bar_height: f32,
    pub activity_bar_width: f32,
    pub status_bar_height: f32,

    /// Cached layout from the last `layout()` call.
    cached_layout: Option<WorkbenchLayout>,
}

impl Workbench {
    /// Creates a workbench with default dimensions derived from the theme.
    pub fn new(_theme: &Theme) -> Self {
        Self {
            sidebar_visible: true,
            sidebar_position: SidebarPosition::Left,
            sidebar_width: 250.0,

            panel_visible: true,
            panel_position: PanelPosition::Bottom,
            panel_height: 250.0,

            title_bar_height: 30.0,
            activity_bar_width: 48.0,
            status_bar_height: 22.0,

            cached_layout: None,
        }
    }

    pub fn toggle_sidebar(&mut self) {
        self.sidebar_visible = !self.sidebar_visible;
    }

    pub fn toggle_panel(&mut self) {
        self.panel_visible = !self.panel_visible;
    }

    /// Computes the workbench layout for the given window dimensions.
    pub fn layout(&mut self, width: f32, height: f32) -> WorkbenchLayout {
        let sidebar_w = if self.sidebar_visible {
            self.sidebar_width
        } else {
            0.0
        };
        let panel_h = if self.panel_visible {
            self.panel_height
        } else {
            0.0
        };

        let root = LayoutNode {
            direction: Direction::Column,
            size: Size::Flex(1.0),
            children: vec![
                LayoutNode::fixed(self.title_bar_height),
                LayoutNode {
                    direction: Direction::Row,
                    size: Size::Flex(1.0),
                    children: self.middle_children(sidebar_w, panel_h),
                    ..LayoutNode::default()
                },
                LayoutNode::fixed(self.status_bar_height),
            ],
            ..LayoutNode::default()
        };

        let rects = compute_layout(&root, Rect::new(0.0, 0.0, width, height));

        let wl = self.extract_layout(&rects, sidebar_w);
        self.cached_layout = Some(wl.clone());
        wl
    }

    /// Renders the workbench chrome (backgrounds, borders) into the GPU
    /// renderer.  Individual components (editor, terminal, etc.) are rendered
    /// separately by their owners.
    pub fn render(&self, renderer: &mut GpuRenderer) {
        let _ = renderer;
        // The workbench itself just computes layout; actual rendering is
        // delegated to each sub-component (title_bar, sidebar, etc.) by the
        // application event loop, which calls widget.render(rect, renderer)
        // for each component with the rects produced by layout().
    }

    /// Routes an event to the appropriate region based on the cached layout.
    pub fn handle_event(&self, _event: &UiEvent) -> EventResult {
        // Top-level routing would check which region the event falls in using
        // cached_layout, then forward to the appropriate component.  The
        // application layer owns the actual widget instances.
        EventResult::Ignored
    }

    /// Returns the most recently computed layout, if any.
    pub fn cached_layout(&self) -> Option<&WorkbenchLayout> {
        self.cached_layout.as_ref()
    }

    fn middle_children(&self, sidebar_w: f32, panel_h: f32) -> Vec<LayoutNode> {
        let activity_bar = LayoutNode::fixed(self.activity_bar_width);
        let sidebar = LayoutNode::fixed(sidebar_w);

        let editor_and_panel = match self.panel_position {
            PanelPosition::Bottom => LayoutNode {
                direction: Direction::Column,
                size: Size::Flex(1.0),
                children: vec![LayoutNode::flex(1.0), LayoutNode::fixed(panel_h)],
                ..LayoutNode::default()
            },
            PanelPosition::Right => LayoutNode {
                direction: Direction::Row,
                size: Size::Flex(1.0),
                children: vec![LayoutNode::flex(1.0), LayoutNode::fixed(panel_h)],
                ..LayoutNode::default()
            },
        };

        match self.sidebar_position {
            SidebarPosition::Left => vec![activity_bar, sidebar, editor_and_panel],
            SidebarPosition::Right => vec![editor_and_panel, sidebar, activity_bar],
        }
    }

    fn extract_layout(&self, rects: &[Rect], sidebar_w: f32) -> WorkbenchLayout {
        // Pre-order indices for the tree:
        //   0: root (column)
        //   1: title_bar
        //   2: middle (row)
        //     The middle row children depend on sidebar position:
        //     Left:  3=activity_bar, 4=sidebar, 5=editor_and_panel
        //     Right: 3=editor_and_panel, 4=sidebar, 5=activity_bar
        //   editor_and_panel has sub-children:
        //     6=editor, 7=panel
        //   8: status_bar

        let title_bar = rects.get(1).copied().unwrap_or(Rect::ZERO);
        let status_bar = rects.get(8).copied().unwrap_or(Rect::ZERO);

        let (activity_bar, sidebar, editor_area, panel);

        match self.sidebar_position {
            SidebarPosition::Left => {
                activity_bar = rects.get(3).copied().unwrap_or(Rect::ZERO);
                sidebar = if sidebar_w > 0.0 {
                    rects.get(4).copied().unwrap_or(Rect::ZERO)
                } else {
                    Rect::ZERO
                };
                editor_area = rects.get(6).copied().unwrap_or(Rect::ZERO);
                panel = rects.get(7).copied().unwrap_or(Rect::ZERO);
            }
            SidebarPosition::Right => {
                activity_bar = rects.get(5).copied().unwrap_or(Rect::ZERO);
                sidebar = if sidebar_w > 0.0 {
                    rects.get(4).copied().unwrap_or(Rect::ZERO)
                } else {
                    Rect::ZERO
                };
                editor_area = rects.get(4).copied().unwrap_or(Rect::ZERO);
                panel = rects.get(5).copied().unwrap_or(Rect::ZERO);
            }
        }

        WorkbenchLayout {
            title_bar,
            activity_bar,
            sidebar,
            editor_area,
            panel,
            status_bar,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sidex_theme::Theme;

    #[test]
    fn default_layout_regions_are_non_zero() {
        let theme = Theme::default_dark();
        let mut wb = Workbench::new(&theme);
        let wl = wb.layout(1280.0, 720.0);

        assert!(wl.title_bar.height > 0.0);
        assert!(wl.status_bar.height > 0.0);
        assert!(wl.activity_bar.width > 0.0);
        assert!(wl.sidebar.width > 0.0);
        assert!(wl.editor_area.width > 0.0);
    }

    #[test]
    fn sidebar_toggle_removes_sidebar() {
        let theme = Theme::default_dark();
        let mut wb = Workbench::new(&theme);
        wb.toggle_sidebar();
        let wl = wb.layout(1280.0, 720.0);
        assert!((wl.sidebar.width - 0.0).abs() < 0.01);
    }

    #[test]
    fn panel_toggle_removes_panel() {
        let theme = Theme::default_dark();
        let mut wb = Workbench::new(&theme);
        wb.toggle_panel();
        let wl = wb.layout(1280.0, 720.0);
        assert!((wl.panel.height - 0.0).abs() < 0.01 || (wl.panel.width - 0.0).abs() < 0.01);
    }

    #[test]
    fn title_bar_spans_full_width() {
        let theme = Theme::default_dark();
        let mut wb = Workbench::new(&theme);
        let wl = wb.layout(1920.0, 1080.0);
        assert!((wl.title_bar.width - 1920.0).abs() < 0.01);
        assert!((wl.title_bar.x - 0.0).abs() < 0.01);
    }

    #[test]
    fn status_bar_at_bottom() {
        let theme = Theme::default_dark();
        let mut wb = Workbench::new(&theme);
        let wl = wb.layout(1280.0, 720.0);
        let bottom = wl.status_bar.y + wl.status_bar.height;
        assert!((bottom - 720.0).abs() < 0.01);
    }
}
