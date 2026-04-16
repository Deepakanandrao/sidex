//! Terminal panel — multiple terminal instances with tabs and split support.

use sidex_gpu::color::Color;
use sidex_gpu::GpuRenderer;

use crate::layout::{LayoutNode, Rect, Size};
use crate::widget::{EventResult, MouseButton, UiEvent, Widget};

// ── Terminal instance ────────────────────────────────────────────────────────

/// A single terminal instance.
#[derive(Clone, Debug)]
pub struct TerminalInstance {
    pub id: u64,
    pub title: String,
    pub shell_type: ShellType,
    pub cwd: Option<String>,
    pub process_id: Option<u32>,
    pub is_busy: bool,
    pub exit_code: Option<i32>,
}

impl TerminalInstance {
    pub fn new(id: u64, title: impl Into<String>, shell_type: ShellType) -> Self {
        Self {
            id,
            title: title.into(),
            shell_type,
            cwd: None,
            process_id: None,
            is_busy: false,
            exit_code: None,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.exit_code.is_none()
    }
}

/// Supported shell types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellType {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Cmd,
    Nushell,
    Custom,
}

impl ShellType {
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Fish => "fish",
            Self::PowerShell => "pwsh",
            Self::Cmd => "cmd",
            Self::Nushell => "nu",
            Self::Custom => "terminal",
        }
    }
}

// ── Split layout ─────────────────────────────────────────────────────────────

/// Layout of terminals within a split group.
#[derive(Clone, Debug)]
pub enum TerminalSplit {
    Single(u64),
    Horizontal(Vec<TerminalSplit>),
    Vertical(Vec<TerminalSplit>),
}

impl TerminalSplit {
    pub fn terminal_ids(&self) -> Vec<u64> {
        match self {
            Self::Single(id) => vec![*id],
            Self::Horizontal(splits) | Self::Vertical(splits) => {
                splits.iter().flat_map(Self::terminal_ids).collect()
            }
        }
    }
}

// ── Terminal actions ─────────────────────────────────────────────────────────

/// Actions the terminal panel can trigger.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TerminalAction {
    Create(ShellType),
    Close(u64),
    Split(u64),
    Focus(u64),
    Rename(u64, String),
    Clear(u64),
    Kill(u64),
    SelectShell,
    ScrollUp(u64),
    ScrollDown(u64),
    ScrollToTop(u64),
    ScrollToBottom(u64),
}

// ── Terminal panel ───────────────────────────────────────────────────────────

/// The Terminal bottom panel.
///
/// Manages multiple terminal instances with a tab bar, split terminal support,
/// shell selector, and delegates grid rendering to `sidex-terminal`.
#[allow(dead_code)]
pub struct TerminalPanel<OnAction>
where
    OnAction: FnMut(TerminalAction),
{
    pub instances: Vec<TerminalInstance>,
    pub active_terminal: Option<u64>,
    pub splits: Vec<TerminalSplit>,
    pub on_action: OnAction,

    tab_scroll_offset: f32,
    focused: bool,
    shell_selector_open: bool,

    tab_bar_height: f32,
    tab_width: f32,
    row_height: f32,

    background: Color,
    tab_bar_bg: Color,
    tab_active_bg: Color,
    tab_inactive_bg: Color,
    tab_active_fg: Color,
    tab_inactive_fg: Color,
    tab_active_border: Color,
    tab_hover_bg: Color,
    border_color: Color,
    close_hover_bg: Color,
    shell_selector_bg: Color,
    shell_selector_hover: Color,
    busy_indicator: Color,
    dead_indicator: Color,
    foreground: Color,
}

