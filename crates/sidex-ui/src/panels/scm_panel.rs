//! Source Control Management panel — staging, commit, branch, and diff.

use std::path::PathBuf;

use sidex_gpu::color::Color;
use sidex_gpu::GpuRenderer;

use crate::layout::{LayoutNode, Rect, Size};
use crate::widget::{EventResult, Key, MouseButton, UiEvent, Widget};

// ── Change status ────────────────────────────────────────────────────────────

/// Git change status for a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangeStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    Conflicted,
    Copied,
    Ignored,
}

impl ChangeStatus {
    pub fn letter(self) -> char {
        match self {
            Self::Modified => 'M',
            Self::Added => 'A',
            Self::Deleted => 'D',
            Self::Renamed => 'R',
            Self::Untracked => 'U',
            Self::Conflicted => '!',
            Self::Copied => 'C',
            Self::Ignored => 'I',
        }
    }

    pub fn color(self) -> Color {
        match self {
            Self::Modified => Color::from_rgb(226, 192, 81),
            Self::Added => Color::from_rgb(81, 154, 81),
            Self::Deleted => Color::from_rgb(193, 74, 74),
            Self::Renamed => Color::from_rgb(115, 196, 143),
            Self::Untracked => Color::from_rgb(115, 196, 143),
            Self::Conflicted => Color::from_rgb(220, 100, 100),
            Self::Copied => Color::from_rgb(115, 196, 143),
            Self::Ignored => Color::from_rgb(128, 128, 128),
        }
    }
}

// ── File change ──────────────────────────────────────────────────────────────

/// A changed file in the working tree or index.
#[derive(Clone, Debug)]
pub struct FileChange {
    pub path: PathBuf,
    pub original_path: Option<PathBuf>,
    pub status: ChangeStatus,
}

impl FileChange {
    pub fn new(path: impl Into<PathBuf>, status: ChangeStatus) -> Self {
        Self {
            path: path.into(),
            original_path: None,
            status,
        }
    }

    pub fn renamed(
        path: impl Into<PathBuf>,
        original: impl Into<PathBuf>,
    ) -> Self {
        Self {
            path: path.into(),
            original_path: Some(original.into()),
            status: ChangeStatus::Renamed,
        }
    }

    pub fn filename(&self) -> &str {
        self.path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
    }
}

// ── SCM actions ──────────────────────────────────────────────────────────────

/// Actions the SCM panel can trigger.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScmAction {
    StageFile(PathBuf),
    UnstageFile(PathBuf),
    StageAll,
    UnstageAll,
    DiscardFile(PathBuf),
    DiscardAll,
    Commit(String),
    Push,
    Pull,
    Fetch,
    OpenDiff(PathBuf),
    StageHunk { file: PathBuf, hunk_index: usize },
}

// ── Change group ─────────────────────────────────────────────────────────────

/// Groups of changes displayed in the SCM panel.
#[derive(Clone, Debug)]
pub struct ChangeGroup {
    pub label: String,
    pub changes: Vec<FileChange>,
    pub expanded: bool,
}

impl ChangeGroup {
    pub fn new(label: impl Into<String>, changes: Vec<FileChange>) -> Self {
        Self {
            label: label.into(),
            changes,
            expanded: true,
        }
    }
}

// ── SCM panel ────────────────────────────────────────────────────────────────

/// The Source Control sidebar panel.
///
/// Shows staged/unstaged/untracked changes with status icons, provides
/// stage/unstage/discard actions, a commit message input, and branch
/// indicator with push/pull buttons.
#[allow(dead_code)]
pub struct ScmPanel<OnAction>
where
    OnAction: FnMut(ScmAction),
{
    pub staged: ChangeGroup,
    pub changes: ChangeGroup,
    pub untracked: ChangeGroup,
    pub commit_message: String,
    pub branch_name: String,
    pub ahead_count: u32,
    pub behind_count: u32,
    pub on_action: OnAction,

    selected_group: Option<usize>,
    selected_file: Option<(usize, usize)>,
    scroll_offset: f32,
    focused: bool,
    commit_input_focused: bool,

    row_height: f32,
    commit_input_height: f32,
    header_height: f32,
    button_height: f32,

    background: Color,
    input_bg: Color,
    input_border: Color,
    input_border_focused: Color,
    header_bg: Color,
    selected_bg: Color,
    hover_bg: Color,
    button_bg: Color,
    button_fg: Color,
    foreground: Color,
    secondary_fg: Color,
    separator_color: Color,
}

