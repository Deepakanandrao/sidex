//! Workspace-wide search panel with regex, case, and word match toggles.

use std::path::PathBuf;

use sidex_gpu::color::Color;
use sidex_gpu::GpuRenderer;

use crate::layout::{LayoutNode, Rect, Size};
use crate::widget::{EventResult, Key, MouseButton, UiEvent, Widget};

// ── Search options ───────────────────────────────────────────────────────────

/// Toggle flags for search mode.
#[derive(Clone, Debug, Default)]
pub struct SearchOptions {
    pub regex: bool,
    pub case_sensitive: bool,
    pub whole_word: bool,
}

// ── Search result model ──────────────────────────────────────────────────────

/// A single line match within a file.
#[derive(Clone, Debug)]
pub struct SearchMatch {
    pub line_number: u32,
    pub column: u32,
    pub length: u32,
    pub line_text: String,
    pub preview_before: String,
    pub preview_match: String,
    pub preview_after: String,
}

/// All matches within a single file.
#[derive(Clone, Debug)]
pub struct FileSearchResult {
    pub path: PathBuf,
    pub matches: Vec<SearchMatch>,
    pub expanded: bool,
}

impl FileSearchResult {
    pub fn match_count(&self) -> usize {
        self.matches.len()
    }
}

// ── Glob patterns ────────────────────────────────────────────────────────────

/// Include/exclude glob patterns for filtering search scope.
#[derive(Clone, Debug, Default)]
pub struct SearchGlobs {
    pub include: String,
    pub exclude: String,
    pub show_globs: bool,
}

// ── Search panel ─────────────────────────────────────────────────────────────

/// The Search sidebar panel.
///
/// Provides workspace-wide text search with regex/case/word toggles,
/// replace mode, results grouped by file, and include/exclude glob patterns.
#[allow(dead_code)]
pub struct SearchPanel<OnSearch, OnReplace>
where
    OnSearch: FnMut(&str, &SearchOptions, &SearchGlobs),
    OnReplace: FnMut(ReplaceScope, &str, &str),
{
    pub query: String,
    pub replace_text: String,
    pub options: SearchOptions,
    pub globs: SearchGlobs,
    pub results: Vec<FileSearchResult>,
    pub replace_mode: bool,
    pub on_search: OnSearch,
    pub on_replace: OnReplace,

    selected_file: Option<usize>,
    selected_match: Option<(usize, usize)>,
    scroll_offset: f32,
    focused: bool,
    focused_field: SearchField,

    total_match_count: u32,
    total_file_count: u32,

    row_height: f32,
    input_height: f32,
    toggle_size: f32,

    background: Color,
    input_bg: Color,
    input_border: Color,
    input_border_focused: Color,
    toggle_active_bg: Color,
    toggle_inactive_bg: Color,
    file_row_bg: Color,
    match_highlight: Color,
    selected_bg: Color,
    badge_bg: Color,
    badge_fg: Color,
    foreground: Color,
}

/// Which input field is focused in the search panel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchField {
    #[default]
    Query,
    Replace,
    IncludeGlob,
    ExcludeGlob,
}

/// Scope of a replace operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplaceScope {
    All,
    File(PathBuf),
    Single { file: PathBuf, line: u32, column: u32 },
}

