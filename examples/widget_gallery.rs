// [WFGY] Zone: RISK | λ: 0.3 | Fallbacks: 0 | Action: Full interactive widget gallery demo with MenuBar, Dropdown, Toast, and Modal overlays in AORUI
use std::sync::Arc;

use ui_core::UiEvent;
use ui_gpu::GpuRenderer;
use ui_layout::{length, AlignItems, AvailableSpace, FlexDirection, Rect, Size, Style};
use ui_widgets::{ColorSpace, InteractionKey, InteractionState, Theme, ToastKind, WidgetId, WidgetTree};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{ResizeDirection, Window, WindowAttributes, WindowId};

/// Translates `ui_widgets::FontFamily` (decoupled from specific font engines)
/// into `glyphon::Family`.
fn font_family(family: &ui_widgets::FontFamily) -> glyphon::Family<'_> {
    match family {
        ui_widgets::FontFamily::SansSerif => glyphon::Family::SansSerif,
        ui_widgets::FontFamily::Serif => glyphon::Family::Serif,
        ui_widgets::FontFamily::Monospace => glyphon::Family::Monospace,
        ui_widgets::FontFamily::Named(name) => glyphon::Family::Name(name),
    }
}

fn leaf(w: f32, h: f32) -> Style {
    Style { size: Size { width: length(w), height: length(h) }, ..Default::default() }
}

fn row(gap: f32) -> Style {
    Style {
        flex_direction: FlexDirection::Row,
        align_items: Some(AlignItems::Center),
        gap: Size { width: length(gap), height: length(0.0) },
        ..Default::default()
    }
}

fn column(gap: f32) -> Style {
    Style { flex_direction: FlexDirection::Column, gap: Size { width: length(0.0), height: length(gap) }, ..Default::default() }
}

/// Root window content container with generous padding.
fn window_content(gap: f32) -> Style {
    Style {
        flex_direction: FlexDirection::Column,
        gap: Size { width: length(0.0), height: length(gap) },
        padding: Rect { left: length(24.0), right: length(24.0), top: length(48.0), bottom: length(20.0) },
        ..Default::default()
    }
}

fn palette_style(x: f32, y: f32, w: f32, h: f32) -> Style {
    Style {
        position: ui_layout::Position::Absolute,
        inset: Rect {
            top: length(y),
            left: length(x),
            right: ui_layout::auto(),
            bottom: ui_layout::auto(),
        },
        size: Size { width: length(w), height: length(h) },
        flex_direction: FlexDirection::Column,
        gap: Size { width: length(0.0), height: length(4.0) },
        padding: Rect { left: length(4.0), right: length(4.0), top: length(4.0), bottom: length(4.0) },
        ..Default::default()
    }
}

fn popover_style(x: f32, y: f32, w: f32, h: f32) -> Style {
    Style {
        position: ui_layout::Position::Absolute,
        inset: Rect {
            top: length(y),
            left: length(x),
            right: ui_layout::auto(),
            bottom: ui_layout::auto(),
        },
        size: Size { width: length(w), height: length(h) },
        flex_direction: FlexDirection::Column,
        gap: Size { width: length(0.0), height: length(2.0) },
        padding: Rect { left: length(4.0), right: length(4.0), top: length(4.0), bottom: length(4.0) },
        ..Default::default()
    }
}

fn toast_style(x: f32, y: f32, w: f32, h: f32) -> Style {
    Style {
        position: ui_layout::Position::Absolute,
        inset: Rect {
            top: length(y),
            left: length(x),
            right: ui_layout::auto(),
            bottom: ui_layout::auto(),
        },
        size: Size { width: length(w), height: length(h) },
        ..Default::default()
    }
}

/// Scroll list geometry constants.
const LIST_ITEM_HEIGHT: f32 = 32.0;
const LIST_ITEM_GAP: f32 = 8.0;
const LIST_ITEM_COUNT: usize = 6;
const LIST_VIEWPORT_HEIGHT: f32 = 120.0;
const LIST_CONTENT_HEIGHT: f32 = LIST_ITEM_COUNT as f32 * LIST_ITEM_HEIGHT + (LIST_ITEM_COUNT as f32 - 1.0) * LIST_ITEM_GAP;
const LIST_MAX_SCROLL_Y: f32 = LIST_CONTENT_HEIGHT - LIST_VIEWPORT_HEIGHT;
const LIST_SCROLL_ID: &str = "demo_list_scroll";

/// Window dimensions.
const WINDOW_WIDTH: f32 = 580.0;
const WINDOW_HEIGHT: f32 = 780.0;

/// Interactive text editor buffer supporting cursor positioning, selection, and multi-line navigation.
#[derive(Debug, Clone)]
struct TextEditorState {
    text: String,
    cursor: usize,
    selection: Option<(usize, usize)>,
}

impl TextEditorState {
    fn new(initial: impl Into<String>) -> Self {
        let text = initial.into();
        let cursor = text.len();
        Self { text, cursor, selection: None }
    }

    fn insert_char(&mut self, ch: char) {
        self.delete_selection();
        self.cursor = self.cursor.min(self.text.len());
        self.text.insert(self.cursor, ch);
        self.cursor += ch.len_utf8();
    }

    #[allow(dead_code)]
    fn insert_str(&mut self, s: &str) {
        self.delete_selection();
        self.cursor = self.cursor.min(self.text.len());
        self.text.insert_str(self.cursor, s);
        self.cursor += s.len();
    }

    fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor > 0 && !self.text.is_empty() {
            let prev_char_idx = self.text[..self.cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            self.text.remove(prev_char_idx);
            self.cursor = prev_char_idx;
        }
    }

    fn delete(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor < self.text.len() {
            self.text.remove(self.cursor);
        }
    }

    fn move_left(&mut self, word: bool, select: bool) {
        let anchor = if select {
            self.selection.map(|(a, _)| a).unwrap_or(self.cursor)
        } else {
            if let Some((s1, s2)) = self.selection.take() {
                if s1 != s2 {
                    self.cursor = s1.min(s2);
                    return;
                }
            }
            self.cursor
        };

        if self.cursor > 0 {
            if word {
                let slice = &self.text[..self.cursor];
                let mut found_non_space = false;
                let mut new_pos = 0;
                for (idx, ch) in slice.char_indices().rev() {
                    if !ch.is_whitespace() {
                        found_non_space = true;
                    } else if found_non_space {
                        new_pos = idx + ch.len_utf8();
                        break;
                    }
                }
                self.cursor = new_pos;
            } else {
                let prev_char_idx = self.text[..self.cursor]
                    .char_indices()
                    .next_back()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                self.cursor = prev_char_idx;
            }
        }

        if select {
            self.selection = if anchor != self.cursor {
                Some((anchor, self.cursor))
            } else {
                None
            };
        }
    }

    fn move_right(&mut self, word: bool, select: bool) {
        let anchor = if select {
            self.selection.map(|(a, _)| a).unwrap_or(self.cursor)
        } else {
            if let Some((s1, s2)) = self.selection.take() {
                if s1 != s2 {
                    self.cursor = s1.max(s2);
                    return;
                }
            }
            self.cursor
        };

        if self.cursor < self.text.len() {
            if word {
                let slice = &self.text[self.cursor..];
                let mut found_space = false;
                let mut new_pos = self.text.len();
                for (idx, ch) in slice.char_indices() {
                    if ch.is_whitespace() {
                        found_space = true;
                    } else if found_space {
                        new_pos = self.cursor + idx;
                        break;
                    }
                }
                self.cursor = new_pos;
            } else {
                let next_char_idx = self.text[self.cursor..]
                    .char_indices()
                    .nth(1)
                    .map(|(i, _)| self.cursor + i)
                    .unwrap_or(self.text.len());
                self.cursor = next_char_idx;
            }
        }

        if select {
            self.selection = if anchor != self.cursor {
                Some((anchor, self.cursor))
            } else {
                None
            };
        }
    }

    fn move_home(&mut self, select: bool) {
        let anchor = if select {
            self.selection.map(|(a, _)| a).unwrap_or(self.cursor)
        } else {
            self.selection = None;
            self.cursor
        };

        let prev_newline = self.text[..self.cursor].rfind('\n').map(|i| i + 1).unwrap_or(0);
        self.cursor = prev_newline;

        if select {
            self.selection = if anchor != self.cursor {
                Some((anchor, self.cursor))
            } else {
                None
            };
        }
    }

    fn move_end(&mut self, select: bool) {
        let anchor = if select {
            self.selection.map(|(a, _)| a).unwrap_or(self.cursor)
        } else {
            self.selection = None;
            self.cursor
        };

        let next_newline = self.text[self.cursor..].find('\n').map(|i| self.cursor + i).unwrap_or(self.text.len());
        self.cursor = next_newline;

        if select {
            self.selection = if anchor != self.cursor {
                Some((anchor, self.cursor))
            } else {
                None
            };
        }
    }

    fn move_up(&mut self, select: bool) {
        let anchor = if select {
            self.selection.map(|(a, _)| a).unwrap_or(self.cursor)
        } else {
            self.selection = None;
            self.cursor
        };

        let lines: Vec<&str> = self.text.split('\n').collect();
        let mut acc = 0;
        let mut cur_line = 0;
        let mut col = 0;
        for (i, l) in lines.iter().enumerate() {
            if acc + l.len() >= self.cursor || i == lines.len() - 1 {
                cur_line = i;
                col = self.cursor.saturating_sub(acc);
                break;
            }
            acc += l.len() + 1;
        }

        if cur_line > 0 {
            let prev_line = cur_line - 1;
            let mut prev_acc = 0;
            for i in 0..prev_line {
                prev_acc += lines[i].len() + 1;
            }
            let target_col = col.min(lines[prev_line].len());
            self.cursor = prev_acc + target_col;
        } else {
            self.cursor = 0;
        }

        if select {
            self.selection = if anchor != self.cursor {
                Some((anchor, self.cursor))
            } else {
                None
            };
        }
    }

