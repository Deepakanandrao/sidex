//! File Explorer panel — tree of workspace files with context menus and drag-and-drop.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use sidex_gpu::color::Color;
use sidex_gpu::GpuRenderer;

use crate::layout::{LayoutNode, Rect, Size};
use crate::widget::{EventResult, Key, MouseButton, UiEvent, Widget};

// ── File icon mapping ────────────────────────────────────────────────────────

/// Icon identifier derived from file extension.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileIcon {
    Folder,
    FolderOpen,
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Json,
    Toml,
    Yaml,
    Markdown,
    Html,
    Css,
    Git,
    Image,
    Binary,
    Default,
}

impl FileIcon {
    pub fn for_extension(ext: &str) -> Self {
        match ext {
            "rs" => Self::Rust,
            "ts" | "tsx" => Self::TypeScript,
            "js" | "jsx" | "mjs" | "cjs" => Self::JavaScript,
            "py" | "pyi" => Self::Python,
            "json" | "jsonc" => Self::Json,
            "toml" => Self::Toml,
            "yaml" | "yml" => Self::Yaml,
            "md" | "mdx" => Self::Markdown,
            "html" | "htm" => Self::Html,
            "css" | "scss" | "less" => Self::Css,
            "gitignore" | "gitattributes" | "gitmodules" => Self::Git,
            "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" | "ico" => Self::Image,
            "wasm" | "exe" | "dll" | "so" | "dylib" => Self::Binary,
            _ => Self::Default,
        }
    }

    pub fn color(&self) -> Color {
        match self {
            Self::Folder | Self::FolderOpen => Color::from_rgb(220, 180, 80),
            Self::Rust => Color::from_rgb(222, 165, 132),
            Self::TypeScript => Color::from_rgb(49, 120, 198),
            Self::JavaScript => Color::from_rgb(226, 211, 100),
            Self::Python => Color::from_rgb(55, 152, 199),
            Self::Json => Color::from_rgb(203, 203, 65),
            Self::Toml => Color::from_rgb(156, 156, 156),
            Self::Yaml => Color::from_rgb(203, 65, 65),
            Self::Markdown => Color::from_rgb(80, 150, 220),
            Self::Html => Color::from_rgb(227, 76, 38),
            Self::Css => Color::from_rgb(86, 156, 214),
            Self::Git => Color::from_rgb(240, 80, 50),
            Self::Image => Color::from_rgb(160, 120, 200),
            Self::Binary => Color::from_rgb(128, 128, 128),
            Self::Default => Color::from_rgb(180, 180, 180),
        }
    }
}

// ── File entry ───────────────────────────────────────────────────────────────

/// A single entry in the file tree.
#[derive(Clone, Debug)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_directory: bool,
    pub children: Vec<FileEntry>,
    pub children_loaded: bool,
}

impl FileEntry {
    pub fn file(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            is_directory: false,
            children: Vec::new(),
            children_loaded: true,
        }
    }

    pub fn directory(
        name: impl Into<String>,
        path: impl Into<PathBuf>,
        children: Vec<FileEntry>,
    ) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            is_directory: true,
            children,
            children_loaded: true,
        }
    }

    pub fn lazy_directory(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            is_directory: true,
            children: Vec::new(),
            children_loaded: false,
        }
    }

    pub fn icon(&self) -> FileIcon {
        if self.is_directory {
            FileIcon::Folder
        } else {
            let ext = self
                .path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            FileIcon::for_extension(ext)
        }
    }
}

// ── Open editors ─────────────────────────────────────────────────────────────

/// An open editor tab shown at the top of the explorer.
#[derive(Clone, Debug)]
pub struct OpenEditor {
    pub name: String,
    pub path: PathBuf,
    pub is_dirty: bool,
    pub is_preview: bool,
}

// ── Context menu actions ─────────────────────────────────────────────────────

/// Actions available from the file explorer context menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplorerAction {
    NewFile,
    NewFolder,
    Rename,
    Delete,
    CopyPath,
    CopyRelativePath,
    OpenInTerminal,
    RevealInFinder,
    CopyFile,
    PasteFile,
    Cut,
}

// ── Drag-and-drop ────────────────────────────────────────────────────────────

