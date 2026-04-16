//! Collapsible tree widget with indent guides and lazy loading.

use sidex_gpu::color::Color;
use sidex_gpu::GpuRenderer;

use crate::layout::{LayoutNode, Rect, Size};
use crate::widget::{EventResult, Key, MouseButton, UiEvent, Widget};

/// A node in the tree data model.
pub struct TreeNode<T> {
    pub data: T,
    pub children: Vec<TreeNode<T>>,
    /// Whether children have been loaded (for lazy loading).
    pub children_loaded: bool,
}

impl<T> TreeNode<T> {
    pub fn leaf(data: T) -> Self {
        Self {
            data,
            children: Vec::new(),
            children_loaded: true,
        }
    }

    pub fn branch(data: T, children: Vec<TreeNode<T>>) -> Self {
        Self {
            data,
            children,
            children_loaded: true,
        }
    }

    pub fn lazy(data: T) -> Self {
        Self {
            data,
            children: Vec::new(),
            children_loaded: false,
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.children_loaded && self.children.is_empty()
    }
}

/// Flat representation of a visible tree row for rendering.
struct FlatRow {
    /// Index path from root (e.g. `[0, 2, 1]`).
    path: Vec<usize>,
    depth: usize,
    has_children: bool,
    is_expanded: bool,
}

/// Pre-rendered description of a tree row.
pub struct TreeRow {
    pub text: String,
    pub icon: Option<String>,
}

/// A tree view with collapsible nodes, indent guides, and keyboard navigation.
#[allow(dead_code)]
pub struct Tree<T, R, E, S>
where
    R: Fn(&T, usize) -> TreeRow,
    E: FnMut(&[usize]),
    S: FnMut(&[usize]),
{
    pub root: Vec<TreeNode<T>>,
    pub render_item: R,
    pub on_toggle: E,
    pub on_select: S,

    expanded: std::collections::HashSet<Vec<usize>>,
    selected_path: Option<Vec<usize>>,

    row_height: f32,
    indent_width: f32,
    scroll_offset: f32,
    focused: bool,

    guide_color: Color,
    selected_bg: Color,
    hover_bg: Color,
}

impl<T, R, E, S> Tree<T, R, E, S>
where
    R: Fn(&T, usize) -> TreeRow,
    E: FnMut(&[usize]),
    S: FnMut(&[usize]),
{
    pub fn new(root: Vec<TreeNode<T>>, render_item: R, on_toggle: E, on_select: S) -> Self {
        Self {
            root,
            render_item,
            on_toggle,
            on_select,
            expanded: std::collections::HashSet::new(),
            selected_path: None,
            row_height: 22.0,
            indent_width: 16.0,
            scroll_offset: 0.0,
            focused: false,
            guide_color: Color::from_hex("#404040").unwrap_or(Color::WHITE),
            selected_bg: Color::from_hex("#04395e").unwrap_or(Color::BLACK),
            hover_bg: Color::from_hex("#2a2d2e").unwrap_or(Color::BLACK),
        }
    }

    fn flatten(&self) -> Vec<FlatRow> {
        let mut rows = Vec::new();
        self.flatten_children(&self.root, &mut vec![], 0, &mut rows);
        rows
    }

    fn flatten_children(
        &self,
        nodes: &[TreeNode<T>],
        parent_path: &mut Vec<usize>,
        depth: usize,
        out: &mut Vec<FlatRow>,
    ) {
        for (i, node) in nodes.iter().enumerate() {
            parent_path.push(i);
            let path = parent_path.clone();
            let has_children = !node.is_leaf();
            let is_expanded = self.expanded.contains(&path);

            out.push(FlatRow {
                path: path.clone(),
                depth,
                has_children,
                is_expanded,
            });

            if is_expanded && has_children {
                self.flatten_children(&node.children, parent_path, depth + 1, out);
            }
            parent_path.pop();
        }
    }

    fn node_at_path(&self, path: &[usize]) -> Option<&TreeNode<T>> {
        let mut nodes = &self.root;
        let mut result = None;
        for &idx in path {
            let node = nodes.get(idx)?;
            result = Some(node);
            nodes = &node.children;
        }
        result
    }

    fn toggle_expanded(&mut self, path: &[usize]) {
        let p = path.to_vec();
        if self.expanded.contains(&p) {
            self.expanded.remove(&p);
        } else {
            self.expanded.insert(p);
        }
        (self.on_toggle)(path);
    }
}

impl<T, R, E, S> Widget for Tree<T, R, E, S>
where
    R: Fn(&T, usize) -> TreeRow,
    E: FnMut(&[usize]),
    S: FnMut(&[usize]),
{
    fn layout(&self) -> LayoutNode {
        LayoutNode {
            size: Size::Flex(1.0),
            ..LayoutNode::default()
        }
    }

    #[allow(clippy::cast_precision_loss)]
    fn render(&self, rect: Rect, renderer: &mut GpuRenderer) {
        let rows = self.flatten();
        let mut rr = sidex_gpu::RectRenderer::new();

        for (i, row) in rows.iter().enumerate() {
            let y = rect.y + i as f32 * self.row_height - self.scroll_offset;
            if y + self.row_height < rect.y || y > rect.y + rect.height {
                continue;
            }

            let is_selected = self.selected_path.as_deref() == Some(&row.path);
            if is_selected {
                rr.draw_rect(rect.x, y, rect.width, self.row_height, self.selected_bg, 0.0);
            }

            for d in 0..row.depth {
                let guide_x = rect.x + d as f32 * self.indent_width + self.indent_width / 2.0;
                rr.draw_rect(guide_x, y, 1.0, self.row_height, self.guide_color, 0.0);
            }

            if let Some(node) = self.node_at_path(&row.path) {
                let _rendered = (self.render_item)(&node.data, row.depth);
            }
        }
        let _ = renderer;
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    fn handle_event(&mut self, event: &UiEvent, rect: Rect) -> EventResult {
        match event {
            UiEvent::Focus => {
                self.focused = true;
                EventResult::Handled
            }
            UiEvent::Blur => {
                self.focused = false;
                EventResult::Handled
            }
            UiEvent::MouseDown {
                x,
                y,
                button: MouseButton::Left,
            } if rect.contains(*x, *y) => {
                self.focused = true;
                let rows = self.flatten();
                let index = ((y - rect.y + self.scroll_offset) / self.row_height).floor() as usize;
                if let Some(row) = rows.get(index) {
                    let path = row.path.clone();
                    if row.has_children {
                        self.toggle_expanded(&path);
                    }
                    self.selected_path = Some(path.clone());
                    (self.on_select)(&path);
                }
                EventResult::Handled
            }
            UiEvent::MouseScroll { dy, .. } => {
                let rows = self.flatten();
                let total = rows.len() as f32 * self.row_height;
                let max = (total - rect.height).max(0.0);
                self.scroll_offset = (self.scroll_offset - dy * 40.0).clamp(0.0, max);
                EventResult::Handled
            }
            UiEvent::KeyPress { key, .. } if self.focused => {
                let rows = self.flatten();
                let current_idx = self
                    .selected_path
                    .as_ref()
                    .and_then(|p| rows.iter().position(|r| r.path == *p))
                    .unwrap_or(0);

                match key {
                    Key::ArrowDown => {
                        let next = (current_idx + 1).min(rows.len().saturating_sub(1));
                        if let Some(row) = rows.get(next) {
                            let path = row.path.clone();
                            self.selected_path = Some(path.clone());
                            (self.on_select)(&path);
                        }
                        EventResult::Handled
                    }
                    Key::ArrowUp => {
                        let next = current_idx.saturating_sub(1);
                        if let Some(row) = rows.get(next) {
                            let path = row.path.clone();
                            self.selected_path = Some(path.clone());
                            (self.on_select)(&path);
                        }
                        EventResult::Handled
                    }
                    Key::ArrowRight => {
                        if let Some(row) = rows.get(current_idx) {
                            if row.has_children && !row.is_expanded {
                                let path = row.path.clone();
                                self.toggle_expanded(&path);
                            }
                        }
                        EventResult::Handled
                    }
                    Key::ArrowLeft => {
                        if let Some(row) = rows.get(current_idx) {
                            if row.has_children && row.is_expanded {
                                let path = row.path.clone();
                                self.toggle_expanded(&path);
                            }
                        }
                        EventResult::Handled
                    }
                    Key::Enter | Key::Space => {
                        if let Some(row) = rows.get(current_idx) {
                            if row.has_children {
                                let path = row.path.clone();
                                self.toggle_expanded(&path);
                            }
                        }
                        EventResult::Handled
                    }
                    _ => EventResult::Ignored,
                }
            }
            _ => EventResult::Ignored,
        }
    }
}