    fn move_down(&mut self, select: bool) {
        let anchor = if select {
            self.selection.map(|(a, _)| a).unwrap_or(self.cursor)
        } else {
            self.selection = None;
            self.cursor
        };

        let lines: Vec<&str> = self.text.split('\n').collect();
        let mut acc = 0;
        let mut cur_line = 0;
        let mut col = 0;
        for (i, l) in lines.iter().enumerate() {
            if acc + l.len() >= self.cursor || i == lines.len() - 1 {
                cur_line = i;
                col = self.cursor.saturating_sub(acc);
                break;
            }
            acc += l.len() + 1;
        }

        if cur_line + 1 < lines.len() {
            let next_line = cur_line + 1;
            let mut next_acc = 0;
            for i in 0..next_line {
                next_acc += lines[i].len() + 1;
            }
            let target_col = col.min(lines[next_line].len());
            self.cursor = next_acc + target_col;
        } else {
            self.cursor = self.text.len();
        }

        if select {
            self.selection = if anchor != self.cursor {
                Some((anchor, self.cursor))
            } else {
                None
            };
        }
    }

    fn select_all(&mut self) {
        self.selection = Some((0, self.text.len()));
        self.cursor = self.text.len();
    }

    fn clear_selection(&mut self) {
        self.selection = None;
    }

    fn delete_selection(&mut self) -> bool {
        if let Some((s1, s2)) = self.selection {
            if s1 != s2 {
                let start = s1.min(s2).min(self.text.len());
                let end = s1.max(s2).min(self.text.len());
                self.text.drain(start..end);
                self.cursor = start;
                self.selection = None;
                return true;
            }
        }
        self.selection = None;
        false
    }
}

/// Application state owned by the demo app.
struct DemoState {
    accept_checked: bool,
    turbo_toggle: bool,
    firewall_toggle: bool,
    slider_value: f32,
    active_tab: usize,
    selected_item: Option<usize>,
    click_count: u32,
    list_scroll: [f32; 2],
    close_requested: bool,
    search_editor: TextEditorState,
    password_editor: TextEditorState,
    password_revealed: bool,
    studio_editor: TextEditorState,
    concurrency_spin: f64,
    port_spin: f64,
    focused_input: Option<String>,
    selected_segment: usize,
    security_policy: String,
    show_modal: bool,
    modal_offset: (f32, f32),
    active_menu: Option<usize>,
    dropdown_open: bool,
    selected_env: String,
    active_toast: Option<(String, String, ToastKind)>,
    split_ratio: f32,
    expanded_nodes: std::collections::HashSet<String>,
    selected_tree_node: Option<String>,
    context_menu: Option<(f32, f32)>,
    accordion_open: bool,
    table_selected_row: Option<usize>,
    table_sort_col: usize,
    table_sort_asc: bool,
    current_page: usize,
    active_crumb: String,
    selected_color: [f32; 4],
    color_space: ColorSpace,
    show_tools_palette: bool,
    tools_palette_pos: (f32, f32),
    tools_palette_size: (f32, f32),
    tools_palette_folded: bool,
    active_tool: String,
    show_inspector_palette: bool,
    inspector_palette_pos: (f32, f32),
    inspector_palette_size: (f32, f32),
    inspector_palette_folded: bool,
}

impl DemoState {
    fn apply(&mut self, event: UiEvent) {
        match event {
            UiEvent::CheckboxToggled { checked, .. } => self.accept_checked = checked,
            UiEvent::ToggleSwitched { widget_id, active } => {
                if widget_id == "turbo_toggle" {
                    self.turbo_toggle = active;
                } else if widget_id == "firewall_toggle" {
                    self.firewall_toggle = active;
                }
            }
            UiEvent::SliderChanged { value, .. } => self.slider_value = value,
            UiEvent::NumberChanged { widget_id, value } => {
                if widget_id == "concurrency_spin" {
                    self.concurrency_spin = value;
                } else if widget_id == "port_spin" {
                    self.port_spin = value;
                }
            }
            UiEvent::PasswordRevealed { revealed, .. } => {
                self.password_revealed = revealed;
            }
            UiEvent::ContextMenuRequested { x, y, .. } => {
                self.context_menu = Some((x, y));
            }
            UiEvent::TabSelected { tab_index, .. } => {
                self.active_tab = tab_index;
                self.active_menu = None;
                self.dropdown_open = false;
                self.context_menu = None;
            }
            UiEvent::ListItemSelected { item_index, .. } => self.selected_item = Some(item_index),
            UiEvent::SegmentSelected { selected_index, .. } => self.selected_segment = selected_index,
            UiEvent::RadioSelected { selected_id, .. } => self.security_policy = selected_id,
            UiEvent::FocusChanged { widget_id } => self.focused_input = widget_id,
            UiEvent::TextCursorMoved { widget_id, cursor } => {
                self.focused_input = Some(widget_id.clone());
                match widget_id.as_str() {
                    "global_search" => {
                        self.search_editor.cursor = cursor;
                        self.search_editor.clear_selection();
                    }
                    "master_token_pwd" => {
                        self.password_editor.cursor = cursor;
                        self.password_editor.clear_selection();
                    }
                    "studio_editor" => {
                        self.studio_editor.cursor = cursor;
                        self.studio_editor.clear_selection();
                    }
                    _ => {}
                }
            }
            UiEvent::ModalDismissed { .. } => self.show_modal = false,
            UiEvent::PaletteFoldToggled { palette_id, folded } => {
                if palette_id == "tools_palette" {
                    self.tools_palette_folded = folded;
                } else if palette_id == "inspector_palette" {
                    self.inspector_palette_folded = folded;
                }
            }
            UiEvent::PaletteClosed { palette_id } => {
                if palette_id == "tools_palette" {
                    self.show_tools_palette = false;
                } else if palette_id == "inspector_palette" {
                    self.show_inspector_palette = false;
                }
                self.active_toast = Some((
                    "Palette Closed".to_string(),
                    format!("Closed {palette_id}. Re-open via View Menu."),
                    ToastKind::Info,
                ));
            }
            UiEvent::PaletteMoved { palette_id, x, y } => {
                if palette_id == "tools_palette" {
                    self.tools_palette_pos = (x, y);
                } else if palette_id == "inspector_palette" {
                    self.inspector_palette_pos = (x, y);
                }
            }
            UiEvent::PaletteResized { palette_id, width, height } => {
                if palette_id == "tools_palette" {
                    self.tools_palette_size = (width.max(120.0), height.max(120.0));
                } else if palette_id == "inspector_palette" {
                    self.inspector_palette_size = (width.max(140.0), height.max(120.0));
                }
            }
            UiEvent::TreeNodeToggled { node_id, expanded, .. } => {
                if expanded {
                    self.expanded_nodes.insert(node_id.clone());
                } else {
                    self.expanded_nodes.remove(&node_id);
                }
                self.selected_tree_node = Some(node_id);
            }
            UiEvent::TreeNodeSelected { node_id, .. } => {
                self.selected_tree_node = Some(node_id);
            }
            UiEvent::SplitRatioChanged { widget_id, ratio } => {
                if widget_id == "studio_split" {
                    self.split_ratio = ratio.clamp(0.20, 0.80);
                }
            }
            UiEvent::TableHeaderClicked { column_index, .. } => {
                if self.table_sort_col == column_index {
                    self.table_sort_asc = !self.table_sort_asc;
                } else {
                    self.table_sort_col = column_index;
                    self.table_sort_asc = true;
                }
                self.active_toast = Some((
                    "Table Sorted".to_string(),
                    format!("Column #{} ordered {}", column_index + 1, if self.table_sort_asc { "Ascending ▲" } else { "Descending ▼" }),
                    ToastKind::Info,
                ));
            }
            UiEvent::TableRowSelected { row_index, .. } => {
                self.table_selected_row = Some(row_index);
            }
            UiEvent::AccordionToggled { expanded, .. } => {
                self.accordion_open = expanded;
            }
            UiEvent::BreadcrumbClicked { item_id, .. } => {
                self.active_crumb = item_id.clone();
                match item_id.as_str() {
                    "nav_general" => self.active_tab = 0,
                    "nav_sec" => self.active_tab = 1,
                    "nav_net" => self.active_tab = 2,
                    "nav_studio" => self.active_tab = 3,
                    _ => {}
                }
            }
            UiEvent::PageSelected { page, .. } => {
                self.current_page = page;
                self.active_toast = Some((
                    "Data Page Changed".to_string(),
                    format!("Viewing cluster page {} of 4", page + 1),
                    ToastKind::Info,
                ));
            }
            UiEvent::ColorSelected { color, .. } => {
                self.selected_color = color;
                self.active_toast = Some((
                    "Color Accent Selected".to_string(),
                    format!("Picked RGBA({:.2}, {:.2}, {:.2}, {:.2})", color[0], color[1], color[2], color[3]),
                    ToastKind::Success,
                ));
            }
            UiEvent::ColorChanged { color, .. } => {
                self.selected_color = color;
            }
            UiEvent::ButtonClicked { widget_id } => {
                if let Some(tool_name) = widget_id.strip_prefix("tool_") {
                    self.active_tool = tool_name.to_string();
                    self.active_toast = Some((
                        "Creative Tool Active".to_string(),
                        format!("Selected: {}", tool_name.to_uppercase()),
                        ToastKind::Success,
                    ));
                } else if widget_id == "open_modal_btn" {
                    self.show_modal = true;
                } else if widget_id == "toast_btn" {
                    self.active_toast = Some((
                        "System Alert".to_string(),
                        "Network latency spike detected on Edge Gateway.".to_string(),
                        ToastKind::Warning,
                    ));
                } else if widget_id == "modal_confirm_btn" || widget_id == "modal_cancel_btn" {
                    self.show_modal = false;
                    println!("[widget_gallery] modal action confirmed: {widget_id}");
                } else if widget_id == "lock_btn" {
                    self.active_toast = Some(("Security Lock".to_string(), "All inbound endpoints locked.".to_string(), ToastKind::Warning));
                } else if widget_id == "refresh_btn" {
                    self.active_toast = Some(("Subsystem Sync".to_string(), "Cluster telemetry synchronized.".to_string(), ToastKind::Info));
                } else if widget_id == "shield_btn" {
                    self.active_toast = Some(("Firewall Shield".to_string(), "Full shield mesh online.".to_string(), ToastKind::Success));
                } else {
                    self.click_count += 1;
                    println!("[widget_gallery] button '{widget_id}' clicked ({} times)", self.click_count);
                }
            }
            UiEvent::MenuToggled { menu_id, open } => {
                if let Some(idx_str) = menu_id.strip_prefix("main_menubar:") {
                    if let Ok(idx) = idx_str.parse::<usize>() {
                        self.active_menu = if open { Some(idx) } else { None };
                        self.dropdown_open = false;
                        self.context_menu = None;
                    }
                } else if menu_id == "env_dropdown" {
                    self.dropdown_open = open;
                    if open {
                        self.active_menu = None;
                        self.context_menu = None;
                    }
                }
            }
            UiEvent::MenuItemClicked { item_id, .. } => {
                self.active_menu = None;
                self.dropdown_open = false;
                self.context_menu = None;
                match item_id.as_str() {
                    "toggle_tools_palette" => {
                        self.show_tools_palette = !self.show_tools_palette;
                    }
                    "toggle_insp_palette" => {
                        self.show_inspector_palette = !self.show_inspector_palette;
                    }
                    "menu_new" => {
                        self.active_toast = Some((
                            "New Workspace".to_string(),
                            "Project initialized successfully.".to_string(),
                            ToastKind::Info,
                        ));
                    }
                    "open_modal_btn" | "menu_modal" => self.show_modal = true,
                    "show_toast_btn" | "menu_toast" => {
                        self.active_toast = Some((
                            "Firewall Shield".to_string(),
                            "Security rule #402 applied successfully.".to_string(),
                            ToastKind::Success,
                        ));
                    }
                    "quit_btn" => self.close_requested = true,
                    "scan_sec" => {
                        self.active_toast = Some((
                            "Heuristic Scan".to_string(),
                            "Scanning 42 cluster nodes in background...".to_string(),
                            ToastKind::Info,
                        ));
                    }
                    "sec_policy_strict" => {
                        self.security_policy = "strict".to_string();
                        self.active_toast = Some((
                            "Security Policy".to_string(),
                            "Zero-Trust enforcement enabled on all routes.".to_string(),
                            ToastKind::Warning,
                        ));
                    }
                    "sec_policy_ia" => {
                        self.security_policy = "adaptive".to_string();
                    }
                    "tab_0" => self.active_tab = 0,
                    "tab_1" => self.active_tab = 1,
                    "tab_2" => self.active_tab = 2,
                    "tab_3" => self.active_tab = 3,
                    "toggle_turbo" => {
                        self.turbo_toggle = !self.turbo_toggle;
                    }
                    "help_doc" => {
                        self.active_toast = Some((
                            "Documentation".to_string(),
                            "AORUI (An Other Rust UI) v0.2.0 active.".to_string(),
                            ToastKind::Info,
                        ));
                    }
                    "env_prod" => self.selected_env = "Production (US-East 01)".to_string(),
                    "env_staging" => self.selected_env = "Staging (EU-West 02)".to_string(),
                    "env_dev" => self.selected_env = "Development (Local Node)".to_string(),
                    "ctx_inspect" => {
                        self.active_toast = Some((
                            "Inspector".to_string(),
                            "Active component tree verified nominal.".to_string(),
                            ToastKind::Info,
                        ));
                    }
                    "ctx_copy" => {
                        self.active_toast = Some((
                            "Clipboard".to_string(),
                            "Node identifier copied to memory buffer.".to_string(),
                            ToastKind::Success,
                        ));
                    }
                    "ctx_refresh" => {
                        self.active_toast = Some((
                            "Subsystem".to_string(),
                            "Layout pipeline flushed and redrawn.".to_string(),
                            ToastKind::Info,
                        ));
                    }
                    "ctx_toast" => {
                        self.active_toast = Some((
                            "Diagnostic".to_string(),
                            "WGPU pipeline buffer integrity 100%.".to_string(),
                            ToastKind::Success,
                        ));
                    }
                    _ => println!("[widget_gallery] selected menu item: {item_id}"),
                }
            }
            UiEvent::SelectChanged { widget_id, selected_id } => {
                if widget_id == "main_peeker" {
                    self.color_space = match selected_id.as_str() {
                        "rgb" => ColorSpace::Rgb,
                        "hex" => ColorSpace::Hex,
                        "lab" => ColorSpace::Lab,
                        "cmyk" => ColorSpace::Cmyk,
                        _ => self.color_space,
                    };
                    self.active_toast = Some((
                        "Color Mode Switched".to_string(),
                        format!("Peeker set to {}", selected_id.to_uppercase()),
                        ToastKind::Info,
                    ));
                } else {
                    self.dropdown_open = false;
                    self.selected_env = selected_id;
                }
            }
            UiEvent::ToastDismissed { .. } => {
                self.active_toast = None;
            }
            UiEvent::WindowCloseRequested { widget_id } => {
                println!("[widget_gallery] close requested for '{widget_id}'");
                self.close_requested = true;
            }
            _ => {}
        }
    }
}