/// State for an in-progress drag-and-drop operation.
#[derive(Clone, Debug)]
pub struct DragState {
    pub source_path: PathBuf,
    pub current_x: f32,
    pub current_y: f32,
    pub drop_target: Option<PathBuf>,
}

// ── Filter ───────────────────────────────────────────────────────────────────

/// Filter state for the explorer's inline search.
#[derive(Clone, Debug, Default)]
pub struct ExplorerFilter {
    pub query: String,
    pub active: bool,
}

impl ExplorerFilter {
    pub fn matches(&self, name: &str) -> bool {
        if !self.active || self.query.is_empty() {
            return true;
        }
        let query = self.query.to_lowercase();
        name.to_lowercase().contains(&query)
    }
}

// ── Flat row for rendering ───────────────────────────────────────────────────

#[allow(dead_code)]
struct FlatFileRow {
    path: Vec<usize>,
    depth: usize,
    is_directory: bool,
    is_expanded: bool,
    name: String,
    file_path: PathBuf,
}

// ── File Explorer ────────────────────────────────────────────────────────────

/// The File Explorer sidebar panel.
///
/// Displays the workspace file tree with collapsible folders, file icons,
/// inline filtering, open editors, and context menu support.
#[allow(dead_code)]
pub struct FileExplorer<OnOpen, OnAction>
where
    OnOpen: FnMut(&Path),
    OnAction: FnMut(ExplorerAction, &Path),
{
    pub root_entries: Vec<FileEntry>,
    pub open_editors: Vec<OpenEditor>,
    pub on_open: OnOpen,
    pub on_action: OnAction,
    pub filter: ExplorerFilter,

    expanded: HashSet<PathBuf>,
    selected_path: Option<PathBuf>,
    drag_state: Option<DragState>,
    context_menu_path: Option<PathBuf>,

    show_open_editors: bool,
    open_editors_expanded: bool,
    row_height: f32,
    indent_width: f32,
    scroll_offset: f32,
    focused: bool,

    background: Color,
    selected_bg: Color,
    hover_bg: Color,
    guide_color: Color,
    header_bg: Color,
    header_fg: Color,
    drop_target_bg: Color,
}

