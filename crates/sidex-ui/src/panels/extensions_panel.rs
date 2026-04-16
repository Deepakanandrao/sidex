//! Extensions panel — browse, install, manage, and update extensions.

use sidex_gpu::color::Color;
use sidex_gpu::GpuRenderer;

use crate::layout::{LayoutNode, Rect, Size};
use crate::widget::{EventResult, Key, MouseButton, UiEvent, Widget};

// ── Extension info ───────────────────────────────────────────────────────────

/// Metadata for an extension card.
#[derive(Clone, Debug)]
pub struct ExtensionInfo {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub author: String,
    pub description: String,
    pub version: String,
    pub install_count: u64,
    pub rating: f32,
    pub rating_count: u32,
    pub icon_url: Option<String>,
    pub state: ExtensionState,
    pub categories: Vec<String>,
}

impl ExtensionInfo {
    pub fn install_count_label(&self) -> String {
        if self.install_count >= 1_000_000 {
            format!("{:.1}M", self.install_count as f64 / 1_000_000.0)
        } else if self.install_count >= 1_000 {
            format!("{:.1}K", self.install_count as f64 / 1_000.0)
        } else {
            self.install_count.to_string()
        }
    }

    pub fn rating_stars(&self) -> u8 {
        self.rating.round() as u8
    }
}

/// Current state of an extension.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtensionState {
    NotInstalled,
    Installed,
    Disabled,
    UpdateAvailable,
    Installing,
    Uninstalling,
}

// ── Extension actions ────────────────────────────────────────────────────────

/// Actions for extension management.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExtensionAction {
    Install(String),
    Uninstall(String),
    Update(String),
    Enable(String),
    Disable(String),
    ShowDetails(String),
    Search(String),
}

// ── View mode ────────────────────────────────────────────────────────────────

/// Which list of extensions is shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExtensionView {
    #[default]
    Installed,
    Recommended,
    Search,
    Popular,
}

// ── Extensions panel ─────────────────────────────────────────────────────────

/// The Extensions sidebar panel.
///
/// Displays installed, recommended, and marketplace search results.
/// Each extension is rendered as a card with icon, name, author, install count,
/// rating, and description. Provides install/uninstall/update actions and
/// extension detail view.
#[allow(dead_code)]
pub struct ExtensionsPanel<OnAction>
where
    OnAction: FnMut(ExtensionAction),
{
    pub installed: Vec<ExtensionInfo>,
    pub recommended: Vec<ExtensionInfo>,
    pub search_results: Vec<ExtensionInfo>,
    pub search_query: String,
    pub view: ExtensionView,
    pub on_action: OnAction,

    selected_index: Option<usize>,
    detail_extension: Option<String>,
    scroll_offset: f32,
    focused: bool,
    search_focused: bool,

    card_height: f32,
    search_bar_height: f32,
    section_header_height: f32,

    background: Color,
    search_bg: Color,
    search_border: Color,
    search_border_focused: Color,
    card_bg: Color,
    card_hover_bg: Color,
    card_selected_bg: Color,
    install_button_bg: Color,
    uninstall_button_bg: Color,
    update_button_bg: Color,
    rating_star: Color,
    rating_star_empty: Color,
    separator_color: Color,
    foreground: Color,
    secondary_fg: Color,
    disabled_fg: Color,
}