fn build_base_ui(tree: &mut WidgetTree, state: &DemoState, width: f32, height: f32) -> ui_layout::NodeId {
    let title = tree.label("AORUI — An Other Rust UI Control Center", leaf(400.0, 20.0)).unwrap();

    // Primary horizontal menu bar
    let menu_items = ["File", "Security", "View", "Help"];
    let menubar = tree
        .menubar(
            WidgetId::new("main_menubar"),
            &menu_items,
            state.active_menu,
            leaf(76.0, 24.0),
            Style {
                size: Size { width: length(508.0), height: length(28.0) },
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                gap: Size { width: length(4.0), height: length(0.0) },
                padding: Rect { left: length(4.0), right: length(4.0), top: length(2.0), bottom: length(2.0) },
                ..Default::default()
            },
        )
        .unwrap();

    // Breadcrumb navigation path
    let tab_names = ["General", "Security", "Network", "Studio"];
    let current_tab_name = tab_names.get(state.active_tab).unwrap_or(&"General");
    let crumbs = [("nav_home", "Workspace"), ("nav_cluster", "US-East-01"), ("nav_active", *current_tab_name)];
    let breadcrumb_node = tree.breadcrumb("main_breadcrumb", &crumbs, leaf(80.0, 18.0), row(2.0)).unwrap();

    let search_focused = state.focused_input.as_deref() == Some("global_search");
    let search_input = tree
        .text_input_with_cursor(
            WidgetId::new("global_search"),
            &state.search_editor.text,
            "Search module, node, or metric... [Type to test]",
            search_focused,
            state.search_editor.cursor,
            state.search_editor.selection,
            leaf(508.0, 30.0),
        )
        .unwrap();

    let divider1 = tree.divider(false, leaf(508.0, 1.0)).unwrap();

    let tabs = ["General", "Security", "Network", "Studio"];
    let tabbar =
        tree.tabbar(WidgetId::new("settings_tabs"), &tabs, state.active_tab, leaf(120.0, 30.0), row(4.0)).unwrap();

    let tab_content = match state.active_tab {
        0 => {
            // --- Tab General: Controls, Dropdowns, Swatches & Modals ---
            let button = tree.button(WidgetId::new("hello_button"), "Quick Action", true, leaf(164.0, 30.0)).unwrap();
            let modal_btn = tree.button(WidgetId::new("open_modal_btn"), "Open Modal", true, leaf(164.0, 30.0)).unwrap();
            let toast_btn = tree.button(WidgetId::new("toast_btn"), "Trigger Toast", true, leaf(164.0, 30.0)).unwrap();
            let buttons_grid = tree.grid(3, 8.0, 0.0, &[button, modal_btn, toast_btn], leaf(508.0, 30.0)).unwrap();

            // Interactive Pro 2D Color Peeker (2D Saturation/Value Canvas, Hue Slider, HEX & 4-Space Matrix)
            let color_peeker = tree.color_picker(
                WidgetId::new("main_peeker"),
                state.selected_color,
                state.color_space,
                leaf(508.0, 208.0),
            ).unwrap();

            // Preset Swatches & Status Badge Row
            let color_label = tree.label_muted("Presets:", leaf(54.0, 24.0)).unwrap();
            let c1 = tree.color_swatch("cyan_swatch", [0.0, 0.85, 1.0, 1.0], Some("Cyan"), leaf(44.0, 32.0)).unwrap();
            let c2 = tree.color_swatch("purple_swatch", [0.55, 0.36, 0.96, 1.0], Some("Violet"), leaf(44.0, 32.0)).unwrap();
            let c3 = tree.color_swatch("green_swatch", [0.06, 0.72, 0.51, 1.0], Some("Emerald"), leaf(44.0, 32.0)).unwrap();
            let c4 = tree.color_swatch("amber_swatch", [0.96, 0.62, 0.04, 1.0], Some("Amber"), leaf(44.0, 32.0)).unwrap();
            let swatches = tree.container(&[c1, c2, c3, c4], row(6.0)).unwrap();
            let badge_status = tree.badge("ONLINE", ui_widgets::ListItemBadge::Success, leaf(72.0, 24.0)).unwrap();
            let palette_row = tree.container(&[color_label, swatches, badge_status], row(8.0)).unwrap();

            let env_label = tree.label_muted("Target Cluster:", leaf(120.0, 28.0)).unwrap();
            let env_dropdown = tree
                .dropdown(
                    WidgetId::new("env_dropdown"),
                    "Cluster",
                    &state.selected_env,
                    state.dropdown_open,
                    leaf(378.0, 28.0),
                )
                .unwrap();
            let env_row = tree.container(&[env_label, env_dropdown], row(10.0)).unwrap();

            // Password Input with eye reveal toggle
            let pwd_focused = state.focused_input.as_deref() == Some("master_token_pwd");
            let pwd_input = tree.password_input_with_cursor(
                WidgetId::new("master_token_pwd"),
                &state.password_editor.text,
                "Master Access Token [Click eye to reveal]...",
                pwd_focused,
                state.password_revealed,
                state.password_editor.cursor,
                leaf(378.0, 28.0),
            ).unwrap();
            let pwd_label = tree.label_muted("Auth Token:", leaf(120.0, 28.0)).unwrap();
            let pwd_row = tree.container(&[pwd_label, pwd_input], row(10.0)).unwrap();

            // Numeric Spinners: Workers limit and Proxy port
            let spin1_focused = state.focused_input.as_deref() == Some("concurrency_spin");
            let spin1_input = tree.number_input(WidgetId::new("concurrency_spin"), state.concurrency_spin, 1.0, 64.0, 1.0, 0, spin1_focused, leaf(138.0, 28.0)).unwrap();
            let spin1_label = tree.label_muted("Workers (1-64):", leaf(100.0, 28.0)).unwrap();
            let spin1_box = tree.container(&[spin1_label, spin1_input], row(6.0)).unwrap();

            let spin2_focused = state.focused_input.as_deref() == Some("port_spin");
            let spin2_input = tree.number_input(WidgetId::new("port_spin"), state.port_spin, 1024.0, 65535.0, 10.0, 0, spin2_focused, leaf(138.0, 28.0)).unwrap();
            let spin2_label = tree.label_muted("Port (1024+):", leaf(90.0, 28.0)).unwrap();
            let spin2_box = tree.container(&[spin2_label, spin2_input], row(6.0)).unwrap();

            let spinners_row = tree.grid(2, 12.0, 0.0, &[spin1_box, spin2_box], leaf(508.0, 28.0)).unwrap();

            let checkbox = tree.checkbox(WidgetId::new("accept_terms"), state.accept_checked, leaf(18.0, 18.0)).unwrap();
            let checkbox_label = tree.label("Enable continuous diagnostics telemetry", leaf(340.0, 18.0)).unwrap();
            let checkbox_row = tree.container(&[checkbox, checkbox_label], row(10.0)).unwrap();

            let toggle = tree.toggle(WidgetId::new("turbo_toggle"), state.turbo_toggle, leaf(38.0, 20.0)).unwrap();
            let toggle_label = tree.label("GPU Turbo Hardware Acceleration", leaf(280.0, 20.0)).unwrap();
            let toggle_row = tree.container(&[toggle, toggle_label], row(10.0)).unwrap();

            let slider_label = tree
                .label(format!("Network Intensity Level: {:.0}%", state.slider_value), leaf(340.0, 16.0))
                .unwrap();
            let slider = tree
                .slider(WidgetId::new("brightness_slider"), 0.0, 100.0, state.slider_value, leaf(508.0, 16.0))
                .unwrap();
            let progress = tree.progress_bar(state.slider_value / 100.0, leaf(508.0, 6.0)).unwrap();
            let slider_group = tree.container(&[slider_label, slider, progress], column(4.0)).unwrap();

            tree.container(&[buttons_grid, color_peeker, palette_row, env_row, pwd_row, spinners_row, checkbox_row, toggle_row, slider_group], column(4.0)).unwrap()
        }
        1 => {
            // --- Tab Security: Firewall, RadioGroup, Action Icons & Accordion ---
            let toggle = tree.toggle(WidgetId::new("firewall_toggle"), state.firewall_toggle, leaf(42.0, 22.0)).unwrap();
            let toggle_label = tree.label("Heuristic Shield & Core Firewall", leaf(280.0, 20.0)).unwrap();
            let toggle_row = tree.container(&[toggle, toggle_label], row(10.0)).unwrap();

            let radio_title = tree.label_muted("Cluster Security Enforcement Mode:", leaf(400.0, 16.0)).unwrap();
            let r1 = tree.radio(
                WidgetId::new("strict"),
                "sec_policy",
                "Strict (Zero-Trust)",
                state.security_policy == "strict",
                leaf(164.0, 22.0),
            ).unwrap();
            let r2 = tree.radio(
                WidgetId::new("adaptive"),
                "sec_policy",
                "Adaptive (AI)",
                state.security_policy == "adaptive",
                leaf(164.0, 22.0),
            ).unwrap();
            let r3 = tree.radio(
                WidgetId::new("audit"),
                "sec_policy",
                "Audit Only",
                state.security_policy == "audit",
                leaf(164.0, 22.0),
            ).unwrap();
            let radio_grid = tree.grid(3, 8.0, 0.0, &[r1, r2, r3], leaf(508.0, 22.0)).unwrap();
            let radio_group = tree.container(&[radio_title, radio_grid], column(4.0)).unwrap();

            let divider_sec = tree.divider(false, leaf(508.0, 1.0)).unwrap();

            // Accordion section with icon action buttons
            let lock_btn = tree.icon_button("lock_btn", ui_widgets::IconKind::Lock, true, leaf(30.0, 30.0)).unwrap();
            let refresh_btn = tree.icon_button("refresh_btn", ui_widgets::IconKind::Refresh, true, leaf(30.0, 30.0)).unwrap();
            let shield_btn = tree.icon_button("shield_btn", ui_widgets::IconKind::Shield, true, leaf(30.0, 30.0)).unwrap();
            let action_icons = tree.container(&[lock_btn, refresh_btn, shield_btn], row(8.0)).unwrap();

            let items = [
                ("TLS 1.3 Certificate valid", ui_widgets::ListItemBadge::Success),
                ("Unauthorized port scan blocked", ui_widgets::ListItemBadge::Warning),
                ("Memory analysis active", ui_widgets::ListItemBadge::Active("Active".to_string())),
                ("Firewall rule #104 allowed", ui_widgets::ListItemBadge::None),
            ];
            let list = tree
                .rich_list(WidgetId::new("sec_list"), &items, state.selected_item, leaf(488.0, 28.0), column(4.0))
                .unwrap();

            let acc_content = tree.container(&[action_icons, list], column(6.0)).unwrap();
            let accordion = tree.accordion("firewall_acc", "Heuristic Firewall Policies", Some("4 containment rules"), state.accordion_open, acc_content, leaf(508.0, if state.accordion_open { 190.0 } else { 46.0 })).unwrap();

            tree.container(&[toggle_row, radio_group, divider_sec, accordion], column(8.0)).unwrap()
        }
        2 => {
            // --- Tab Network: Metrics, SegmentedControl, Data Table & Pagination ---
            let seg_options = ["Real-Time", "24 Hours", "7 Days"];
            let segment_bar = tree
                .segmented_control(
                    WidgetId::new("net_timeframe"),
                    &seg_options,
                    state.selected_segment,
                    leaf(164.0, 26.0),
                    row(4.0),
                )
                .unwrap();

            let card1 = tree.metric_card("Inbound Bandwidth", "1.24 Gbps", Some(("+18%", true)), leaf(248.0, 56.0)).unwrap();
            let card2 = tree.metric_card("Average Latency", "3.8 ms", Some(("-12%", true)), leaf(248.0, 56.0)).unwrap();
            let metrics_grid = tree.grid(2, 12.0, 0.0, &[card1, card2], leaf(508.0, 56.0)).unwrap();

            // Multi-column sortable data TableView
            let cols = [
                ("Service / Node", 180.0, if state.table_sort_col == 0 { Some(state.table_sort_asc) } else { None }),
                ("Status", 100.0, if state.table_sort_col == 1 { Some(state.table_sort_asc) } else { None }),
                ("Latency", 96.0, if state.table_sort_col == 2 { Some(state.table_sort_asc) } else { None }),
                ("Traffic", 112.0, if state.table_sort_col == 3 { Some(state.table_sort_asc) } else { None }),
            ];
            let row1 = [
                ("Primary Core Node (Tokyo)", ui_widgets::ListItemBadge::None),
                ("Nominal", ui_widgets::ListItemBadge::Success),
                ("1.8 ms", ui_widgets::ListItemBadge::None),
                ("1.24 Gbps", ui_widgets::ListItemBadge::None),
            ];
            let row2 = [
                ("Edge Gateway (Paris)", ui_widgets::ListItemBadge::None),
                ("Nominal", ui_widgets::ListItemBadge::Success),
                ("4.2 ms", ui_widgets::ListItemBadge::None),
                ("840 Mbps", ui_widgets::ListItemBadge::None),
            ];
            let row3 = [
                ("CDN Relay #04 (Frankfurt)", ui_widgets::ListItemBadge::None),
                ("", ui_widgets::ListItemBadge::Active("Active".to_string())),
                ("6.1 ms", ui_widgets::ListItemBadge::None),
                ("2.10 Gbps", ui_widgets::ListItemBadge::None),
            ];
            let row4 = [
                ("Backup Node (Sydney)", ui_widgets::ListItemBadge::None),
                ("Degraded", ui_widgets::ListItemBadge::Warning),
                ("24.0 ms", ui_widgets::ListItemBadge::None),
                ("120 Mbps", ui_widgets::ListItemBadge::None),
            ];
            let rows = [row1, row2, row3, row4];
            let table_node = tree.table("net_table", &cols, &rows, state.table_selected_row, 26.0, leaf(508.0, 140.0)).unwrap();

            let pag_node = tree.pagination("net_pag", state.current_page, 4, leaf(32.0, 26.0), row(6.0)).unwrap();

            tree.container(&[segment_bar, metrics_grid, table_node, pag_node], column(10.0)).unwrap()
        }
        _ => {
            // --- Tab Studio: Resizable SplitView & TreeView ---
            let is_crates_open = state.expanded_nodes.contains("crates_dir");
            let is_widgets_open = state.expanded_nodes.contains("widgets_dir");
            let is_examples_open = state.expanded_nodes.contains("examples_dir");

            let mut tree_nodes: Vec<(&str, &str, usize, bool, bool, bool)> = Vec::new();
            tree_nodes.push(("crates_dir", "crates/", 0, true, is_crates_open, state.selected_tree_node.as_deref() == Some("crates_dir")));
            if is_crates_open {
                tree_nodes.push(("core_rs", "ui-core", 1, false, false, state.selected_tree_node.as_deref() == Some("core_rs")));
                tree_nodes.push(("layout_rs", "ui-layout", 1, false, false, state.selected_tree_node.as_deref() == Some("layout_rs")));
                tree_nodes.push(("widgets_dir", "ui-widgets/", 1, true, is_widgets_open, state.selected_tree_node.as_deref() == Some("widgets_dir")));
                if is_widgets_open {
                    tree_nodes.push(("tree_rs", "tree.rs", 2, false, false, state.selected_tree_node.as_deref() == Some("tree_rs")));
                    tree_nodes.push(("frame_rs", "frame.rs", 2, false, false, state.selected_tree_node.as_deref() == Some("frame_rs")));
                    tree_nodes.push(("kind_rs", "kind.rs", 2, false, false, state.selected_tree_node.as_deref() == Some("kind_rs")));
                }
                tree_nodes.push(("gpu_rs", "ui-gpu", 1, false, false, state.selected_tree_node.as_deref() == Some("gpu_rs")));
            }
            tree_nodes.push(("examples_dir", "examples/", 0, true, is_examples_open, state.selected_tree_node.as_deref() == Some("examples_dir")));
            if is_examples_open {
                tree_nodes.push(("gallery_rs", "widget_gallery.rs", 1, false, false, state.selected_tree_node.as_deref() == Some("gallery_rs")));
            }
            tree_nodes.push(("cargo_toml", "Cargo.toml", 0, false, false, state.selected_tree_node.as_deref() == Some("cargo_toml")));

            let left_w = (508.0 * state.split_ratio).clamp(120.0, 360.0);
            let right_w = (508.0 - left_w - 6.0).max(120.0);
            let inspector_w = (right_w - 20.0).max(100.0);

            let tree_list = tree.tree_view("studio_tree", &tree_nodes, leaf(left_w - 8.0, 22.0), column(2.0)).unwrap();

            // Right inspector panel
            let selected_name = state.selected_tree_node.as_deref().unwrap_or("tree_rs");
            let inspector_title = tree.label(format!("Node: {}", selected_name), leaf(inspector_w, 20.0)).unwrap();
            let inspector_desc = tree.label_muted("AORUI Scene Component Inspector", leaf(inspector_w, 16.0)).unwrap();
            let insp_card1 = tree.metric_card("Status", "Compiled & Linked", Some(("OK", true)), leaf(inspector_w, 50.0)).unwrap();
            let editor_focused = state.focused_input.as_deref() == Some("studio_editor");
            let text_editor = tree.text_area_with_cursor(
                WidgetId::new("studio_editor"),
                &state.studio_editor.text,
                "// Type script...",
                editor_focused,
                true,
                state.studio_editor.cursor,
                state.studio_editor.selection,
                leaf(inspector_w, 66.0),
            ).unwrap();
            let action_btn = tree.button(WidgetId::new("inspect_btn"), "Inspect Properties", true, leaf(inspector_w, 30.0)).unwrap();
            let inspector_content = tree.container(&[inspector_title, inspector_desc, insp_card1, text_editor, action_btn], column(6.0)).unwrap();

            let left_panel_style = Style {
                size: Size { width: length(left_w), height: length(220.0) },
                padding: Rect { left: length(2.0), right: length(6.0), top: length(0.0), bottom: length(0.0) },
                ..Default::default()
            };
            let left_panel = tree.container(&[tree_list], left_panel_style).unwrap();

            let right_panel_style = Style {
                size: Size { width: length(right_w), height: length(220.0) },
                padding: Rect { left: length(16.0), right: length(4.0), top: length(0.0), bottom: length(0.0) },
                ..Default::default()
            };
            let right_panel = tree.container(&[inspector_content], right_panel_style).unwrap();

            let split = tree.split_view("studio_split", ui_widgets::SplitOrientation::Horizontal, state.split_ratio, left_panel, right_panel, leaf(508.0, 220.0)).unwrap();
            let studio_hint = tree.label_muted("Drag splitter ↔ to resize | Click ▶/▼ to expand | Right-click for context", leaf(508.0, 16.0)).unwrap();

            tree.container(&[split, studio_hint], column(6.0)).unwrap()
        }
    };

    let main_content = tree.container(&[title, menubar, breadcrumb_node, search_input, divider1, tabbar, tab_content], window_content(8.0)).unwrap();
    tree.window(WidgetId::new("main_window"), "AORUI — An Other Rust UI", &[main_content], leaf(width, height)).unwrap()
}