impl<OnOpen, OnAction> FileExplorer<OnOpen, OnAction>
where
    OnOpen: FnMut(&Path),
    OnAction: FnMut(ExplorerAction, &Path),
{
    pub fn new(root_entries: Vec<FileEntry>, on_open: OnOpen, on_action: OnAction) -> Self {
        Self {
            root_entries,
            open_editors: Vec::new(),
            on_open,
            on_action,
            filter: ExplorerFilter::default(),

            expanded: HashSet::new(),
            selected_path: None,
            drag_state: None,
            context_menu_path: None,

            show_open_editors: true,
            open_editors_expanded: true,
            row_height: 22.0,
            indent_width: 16.0,
            scroll_offset: 0.0,
            focused: false,

            background: Color::from_hex("#252526").unwrap_or(Color::BLACK),
            selected_bg: Color::from_hex("#04395e").unwrap_or(Color::BLACK),
            hover_bg: Color::from_hex("#2a2d2e").unwrap_or(Color::BLACK),
            guide_color: Color::from_hex("#404040").unwrap_or(Color::WHITE),
            header_bg: Color::from_hex("#383838").unwrap_or(Color::BLACK),
            header_fg: Color::from_hex("#cccccc").unwrap_or(Color::WHITE),
            drop_target_bg: Color::from_hex("#062f4a").unwrap_or(Color::BLACK),
        }
    }

    pub fn set_filter(&mut self, query: impl Into<String>) {
        self.filter.query = query.into();
        self.filter.active = !self.filter.query.is_empty();
    }

    pub fn toggle_open_editors(&mut self) {
        self.open_editors_expanded = !self.open_editors_expanded;
    }

    pub fn expand(&mut self, path: &Path) {
        self.expanded.insert(path.to_path_buf());
    }

    pub fn collapse(&mut self, path: &Path) {
        self.expanded.remove(path);
    }

    pub fn toggle_expand(&mut self, path: &Path) {
        if self.expanded.contains(path) {
            self.expanded.remove(path);
        } else {
            self.expanded.insert(path.to_path_buf());
        }
    }

    pub fn select(&mut self, path: &Path) {
        self.selected_path = Some(path.to_path_buf());
    }

    pub fn begin_drag(&mut self, source: &Path, x: f32, y: f32) {
        self.drag_state = Some(DragState {
            source_path: source.to_path_buf(),
            current_x: x,
            current_y: y,
            drop_target: None,
        });
    }

    pub fn end_drag(&mut self) -> Option<(PathBuf, PathBuf)> {
        let state = self.drag_state.take()?;
        let target = state.drop_target?;
        Some((state.source_path, target))
    }

    #[allow(clippy::cast_precision_loss)]
    fn open_editors_height(&self) -> f32 {
        if !self.show_open_editors {
            return 0.0;
        }
        let header = self.row_height;
        if !self.open_editors_expanded {
            return header;
        }
        header + self.open_editors.len() as f32 * self.row_height
    }

    fn flatten(&self) -> Vec<FlatFileRow> {
        let mut rows = Vec::new();
        self.flatten_children(&self.root_entries, &mut vec![], 0, &mut rows);
        rows
    }

    fn flatten_children(
        &self,
        entries: &[FileEntry],
        parent_path: &mut Vec<usize>,
        depth: usize,
        out: &mut Vec<FlatFileRow>,
    ) {
        for (i, entry) in entries.iter().enumerate() {
            if !self.filter.matches(&entry.name) {
                continue;
            }
            parent_path.push(i);
            let is_expanded = self.expanded.contains(&entry.path);

            out.push(FlatFileRow {
                path: parent_path.clone(),
                depth,
                is_directory: entry.is_directory,
                is_expanded,
                name: entry.name.clone(),
                file_path: entry.path.clone(),
            });

            if entry.is_directory && is_expanded {
                self.flatten_children(&entry.children, parent_path, depth + 1, out);
            }
            parent_path.pop();
        }
    }

    #[allow(dead_code)]
    fn entry_at_index_path(&self, path: &[usize]) -> Option<&FileEntry> {
        let mut entries = &self.root_entries;
        let mut result = None;
        for &idx in path {
            let entry = entries.get(idx)?;
            result = Some(entry);
            entries = &entry.children;
        }
        result
    }
}

