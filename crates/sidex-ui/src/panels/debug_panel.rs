//! Debug panel — variables, call stack, breakpoints, watch, toolbar, console.

use std::path::PathBuf;

use sidex_gpu::color::Color;
use sidex_gpu::GpuRenderer;

use crate::layout::{LayoutNode, Rect, Size};
use crate::widget::{EventResult, Key, MouseButton, UiEvent, Widget};

// ── Debug toolbar actions ────────────────────────────────────────────────────

/// Debug session control actions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DebugAction {
    Continue,
    Pause,
    StepOver,
    StepInto,
    StepOut,
    Restart,
    Stop,
    Disconnect,
}

// ── Variables ────────────────────────────────────────────────────────────────

/// A variable in the current scope.
#[derive(Clone, Debug)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub var_type: Option<String>,
    pub children: Vec<Variable>,
    pub expanded: bool,
    pub changed: bool,
}

impl Variable {
    pub fn leaf(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            var_type: None,
            children: Vec::new(),
            expanded: false,
            changed: false,
        }
    }

    pub fn object(
        name: impl Into<String>,
        value: impl Into<String>,
        children: Vec<Variable>,
    ) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            var_type: None,
            children,
            expanded: false,
            changed: false,
        }
    }

    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }
}

// ── Call stack ────────────────────────────────────────────────────────────────

/// A single frame in the call stack.
#[derive(Clone, Debug)]
pub struct StackFrame {
    pub id: u64,
    pub name: String,
    pub source_path: Option<PathBuf>,
    pub line: u32,
    pub column: u32,
    pub is_subtle: bool,
}

/// A thread with its call stack.
#[derive(Clone, Debug)]
pub struct DebugThread {
    pub id: u64,
    pub name: String,
    pub paused: bool,
    pub frames: Vec<StackFrame>,
    pub expanded: bool,
}

// ── Breakpoints ──────────────────────────────────────────────────────────────

/// A breakpoint set in the editor.
#[derive(Clone, Debug)]
pub struct Breakpoint {
    pub id: u64,
    pub path: PathBuf,
    pub line: u32,
    pub column: Option<u32>,
    pub enabled: bool,
    pub verified: bool,
    pub condition: Option<String>,
    pub hit_condition: Option<String>,
    pub log_message: Option<String>,
}

impl Breakpoint {
    pub fn new(path: impl Into<PathBuf>, line: u32) -> Self {
        Self {
            id: 0,
            path: path.into(),
            line,
            column: None,
            enabled: true,
            verified: true,
            condition: None,
            hit_condition: None,
            log_message: None,
        }
    }

    pub fn filename(&self) -> &str {
        self.path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
    }
}

// ── Watch expressions ────────────────────────────────────────────────────────

/// A user-defined watch expression.
#[derive(Clone, Debug)]
pub struct WatchExpression {
    pub id: u64,
    pub expression: String,
    pub value: Option<String>,
    pub error: Option<String>,
    pub children: Vec<Variable>,
    pub expanded: bool,
}

impl WatchExpression {
    pub fn new(id: u64, expression: impl Into<String>) -> Self {
        Self {
            id,
            expression: expression.into(),
            value: None,
            error: None,
            children: Vec::new(),
            expanded: false,
        }
    }
}

// ── Debug console entry ──────────────────────────────────────────────────────

/// An entry in the debug console output.
#[derive(Clone, Debug)]
pub enum ConsoleEntry {
    Output { text: String, category: OutputCategory },
    Evaluation { expression: String, result: String },
    Error { text: String },
}

/// Category of debug console output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputCategory {
    Stdout,
    Stderr,
    Console,
    Important,
}

// ── Debug session state ──────────────────────────────────────────────────────

/// Current state of the debug session.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DebugSessionState {
    #[default]
    Inactive,
    Running,
    Paused,
    Initializing,
}

// ── Section visibility ───────────────────────────────────────────────────────

/// Which sections of the debug panel are expanded.
#[derive(Clone, Debug)]
pub struct DebugSections {
    pub variables: bool,
    pub watch: bool,
    pub call_stack: bool,
    pub breakpoints: bool,
}

impl Default for DebugSections {
    fn default() -> Self {
        Self {
            variables: true,
            watch: true,
            call_stack: true,
            breakpoints: true,
        }
    }
}

// ── Debug panel ──────────────────────────────────────────────────────────────

/// The Debug sidebar panel.
///
/// Provides variables tree, call stack, breakpoints, watch expressions,
/// debug toolbar, and debug console output.
#[allow(dead_code)]
pub struct DebugPanel<OnAction, OnWatch>
where
    OnAction: FnMut(DebugAction),
    OnWatch: FnMut(WatchEvent),
{
    pub session_state: DebugSessionState,
    pub variables: Vec<Variable>,
    pub threads: Vec<DebugThread>,
    pub breakpoints: Vec<Breakpoint>,
    pub watch_expressions: Vec<WatchExpression>,
    pub console_entries: Vec<ConsoleEntry>,
    pub console_input: String,
    pub sections: DebugSections,

    pub on_action: OnAction,
    pub on_watch: OnWatch,

    selected_section: Option<usize>,
    scroll_offset: f32,
    focused: bool,
    console_scroll_offset: f32,

    row_height: f32,
    section_header_height: f32,
    toolbar_height: f32,
    indent_width: f32,

    background: Color,
    toolbar_bg: Color,
    toolbar_button_hover: Color,
    section_header_bg: Color,
    selected_bg: Color,
    changed_value_fg: Color,
    error_fg: Color,
    separator_color: Color,
    foreground: Color,
    secondary_fg: Color,
    breakpoint_enabled: Color,
    breakpoint_disabled: Color,
    console_bg: Color,
    console_input_bg: Color,
}