impl<OnSearch, OnReplace> SearchPanel<OnSearch, OnReplace>
where
    OnSearch: FnMut(&str, &SearchOptions, &SearchGlobs),
    OnReplace: FnMut(ReplaceScope, &str, &str),
{
    pub fn new(on_search: OnSearch, on_replace: OnReplace) -> Self {
        Self {
            query: String::new(),
            replace_text: String::new(),
            options: SearchOptions::default(),
            globs: SearchGlobs::default(),
            results: Vec::new(),
            replace_mode: false,
            on_search,
            on_replace,

            selected_file: None,
            selected_match: None,
            scroll_offset: 0.0,
            focused: false,
            focused_field: SearchField::Query,

            total_match_count: 0,
            total_file_count: 0,

            row_height: 22.0,
            input_height: 28.0,
            toggle_size: 22.0,

            background: Color::from_hex("#252526").unwrap_or(Color::BLACK),
            input_bg: Color::from_hex("#3c3c3c").unwrap_or(Color::BLACK),
            input_border: Color::from_hex("#3c3c3c").unwrap_or(Color::BLACK),
            input_border_focused: Color::from_hex("#007fd4").unwrap_or(Color::WHITE),
            toggle_active_bg: Color::from_hex("#5a5d5e80").unwrap_or(Color::BLACK),
            toggle_inactive_bg: Color::TRANSPARENT,
            file_row_bg: Color::from_hex("#2a2d2e").unwrap_or(Color::BLACK),
            match_highlight: Color::from_hex("#ea5c0055").unwrap_or(Color::BLACK),
            selected_bg: Color::from_hex("#04395e").unwrap_or(Color::BLACK),
            badge_bg: Color::from_hex("#4d4d4d").unwrap_or(Color::BLACK),
            badge_fg: Color::WHITE,
            foreground: Color::from_hex("#cccccc").unwrap_or(Color::WHITE),
        }
    }

    pub fn set_results(&mut self, results: Vec<FileSearchResult>) {
        self.total_file_count = results.len() as u32;
        self.total_match_count = results.iter().map(|f| f.matches.len() as u32).sum();
        self.results = results;
    }

    pub fn toggle_regex(&mut self) {
        self.options.regex = !self.options.regex;
        self.trigger_search();
    }

    pub fn toggle_case(&mut self) {
        self.options.case_sensitive = !self.options.case_sensitive;
        self.trigger_search();
    }

    pub fn toggle_whole_word(&mut self) {
        self.options.whole_word = !self.options.whole_word;
        self.trigger_search();
    }

    pub fn toggle_replace_mode(&mut self) {
        self.replace_mode = !self.replace_mode;
    }

    pub fn toggle_globs(&mut self) {
        self.globs.show_globs = !self.globs.show_globs;
    }

    pub fn replace_all(&mut self) {
        if !self.query.is_empty() {
            (self.on_replace)(ReplaceScope::All, &self.query.clone(), &self.replace_text.clone());
        }
    }

    pub fn replace_in_file(&mut self, index: usize) {
        if let Some(file) = self.results.get(index) {
            let path = file.path.clone();
            let q = self.query.clone();
            let r = self.replace_text.clone();
            (self.on_replace)(ReplaceScope::File(path), &q, &r);
        }
    }

    pub fn result_count_label(&self) -> String {
        format!(
            "{} results in {} files",
            self.total_match_count, self.total_file_count
        )
    }

    fn trigger_search(&mut self) {
        if !self.query.is_empty() {
            let q = self.query.clone();
            let opts = self.options.clone();
            let globs = self.globs.clone();
            (self.on_search)(&q, &opts, &globs);
        }
    }

    #[allow(clippy::cast_precision_loss)]
    fn header_height(&self) -> f32 {
        let mut h = self.input_height + 8.0;
        if self.replace_mode {
            h += self.input_height + 4.0;
        }
        if self.globs.show_globs {
            h += (self.input_height + 4.0) * 2.0;
        }
        h += self.row_height; // result count
        h
    }

    fn toggle_file_expanded(&mut self, index: usize) {
        if let Some(file) = self.results.get_mut(index) {
            file.expanded = !file.expanded;
        }
    }
}