impl<OnAction> ScmPanel<OnAction>
where
    OnAction: FnMut(ScmAction),
{
    pub fn new(on_action: OnAction) -> Self {
        Self {
            staged: ChangeGroup::new("Staged Changes", Vec::new()),
            changes: ChangeGroup::new("Changes", Vec::new()),
            untracked: ChangeGroup::new("Untracked", Vec::new()),
            commit_message: String::new(),
            branch_name: String::from("main"),
            ahead_count: 0,
            behind_count: 0,
            on_action,

            selected_group: None,
            selected_file: None,
            scroll_offset: 0.0,
            focused: false,
            commit_input_focused: false,

            row_height: 22.0,
            commit_input_height: 60.0,
            header_height: 26.0,
            button_height: 28.0,

            background: Color::from_hex("#252526").unwrap_or(Color::BLACK),
            input_bg: Color::from_hex("#3c3c3c").unwrap_or(Color::BLACK),
            input_border: Color::from_hex("#3c3c3c").unwrap_or(Color::BLACK),
            input_border_focused: Color::from_hex("#007fd4").unwrap_or(Color::WHITE),
            header_bg: Color::from_hex("#383838").unwrap_or(Color::BLACK),
            selected_bg: Color::from_hex("#04395e").unwrap_or(Color::BLACK),
            hover_bg: Color::from_hex("#2a2d2e").unwrap_or(Color::BLACK),
            button_bg: Color::from_hex("#0e639c").unwrap_or(Color::BLACK),
            button_fg: Color::WHITE,
            foreground: Color::from_hex("#cccccc").unwrap_or(Color::WHITE),
            secondary_fg: Color::from_hex("#969696").unwrap_or(Color::WHITE),
            separator_color: Color::from_hex("#2b2b2b").unwrap_or(Color::BLACK),
        }
    }

    pub fn set_changes(
        &mut self,
        staged: Vec<FileChange>,
        unstaged: Vec<FileChange>,
        untracked: Vec<FileChange>,
    ) {
        self.staged.changes = staged;
        self.changes.changes = unstaged;
        self.untracked.changes = untracked;
    }

    pub fn set_branch(&mut self, name: impl Into<String>, ahead: u32, behind: u32) {
        self.branch_name = name.into();
        self.ahead_count = ahead;
        self.behind_count = behind;
    }

    pub fn commit(&mut self) {
        if !self.commit_message.is_empty() {
            let msg = self.commit_message.clone();
            (self.on_action)(ScmAction::Commit(msg));
            self.commit_message.clear();
        }
    }

    pub fn stage_all(&mut self) {
        (self.on_action)(ScmAction::StageAll);
    }

    pub fn unstage_all(&mut self) {
        (self.on_action)(ScmAction::UnstageAll);
    }

    pub fn push(&mut self) {
        (self.on_action)(ScmAction::Push);
    }

    pub fn pull(&mut self) {
        (self.on_action)(ScmAction::Pull);
    }

    fn groups(&self) -> [&ChangeGroup; 3] {
        [&self.staged, &self.changes, &self.untracked]
    }

    fn toggle_group_expanded(&mut self, gi: usize) {
        match gi {
            0 => self.staged.expanded = !self.staged.expanded,
            1 => self.changes.expanded = !self.changes.expanded,
            2 => self.untracked.expanded = !self.untracked.expanded,
            _ => {}
        }
    }

    #[allow(dead_code)]
    fn group_expanded(&self, gi: usize) -> bool {
        match gi {
            0 => self.staged.expanded,
            1 => self.changes.expanded,
            2 => self.untracked.expanded,
            _ => false,
        }
    }

    fn commit_area_height(&self) -> f32 {
        self.commit_input_height + self.button_height + 12.0
    }
}