fn build_modal_ui(tree: &mut WidgetTree, state: &DemoState, width: f32, height: f32) -> ui_layout::NodeId {
    let modal_title_div = tree.divider(false, leaf(390.0, 1.0)).unwrap();
    let modal_msg1 = tree.label("Do you want to reset cluster network parameters", leaf(390.0, 20.0)).unwrap();
    let modal_msg2 = tree.label_muted("and trigger a full heuristic cluster diagnostic?", leaf(390.0, 18.0)).unwrap();
    let cancel_btn = tree.button(WidgetId::new("modal_cancel_btn"), "Cancel", true, leaf(120.0, 36.0)).unwrap();
    let confirm_btn = tree.button(WidgetId::new("modal_confirm_btn"), "Confirm", true, leaf(130.0, 36.0)).unwrap();
    let modal_btns = tree.container(&[cancel_btn, confirm_btn], row(16.0)).unwrap();

    let modal_dialog_content = tree.container(&[modal_title_div, modal_msg1, modal_msg2, modal_btns], column(10.0)).unwrap();

    let dialog_w = 440.0;
    let dialog_h = 210.0;
    let base_x = (width - dialog_w) * 0.5;
    let base_y = (height - dialog_h) * 0.5;
    let dialog_x = (base_x + state.modal_offset.0).clamp(10.0, width - dialog_w - 10.0);
    let dialog_y = (base_y + state.modal_offset.1).clamp(10.0, height - dialog_h - 10.0);

    let dialog_style = Style {
        position: ui_layout::Position::Absolute,
        inset: Rect {
            top: length(dialog_y),
            left: length(dialog_x),
            right: ui_layout::auto(),
            bottom: ui_layout::auto(),
        },
        size: Size { width: length(dialog_w), height: length(dialog_h) },
        padding: Rect { left: length(24.0), right: length(24.0), top: length(44.0), bottom: length(20.0) },
        ..Default::default()
    };

    tree.modal_dialog(WidgetId::new("demo_modal"), "Confirmation Required", &[modal_dialog_content], dialog_style).unwrap()
}