impl<OnSearch, OnReplace> Widget for SearchPanel<OnSearch, OnReplace>
where
    OnSearch: FnMut(&str, &SearchOptions, &SearchGlobs),
    OnReplace: FnMut(ReplaceScope, &str, &str),
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

        let mut y = rect.y + 8.0;
        let input_x = rect.x + 8.0;
        let input_w = rect.width - 16.0 - (self.toggle_size + 2.0) * 3.0;

        // Search input
        let border = if self.focused_field == SearchField::Query {
            self.input_border_focused
        } else {
            self.input_border
        };
        rr.draw_rect(input_x, y, input_w, self.input_height, self.input_bg, 2.0);
        rr.draw_border(input_x, y, input_w, self.input_height, border, 1.0);

        // Toggle buttons (regex, case, word)
        let toggle_y = y + (self.input_height - self.toggle_size) / 2.0;
        let mut tx = input_x + input_w + 4.0;
        for active in [self.options.regex, self.options.case_sensitive, self.options.whole_word] {
            let bg = if active {
                self.toggle_active_bg
            } else {
                self.toggle_inactive_bg
            };
            rr.draw_rect(tx, toggle_y, self.toggle_size, self.toggle_size, bg, 3.0);
            tx += self.toggle_size + 2.0;
        }
        y += self.input_height + 4.0;

        // Replace input
        if self.replace_mode {
            let rborder = if self.focused_field == SearchField::Replace {
                self.input_border_focused
            } else {
                self.input_border
            };
            rr.draw_rect(input_x, y, input_w + (self.toggle_size + 2.0) * 3.0, self.input_height, self.input_bg, 2.0);
            rr.draw_border(input_x, y, input_w + (self.toggle_size + 2.0) * 3.0, self.input_height, rborder, 1.0);
            y += self.input_height + 4.0;
        }

        // Glob patterns
        if self.globs.show_globs {
            let full_w = rect.width - 16.0;
            for field in [SearchField::IncludeGlob, SearchField::ExcludeGlob] {
                let gb = if self.focused_field == field {
                    self.input_border_focused
                } else {
                    self.input_border
                };
                rr.draw_rect(input_x, y, full_w, self.input_height, self.input_bg, 2.0);
                rr.draw_border(input_x, y, full_w, self.input_height, gb, 1.0);
                y += self.input_height + 4.0;
            }
        }

        // Result count badge
        if self.total_match_count > 0 {
            let badge_w = 60.0;
            rr.draw_rect(
                rect.x + rect.width - badge_w - 8.0,
                y + 2.0,
                badge_w,
                self.row_height - 4.0,
                self.badge_bg,
                8.0,
            );
        }
        y += self.row_height;

        // Results tree
        for (fi, file) in self.results.iter().enumerate() {
            if y > rect.y + rect.height {
                break;
            }
            // File header
            let is_sel_file = self.selected_file == Some(fi);
            if is_sel_file {
                rr.draw_rect(rect.x, y, rect.width, self.row_height, self.selected_bg, 0.0);
            }
            rr.draw_rect(rect.x, y, rect.width, self.row_height, self.file_row_bg, 0.0);

            // Match count badge per file
            let mc = file.match_count();
            if mc > 0 {
                let badge_w = 24.0;
                rr.draw_rect(
                    rect.x + rect.width - badge_w - 8.0,
                    y + 3.0,
                    badge_w,
                    self.row_height - 6.0,
                    self.badge_bg,
                    7.0,
                );
            }
            y += self.row_height;

            // Individual matches
            if file.expanded {
                for (mi, _m) in file.matches.iter().enumerate() {
                    if y > rect.y + rect.height {
                        break;
                    }
                    let is_sel_match = self.selected_match == Some((fi, mi));
                    if is_sel_match {
                        rr.draw_rect(
                            rect.x,
                            y,
                            rect.width,
                            self.row_height,
                            self.selected_bg,
                            0.0,
                        );
                    }
                    y += self.row_height;
                }
            }
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
                let results_top = rect.y + self.header_height();
                if *y >= results_top {
                    let mut row_y = results_top - self.scroll_offset;
                    for (fi, file) in self.results.iter().enumerate() {
                        if *y >= row_y && *y < row_y + self.row_height {
                            self.selected_file = Some(fi);
                            self.selected_match = None;
                            self.toggle_file_expanded(fi);
                            return EventResult::Handled;
                        }
                        row_y += self.row_height;
                        if file.expanded {
                            for mi in 0..file.matches.len() {
                                if *y >= row_y && *y < row_y + self.row_height {
                                    self.selected_file = Some(fi);
                                    self.selected_match = Some((fi, mi));
                                    return EventResult::Handled;
                                }
                                row_y += self.row_height;
                            }
                        }
                    }
                }
                EventResult::Handled
            }
            UiEvent::MouseScroll { dy, .. } => {
                let total = self.results.iter().fold(0.0_f32, |acc, f| {
                    acc + self.row_height
                        + if f.expanded {
                            f.matches.len() as f32 * self.row_height
                        } else {
                            0.0
                        }
                });
                let max = (total - rect.height + self.header_height()).max(0.0);
                self.scroll_offset = (self.scroll_offset - dy * 40.0).clamp(0.0, max);
                EventResult::Handled
            }
            UiEvent::KeyPress { key: Key::Enter, .. } if self.focused => {
                self.trigger_search();
                EventResult::Handled
            }
            UiEvent::KeyPress { key: Key::Escape, .. } if self.focused => {
                self.query.clear();
                self.results.clear();
                self.total_match_count = 0;
                self.total_file_count = 0;
                EventResult::Handled
            }
            _ => EventResult::Ignored,
        }
    }
}