impl<OnAction> TerminalPanel<OnAction>
where
    OnAction: FnMut(TerminalAction),
{
    pub fn new(on_action: OnAction) -> Self {
        Self {
            instances: Vec::new(),
            active_terminal: None,
            splits: Vec::new(),
            on_action,

            tab_scroll_offset: 0.0,
            focused: false,
            shell_selector_open: false,

            tab_bar_height: 28.0,
            tab_width: 120.0,
            row_height: 24.0,

            background: Color::from_hex("#1e1e1e").unwrap_or(Color::BLACK),
            tab_bar_bg: Color::from_hex("#252526").unwrap_or(Color::BLACK),
            tab_active_bg: Color::from_hex("#1e1e1e").unwrap_or(Color::BLACK),
            tab_inactive_bg: Color::from_hex("#2d2d2d").unwrap_or(Color::BLACK),
            tab_active_fg: Color::from_hex("#e7e7e7").unwrap_or(Color::WHITE),
            tab_inactive_fg: Color::from_hex("#969696").unwrap_or(Color::WHITE),
            tab_active_border: Color::from_hex("#e7e7e7").unwrap_or(Color::WHITE),
            tab_hover_bg: Color::from_hex("#2a2d2e").unwrap_or(Color::BLACK),
            border_color: Color::from_hex("#2b2b2b").unwrap_or(Color::BLACK),
            close_hover_bg: Color::from_hex("#404040").unwrap_or(Color::BLACK),
            shell_selector_bg: Color::from_hex("#3c3c3c").unwrap_or(Color::BLACK),
            shell_selector_hover: Color::from_hex("#04395e").unwrap_or(Color::BLACK),
            busy_indicator: Color::from_rgb(226, 192, 81),
            dead_indicator: Color::from_rgb(193, 74, 74),
            foreground: Color::from_hex("#cccccc").unwrap_or(Color::WHITE),
        }
    }

    pub fn add_terminal(&mut self, instance: TerminalInstance) {
        let id = instance.id;
        self.splits.push(TerminalSplit::Single(id));
        self.instances.push(instance);
        self.active_terminal = Some(id);
    }

    pub fn close_terminal(&mut self, id: u64) {
        self.instances.retain(|t| t.id != id);
        self.splits.retain(|s| !matches!(s, TerminalSplit::Single(sid) if *sid == id));
        if self.active_terminal == Some(id) {
            self.active_terminal = self.instances.first().map(|t| t.id);
        }
        (self.on_action)(TerminalAction::Close(id));
    }

    pub fn split_active(&mut self) {
        if let Some(id) = self.active_terminal {
            (self.on_action)(TerminalAction::Split(id));
        }
    }

    pub fn create_terminal(&mut self, shell: ShellType) {
        (self.on_action)(TerminalAction::Create(shell));
    }

    pub fn focus_terminal(&mut self, id: u64) {
        self.active_terminal = Some(id);
        (self.on_action)(TerminalAction::Focus(id));
    }

    fn tab_rect_at(&self, index: usize, rect: Rect) -> Rect {
        Rect::new(
            rect.x + index as f32 * self.tab_width - self.tab_scroll_offset,
            rect.y,
            self.tab_width,
            self.tab_bar_height,
        )
    }

    fn close_button_rect(&self, tab_rect: Rect) -> Rect {
        let s = 14.0;
        Rect::new(
            tab_rect.x + tab_rect.width - s - 6.0,
            tab_rect.y + (tab_rect.height - s) / 2.0,
            s,
            s,
        )
    }
}