fn build_popover_ui(tree: &mut WidgetTree, state: &DemoState, width: f32, height: f32) -> Option<ui_layout::NodeId> {
    let mut overlay_nodes = Vec::new();

    // 1. Floating Creative Tools Palette (Photoshop-like floating sub-window)
    if state.show_tools_palette {
        let content_node = if !state.tools_palette_folded {
            let t1 = tree.button(WidgetId::new("tool_select"), "↖ Select", state.active_tool == "select", leaf(60.0, 26.0)).unwrap();
            let t2 = tree.button(WidgetId::new("tool_brush"), "🖌 Brush", state.active_tool == "brush", leaf(60.0, 26.0)).unwrap();
            let t3 = tree.button(WidgetId::new("tool_eraser"), "⌫ Eraser", state.active_tool == "eraser", leaf(60.0, 26.0)).unwrap();
            let t4 = tree.button(WidgetId::new("tool_picker"), "⌖ Sample", state.active_tool == "picker", leaf(60.0, 26.0)).unwrap();
            let t5 = tree.button(WidgetId::new("tool_gradient"), "◩ Grad", state.active_tool == "gradient", leaf(60.0, 26.0)).unwrap();
            let t6 = tree.button(WidgetId::new("tool_shapes"), "⬡ Shape", state.active_tool == "shapes", leaf(60.0, 26.0)).unwrap();
            let t7 = tree.button(WidgetId::new("tool_text"), "T Type", state.active_tool == "text", leaf(60.0, 26.0)).unwrap();
            let t8 = tree.button(WidgetId::new("tool_bucket"), "🪣 Fill", state.active_tool == "bucket", leaf(60.0, 26.0)).unwrap();

            let grid_w = (state.tools_palette_size.0 - 12.0).max(120.0);
            let tools_grid = tree.grid(2, 4.0, 4.0, &[t1, t2, t3, t4, t5, t6, t7, t8], leaf(grid_w, 120.0)).unwrap();
            Some(tools_grid)
        } else {
            None
        };

        let pal_h = if state.tools_palette_folded { 28.0 } else { state.tools_palette_size.1 };
        let tools_palette_node = tree.palette(
            "tools_palette",
            "Tools",
            state.tools_palette_folded,
            content_node,
            palette_style(state.tools_palette_pos.0, state.tools_palette_pos.1, state.tools_palette_size.0, pal_h),
        ).unwrap();
        overlay_nodes.push(tools_palette_node);
    }

    // 2. Floating Inspector Palette (Blend mode, Opacity & Properties)
    if state.show_inspector_palette {
        let content_node = if !state.inspector_palette_folded {
            let active_tool_lbl = tree.label(format!("Tool: {}", state.active_tool.to_uppercase()), leaf(140.0, 18.0)).unwrap();
            let opacity_lbl = tree.label_muted(format!("Intensity: {:.0}%", state.slider_value), leaf(140.0, 16.0)).unwrap();
            let insp_slider = tree.slider(WidgetId::new("palette_intensity_slider"), 0.0, 100.0, state.slider_value, leaf(140.0, 16.0)).unwrap();
            let insp_btn = tree.button(WidgetId::new("palette_preset_btn"), "Apply Blend", true, leaf(140.0, 26.0)).unwrap();
            let insp_box = tree.container(&[active_tool_lbl, opacity_lbl, insp_slider, insp_btn], column(6.0)).unwrap();
            Some(insp_box)
        } else {
            None
        };

        let pal_h = if state.inspector_palette_folded { 28.0 } else { state.inspector_palette_size.1 };
        let insp_palette_node = tree.palette(
            "inspector_palette",
            "Inspector",
            state.inspector_palette_folded,
            content_node,
            palette_style(state.inspector_palette_pos.0, state.inspector_palette_pos.1, state.inspector_palette_size.0, pal_h),
        ).unwrap();
        overlay_nodes.push(insp_palette_node);
    }

    // 3. Menu popover if a menubar item is active
    if let Some(active_idx) = state.active_menu {
        let (x, items): (f32, Vec<(&str, &str, Option<&str>, bool)>) = match active_idx {
            0 => (
                28.0,
                vec![
                    ("menu_new", "New Workspace", Some("Ctrl+N"), true),
                    ("open_modal_btn", "Open Modal Dialog", Some("Ctrl+M"), true),
                    ("show_toast_btn", "Trigger Alert Toast", Some("Ctrl+T"), true),
                    ("quit_btn", "Exit Application", Some("Alt+F4"), true),
                ],
            ),
            1 => (
                108.0,
                vec![
                    ("scan_sec", "Quick Heuristic Scan", Some("F5"), true),
                    ("sec_policy_strict", "Force Zero-Trust Mode", None, true),
                    ("sec_policy_ia", "Enable AI Protection", None, true),
                ],
            ),
            2 => (
                188.0,
                vec![
                    ("toggle_tools_palette", "Toggle Tools Palette", Some("F2"), true),
                    ("toggle_insp_palette", "Toggle Inspector Palette", Some("F3"), true),
                    ("tab_0", "General Tab", Some("Ctrl+1"), true),
                    ("tab_1", "Security Tab", Some("Ctrl+2"), true),
                    ("tab_2", "Network Tab", Some("Ctrl+3"), true),
                    ("tab_3", "Studio Tab", Some("Ctrl+4"), true),
                    ("toggle_turbo", "Toggle Turbo Mode", None, true),
                ],
            ),
            _ => (
                268.0,
                vec![
                    ("help_doc", "AORUI Documentation", Some("F1"), true),
                    ("show_toast_btn", "System Health Check", None, true),
                ],
            ),
        };

        let item_h = 26.0;
        let popover_h = items.len() as f32 * (item_h + 2.0) + 12.0;
        let popover = tree
            .menu_popover(
                format!("main_menubar:{}", active_idx),
                &items,
                leaf(202.0, item_h),
                popover_style(x, 108.0, 210.0, popover_h),
            )
            .unwrap();
        overlay_nodes.push(popover);
    }

    // 4. Dropdown popover if dropdown is open
    if state.dropdown_open {
        let env_items: Vec<(&str, &str, Option<&str>, bool)> = vec![
            ("env_prod", "Production (US-East 01)", None, true),
            ("env_staging", "Staging (EU-West 02)", None, true),
            ("env_dev", "Development (Local Node)", None, true),
        ];
        let popover = tree
            .menu_popover(
                "env_dropdown",
                &env_items,
                leaf(370.0, 26.0),
                popover_style(154.0, 286.0, 378.0, 94.0),
            )
            .unwrap();
        overlay_nodes.push(popover);
    }

    // 5. Floating Toast if active
    if let Some((title, msg, kind)) = &state.active_toast {
        let toast_node = tree
            .toast(
                WidgetId::new("active_toast"),
                title,
                msg,
                *kind,
                toast_style(width - 340.0, height - 76.0, 320.0, 60.0),
            )
            .unwrap();
        overlay_nodes.push(toast_node);
    }

    // 6. Context Menu popover if open
    if let Some((cx, cy)) = state.context_menu {
        let ctx_items: Vec<(&str, &str, Option<&str>, bool)> = vec![
            ("ctx_inspect", "Inspect Component", Some("F12"), true),
            ("ctx_copy", "Copy Node ID", Some("Ctrl+C"), true),
            ("ctx_refresh", "Reload Subsystem", Some("F5"), true),
            ("ctx_toast", "Trigger Diagnostic", None, true),
        ];
        let item_h = 26.0;
        let popover_h = ctx_items.len() as f32 * (item_h + 2.0) + 12.0;
        let popover_x = cx.min(width - 210.0).max(10.0);
        let popover_y = cy.min(height - popover_h - 10.0).max(10.0);
        let popover = tree
            .context_menu(
                "context_menu",
                &ctx_items,
                leaf(192.0, item_h),
                popover_style(popover_x, popover_y, 200.0, popover_h),
            )
            .unwrap();
        overlay_nodes.push(popover);
    }

    if overlay_nodes.is_empty() {
        None
    } else {
        Some(tree.container(&overlay_nodes, leaf(width, height)).unwrap())
    }
}