impl<OnOpen, OnAction> Widget for FileExplorer<OnOpen, OnAction>
where
    OnOpen: FnMut(&Path),
    OnAction: FnMut(ExplorerAction, &Path),
{
    fn layout(&self) -> LayoutNode {
        LayoutNode {
            size: Size::Flex(1.0),
            ..LayoutNode::default()
        }
    }

    #[allow(clippy::cast_precision_loss)]
    fn render(&self, rect: Rect, renderer: &mut GpuRenderer) {
        let mut rr = sidex_gpu::RectRenderer::new();
        rr.draw_rect(rect.x, rect.y, rect.width, rect.height, self.background, 0.0);

        let mut y = rect.y;

        // Open editors section header
        if self.show_open_editors {
            rr.draw_rect(rect.x, y, rect.width, self.row_height, self.header_bg, 0.0);
            y += self.row_height;

            if self.open_editors_expanded {
                for editor in &self.open_editors {
                    if editor.is_dirty {
                        let dot_r = 3.0;
                        rr.draw_rect(
                            rect.x + 24.0 - dot_r,
                            y + self.row_height / 2.0 - dot_r,
                            dot_r * 2.0,
                            dot_r * 2.0,
                            Color::WHITE,
                            dot_r,
                        );
                    }
                    y += self.row_height;
                }
            }
        }

        // File tree
        let rows = self.flatten();
        for row in &rows {
            let ry = y + row.depth as f32 * 0.0 - self.scroll_offset;
            if ry + self.row_height < rect.y || ry > rect.y + rect.height {
                y += self.row_height;
                continue;
            }

            let is_selected = self.selected_path.as_deref() == Some(&row.file_path);
            if is_selected {
                rr.draw_rect(rect.x, y, rect.width, self.row_height, self.selected_bg, 0.0);
            }

            // Indent guides
            for d in 0..row.depth {
                let gx = rect.x + d as f32 * self.indent_width + self.indent_width / 2.0;
                rr.draw_rect(gx, y, 1.0, self.row_height, self.guide_color, 0.0);
            }

            // File/folder icon indicator
            let icon = if row.is_directory {
                if row.is_expanded {
                    FileIcon::FolderOpen
                } else {
                    FileIcon::Folder
                }
            } else {
                let ext = row
                    .file_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                FileIcon::for_extension(ext)
            };
            let icon_x = rect.x + row.depth as f32 * self.indent_width + 4.0;
            rr.draw_rect(
                icon_x,
                y + 4.0,
                14.0,
                14.0,
                icon.color(),
                if row.is_directory { 2.0 } else { 0.0 },
            );

            // Drop target highlight
            if let Some(ref drag) = self.drag_state {
                if drag.drop_target.as_deref() == Some(&row.file_path) {
                    rr.draw_rect(
                        rect.x,
                        y,
                        rect.width,
                        self.row_height,
                        self.drop_target_bg,
                        0.0,
                    );
                }
            }

            y += self.row_height;
        }

        let _ = renderer;
    }

    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss
    )]
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
                let tree_top = rect.y + self.open_editors_height();
                if *y >= tree_top {
                    let rows = self.flatten();
                    let idx =
                        ((y - tree_top + self.scroll_offset) / self.row_height).floor() as usize;
                    if let Some(row) = rows.get(idx) {
                        let fp = row.file_path.clone();
                        if row.is_directory {
                            self.toggle_expand(&fp);
                        } else {
                            (self.on_open)(&fp);
                        }
                        self.selected_path = Some(fp);
                    }
                } else if self.show_open_editors && *y < rect.y + self.row_height {
                    self.toggle_open_editors();
                }
                EventResult::Handled
            }
            UiEvent::MouseDown {
                x,
                y,
                button: MouseButton::Right,
            } if rect.contains(*x, *y) => {
                let tree_top = rect.y + self.open_editors_height();
                if *y >= tree_top {
                    let rows = self.flatten();
                    let idx =
                        ((y - tree_top + self.scroll_offset) / self.row_height).floor() as usize;
                    if let Some(row) = rows.get(idx) {
                        self.context_menu_path = Some(row.file_path.clone());
                        self.selected_path = Some(row.file_path.clone());
                    }
                }
                EventResult::Handled
            }
            UiEvent::MouseScroll { dy, .. } if rect.contains(rect.x, rect.y) => {
                let rows = self.flatten();
                let total = rows.len() as f32 * self.row_height;
                let max = (total - rect.height + self.open_editors_height()).max(0.0);
                self.scroll_offset = (self.scroll_offset - dy * 40.0).clamp(0.0, max);
                EventResult::Handled
            }
            UiEvent::KeyPress { key, .. } if self.focused => match key {
                Key::ArrowDown | Key::ArrowUp => {
                    let rows = self.flatten();
                    if rows.is_empty() {
                        return EventResult::Handled;
                    }
                    let current = self
                        .selected_path
                        .as_ref()
                        .and_then(|sel| rows.iter().position(|r| r.file_path == *sel))
                        .unwrap_or(0);
                    let next = match key {
                        Key::ArrowDown => (current + 1).min(rows.len() - 1),
                        _ => current.saturating_sub(1),
                    };
                    if let Some(row) = rows.get(next) {
                        self.selected_path = Some(row.file_path.clone());
                    }
                    EventResult::Handled
                }
                Key::ArrowRight => {
                    if let Some(ref sel) = self.selected_path {
                        let sel = sel.clone();
                        self.expand(&sel);
                    }
                    EventResult::Handled
                }
                Key::ArrowLeft => {
                    if let Some(ref sel) = self.selected_path {
                        let sel = sel.clone();
                        self.collapse(&sel);
                    }
                    EventResult::Handled
                }
                Key::Enter => {
                    if let Some(ref sel) = self.selected_path.clone() {
                        let rows = self.flatten();
                        if let Some(row) = rows.iter().find(|r| r.file_path == *sel) {
                            if row.is_directory {
                                self.toggle_expand(sel);
                            } else {
                                (self.on_open)(sel);
                            }
                        }
                    }
                    EventResult::Handled
                }
                _ => EventResult::Ignored,
            },
            _ => EventResult::Ignored,
        }
    }
}