impl<OnAction> ExtensionsPanel<OnAction>
where
    OnAction: FnMut(ExtensionAction),
{
    pub fn new(on_action: OnAction) -> Self {
        Self {
            installed: Vec::new(),
            recommended: Vec::new(),
            search_results: Vec::new(),
            search_query: String::new(),
            view: ExtensionView::Installed,
            on_action,

            selected_index: None,
            detail_extension: None,
            scroll_offset: 0.0,
            focused: false,
            search_focused: false,

            card_height: 56.0,
            search_bar_height: 32.0,
            section_header_height: 26.0,

            background: Color::from_hex("#252526").unwrap_or(Color::BLACK),
            search_bg: Color::from_hex("#3c3c3c").unwrap_or(Color::BLACK),
            search_border: Color::from_hex("#3c3c3c").unwrap_or(Color::BLACK),
            search_border_focused: Color::from_hex("#007fd4").unwrap_or(Color::WHITE),
            card_bg: Color::from_hex("#252526").unwrap_or(Color::BLACK),
            card_hover_bg: Color::from_hex("#2a2d2e").unwrap_or(Color::BLACK),
            card_selected_bg: Color::from_hex("#04395e").unwrap_or(Color::BLACK),
            install_button_bg: Color::from_hex("#0e639c").unwrap_or(Color::BLACK),
            uninstall_button_bg: Color::from_hex("#6c2020").unwrap_or(Color::BLACK),
            update_button_bg: Color::from_hex("#0e639c").unwrap_or(Color::BLACK),
            rating_star: Color::from_rgb(255, 190, 0),
            rating_star_empty: Color::from_hex("#555555").unwrap_or(Color::BLACK),
            separator_color: Color::from_hex("#2b2b2b").unwrap_or(Color::BLACK),
            foreground: Color::from_hex("#cccccc").unwrap_or(Color::WHITE),
            secondary_fg: Color::from_hex("#969696").unwrap_or(Color::WHITE),
            disabled_fg: Color::from_hex("#6b6b6b").unwrap_or(Color::WHITE),
        }
    }

    pub fn search(&mut self, query: impl Into<String>) {
        self.search_query = query.into();
        if self.search_query.is_empty() {
            self.view = ExtensionView::Installed;
        } else {
            self.view = ExtensionView::Search;
            let q = self.search_query.clone();
            (self.on_action)(ExtensionAction::Search(q));
        }
    }

    pub fn show_installed(&mut self) {
        self.view = ExtensionView::Installed;
        self.scroll_offset = 0.0;
    }

    pub fn show_recommended(&mut self) {
        self.view = ExtensionView::Recommended;
        self.scroll_offset = 0.0;
    }

    pub fn install(&mut self, id: &str) {
        (self.on_action)(ExtensionAction::Install(id.to_string()));
    }

    pub fn uninstall(&mut self, id: &str) {
        (self.on_action)(ExtensionAction::Uninstall(id.to_string()));
    }

    pub fn show_detail(&mut self, id: &str) {
        self.detail_extension = Some(id.to_string());
        (self.on_action)(ExtensionAction::ShowDetails(id.to_string()));
    }

    fn active_list(&self) -> &[ExtensionInfo] {
        match self.view {
            ExtensionView::Installed => &self.installed,
            ExtensionView::Recommended | ExtensionView::Popular => &self.recommended,
            ExtensionView::Search => &self.search_results,
        }
    }

    fn action_button_for(state: ExtensionState) -> Option<(&'static str, Color)> {
        match state {
            ExtensionState::NotInstalled => Some((
                "Install",
                Color::from_hex("#0e639c").unwrap_or(Color::BLACK),
            )),
            ExtensionState::UpdateAvailable => Some((
                "Update",
                Color::from_hex("#0e639c").unwrap_or(Color::BLACK),
            )),
            ExtensionState::Installed => Some((
                "Uninstall",
                Color::from_hex("#6c2020").unwrap_or(Color::BLACK),
            )),
            _ => None,
        }
    }
}