/// Events from watch expression interactions.
#[derive(Clone, Debug)]
pub enum WatchEvent {
    Add(String),
    Edit(u64, String),
    Remove(u64),
    Evaluate(String),
}

impl<OnAction, OnWatch> DebugPanel<OnAction, OnWatch>
where
    OnAction: FnMut(DebugAction),
    OnWatch: FnMut(WatchEvent),
{
    pub fn new(on_action: OnAction, on_watch: OnWatch) -> Self {
        Self {
            session_state: DebugSessionState::Inactive,
            variables: Vec::new(),
            threads: Vec::new(),
            breakpoints: Vec::new(),
            watch_expressions: Vec::new(),
            console_entries: Vec::new(),
            console_input: String::new(),
            sections: DebugSections::default(),

            on_action,
            on_watch,

            selected_section: None,
            scroll_offset: 0.0,
            focused: false,
            console_scroll_offset: 0.0,

            row_height: 22.0,
            section_header_height: 22.0,
            toolbar_height: 28.0,
            indent_width: 16.0,

            background: Color::from_hex("#252526").unwrap_or(Color::BLACK),
            toolbar_bg: Color::from_hex("#383838").unwrap_or(Color::BLACK),
            toolbar_button_hover: Color::from_hex("#505050").unwrap_or(Color::BLACK),
            section_header_bg: Color::from_hex("#383838").unwrap_or(Color::BLACK),
            selected_bg: Color::from_hex("#04395e").unwrap_or(Color::BLACK),
            changed_value_fg: Color::from_rgb(220, 100, 100),
            error_fg: Color::from_rgb(220, 80, 80),
            separator_color: Color::from_hex("#2b2b2b").unwrap_or(Color::BLACK),
            foreground: Color::from_hex("#cccccc").unwrap_or(Color::WHITE),
            secondary_fg: Color::from_hex("#969696").unwrap_or(Color::WHITE),
            breakpoint_enabled: Color::from_rgb(220, 60, 60),
            breakpoint_disabled: Color::from_rgb(120, 120, 120),
            console_bg: Color::from_hex("#1e1e1e").unwrap_or(Color::BLACK),
            console_input_bg: Color::from_hex("#3c3c3c").unwrap_or(Color::BLACK),
        }
    }

    pub fn set_paused(&mut self, variables: Vec<Variable>, threads: Vec<DebugThread>) {
        self.session_state = DebugSessionState::Paused;
        self.variables = variables;
        self.threads = threads;
    }

    pub fn set_running(&mut self) {
        self.session_state = DebugSessionState::Running;
        self.variables.clear();
    }

    pub fn stop_session(&mut self) {
        self.session_state = DebugSessionState::Inactive;
        self.variables.clear();
        self.threads.clear();
    }

    pub fn add_console_output(&mut self, text: impl Into<String>, category: OutputCategory) {
        self.console_entries.push(ConsoleEntry::Output {
            text: text.into(),
            category,
        });
    }

    pub fn toggle_breakpoint_enabled(&mut self, index: usize) {
        if let Some(bp) = self.breakpoints.get_mut(index) {
            bp.enabled = !bp.enabled;
        }
    }

    pub fn add_watch(&mut self, expression: impl Into<String>) {
        let expr = expression.into();
        let id = self.watch_expressions.len() as u64;
        (self.on_watch)(WatchEvent::Add(expr.clone()));
        self.watch_expressions.push(WatchExpression::new(id, expr));
    }

    pub fn remove_watch(&mut self, index: usize) {
        if let Some(w) = self.watch_expressions.get(index) {
            let id = w.id;
            (self.on_watch)(WatchEvent::Remove(id));
            self.watch_expressions.remove(index);
        }
    }

    fn count_variable_rows(vars: &[Variable], depth: usize) -> usize {
        let mut count = 0;
        for var in vars {
            count += 1;
            if var.expanded && var.has_children() {
                count += Self::count_variable_rows(&var.children, depth + 1);
            }
        }
        count
    }

    fn toolbar_buttons() -> &'static [(&'static str, DebugAction)] {
        &[
            ("Continue", DebugAction::Continue),
            ("Step Over", DebugAction::StepOver),
            ("Step Into", DebugAction::StepInto),
            ("Step Out", DebugAction::StepOut),
            ("Restart", DebugAction::Restart),
            ("Stop", DebugAction::Stop),
        ]
    }
}