struct App {
    window: Option<Arc<Window>>,
    renderer: Option<GpuRenderer>,
    state: DemoState,
    tree: WidgetTree,
    root: Option<ui_layout::NodeId>,
    overlay_tree: Option<WidgetTree>,
    overlay_root: Option<ui_layout::NodeId>,
    theme: Theme,
    cursor_pos: (f32, f32),
    pressed: Option<InteractionKey>,
    scrollbar_drag: Option<(f32, f32)>,
    slider_drag: Option<String>,
    splitter_drag: Option<String>,
    modal_drag: Option<(f32, f32)>,
    color_picker_drag: Option<String>,
    palette_drag: Option<(String, (f32, f32))>,
    palette_resize: Option<(String, (f32, f32), (f32, f32))>,
    text_drag: Option<(String, usize)>,
    shift_held: bool,
    ctrl_held: bool,
}

impl App {
    fn new() -> Self {
        let mut expanded_nodes = std::collections::HashSet::new();
        expanded_nodes.insert("crates_dir".to_string());
        expanded_nodes.insert("widgets_dir".to_string());

        Self {
            window: None,
            renderer: None,
            state: DemoState {
                accept_checked: false,
                turbo_toggle: true,
                firewall_toggle: true,
                slider_value: 72.0,
                active_tab: 0,
                selected_item: Some(2),
                click_count: 0,
                list_scroll: [0.0, 0.0],
                close_requested: false,
                search_editor: TextEditorState::new(""),
                password_editor: TextEditorState::new("cyber-key-7749"),
                password_revealed: false,
                studio_editor: TextEditorState::new("// AORUI Shader Node\nfn evaluate_node() -> bool {\n    let status = verify_mesh();\n    return status;\n}"),
                concurrency_spin: 8.0,
                port_spin: 8080.0,
                focused_input: Some("global_search".to_string()),
                selected_segment: 0,
                security_policy: "strict".to_string(),
                show_modal: false,
                modal_offset: (0.0, 0.0),
                active_menu: None,
                dropdown_open: false,
                selected_env: "Production (US-East 01)".to_string(),
                active_toast: None,
                split_ratio: 0.45,
                expanded_nodes,
                selected_tree_node: Some("tree_rs".to_string()),
                context_menu: None,
                accordion_open: true,
                table_selected_row: Some(0),
                table_sort_col: 0,
                table_sort_asc: true,
                current_page: 0,
                active_crumb: "nav_general".to_string(),
                selected_color: [0.0, 0.85, 1.0, 1.0],
                color_space: ColorSpace::Lab,
                show_tools_palette: true,
                tools_palette_pos: (24.0, 110.0),
                tools_palette_size: (150.0, 180.0),
                tools_palette_folded: false,
                active_tool: "brush".to_string(),
                show_inspector_palette: true,
                inspector_palette_pos: (390.0, 110.0),
                inspector_palette_size: (166.0, 180.0),
                inspector_palette_folded: false,
            },
            tree: WidgetTree::new(),
            root: None,
            overlay_tree: None,
            overlay_root: None,
            theme: Theme::cyber_glass(),
            cursor_pos: (0.0, 0.0),
            pressed: None,
            scrollbar_drag: None,
            slider_drag: None,
            splitter_drag: None,
            modal_drag: None,
            color_picker_drag: None,
            palette_drag: None,
            palette_resize: None,
            text_drag: None,
            shift_held: false,
            ctrl_held: false,
        }
    }

