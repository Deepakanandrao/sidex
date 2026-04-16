//! UI framework and widget library for `SideX`.
//!
//! This crate provides:
//!
//! - [`layout`] — A flexbox-style layout engine for computing widget positions.
//! - [`widget`] — The core [`Widget`](widget::Widget) trait and event types.
//! - [`widgets`] — Built-in widget implementations (buttons, lists, trees,
//!   tabs, menus, etc.).
//! - [`workbench`] — VS Code workbench layout components (title bar, activity
//!   bar, sidebar, editor area, panel, status bar).
//! - [`panels`] — Workbench panel implementations (file explorer, search,
//!   source control, debug, problems, output, terminal, extensions, settings,
//!   welcome).

pub mod layout;
pub mod panels;
pub mod widget;
pub mod widgets;
pub mod workbench;

// ── Convenience re-exports ───────────────────────────────────────────────────

pub use layout::{compute_layout, Direction, Edges, LayoutNode, Rect, Size};
pub use widget::{EventResult, Key, Modifiers, MouseButton, UiEvent, Widget};

pub use widgets::breadcrumbs::{BreadcrumbSegment, Breadcrumbs};
pub use widgets::button::{Button, ButtonStyle};
pub use widgets::context_menu::{ContextMenu, MenuItem};
pub use widgets::label::Label;
pub use widgets::list::{List, ListRow, SelectionMode};
pub use widgets::notification::{NotificationAction, NotificationToast, Severity};
pub use widgets::quick_pick::{QuickPick, QuickPickItem};
pub use widgets::scrollbar::{Orientation, Scrollbar};
pub use widgets::split_pane::SplitPane;
pub use widgets::tabs::{Tab, TabBar};
pub use widgets::text_input::TextInput;
pub use widgets::tooltip::{Tooltip, TooltipPosition};
pub use widgets::tree::{Tree, TreeNode, TreeRow};

pub use workbench::activity_bar::{ActivityBar, ActivityBarItem};
pub use workbench::editor_area::{DropZone, EditorArea, EditorGroup};
pub use workbench::panel::{Panel, PanelTab};
pub use workbench::sidebar::{Sidebar, SidebarSection};
pub use workbench::status_bar::{StatusBar, StatusBarAlignment, StatusBarItem};
pub use workbench::title_bar::{MenuBarItem, Platform, TitleBar};
pub use workbench::workbench::{PanelPosition, SidebarPosition, Workbench, WorkbenchLayout};

// ── Panel re-exports ─────────────────────────────────────────────────────────

pub use panels::debug_panel::{
    Breakpoint as DebugBreakpoint, ConsoleEntry, DebugAction, DebugPanel, DebugSections,
    DebugSessionState, DebugThread, OutputCategory, StackFrame, Variable, WatchEvent,
    WatchExpression,
};
pub use panels::extensions_panel::{
    ExtensionAction, ExtensionInfo, ExtensionState, ExtensionView, ExtensionsPanel,
};
pub use panels::file_explorer::{
    DragState, ExplorerAction, ExplorerFilter, FileEntry, FileExplorer, FileIcon, OpenEditor,
};
pub use panels::output_panel::{OutputChannel, OutputLevel, OutputLine, OutputPanel};
pub use panels::problems_panel::{
    Diagnostic, DiagnosticSeverity, FileDiagnostics, ProblemsFilter, ProblemsPanel,
};
pub use panels::scm_panel::{
    ChangeGroup, ChangeStatus, FileChange, ScmAction, ScmPanel,
};
pub use panels::search_panel::{
    FileSearchResult, ReplaceScope, SearchField, SearchGlobs, SearchMatch, SearchOptions,
    SearchPanel,
};
pub use panels::settings_panel::{
    SettingControl, SettingEntry, SettingScope, SettingsCategory, SettingsPanel, SettingsViewMode,
};
pub use panels::terminal_panel::{
    ShellType, TerminalAction, TerminalInstance, TerminalPanel, TerminalSplit,
};
pub use panels::welcome_panel::{
    RecentItem, ShortcutEntry, Walkthrough, WalkthroughStep, WelcomeAction, WelcomePanel,
};