impl<OnAction> Widget for TerminalPanel<OnAction>
where
    OnAction: FnMut(TerminalAction),
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

        // Tab bar
        rr.draw_rect(
            rect.x,
            rect.y,
            rect.width,
            self.tab_bar_height,
            self.tab_bar_bg,
            0.0,
        );

        for (i, inst) in self.instances.iter().enumerate() {
            let tr = self.tab_rect_at(i, rect);
            if tr.right() < rect.x || tr.x > rect.right() {
                continue;
            }

            let is_active = self.active_terminal == Some(inst.id);
            let bg = if is_active {
                self.tab_active_bg
            } else {
                self.tab_inactive_bg
            };
            rr.draw_rect(tr.x, tr.y, tr.width, tr.height, bg, 0.0);

            if is_active {
                rr.draw_rect(
                    tr.x,
                    tr.y + tr.height - 2.0,
                    tr.width,
                    2.0,
                    self.tab_active_border,
                    0.0,
                );
            }

            // Status indicator
            if inst.is_busy {
                rr.draw_rect(tr.x + 6.0, tr.y + tr.height / 2.0 - 3.0, 6.0, 6.0, self.busy_indicator, 3.0);
            } else if !inst.is_alive() {
                rr.draw_rect(tr.x + 6.0, tr.y + tr.height / 2.0 - 3.0, 6.0, 6.0, self.dead_indicator, 3.0);
            }

            // Close button
            let cr = self.close_button_rect(tr);
            rr.draw_rect(cr.x, cr.y, cr.width, cr.height, Color::TRANSPARENT, 2.0);

            // Tab separator
            rr.draw_rect(tr.right() - 1.0, tr.y + 4.0, 1.0, tr.height - 8.0, self.border_color, 0.0);
        }

        // New terminal / split buttons at right of tab bar
        let btn_s = 20.0;
        let new_x = rect.x + rect.width - btn_s * 2.0 - 12.0;
        rr.draw_rect(new_x, rect.y + 4.0, btn_s, btn_s, self.shell_selector_bg, 3.0);
        rr.draw_rect(new_x + btn_s + 4.0, rect.y + 4.0, btn_s, btn_s, self.shell_selector_bg, 3.0);

        // Border below tab bar
        rr.draw_rect(
            rect.x,
            rect.y + self.tab_bar_height,
            rect.width,
            1.0,
            self.border_color,
            0.0,
        );

        // Shell selector dropdown
        if self.shell_selector_open {
            let shells = [ShellType::Bash, ShellType::Zsh, ShellType::Fish, ShellType::PowerShell];
            let menu_h = shells.len() as f32 * self.row_height;
            let menu_y = rect.y + self.tab_bar_height + 1.0;
            rr.draw_rect(new_x, menu_y, 140.0, menu_h, self.shell_selector_bg, 2.0);
        }

        // Terminal content area (delegated to sidex-terminal at runtime)
        let content_rect = Rect::new(
            rect.x,
            rect.y + self.tab_bar_height + 1.0,
            rect.width,
            rect.height - self.tab_bar_height - 1.0,
        );
        rr.draw_rect(
            content_rect.x,
            content_rect.y,
            content_rect.width,
            content_rect.height,
            self.background,
            0.0,
        );

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
                self.shell_selector_open = false;
                EventResult::Handled
            }
            UiEvent::MouseDown {
                x,
                y,
                button: MouseButton::Left,
            } if rect.contains(*x, *y) => {
                self.focused = true;

                // Tab bar clicks
                if *y < rect.y + self.tab_bar_height {
                    // New terminal button
                    let btn_s = 20.0;
                    let new_x = rect.x + rect.width - btn_s * 2.0 - 12.0;
                    if *x >= new_x && *x < new_x + btn_s {
                        self.shell_selector_open = !self.shell_selector_open;
                        return EventResult::Handled;
                    }
                    if *x >= new_x + btn_s + 4.0 && *x < new_x + btn_s * 2.0 + 4.0 {
                        self.split_active();
                        return EventResult::Handled;
                    }

                    // Tab clicks
                    for (i, inst) in self.instances.iter().enumerate() {
                        let tr = self.tab_rect_at(i, rect);
                        if tr.contains(*x, *y) {
                            let cr = self.close_button_rect(tr);
                            if cr.contains(*x, *y) {
                                let id = inst.id;
                                self.close_terminal(id);
                            } else {
                                self.focus_terminal(inst.id);
                            }
                            return EventResult::Handled;
                        }
                    }
                    return EventResult::Handled;
                }

                // Shell selector dropdown
                if self.shell_selector_open {
                    let shells = [ShellType::Bash, ShellType::Zsh, ShellType::Fish, ShellType::PowerShell];
                    let menu_y = rect.y + self.tab_bar_height + 1.0;
                    let idx = ((*y - menu_y) / self.row_height) as usize;
                    if idx < shells.len() {
                        self.create_terminal(shells[idx]);
                    }
                    self.shell_selector_open = false;
                    return EventResult::Handled;
                }

                EventResult::Handled
            }
            UiEvent::MouseScroll { dx, .. } if rect.contains(rect.x, rect.y) => {
                // Tab bar horizontal scroll
                let total = self.instances.len() as f32 * self.tab_width;
                let max = (total - rect.width).max(0.0);
                self.tab_scroll_offset = (self.tab_scroll_offset - dx * 30.0).clamp(0.0, max);
                EventResult::Handled
            }
            _ => EventResult::Ignored,
        }
    }
}