    fn redraw(&mut self) {
        let Some(renderer) = self.renderer.as_mut() else { return };

        let (width, height) = renderer.window_size();
        let width_f = width as f32;
        let height_f = height as f32;
        let available = Size {
            width: AvailableSpace::Definite(width_f),
            height: AvailableSpace::Definite(height_f),
        };

        // --- Layer 0: Base UI ---
        let mut base_tree = WidgetTree::new();
        let base_root = build_base_ui(&mut base_tree, &self.state, width_f, height_f);
        if base_tree.compute(base_root, available).is_err() {
            return;
        }

        let base_hovered = if !self.state.show_modal && self.state.active_menu.is_none() && !self.state.dropdown_open {
            base_tree.interaction_key_at(base_root, self.cursor_pos).unwrap_or(None)
        } else {
            None
        };
        let base_interaction = InteractionState {
            hovered: base_hovered.as_ref(),
            pressed: if !self.state.show_modal { self.pressed.as_ref() } else { None },
        };

        let base_frame = match base_tree.build_frame(base_root, &self.theme, base_interaction) {
            Ok(f) => f,
            Err(err) => {
                eprintln!("[widget_gallery] base build_frame failed: {err}");
                return;
            }
        };

        let family = font_family(&self.theme.typography.family);
        let base_text_runs: Vec<_> = base_frame
            .texts
            .iter()
            .map(|spec| {
                let align = match spec.align {
                    ui_widgets::TextAlign::Left => glyphon::cosmic_text::Align::Left,
                    ui_widgets::TextAlign::Center => glyphon::cosmic_text::Align::Center,
                    ui_widgets::TextAlign::Right => glyphon::cosmic_text::Align::Right,
                };
                let weight = match spec.weight {
                    ui_widgets::FontWeight::Normal => glyphon::Weight::NORMAL,
                    ui_widgets::FontWeight::Bold => glyphon::Weight::BOLD,
                };
                renderer.make_text_run(&spec.text, spec.bounds, spec.font_size, spec.color, align, family, weight, spec.clip)
            })
            .collect();

        let base_layer = ui_gpu::RenderLayer {
            instances: &base_frame.instances,
            texts: &base_text_runs,
        };

        let media: Vec<ui_gpu::MediaInstance> = base_frame
            .media
            .iter()
            .map(|spec| ui_gpu::MediaInstance { kind: spec.kind.0.to_string(), resource_id: spec.resource_id.clone(), bounds: spec.bounds })
            .collect();
        let resources = ui_gpu::ResourceTable::new();
        let background = wgpu::Color { r: 0.02, g: 0.03, b: 0.06, a: 1.0 };

        if self.state.show_modal {
            // --- Layer 1: Modal Overlay ---
            let mut modal_tree = WidgetTree::new();
            let modal_root = build_modal_ui(&mut modal_tree, &self.state, width_f, height_f);
            if modal_tree.compute(modal_root, available).is_err() {
                return;
            }

            let modal_hovered = modal_tree.interaction_key_at(modal_root, self.cursor_pos).unwrap_or(None);
            let modal_interaction = InteractionState {
                hovered: modal_hovered.as_ref(),
                pressed: self.pressed.as_ref(),
            };

            let modal_frame = match modal_tree.build_frame(modal_root, &self.theme, modal_interaction) {
                Ok(f) => f,
                Err(err) => {
                    eprintln!("[widget_gallery] modal build_frame failed: {err}");
                    return;
                }
            };

            let modal_text_runs: Vec<_> = modal_frame
                .texts
                .iter()
                .map(|spec| {
                    let align = match spec.align {
                        ui_widgets::TextAlign::Left => glyphon::cosmic_text::Align::Left,
                        ui_widgets::TextAlign::Center => glyphon::cosmic_text::Align::Center,
                        ui_widgets::TextAlign::Right => glyphon::cosmic_text::Align::Right,
                    };
                    let weight = match spec.weight {
                        ui_widgets::FontWeight::Normal => glyphon::Weight::NORMAL,
                        ui_widgets::FontWeight::Bold => glyphon::Weight::BOLD,
                    };
                    renderer.make_text_run(&spec.text, spec.bounds, spec.font_size, spec.color, align, family, weight, spec.clip)
                })
                .collect();

            let modal_layer = ui_gpu::RenderLayer {
                instances: &modal_frame.instances,
                texts: &modal_text_runs,
            };

            if let Err(err) = renderer.render_layers(background, &[base_layer, modal_layer], &media, &resources) {
                eprintln!("[widget_gallery] modal render failed: {err}");
            }

            self.overlay_tree = Some(modal_tree);
            self.overlay_root = Some(modal_root);
        } else {
            // --- Layer 1: Popover / Palette / Toast / Dropdown Overlays (if active) ---
            let mut popover_tree = WidgetTree::new();
            let popover_root = build_popover_ui(&mut popover_tree, &self.state, width_f, height_f);

            if let Some(pop_root) = popover_root {
                if popover_tree.compute(pop_root, available).is_ok() {
                    let pop_hovered = popover_tree.interaction_key_at(pop_root, self.cursor_pos).unwrap_or(None);
                    let pop_interaction = InteractionState {
                        hovered: pop_hovered.as_ref(),
                        pressed: self.pressed.as_ref(),
                    };

                    if let Ok(pop_frame) = popover_tree.build_frame(pop_root, &self.theme, pop_interaction) {
                        let pop_text_runs: Vec<_> = pop_frame
                            .texts
                            .iter()
                            .map(|spec| {
                                let align = match spec.align {
                                    ui_widgets::TextAlign::Left => glyphon::cosmic_text::Align::Left,
                                    ui_widgets::TextAlign::Center => glyphon::cosmic_text::Align::Center,
                                    ui_widgets::TextAlign::Right => glyphon::cosmic_text::Align::Right,
                                    };
                                let weight = match spec.weight {
                                    ui_widgets::FontWeight::Normal => glyphon::Weight::NORMAL,
                                    ui_widgets::FontWeight::Bold => glyphon::Weight::BOLD,
                                };
                                renderer.make_text_run(&spec.text, spec.bounds, spec.font_size, spec.color, align, family, weight, spec.clip)
                            })
                            .collect();

                        let pop_layer = ui_gpu::RenderLayer {
                            instances: &pop_frame.instances,
                            texts: &pop_text_runs,
                        };

                        if let Err(err) = renderer.render_layers(background, &[base_layer, pop_layer], &media, &resources) {
                            eprintln!("[widget_gallery] overlay render failed: {err}");
                        }
                    }
                }
                self.overlay_tree = Some(popover_tree);
                self.overlay_root = Some(pop_root);
            } else {
                if let Err(err) = renderer.render_layers(background, &[base_layer], &media, &resources) {
                    eprintln!("[widget_gallery] base render failed: {err}");
                }
                self.overlay_tree = None;
                self.overlay_root = None;
            }
        }

        self.tree = base_tree;
        self.root = Some(base_root);
    }

    fn handle_press(&mut self) {
        if self.state.show_modal {
            if let (Some(m_tree), Some(m_root)) = (&self.overlay_tree, self.overlay_root) {
                self.pressed = m_tree.interaction_key_at(m_root, self.cursor_pos).unwrap_or(None);
                if m_tree.is_modal_title_bar(m_root, self.cursor_pos).unwrap_or(false) {
                    self.modal_drag = Some((self.cursor_pos.0 - self.state.modal_offset.0, self.cursor_pos.1 - self.state.modal_offset.1));
                }
            }
            return;
        }

        // Test active overlays first (palettes, dropdowns, popovers, toasts)
        if let (Some(o_tree), Some(o_root)) = (&self.overlay_tree, self.overlay_root) {
            // Check palette drag header
            if let Ok(Some((id, (ax, ay)))) = o_tree.palette_drag_anchor_at(o_root, self.cursor_pos) {
                self.palette_drag = Some((id, (ax, ay)));
                return;
            }
            // Check palette resize grip
            if let Ok(Some((id, (cw, ch)))) = o_tree.palette_resize_at(o_root, self.cursor_pos) {
                self.palette_resize = Some((id, self.cursor_pos, (cw, ch)));
                return;
            }

            if let Ok(Some(key)) = o_tree.interaction_key_at(o_root, self.cursor_pos) {
                self.pressed = Some(key);
                return;
            }
        }

        let Some(root) = self.root else { return };
        self.pressed = self.tree.interaction_key_at(root, self.cursor_pos).unwrap_or(None);

        let scrollbar_key = ui_widgets::InteractionKey { widget_id: WidgetId::new(LIST_SCROLL_ID), index: None };
        if self.pressed.as_ref() == Some(&scrollbar_key) {
            self.scrollbar_drag = Some((self.cursor_pos.1, self.state.list_scroll[1]));
        }

        let splitter_key = ui_widgets::InteractionKey { widget_id: WidgetId::new("studio_split"), index: None };
        if self.pressed.as_ref() == Some(&splitter_key) {
            self.splitter_drag = Some("studio_split".to_string());
        }

        if let Ok(Some((id, value))) = self.tree.slider_value_at(root, self.cursor_pos) {
            self.state.slider_value = value;
            self.slider_drag = Some(id);
        }

        if let Ok(Some((id, color))) = self.tree.color_picker_hue_at(root, self.cursor_pos) {
            self.state.selected_color = color;
            self.color_picker_drag = Some(id);
        }

        if let Ok(Some((widget_id, cursor_idx))) = self.tree.text_cursor_at(root, self.cursor_pos, self.theme.typography.body_size) {
            self.state.focused_input = Some(widget_id.clone());
            self.text_drag = Some((widget_id.clone(), cursor_idx));
            match widget_id.as_str() {
                "global_search" => {
                    self.state.search_editor.cursor = cursor_idx;
                    self.state.search_editor.clear_selection();
                }
                "master_token_pwd" => {
                    self.state.password_editor.cursor = cursor_idx;
                    self.state.password_editor.clear_selection();
                }
                "studio_editor" => {
                    self.state.studio_editor.cursor = cursor_idx;
                    self.state.studio_editor.clear_selection();
                }
                _ => {}
            }
        }
    }

    fn try_start_window_resize(&self) -> bool {
        if self.state.show_modal {
            return false;
        }
        let Some(window) = &self.window else { return false };
        let Some(renderer) = &self.renderer else { return false };
        let (width, height) = renderer.window_size();
        let (x, y) = self.cursor_pos;
        let w = width as f32;
        let h = height as f32;
        let margin = 8.0;

        let direction = if x <= margin && y <= margin {
            Some(ResizeDirection::NorthWest)
        } else if x >= w - margin && y <= margin {
            Some(ResizeDirection::NorthEast)
        } else if x <= margin && y >= h - margin {
            Some(ResizeDirection::SouthWest)
        } else if x >= w - margin && y >= h - margin {
            Some(ResizeDirection::SouthEast)
        } else if x <= margin {
            Some(ResizeDirection::West)
        } else if x >= w - margin {
            Some(ResizeDirection::East)
        } else if y <= margin {
            Some(ResizeDirection::North)
        } else if y >= h - margin {
            Some(ResizeDirection::South)
        } else {
            None
        };

        if let Some(dir) = direction {
            if let Err(err) = window.drag_resize_window(dir) {
                eprintln!("[widget_gallery] drag_resize_window failed: {err:?}");
            }
            return true;
        }
        false
    }

    fn try_start_window_drag(&self) -> bool {
        if self.state.show_modal || self.state.active_menu.is_some() || self.state.dropdown_open {
            return false;
        }

        let Some(root) = self.root else { return false };
        if self.tree.is_window_title_bar(root, self.cursor_pos).unwrap_or(false) {
            if let Some(window) = &self.window {
                if let Err(err) = window.drag_window() {
                    eprintln!("[widget_gallery] drag_window failed: {err:?}");
                }
            }
            return true;
        }
        false
    }

    fn update_modal_drag(&mut self) {
        if !self.state.show_modal {
            return;
        }
        if let Some((anchor_x, anchor_y)) = self.modal_drag {
            self.state.modal_offset = (
                self.cursor_pos.0 - anchor_x,
                self.cursor_pos.1 - anchor_y,
            );
        }
    }

    fn update_palette_drag(&mut self) {
        if let Some((ref id, (anchor_x, anchor_y))) = self.palette_drag {
            let new_x = (self.cursor_pos.0 - anchor_x).max(0.0);
            let new_y = (self.cursor_pos.1 - anchor_y).max(0.0);
            self.state.apply(UiEvent::PaletteMoved { palette_id: id.clone(), x: new_x, y: new_y });
        }
    }

    fn update_palette_resize(&mut self) {
        if let Some((ref id, (start_cx, start_cy), (start_w, start_h))) = self.palette_resize {
            let delta_x = self.cursor_pos.0 - start_cx;
            let delta_y = self.cursor_pos.1 - start_cy;
            let new_w = (start_w + delta_x).max(120.0);
            let new_h = (start_h + delta_y).max(120.0);
            self.state.apply(UiEvent::PaletteResized { palette_id: id.clone(), width: new_w, height: new_h });
        }
    }

    fn update_scrollbar_drag(&mut self) {
        if self.state.show_modal {
            return;
        }
        let Some((start_cursor_y, start_scroll_y)) = self.scrollbar_drag else { return };
        let delta_cursor = self.cursor_pos.1 - start_cursor_y;
        let scale = LIST_CONTENT_HEIGHT / LIST_VIEWPORT_HEIGHT;
        self.state.list_scroll[1] = (start_scroll_y + delta_cursor * scale).clamp(0.0, LIST_MAX_SCROLL_Y.max(0.0));
    }