impl<OnAction> Widget for ExtensionsPanel<OnAction>
where
    OnAction: FnMut(ExtensionAction),
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
        let pad = 8.0;

        // Search bar
        let sb = if self.search_focused {
            self.search_border_focused
        } else {
            self.search_border
        };
        rr.draw_rect(
            rect.x + pad,
            y,
            rect.width - pad * 2.0,
            self.search_bar_height,
            self.search_bg,
            2.0,
        );
        rr.draw_border(
            rect.x + pad,
            y,
            rect.width - pad * 2.0,
            self.search_bar_height,
            sb,
            1.0,
        );
        y += self.search_bar_height + 4.0;

        // View tabs (Installed / Recommended)
        let tab_w = (rect.width - pad * 2.0) / 2.0;
        for (i, _label) in ["INSTALLED", "RECOMMENDED"].iter().enumerate() {
            let tab_x = rect.x + pad + i as f32 * tab_w;
            let is_active = (i == 0 && self.view == ExtensionView::Installed)
                || (i == 1 && self.view == ExtensionView::Recommended);
            if is_active {
                rr.draw_rect(
                    tab_x,
                    y + self.section_header_height - 2.0,
                    tab_w,
                    2.0,
                    Color::WHITE,
                    0.0,
                );
            }
        }
        y += self.section_header_height;
        rr.draw_rect(rect.x, y, rect.width, 1.0, self.separator_color, 0.0);
        y += 1.0;

        // Extension cards
        let list = self.active_list();
        for (i, ext) in list.iter().enumerate() {
            let cy = y + i as f32 * self.card_height - self.scroll_offset;
            if cy + self.card_height < rect.y || cy > rect.y + rect.height {
                continue;
            }

            let is_sel = self.selected_index == Some(i);
            let bg = if is_sel {
                self.card_selected_bg
            } else {
                self.card_bg
            };
            rr.draw_rect(rect.x, cy, rect.width, self.card_height, bg, 0.0);

            // Icon placeholder
            let icon_s = 36.0;
            rr.draw_rect(
                rect.x + pad,
                cy + (self.card_height - icon_s) / 2.0,
                icon_s,
                icon_s,
                Color::from_hex("#3c3c3c").unwrap_or(Color::BLACK),
                4.0,
            );

            // Rating stars
            let stars_y = cy + self.card_height - 14.0;
            for s in 0..5u8 {
                let star_color = if s < ext.rating_stars() {
                    self.rating_star
                } else {
                    self.rating_star_empty
                };
                rr.draw_rect(
                    rect.x + pad + icon_s + 8.0 + f32::from(s) * 12.0,
                    stars_y,
                    10.0,
                    10.0,
                    star_color,
                    5.0,
                );
            }

            // Action button
            if let Some((_label, btn_color)) = Self::action_button_for(ext.state) {
                let btn_w = 60.0;
                let btn_h = 22.0;
                rr.draw_rect(
                    rect.x + rect.width - btn_w - pad,
                    cy + (self.card_height - btn_h) / 2.0,
                    btn_w,
                    btn_h,
                    btn_color,
                    3.0,
                );
            }

            // Card separator
            rr.draw_rect(
                rect.x + pad,
                cy + self.card_height - 1.0,
                rect.width - pad * 2.0,
                1.0,
                self.separator_color,
                0.0,
            );
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
                self.search_focused = false;
                EventResult::Handled
            }
            UiEvent::MouseDown {
                x,
                y,
                button: MouseButton::Left,
            } if rect.contains(*x, *y) => {
                self.focused = true;
                let pad = 8.0;
                let search_bottom = rect.y + 8.0 + self.search_bar_height;

                if *y < search_bottom {
                    self.search_focused = true;
                    return EventResult::Handled;
                }
                self.search_focused = false;

                let tabs_bottom = search_bottom + 4.0 + self.section_header_height + 1.0;
                if *y < tabs_bottom && *y >= search_bottom + 4.0 {
                    let tab_w = (rect.width - pad * 2.0) / 2.0;
                    if *x < rect.x + pad + tab_w {
                        self.show_installed();
                    } else {
                        self.show_recommended();
                    }
                    return EventResult::Handled;
                }

                // Card clicks
                let list_top = tabs_bottom;
                if *y >= list_top {
                    let idx = ((*y - list_top + self.scroll_offset) / self.card_height) as usize;
                    let list_len = self.active_list().len();
                    if idx < list_len {
                        self.selected_index = Some(idx);
                        let btn_w = 60.0;
                        let btn_x = rect.x + rect.width - btn_w - pad;
                        if *x >= btn_x {
                            let ext_id = self.active_list()[idx].id.clone();
                            let ext_state = self.active_list()[idx].state;
                            match ext_state {
                                ExtensionState::NotInstalled => (self.on_action)(ExtensionAction::Install(ext_id)),
                                ExtensionState::Installed => (self.on_action)(ExtensionAction::Uninstall(ext_id)),
                                ExtensionState::UpdateAvailable => (self.on_action)(ExtensionAction::Update(ext_id)),
                                _ => {}
                            }
                        } else {
                            let id = self.active_list()[idx].id.clone();
                            self.show_detail(&id);
                        }
                    }
                }
                EventResult::Handled
            }
            UiEvent::MouseScroll { dy, .. } => {
                let list = self.active_list();
                let total = list.len() as f32 * self.card_height;
                let max = (total - rect.height).max(0.0);
                self.scroll_offset = (self.scroll_offset - dy * 40.0).clamp(0.0, max);
                EventResult::Handled
            }
            UiEvent::KeyPress { key: Key::Enter, .. } if self.search_focused => {
                let q = self.search_query.clone();
                self.search(q);
                EventResult::Handled
            }
            _ => EventResult::Ignored,
        }
    }
}