impl<OnAction, OnWatch> Widget for DebugPanel<OnAction, OnWatch>
where
    OnAction: FnMut(DebugAction),
    OnWatch: FnMut(WatchEvent),
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

        // Debug toolbar
        if self.session_state != DebugSessionState::Inactive {
            rr.draw_rect(rect.x, y, rect.width, self.toolbar_height, self.toolbar_bg, 0.0);
            let buttons = Self::toolbar_buttons();
            let btn_size = 24.0;
            let btn_pad = 4.0;
            let total_w = buttons.len() as f32 * (btn_size + btn_pad);
            let mut bx = rect.x + (rect.width - total_w) / 2.0;
            for _btn in buttons {
                rr.draw_rect(bx, y + 2.0, btn_size, btn_size, self.toolbar_button_hover, 3.0);
                bx += btn_size + btn_pad;
            }
            y += self.toolbar_height;
            rr.draw_rect(rect.x, y, rect.width, 1.0, self.separator_color, 0.0);
            y += 1.0;
        }

        let sections: &[(&str, bool, usize)] = &[
            (
                "VARIABLES",
                self.sections.variables,
                Self::count_variable_rows(&self.variables, 0),
            ),
            ("WATCH", self.sections.watch, self.watch_expressions.len()),
            (
                "CALL STACK",
                self.sections.call_stack,
                self.threads.iter().map(|t| 1 + if t.expanded { t.frames.len() } else { 0 }).sum(),
            ),
            ("BREAKPOINTS", self.sections.breakpoints, self.breakpoints.len()),
        ];

        for (label, expanded, item_count) in sections {
            if y > rect.y + rect.height {
                break;
            }
            // Section header
            rr.draw_rect(
                rect.x,
                y,
                rect.width,
                self.section_header_height,
                self.section_header_bg,
                0.0,
            );
            y += self.section_header_height;

            if *expanded {
                let rows = *item_count;
                let section_h = rows as f32 * self.row_height;

                // Placeholder rows
                for r in 0..rows {
                    let ry = y + r as f32 * self.row_height;
                    if ry > rect.y + rect.height {
                        break;
                    }
                    // Just draw alternating for readability
                    if r % 2 == 1 {
                        rr.draw_rect(
                            rect.x,
                            ry,
                            rect.width,
                            self.row_height,
                            Color::from_hex("#ffffff06").unwrap_or(Color::TRANSPARENT),
                            0.0,
                        );
                    }
                }
                y += section_h;
            }

            let _ = label;
        }

        // Breakpoint indicators
        if self.sections.breakpoints {
            for bp in &self.breakpoints {
                let dot_color = if bp.enabled {
                    self.breakpoint_enabled
                } else {
                    self.breakpoint_disabled
                };
                let _ = dot_color;
            }
        }

        let _ = renderer;
    }

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

                // Toolbar click
                if self.session_state != DebugSessionState::Inactive {
                    let toolbar_bottom = rect.y + self.toolbar_height;
                    if *y < toolbar_bottom {
                        let buttons = Self::toolbar_buttons();
                        let btn_size = 24.0;
                        let btn_pad = 4.0;
                        let total_w = buttons.len() as f32 * (btn_size + btn_pad);
                        let start_x = rect.x + (rect.width - total_w) / 2.0;
                        for (i, (_label, action)) in buttons.iter().enumerate() {
                            let bx = start_x + i as f32 * (btn_size + btn_pad);
                            if *x >= bx && *x < bx + btn_size {
                                (self.on_action)(*action);
                                return EventResult::Handled;
                            }
                        }
                        return EventResult::Handled;
                    }
                }

                // Section header toggles
                let toolbar_h = if self.session_state != DebugSessionState::Inactive {
                    self.toolbar_height + 1.0
                } else {
                    0.0
                };
                let mut section_y = rect.y + toolbar_h;
                let section_expanded = [
                    &mut self.sections.variables,
                    &mut self.sections.watch,
                    &mut self.sections.call_stack,
                    &mut self.sections.breakpoints,
                ];
                for expanded in section_expanded {
                    if *y >= section_y && *y < section_y + self.section_header_height {
                        *expanded = !*expanded;
                        return EventResult::Handled;
                    }
                    section_y += self.section_header_height;
                    if *expanded {
                        section_y += self.row_height * 5.0;
                    }
                }

                EventResult::Handled
            }
            UiEvent::MouseScroll { dy, .. } => {
                self.scroll_offset = (self.scroll_offset - dy * 40.0).max(0.0);
                EventResult::Handled
            }
            UiEvent::KeyPress { key: Key::F(5), .. } => {
                (self.on_action)(DebugAction::Continue);
                EventResult::Handled
            }
            UiEvent::KeyPress { key: Key::F(10), .. } => {
                (self.on_action)(DebugAction::StepOver);
                EventResult::Handled
            }
            UiEvent::KeyPress { key: Key::F(11), .. } => {
                (self.on_action)(DebugAction::StepInto);
                EventResult::Handled
            }
            _ => EventResult::Ignored,
        }
    }
}