    fn update_slider_drag(&mut self) {
        if self.state.show_modal {
            return;
        }
        let Some(root) = self.root else { return };
        if self.slider_drag.is_some() {
            if let Ok(Some((_, value))) = self.tree.slider_value_at(root, self.cursor_pos) {
                self.state.slider_value = value;
            }
        }
    }

    fn update_color_picker_drag(&mut self) {
        if self.state.show_modal {
            return;
        }
        let Some(root) = self.root else { return };
        if self.color_picker_drag.is_some() {
            if let Ok(Some((_, color))) = self.tree.color_picker_hue_at(root, self.cursor_pos) {
                self.state.selected_color = color;
            }
        }
    }

    fn update_splitter_drag(&mut self) {
        if self.state.show_modal {
            return;
        }
        let Some(root) = self.root else { return };
        if let Some(ref owner) = self.splitter_drag {
            if let Ok(Some(ratio)) = self.tree.split_ratio_at(root, owner, self.cursor_pos) {
                self.state.split_ratio = ratio.clamp(0.20, 0.80);
            }
        }
    }

    fn update_text_drag(&mut self) {
        if self.state.show_modal {
            return;
        }
        let Some((ref drag_id, anchor_idx)) = self.text_drag else { return };
        let Some(root) = self.root else { return };
        if let Ok(Some((widget_id, cur_idx))) = self.tree.text_cursor_at(root, self.cursor_pos, self.theme.typography.body_size) {
            if &widget_id == drag_id {
                let sel = if anchor_idx != cur_idx {
                    Some((anchor_idx, cur_idx))
                } else {
                    None
                };
                match widget_id.as_str() {
                    "global_search" => {
                        self.state.search_editor.cursor = cur_idx;
                        self.state.search_editor.selection = sel;
                    }
                    "master_token_pwd" => {
                        self.state.password_editor.cursor = cur_idx;
                        self.state.password_editor.selection = sel;
                    }
                    "studio_editor" => {
                        self.state.studio_editor.cursor = cur_idx;
                        self.state.studio_editor.selection = sel;
                    }
                    _ => {}
                }
            }
        }
    }

    fn handle_release(&mut self) {
        self.scrollbar_drag = None;
        self.slider_drag = None;
        self.color_picker_drag = None;
        self.splitter_drag = None;
        self.modal_drag = None;
        self.palette_drag = None;
        self.palette_resize = None;
        self.text_drag = None;

        if self.state.show_modal {
            if let (Some(m_tree), Some(m_root)) = (&self.overlay_tree, self.overlay_root) {
                let released_on = m_tree.interaction_key_at(m_root, self.cursor_pos).unwrap_or(None);
                if self.pressed.is_some() && self.pressed == released_on {
                    match m_tree.dispatch_click(m_root, self.cursor_pos) {
                        Ok(Some(event)) => self.state.apply(event),
                        Ok(None) => {}
                        Err(err) => eprintln!("[widget_gallery] modal dispatch_click failed: {err}"),
                    }
                }
            }
            self.pressed = None;
            return;
        }

        // Test overlay first (palettes, menu, or toast)
        if let (Some(o_tree), Some(o_root)) = (&self.overlay_tree, self.overlay_root) {
            let released_on = o_tree.interaction_key_at(o_root, self.cursor_pos).unwrap_or(None);
            if self.pressed.is_some() && self.pressed == released_on && released_on.is_some() {
                match o_tree.dispatch_click(o_root, self.cursor_pos) {
                    Ok(Some(event)) => {
                        self.state.apply(event);
                        self.pressed = None;
                        return;
                    }
                    Ok(None) => {}
                    Err(err) => eprintln!("[widget_gallery] overlay dispatch_click failed: {err}"),
                }
            }
        }

        let Some(root) = self.root else {
            self.pressed = None;
            return;
        };
        let released_on = self.tree.interaction_key_at(root, self.cursor_pos).unwrap_or(None);
        if self.pressed.is_some() && self.pressed == released_on {
            match self.tree.dispatch_click(root, self.cursor_pos) {
                Ok(Some(event)) => self.state.apply(event),
                Ok(None) => {
                    // Clicking outside open menu/dropdown/context closes it
                    if self.state.active_menu.is_some() || self.state.dropdown_open || self.state.context_menu.is_some() {
                        self.state.active_menu = None;
                        self.state.dropdown_open = false;
                        self.state.context_menu = None;
                    }
                }
                Err(err) => eprintln!("[widget_gallery] dispatch_click failed: {err}"),
            }
        } else if self.state.active_menu.is_some() || self.state.dropdown_open || self.state.context_menu.is_some() {
            // Click outside active overlay
            self.state.active_menu = None;
            self.state.dropdown_open = false;
            self.state.context_menu = None;
        }
        self.pressed = None;
    }

    fn handle_scroll(&mut self, delta_y: f32) {
        let Some(root) = self.root else { return };
        match self.tree.dispatch_scroll(root, self.cursor_pos, [0.0, delta_y]) {
            Ok(Some(UiEvent::ScrollChanged { offset, .. })) => {
                self.state.list_scroll = [offset[0], offset[1].clamp(0.0, LIST_MAX_SCROLL_Y.max(0.0))];
            }
            Ok(_) => {}
            Err(err) => eprintln!("[widget_gallery] dispatch_scroll failed: {err}"),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = WindowAttributes::default()
            .with_title("AORUI — Widget Gallery")
            .with_decorations(false)
            .with_inner_size(winit::dpi::PhysicalSize::new(WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32));
        let window = Arc::new(event_loop.create_window(attrs).expect("failed to create OS window"));
        self.renderer = Some(GpuRenderer::new(window.clone()));
        self.window = Some(window);
        event_loop.set_control_flow(ControlFlow::Wait);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size);
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.shift_held = modifiers.state().shift_key();
                self.ctrl_held = modifiers.state().control_key();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = (position.x as f32, position.y as f32);
                self.update_modal_drag();
                self.update_palette_drag();
                self.update_palette_resize();
                self.update_scrollbar_drag();
                self.update_slider_drag();
                self.update_color_picker_drag();
                self.update_splitter_drag();
                self.update_text_drag();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                if !self.try_start_window_resize() && !self.try_start_window_drag() {
                    self.handle_press();
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Right, .. } => {
                if !self.state.show_modal {
                    self.state.context_menu = Some(self.cursor_pos);
                    self.state.active_menu = None;
                    self.state.dropdown_open = false;
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::MouseInput { state: ElementState::Released, button: MouseButton::Left, .. } => {
                self.handle_release();
                if self.state.close_requested {
                    event_loop.exit();
                    return;
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let delta_y = match delta {
                    MouseScrollDelta::LineDelta(_, y) => -y * LIST_ITEM_HEIGHT,
                    MouseScrollDelta::PixelDelta(pos) => -pos.y as f32,
                };
                self.handle_scroll(delta_y);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event: key_event, .. } => {
                if key_event.state == ElementState::Pressed {
                    match key_event.logical_key {
                        winit::keyboard::Key::Named(winit::keyboard::NamedKey::Tab) => {
                            if let Some(root) = self.root {
                                let next = self.tree.next_focusable(root, self.state.focused_input.as_deref(), self.shift_held).unwrap_or(None);
                                self.state.focused_input = next;
                            }
                        }
                        winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) => {
                            if self.state.show_modal {
                                self.state.show_modal = false;
                            } else if self.state.context_menu.is_some() {
                                self.state.context_menu = None;
                            } else if self.state.active_menu.is_some() || self.state.dropdown_open {
                                self.state.active_menu = None;
                                self.state.dropdown_open = false;
                            } else {
                                self.state.focused_input = None;
                            }
                        }
                        _ => {
                            let ctrl = self.ctrl_held;
                            let shift = self.shift_held;
                            if let Some(focused) = &self.state.focused_input {
                                let (editor, is_multiline) = match focused.as_str() {
                                    "global_search" => (Some(&mut self.state.search_editor), false),
                                    "master_token_pwd" => (Some(&mut self.state.password_editor), false),
                                    "studio_editor" => (Some(&mut self.state.studio_editor), true),
                                    _ => (None, false),
                                };

                                if let Some(ed) = editor {
                                    match key_event.logical_key {
                                        winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowLeft) => {
                                            ed.move_left(ctrl, shift);
                                        }
                                        winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowRight) => {
                                            ed.move_right(ctrl, shift);
                                        }
                                        winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowUp) => {
                                            if is_multiline {
                                                ed.move_up(shift);
                                            } else {
                                                ed.move_home(shift);
                                            }
                                        }
                                        winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowDown) => {
                                            if is_multiline {
                                                ed.move_down(shift);
                                            } else {
                                                ed.move_end(shift);
                                            }
                                        }
                                        winit::keyboard::Key::Named(winit::keyboard::NamedKey::Home) => {
                                            ed.move_home(shift);
                                        }
                                        winit::keyboard::Key::Named(winit::keyboard::NamedKey::End) => {
                                            ed.move_end(shift);
                                        }
                                        winit::keyboard::Key::Named(winit::keyboard::NamedKey::Backspace) => {
                                            ed.backspace();
                                        }
                                        winit::keyboard::Key::Named(winit::keyboard::NamedKey::Delete) => {
                                            ed.delete();
                                        }
                                        winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter) => {
                                            if is_multiline {
                                                ed.insert_char('\n');
                                            } else {
                                                println!("[widget_gallery] input '{focused}' submitted: '{}'", ed.text);
                                            }
                                        }
                                        _ => {
                                            if ctrl {
                                                if let winit::keyboard::Key::Character(ref s) = key_event.logical_key {
                                                    if s.eq_ignore_ascii_case("a") {
                                                        ed.select_all();
                                                    }
                                                }
                                            } else if let Some(text) = &key_event.text {
                                                for c in text.chars() {
                                                    if !c.is_control() {
                                                        ed.insert_char(c);
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::RedrawRequested => self.redraw(),
            _ => {}
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("failed to create winit event loop");
    let mut app = App::new();
    event_loop.run_app(&mut app).expect("failed to run winit event loop");
}