impl<OnAction> Widget for ScmPanel<OnAction>
where
    OnAction: FnMut(ScmAction),
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

        let mut y = rect.y + 4.0;
        let pad = 8.0;

        // Commit message input
        let ib = if self.commit_input_focused {
            self.input_border_focused
        } else {
            self.input_border
        };
        rr.draw_rect(
            rect.x + pad,
            y,
            rect.width - pad * 2.0,
            self.commit_input_height,
            self.input_bg,
            2.0,
        );
        rr.draw_border(
            rect.x + pad,
            y,
            rect.width - pad * 2.0,
            self.commit_input_height,
            ib,
            1.0,
        );
        y += self.commit_input_height + 4.0;

        // Commit button
        rr.draw_rect(
            rect.x + pad,
            y,
            rect.width - pad * 2.0,
            self.button_height,
            self.button_bg,
            3.0,
        );
        y += self.button_height + 8.0;

        // Separator
        rr.draw_rect(rect.x, y, rect.width, 1.0, self.separator_color, 0.0);
        y += 1.0;

        // Change groups
        for (gi, group) in self.groups().iter().enumerate() {
            if group.changes.is_empty() {
                continue;
            }
            // Group header
            rr.draw_rect(
                rect.x,
                y,
                rect.width,
                self.header_height,
                self.header_bg,
                0.0,
            );

            // Count badge
            let count = group.changes.len();
            if count > 0 {
                let badge_w = 22.0;
                rr.draw_rect(
                    rect.x + rect.width - badge_w - 8.0,
                    y + 4.0,
                    badge_w,
                    self.header_height - 8.0,
                    Color::from_hex("#4d4d4d").unwrap_or(Color::BLACK),
                    7.0,
                );
            }
            y += self.header_height;

            // File changes
            if group.expanded {
                for (fi, change) in group.changes.iter().enumerate() {
                    if y > rect.y + rect.height {
                        break;
                    }
                    let is_sel = self.selected_file == Some((gi, fi));
                    if is_sel {
                        rr.draw_rect(
                            rect.x,
                            y,
                            rect.width,
                            self.row_height,
                            self.selected_bg,
                            0.0,
                        );
                    }

                    // Status letter badge
                    let status_color = change.status.color();
                    let badge_x = rect.x + rect.width - 24.0;
                    rr.draw_rect(
                        badge_x,
                        y + 3.0,
                        16.0,
                        self.row_height - 6.0,
                        status_color,
                        2.0,
                    );

                    y += self.row_height;
                }
            }
        }

        // Branch indicator at bottom
        let branch_y = rect.y + rect.height - self.row_height;
        rr.draw_rect(
            rect.x,
            branch_y,
            rect.width,
            self.row_height,
            self.header_bg,
            0.0,
        );
        if self.ahead_count > 0 || self.behind_count > 0 {
            let sync_x = rect.x + rect.width - 60.0;
            rr.draw_rect(sync_x, branch_y + 3.0, 50.0, self.row_height - 6.0, self.button_bg, 3.0);
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
                self.commit_input_focused = false;
                EventResult::Handled
            }
            UiEvent::MouseDown {
                x,
                y,
                button: MouseButton::Left,
            } if rect.contains(*x, *y) => {
                self.focused = true;
                let commit_area_end = rect.y + 4.0 + self.commit_area_height();

                if *y < commit_area_end {
                    let input_bottom = rect.y + 4.0 + self.commit_input_height;
                    if *y < input_bottom {
                        self.commit_input_focused = true;
                    } else {
                        self.commit_input_focused = false;
                        let btn_y = input_bottom + 4.0;
                        if *y >= btn_y && *y < btn_y + self.button_height {
                            self.commit();
                        }
                    }
                    return EventResult::Handled;
                }

                self.commit_input_focused = false;
                let mut row_y = commit_area_end + 1.0;
                let group_infos: Vec<(bool, usize, Vec<FileChange>)> = [
                    (&self.staged, 0usize),
                    (&self.changes, 1),
                    (&self.untracked, 2),
                ]
                .iter()
                .map(|(g, gi)| (g.expanded, *gi, g.changes.clone()))
                .collect();

                for (expanded, gi, changes) in &group_infos {
                    if changes.is_empty() {
                        continue;
                    }
                    if *y >= row_y && *y < row_y + self.header_height {
                        self.toggle_group_expanded(*gi);
                        return EventResult::Handled;
                    }
                    row_y += self.header_height;
                    if *expanded {
                        for (fi, change) in changes.iter().enumerate() {
                            if *y >= row_y && *y < row_y + self.row_height {
                                self.selected_file = Some((*gi, fi));
                                let path = change.path.clone();
                                (self.on_action)(ScmAction::OpenDiff(path));
                                return EventResult::Handled;
                            }
                            row_y += self.row_height;
                        }
                    }
                }
                EventResult::Handled
            }
            UiEvent::KeyPress {
                key: Key::Enter, ..
            } if self.commit_input_focused => {
                self.commit();
                EventResult::Handled
            }
            _ => EventResult::Ignored,
        }
    }
}
