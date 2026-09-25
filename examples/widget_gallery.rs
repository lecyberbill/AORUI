// [WFGY] Zone: RISK | λ: 0.3 | Fallbacks: 0 | Action: Full interactive widget gallery demo with MenuBar, Dropdown, Toast, and Modal overlays in AORUI
use std::sync::{Arc, RwLock};

use ui_core::UiEvent;
use ui_gpu::GpuRenderer;
use ui_layout::{length, AlignItems, AvailableSpace, FlexDirection, Rect, Size, Style};
use ui_widgets::{
    ColorSpace, InteractionKey, InteractionState, MediaFit, Theme, ThemeWatcher, ToastKind, WidgetId, WidgetTree,
};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{ResizeDirection, Window, WindowAttributes, WindowId};

/// Smart string middle truncation for safe, clean badge and toast display
fn truncate_middle(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        return text.to_string();
    }
    let half = (max_len.saturating_sub(3)) / 2;
    if half < 3 {
        return format!("{}...", &text[..max_len.saturating_sub(3).max(1)]);
    }
    format!("{}...{}", &text[..half], &text[text.len() - half..])
}

/// Generates a procedural cybernetic artwork image (512x512 RGBA8)
fn generate_cyber_artwork(width: u32, height: u32) -> Vec<u8> {
    let mut buffer = vec![0u8; (width * height * 4) as usize];
    let wf = width as f32;
    let hf = height as f32;
    let cx = wf * 0.5;
    let cy = hf * 0.44;
    let radius = wf * 0.26;

    for y in 0..height {
        let yf = y as f32;
        let ny = yf / hf;
        for x in 0..width {
            let xf = x as f32;
            let idx = ((y * width + x) * 4) as usize;

            // Sky background: Deep cosmic gradient
            let mut r = (10.0 + ny * 25.0).min(255.0);
            let mut g = (14.0 + ny * 10.0).min(255.0);
            let mut b = (35.0 + ny * 45.0).min(255.0);

            // Pseudo starfield
            let seed = ((x.wrapping_mul(374761393) ^ y.wrapping_mul(668265263)) % 1000) as f32;
            if ny < 0.65 && seed > 994.0 {
                let star_bright = ((seed - 994.0) / 6.0 * 255.0) as f32;
                r = (r + star_bright).min(255.0);
                g = (g + star_bright).min(255.0);
                b = (b + star_bright).min(255.0);
            }

            // Cyber Sun / Sphere
            let dx = xf - cx;
            let dy = yf - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist < radius {
                let norm_dist = dist / radius;
                let slice = (yf * 0.12).sin();
                if !(yf > cy && slice > 0.6) {
                    let sun_r = (255.0 * (1.0 - norm_dist * 0.2)).min(255.0);
                    let sun_g = (80.0 + (1.0 - ny) * 160.0).min(255.0);
                    let sun_b = (180.0 * (1.0 - norm_dist)).min(255.0);
                    r = sun_r;
                    g = sun_g;
                    b = sun_b;
                }
            }

            // Perspective Grid Floor (lower 40%)
            if ny >= 0.60 {
                let horizon_dist = (ny - 0.60) / 0.40;
                let z = 1.0 / (horizon_dist + 0.05);
                let grid_x = ((xf - cx) * z * 0.04).sin();
                let grid_y = (z * 1.8).sin();

                if grid_x.abs() > 0.92 || grid_y.abs() > 0.88 {
                    let glow = horizon_dist.powf(0.5);
                    r = (r * 0.2 + 0.0 * glow).min(255.0);
                    g = (g * 0.2 + 220.0 * glow).min(255.0);
                    b = (b * 0.2 + 255.0 * glow).min(255.0);
                } else {
                    r = (r * 0.4).min(255.0);
                    g = (g * 0.4 + 10.0).min(255.0);
                    b = (b * 0.4 + 30.0).min(255.0);
                }
            }

            buffer[idx] = r as u8;
            buffer[idx + 1] = g as u8;
            buffer[idx + 2] = b as u8;
            buffer[idx + 3] = 255;
        }
    }
    buffer
}

/// Generates an animated multi-wave plasma stream frame (256x160 RGBA8)
fn generate_plasma_frame(width: u32, height: u32, time: f32) -> Vec<u8> {
    let mut buffer = vec![0u8; (width * height * 4) as usize];
    let wf = width as f32;
    let hf = height as f32;

    for y in 0..height {
        let yf = y as f32;
        for x in 0..width {
            let xf = x as f32;
            let idx = ((y * width + x) * 4) as usize;

            let v1 = ((xf * 0.04 + time).sin() + 1.0) * 0.5;
            let v2 = ((yf * 0.04 - time * 1.2).sin() + 1.0) * 0.5;
            let cx = xf - wf * 0.5;
            let cy = yf - hf * 0.5;
            let dist = (cx * cx + cy * cy).sqrt() * 0.05;
            let v3 = ((dist - time * 2.0).sin() + 1.0) * 0.5;

            let wave = (v1 + v2 + v3) / 3.0;

            let pi = std::f32::consts::PI;
            let r = ((wave * pi).sin() * 180.0 + 30.0).clamp(0.0, 255.0) as u8;
            let g = (((wave + 0.33) * pi).sin() * 220.0 + 35.0).clamp(0.0, 255.0) as u8;
            let b = (((wave + 0.66) * pi).sin() * 255.0 + 50.0).clamp(0.0, 255.0) as u8;

            buffer[idx] = r;
            buffer[idx + 1] = g;
            buffer[idx + 2] = b;
            buffer[idx + 3] = 255;
        }
    }
    buffer
}

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
    Style {
        size: Size {
            width: length(w),
            height: length(h),
        },
        ..Default::default()
    }
}

fn row(gap: f32) -> Style {
    Style {
        flex_direction: FlexDirection::Row,
        align_items: Some(AlignItems::Center),
        gap: Size {
            width: length(gap),
            height: length(0.0),
        },
        ..Default::default()
    }
}

fn column(gap: f32) -> Style {
    Style {
        flex_direction: FlexDirection::Column,
        gap: Size {
            width: length(0.0),
            height: length(gap),
        },
        ..Default::default()
    }
}

/// Root window content container with generous padding.
fn window_content(gap: f32) -> Style {
    Style {
        flex_direction: FlexDirection::Column,
        gap: Size {
            width: length(0.0),
            height: length(gap),
        },
        padding: Rect {
            left: length(24.0),
            right: length(24.0),
            top: length(48.0),
            bottom: length(20.0),
        },
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
        size: Size {
            width: length(w),
            height: length(h),
        },
        flex_direction: FlexDirection::Column,
        gap: Size {
            width: length(0.0),
            height: length(4.0),
        },
        padding: Rect {
            left: length(4.0),
            right: length(4.0),
            top: length(4.0),
            bottom: length(4.0),
        },
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
        size: Size {
            width: length(w),
            height: length(h),
        },
        flex_direction: FlexDirection::Column,
        gap: Size {
            width: length(0.0),
            height: length(2.0),
        },
        padding: Rect {
            left: length(4.0),
            right: length(4.0),
            top: length(4.0),
            bottom: length(4.0),
        },
        ..Default::default()
    }
}

fn card_style(w: f32) -> Style {
    Style {
        size: Size {
            width: length(w),
            height: ui_layout::auto(),
        },
        flex_direction: FlexDirection::Column,
        gap: Size {
            width: length(0.0),
            height: length(8.0),
        },
        padding: Rect {
            left: length(14.0),
            right: length(14.0),
            top: length(12.0),
            bottom: length(14.0),
        },
        ..Default::default()
    }
}

#[allow(dead_code)]
fn card_style_h(w: f32, h: f32) -> Style {
    Style {
        size: Size {
            width: length(w),
            height: length(h),
        },
        flex_direction: FlexDirection::Column,
        gap: Size {
            width: length(0.0),
            height: length(8.0),
        },
        padding: Rect {
            left: length(14.0),
            right: length(14.0),
            top: length(12.0),
            bottom: length(14.0),
        },
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
        size: Size {
            width: length(w),
            height: length(h),
        },
        ..Default::default()
    }
}

/// Scroll list geometry constants.
const LIST_ITEM_HEIGHT: f32 = 32.0;
const LIST_ITEM_GAP: f32 = 8.0;
const LIST_ITEM_COUNT: usize = 6;
const LIST_VIEWPORT_HEIGHT: f32 = 120.0;
const LIST_CONTENT_HEIGHT: f32 =
    LIST_ITEM_COUNT as f32 * LIST_ITEM_HEIGHT + (LIST_ITEM_COUNT as f32 - 1.0) * LIST_ITEM_GAP;
const LIST_MAX_SCROLL_Y: f32 = LIST_CONTENT_HEIGHT - LIST_VIEWPORT_HEIGHT;
const LIST_SCROLL_ID: &str = "demo_list_scroll";

/// Window dimensions.
const WINDOW_MARGIN: f32 = 14.0;
const WINDOW_WIDTH: f32 = 1040.0;
const WINDOW_HEIGHT: f32 = 820.0;

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
        Self {
            text,
            cursor,
            selection: None,
        }
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

        let prev_newline = self.text[..self.cursor]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
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

        let next_newline = self.text[self.cursor..]
            .find('\n')
            .map(|i| self.cursor + i)
            .unwrap_or(self.text.len());
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
    network_intensity: f32,
    brush_intensity: f32,
    fader_low: f32,
    fader_mid: f32,
    fader_high: f32,
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
    spinners_enabled: bool,
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
    video_playing: bool,
    video_progress: f32,
    audio_spectrum: Vec<f32>,
    media_fit_mode: MediaFit,
    canvas_strokes: Vec<(Vec<[f32; 2]>, [f32; 4], f32)>,
    canvas_active_stroke: Vec<[f32; 2]>,
    knob_gain: f32,
    knob_freq: f32,
    knob_mix: f32,
    tags_list: Vec<String>,
    chart_inspected: Option<(usize, usize)>,
    node_graph_nodes: Vec<ui_widgets::GraphNodeSpec>,
    node_graph_connections: Vec<ui_widgets::GraphConnectionSpec>,
    selected_graph_node: Option<String>,
    file_hovered: bool,
    hovered_file: Option<std::path::PathBuf>,
    dropped_file_info: Option<(String, u64)>,
    show_command_palette: bool,
    command_palette_search: TextEditorState,
    command_palette_selected: usize,
    show_notifications_drawer: bool,
    notifications_history: Vec<(String, String, ToastKind, String)>,
    user_avatar_status: ui_widgets::AvatarStatus,
    workflow_step: usize,
    selected_chips: std::collections::HashSet<String>,
    dismissed_chips: std::collections::HashSet<String>,
    user_rating: u8,
    selected_timeline_item: Option<usize>,
}

impl DemoState {
    fn push_toast(&mut self, title: impl Into<String>, msg: impl Into<String>, kind: ToastKind) {
        let t = title.into();
        let m = msg.into();
        self.notifications_history.insert(0, (t.clone(), m.clone(), kind, "Just now".to_string()));
        if self.notifications_history.len() > 20 {
            self.notifications_history.truncate(20);
        }
        self.active_toast = Some((t, m, kind));
    }

    fn apply(&mut self, event: UiEvent) {
        match event {
            UiEvent::StepClicked { step_index, .. } => {
                self.workflow_step = step_index;
                self.push_toast(
                    "Workflow Step Selected",
                    format!("Switched to pipeline stage #{}", step_index + 1),
                    ToastKind::Info,
                );
            }
            UiEvent::ChipClicked { widget_id, .. } => {
                if self.selected_chips.contains(&widget_id) {
                    self.selected_chips.remove(&widget_id);
                } else {
                    self.selected_chips.insert(widget_id.clone());
                }
                let is_active = self.selected_chips.contains(&widget_id);
                self.push_toast(
                    "Filter Chip Toggled",
                    format!("Tag '{widget_id}' {}", if is_active { "Enabled [Active]" } else { "Disabled" }),
                    ToastKind::Info,
                );
            }
            UiEvent::ChipDismissed { widget_id, .. } => {
                self.dismissed_chips.insert(widget_id.clone());
                self.push_toast(
                    "Chip Dismissed",
                    format!("Removed tag '{widget_id}' from active workspace"),
                    ToastKind::Warning,
                );
            }
            UiEvent::RatingChanged { rating, .. } => {
                self.user_rating = rating;
                self.push_toast(
                    "Rating Feedback",
                    format!("Assigned rating score: {}/5 Stars", rating),
                    ToastKind::Success,
                );
            }
            UiEvent::TimelineItemClicked { item_index, .. } => {
                self.selected_timeline_item = Some(item_index);
                self.push_toast(
                    "Timeline Event Selected",
                    format!("Inspecting audit record #{}", item_index + 1),
                    ToastKind::Info,
                );
            }
            UiEvent::AvatarClicked { widget_id } => {
                if widget_id == "user_profile_avatar" {
                    self.user_avatar_status = match self.user_avatar_status {
                        ui_widgets::AvatarStatus::Online => ui_widgets::AvatarStatus::Away,
                        ui_widgets::AvatarStatus::Away => ui_widgets::AvatarStatus::Busy,
                        ui_widgets::AvatarStatus::Busy => ui_widgets::AvatarStatus::Offline,
                        ui_widgets::AvatarStatus::Offline => ui_widgets::AvatarStatus::Online,
                        ui_widgets::AvatarStatus::None => ui_widgets::AvatarStatus::Online,
                    };
                    self.push_toast(
                        "Presence Updated",
                        format!("Status set to {:?}", self.user_avatar_status),
                        ToastKind::Info,
                    );
                }
            }
            UiEvent::CommandExecuted { command_id } => {
                self.show_command_palette = false;
                match command_id.as_str() {
                    "cmd_tab_0" => self.active_tab = 0,
                    "cmd_tab_1" => self.active_tab = 1,
                    "cmd_tab_2" => self.active_tab = 2,
                    "cmd_tab_3" => self.active_tab = 3,
                    "cmd_tab_4" => self.active_tab = 4,
                    "cmd_tab_5" => self.active_tab = 5,
                    "cmd_tab_6" => self.active_tab = 6,
                    "cmd_turbo" => {
                        self.turbo_toggle = !self.turbo_toggle;
                        self.push_toast(
                            "Turbo Mode",
                            format!("Turbo set to {}", self.turbo_toggle),
                            ToastKind::Success,
                        );
                    }
                    "cmd_scan" => {
                        self.push_toast(
                            "Heuristic Scan",
                            "Security audit complete: All clusters operational",
                            ToastKind::Success,
                        );
                    }
                    "cmd_notifications" => {
                        self.show_notifications_drawer = !self.show_notifications_drawer;
                    }
                    "cmd_status_online" => self.user_avatar_status = ui_widgets::AvatarStatus::Online,
                    "cmd_status_busy" => self.user_avatar_status = ui_widgets::AvatarStatus::Busy,
                    _ => {}
                }
            }
            UiEvent::NotificationCleared { notification_id } => {
                if let Some(id_str) = notification_id {
                    if let Ok(idx) = id_str.parse::<usize>() {
                        if idx < self.notifications_history.len() {
                            self.notifications_history.remove(idx);
                        }
                    }
                } else {
                    self.notifications_history.clear();
                }
            }
            UiEvent::FileHovered { path, .. } => {
                self.file_hovered = true;
                self.hovered_file = Some(path);
            }
            UiEvent::FileHoverCancelled => {
                self.file_hovered = false;
                self.hovered_file = None;
            }
            UiEvent::FileDropped { path, .. } => {
                self.file_hovered = false;
                self.hovered_file = None;
                let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("file").to_string();
                let fsize = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                self.dropped_file_info = Some((fname, fsize));
            }
            UiEvent::CustomPaintPointerDown {
                widget_id,
                local_pos,
                ..
            } => {
                if widget_id == "studio_canvas" {
                    if self.active_tool == "eraser" {
                        let r = (self.brush_intensity * 0.35).max(12.0);
                        self.canvas_strokes.retain(|(pts, _, _)| {
                            !pts.iter().any(|p| {
                                ((p[0] - local_pos[0]).powi(2) + (p[1] - local_pos[1]).powi(2))
                                    .sqrt()
                                    <= r
                            })
                        });
                    } else {
                        self.canvas_active_stroke = vec![local_pos];
                    }
                }
            }
            UiEvent::CustomPaintPointerMove {
                widget_id,
                local_pos,
                ..
            } => {
                if widget_id == "studio_canvas" {
                    if self.active_tool == "eraser" {
                        let r = (self.brush_intensity * 0.35).max(12.0);
                        self.canvas_strokes.retain(|(pts, _, _)| {
                            !pts.iter().any(|p| {
                                ((p[0] - local_pos[0]).powi(2) + (p[1] - local_pos[1]).powi(2))
                                    .sqrt()
                                    <= r
                            })
                        });
                    } else if !self.canvas_active_stroke.is_empty() {
                        self.canvas_active_stroke.push(local_pos);
                    }
                }
            }
            UiEvent::PointerUp { .. } => {
                if !self.canvas_active_stroke.is_empty() {
                    let pts = std::mem::take(&mut self.canvas_active_stroke);
                    if !pts.is_empty() {
                        let stroke_w = (self.brush_intensity * 0.05).clamp(1.5, 12.0);
                        let col = self.selected_color;
                        self.canvas_strokes.push((pts, col, stroke_w));
                    }
                }
            }
            UiEvent::CheckboxToggled { widget_id, checked } => {
                if widget_id == "spinners_toggle" {
                    self.spinners_enabled = checked;
                } else {
                    self.accept_checked = checked;
                }
            }
            UiEvent::ToggleSwitched { widget_id, active } => {
                if widget_id == "turbo_toggle" {
                    self.turbo_toggle = active;
                } else if widget_id == "firewall_toggle" {
                    self.firewall_toggle = active;
                }
            }
            UiEvent::SliderChanged { widget_id, value } => match widget_id.as_str() {
                "palette_intensity_slider" => self.brush_intensity = value,
                "fader_low" => self.fader_low = value,
                "fader_mid" => self.fader_mid = value,
                "fader_high" => self.fader_high = value,
                _ => self.network_intensity = value,
            },
            UiEvent::NumberChanged { widget_id, value } => {
                if widget_id == "concurrency_spin" {
                    self.concurrency_spin = value;
                } else if widget_id == "port_spin" {
                    self.port_spin = value;
                }
            }
            UiEvent::KnobChanged { widget_id, value } => {
                match widget_id.as_str() {
                    "knob_gain" => self.knob_gain = value,
                    "knob_freq" => self.knob_freq = value,
                    "knob_mix" => self.knob_mix = value,
                    _ => {}
                }
            }
            UiEvent::ChartInspected {
                series_index,
                point_index,
                x,
                y,
                ..
            } => {
                self.chart_inspected = Some((series_index, point_index));
                self.active_toast = Some((
                    "Data Point Inspected".to_string(),
                    format!("Series #{} @ X={:.1}, Y={:.1}", series_index + 1, x, y),
                    ToastKind::Info,
                ));
            }
            UiEvent::NodeSelected { node_id, .. } => {
                self.selected_graph_node = node_id.clone();
                for n in &mut self.node_graph_nodes {
                    n.selected = Some(&n.id) == node_id.as_ref();
                }
            }
            UiEvent::TagRemoved { index, .. } => {
                if index < self.tags_list.len() {
                    let removed = self.tags_list.remove(index);
                    self.active_toast = Some((
                        "Tag Removed".to_string(),
                        format!("Tag '{}' removed from active filters.", removed),
                        ToastKind::Warning,
                    ));
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
            UiEvent::SegmentSelected {
                widget_id,
                selected_index,
                ..
            } => {
                if widget_id == "media_fit_selector" {
                    self.media_fit_mode = match selected_index {
                        0 => MediaFit::Cover,
                        1 => MediaFit::Contain,
                        _ => MediaFit::Fill,
                    };
                } else {
                    self.selected_segment = selected_index;
                }
            }
            UiEvent::MediaPlayToggled { widget_id, playing } => {
                if widget_id == "stream_video" {
                    self.video_playing = playing;
                }
            }
            UiEvent::MediaSeeked {
                widget_id,
                progress,
            } => {
                if widget_id == "stream_video" {
                    self.video_progress = progress;
                }
            }
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
            UiEvent::PaletteResized {
                palette_id,
                width,
                height,
            } => {
                if palette_id == "tools_palette" {
                    self.tools_palette_size = (width.max(120.0), height.max(120.0));
                } else if palette_id == "inspector_palette" {
                    self.inspector_palette_size = (width.max(140.0), height.max(120.0));
                }
            }
            UiEvent::TreeNodeToggled {
                node_id, expanded, ..
            } => {
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
                    format!(
                        "Column #{} ordered {}",
                        column_index + 1,
                        if self.table_sort_asc {
                            "Ascending ▲"
                        } else {
                            "Descending ▼"
                        }
                    ),
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
                    "nav_primitives" => self.active_tab = 1,
                    "nav_sec" => self.active_tab = 2,
                    "nav_net" => self.active_tab = 3,
                    "nav_studio" => self.active_tab = 4,
                    "nav_media" => self.active_tab = 5,
                    "nav_nodes" => self.active_tab = 6,
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
                    format!(
                        "Picked RGBA({:.2}, {:.2}, {:.2}, {:.2})",
                        color[0], color[1], color[2], color[3]
                    ),
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
                } else if widget_id == "clear_canvas_btn" {
                    self.canvas_strokes.clear();
                    self.canvas_active_stroke.clear();
                    self.active_toast = Some((
                        "Canvas Cleared".to_string(),
                        "All vector strokes removed.".to_string(),
                        ToastKind::Info,
                    ));
                } else if widget_id == "add_tag_btn" {
                    let next_idx = self.tags_list.len() + 1;
                    let tag_name = format!("DSP-Filter-{}", next_idx);
                    self.tags_list.push(tag_name.clone());
                    self.active_toast = Some((
                        "Tag Added".to_string(),
                        format!("Tag '{}' added.", tag_name),
                        ToastKind::Success,
                    ));
                } else if widget_id == "lock_btn" {
                    self.active_toast = Some((
                        "Security Lock".to_string(),
                        "All inbound endpoints locked.".to_string(),
                        ToastKind::Warning,
                    ));
                } else if widget_id == "refresh_btn" {
                    self.active_toast = Some((
                        "Subsystem Sync".to_string(),
                        "Cluster telemetry synchronized.".to_string(),
                        ToastKind::Info,
                    ));
                } else if widget_id == "quick_cmd_btn" {
                    self.show_command_palette = !self.show_command_palette;
                    if self.show_command_palette {
                        self.focused_input = Some("command_palette_search".to_string());
                        self.command_palette_search.text.clear();
                        self.command_palette_search.cursor = 0;
                        self.command_palette_selected = 0;
                    }
                } else if widget_id == "bell_notifications_btn" {
                    self.show_notifications_drawer = !self.show_notifications_drawer;
                } else if widget_id == "clear_notifications_btn" {
                    self.notifications_history.clear();
                } else if widget_id == "close_drawer_btn" {
                    self.show_notifications_drawer = false;
                } else if widget_id == "close_cmd_palette_btn" {
                    self.show_command_palette = false;
                } else if let Some(action_id) = widget_id.strip_prefix("cmd_action_") {
                    self.show_command_palette = false;
                    self.apply(UiEvent::CommandExecuted {
                        command_id: action_id.to_string(),
                    });
                } else {
                    self.click_count += 1;
                    println!(
                        "[widget_gallery] button '{widget_id}' clicked ({} times)",
                        self.click_count
                    );
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
                    "tab_4" => self.active_tab = 4,
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
            UiEvent::SelectChanged {
                widget_id,
                selected_id,
            } => {
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

fn build_base_ui(
    tree: &mut WidgetTree,
    state: &DemoState,
    width: f32,
    height: f32,
) -> ui_layout::NodeId {
    let content_w = (width - WINDOW_MARGIN * 2.0 - 48.0).max(500.0);
    let half_col_w = ((content_w - 16.0) * 0.5).max(200.0);

    // 1. Unified Top Workstation Header Bar (32px)
    let logo_label = tree
        .label("⬡ AORUI Studio", leaf(120.0, 24.0))
        .unwrap();

    let menu_items = ["File", "Security", "View", "Help"];
    let menubar = tree
        .menubar(
            WidgetId::new("main_menubar"),
            &menu_items,
            state.active_menu,
            leaf(58.0, 24.0),
            row(2.0),
        )
        .unwrap();

    let left_header_group = tree
        .container(&[logo_label, menubar], row(8.0))
        .unwrap();

    let tab_names = ["General", "UI Primitives", "Security", "Network", "Studio", "Media", "Nodes & Data"];
    let current_tab_name = tab_names.get(state.active_tab).unwrap_or(&"General");
    let crumbs = [
        ("nav_home", "Workspace"),
        ("nav_cluster", "US-East-01"),
        ("nav_active", *current_tab_name),
    ];
    let breadcrumb_node = tree
        .breadcrumb("main_breadcrumb", &crumbs, leaf(72.0, 20.0), row(2.0))
        .unwrap();

    let cmd_btn = tree
        .button(
            WidgetId::new("quick_cmd_btn"),
            "⌕ Search  [Ctrl+K]",
            true,
            leaf(132.0, 24.0),
        )
        .unwrap();

    let bell_btn = tree
        .icon_button(
            "bell_notifications_btn",
            ui_widgets::IconKind::Alert,
            true,
            leaf(24.0, 24.0),
        )
        .unwrap();

    let avatar_widget = tree
        .avatar(
            "user_profile_avatar",
            Some("cyber_art"),
            Some("CB"),
            state.user_avatar_status,
            24.0,
            Some([0.0, 0.85, 1.0, 1.0]),
            true,
            leaf(24.0, 24.0),
        )
        .unwrap();

    let user_lbl = tree
        .label("CyberBill", leaf(58.0, 20.0))
        .unwrap();

    let right_header_group = tree
        .container(&[cmd_btn, bell_btn, avatar_widget, user_lbl], row(8.0))
        .unwrap();

    let top_header = tree
        .container(
            &[left_header_group, breadcrumb_node, right_header_group],
            Style {
                size: Size {
                    width: length(content_w),
                    height: length(30.0),
                },
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                justify_content: Some(ui_layout::JustifyContent::SpaceBetween),
                ..Default::default()
            },
        )
        .unwrap();

    let top_divider = tree.divider(false, leaf(content_w, 1.0)).unwrap();

    let tabs = ["General", "UI Primitives", "Security", "Network", "Studio", "Media", "Nodes & Data"];
    let tabbar = tree
        .tabbar(
            WidgetId::new("settings_tabs"),
            &tabs,
            state.active_tab,
            leaf(110.0, 28.0),
            row(4.0),
        )
        .unwrap();

    let tab_content = match state.active_tab {
        0 => {
            // --- Tab 0: General (2 Grounded Cyber-Glass Workstation Cards) ---
            // Left Card: Color Studio & Chromatic Matrix
            let left_card_title = tree
                .label("Color Studio & Chromatic Matrix", leaf(half_col_w - 28.0, 18.0))
                .unwrap();
            let left_card_sub = tree
                .label_muted("Interactive 2D saturation/value canvas with RGB, CMYK, HSV & CIELAB matrix", leaf(half_col_w - 28.0, 14.0))
                .unwrap();

            let color_peeker = tree
                .color_picker(
                    WidgetId::new("main_peeker"),
                    state.selected_color,
                    state.color_space,
                    leaf(half_col_w - 28.0, 196.0),
                )
                .unwrap();

            let color_label = tree.label_muted("Presets:", leaf(50.0, 24.0)).unwrap();
            let c1 = tree
                .color_swatch(
                    "cyan_swatch",
                    [0.0, 0.85, 1.0, 1.0],
                    Some("Cyan"),
                    leaf(42.0, 28.0),
                )
                .unwrap();
            let c2 = tree
                .color_swatch(
                    "purple_swatch",
                    [0.55, 0.36, 0.96, 1.0],
                    Some("Violet"),
                    leaf(42.0, 28.0),
                )
                .unwrap();
            let c3 = tree
                .color_swatch(
                    "green_swatch",
                    [0.06, 0.72, 0.51, 1.0],
                    Some("Emerald"),
                    leaf(42.0, 28.0),
                )
                .unwrap();
            let c4 = tree
                .color_swatch(
                    "amber_swatch",
                    [0.96, 0.62, 0.04, 1.0],
                    Some("Amber"),
                    leaf(42.0, 28.0),
                )
                .unwrap();
            let swatches = tree.container(&[c1, c2, c3, c4], row(4.0)).unwrap();
            let badge_status = tree
                .badge(
                    "ONLINE",
                    ui_widgets::ListItemBadge::Success,
                    leaf(68.0, 22.0),
                )
                .unwrap();
            let palette_row = tree
                .container(&[color_label, swatches, badge_status], row(6.0))
                .unwrap();

            let div_left = tree.divider(false, leaf(half_col_w - 28.0, 1.0)).unwrap();

            let actions_label = tree
                .label_muted("Quick Workspace Actions:", leaf(half_col_w - 28.0, 14.0))
                .unwrap();

            let button = tree
                .button(
                    WidgetId::new("hello_button"),
                    "Quick Action",
                    true,
                    leaf((half_col_w - 44.0) / 3.0, 28.0),
                )
                .unwrap();
            let modal_btn = tree
                .button(
                    WidgetId::new("open_modal_btn"),
                    "Open Modal",
                    true,
                    leaf((half_col_w - 44.0) / 3.0, 28.0),
                )
                .unwrap();
            let toast_btn = tree
                .button(
                    WidgetId::new("toast_btn"),
                    "Trigger Toast",
                    true,
                    leaf((half_col_w - 44.0) / 3.0, 28.0),
                )
                .unwrap();
            let buttons_grid = tree
                .grid(
                    3,
                    6.0,
                    0.0,
                    &[button, modal_btn, toast_btn],
                    leaf(half_col_w - 28.0, 28.0),
                )
                .unwrap();

            let card_left = tree
                .card(
                    &[
                        left_card_title,
                        left_card_sub,
                        color_peeker,
                        palette_row,
                        div_left,
                        actions_label,
                        buttons_grid,
                    ],
                    None,
                    None,
                    Some(8.0),
                    card_style(half_col_w),
                )
                .unwrap();

            // Right Card: Cluster Configuration & Telemetry Controls
            let right_card_title = tree
                .label("Cluster Configuration & Telemetry", leaf(half_col_w - 28.0, 18.0))
                .unwrap();
            let right_card_sub = tree
                .label_muted("Hardware acceleration, credentials, worker count and audio DSP faders", leaf(half_col_w - 28.0, 14.0))
                .unwrap();

            let env_label = tree
                .label_muted("Target Cluster:", leaf(100.0, 26.0))
                .unwrap();
            let env_dropdown = tree
                .dropdown(
                    WidgetId::new("env_dropdown"),
                    "Cluster",
                    &state.selected_env,
                    state.dropdown_open,
                    leaf(half_col_w - 138.0, 26.0),
                )
                .unwrap();
            let env_row = tree
                .container(&[env_label, env_dropdown], row(8.0))
                .unwrap();

            let pwd_focused = state.focused_input.as_deref() == Some("master_token_pwd");
            let pwd_input = tree
                .password_input_with_cursor(
                    WidgetId::new("master_token_pwd"),
                    &state.password_editor.text,
                    "Master Access Token [Click eye to reveal]...",
                    pwd_focused,
                    state.password_revealed,
                    state.password_editor.cursor,
                    leaf(half_col_w - 138.0, 26.0),
                )
                .unwrap();
            let pwd_label = tree.label_muted("Auth Token:", leaf(100.0, 26.0)).unwrap();
            let pwd_row = tree.container(&[pwd_label, pwd_input], row(8.0)).unwrap();

            let spin1_focused = state.focused_input.as_deref() == Some("concurrency_spin");
            let spin1_input = tree
                .number_input_state(
                    WidgetId::new("concurrency_spin"),
                    state.concurrency_spin,
                    1.0,
                    64.0,
                    1.0,
                    0,
                    spin1_focused,
                    state.spinners_enabled,
                    leaf((half_col_w - 200.0) * 0.5, 26.0),
                )
                .unwrap();
            let spin1_label = tree
                .label_muted("Workers (1-64):", leaf(90.0, 26.0))
                .unwrap();
            let spin1_box = tree
                .container(&[spin1_label, spin1_input], row(4.0))
                .unwrap();

            let spin2_focused = state.focused_input.as_deref() == Some("port_spin");
            let spin2_input = tree
                .number_input_state(
                    WidgetId::new("port_spin"),
                    state.port_spin,
                    1024.0,
                    65535.0,
                    10.0,
                    0,
                    spin2_focused,
                    state.spinners_enabled,
                    leaf((half_col_w - 200.0) * 0.5, 26.0),
                )
                .unwrap();
            let spin2_label = tree.label_muted("Port (1024+):", leaf(80.0, 26.0)).unwrap();
            let spin2_box = tree
                .container(&[spin2_label, spin2_input], row(4.0))
                .unwrap();

            let spinners_grid = tree
                .grid(2, 8.0, 0.0, &[spin1_box, spin2_box], leaf(half_col_w - 28.0, 26.0))
                .unwrap();

            let spin_toggle = tree
                .checkbox(
                    WidgetId::new("spinners_toggle"),
                    state.spinners_enabled,
                    leaf(16.0, 16.0),
                )
                .unwrap();
            let spin_toggle_lbl = tree
                .label_muted(
                    if state.spinners_enabled {
                        "Hybrid Input: Enabled (Type Digits or Up/Down Steppers)"
                    } else {
                        "Hybrid Input: Disabled (Locked)"
                    },
                    leaf(half_col_w - 56.0, 16.0),
                )
                .unwrap();
            let spin_toggle_row = tree
                .container(&[spin_toggle, spin_toggle_lbl], row(6.0))
                .unwrap();

            let checkbox = tree
                .checkbox(
                    WidgetId::new("accept_terms"),
                    state.accept_checked,
                    leaf(16.0, 16.0),
                )
                .unwrap();
            let checkbox_label = tree
                .label("Enable continuous diagnostics telemetry", leaf(300.0, 16.0))
                .unwrap();
            let checkbox_row = tree
                .container(&[checkbox, checkbox_label], row(8.0))
                .unwrap();

            let toggle = tree
                .toggle(
                    WidgetId::new("turbo_toggle"),
                    state.turbo_toggle,
                    leaf(36.0, 18.0),
                )
                .unwrap();
            let toggle_label = tree
                .label("GPU Turbo Hardware Acceleration", leaf(260.0, 18.0))
                .unwrap();
            let toggle_row = tree.container(&[toggle, toggle_label], row(8.0)).unwrap();

            let div_right = tree.divider(false, leaf(half_col_w - 28.0, 1.0)).unwrap();

            let slider_label = tree
                .label_muted("Multi-Shape Controls (Sliders, EQ Faders, Donut Ring, Pie):", leaf(half_col_w - 28.0, 14.0))
                .unwrap();
            let slider = tree
                .slider(
                    WidgetId::new("brightness_slider"),
                    0.0,
                    100.0,
                    state.network_intensity,
                    leaf(120.0, 14.0),
                )
                .unwrap();
            let progress_h = tree
                .progress_bar(state.network_intensity / 100.0, leaf(120.0, 5.0))
                .unwrap();
            let slider_h_box = tree.container(&[slider, progress_h], column(6.0)).unwrap();

            let f1 = tree
                .slider_vertical(
                    WidgetId::new("fader_low"),
                    0.0,
                    100.0,
                    state.fader_low,
                    leaf(18.0, 36.0),
                )
                .unwrap();
            let f1_lbl = tree.label_muted("Lo", leaf(18.0, 12.0)).unwrap();
            let f1_box = tree.container(&[f1, f1_lbl], column(2.0)).unwrap();

            let f2 = tree
                .slider_vertical(
                    WidgetId::new("fader_mid"),
                    0.0,
                    100.0,
                    state.fader_mid,
                    leaf(18.0, 36.0),
                )
                .unwrap();
            let f2_lbl = tree.label_muted("Mid", leaf(18.0, 12.0)).unwrap();
            let f2_box = tree.container(&[f2, f2_lbl], column(2.0)).unwrap();

            let f3 = tree
                .slider_vertical(
                    WidgetId::new("fader_high"),
                    0.0,
                    100.0,
                    state.fader_high,
                    leaf(18.0, 36.0),
                )
                .unwrap();
            let f3_lbl = tree.label_muted("Hi", leaf(18.0, 12.0)).unwrap();
            let f3_box = tree.container(&[f3, f3_lbl], column(2.0)).unwrap();
            let faders_row = tree.container(&[f1_box, f2_box, f3_box], row(4.0)).unwrap();

            let progress_v = tree
                .progress_bar_vertical(state.fader_mid / 100.0, leaf(8.0, 40.0))
                .unwrap();
            let ring_gauge = tree
                .progress_ring(
                    state.network_intensity / 100.0,
                    None::<&str>,
                    leaf(40.0, 40.0),
                )
                .unwrap();
            let pie_disc = tree
                .progress_pie(
                    state.brush_intensity / 100.0,
                    None::<&str>,
                    leaf(40.0, 40.0),
                )
                .unwrap();

            let control_row = tree
                .container(
                    &[slider_h_box, faders_row, progress_v, ring_gauge, pie_disc],
                    row(8.0),
                )
                .unwrap();

            let card_right = tree
                .card(
                    &[
                        right_card_title,
                        right_card_sub,
                        env_row,
                        pwd_row,
                        spinners_grid,
                        spin_toggle_row,
                        checkbox_row,
                        toggle_row,
                        div_right,
                        slider_label,
                        control_row,
                    ],
                    None,
                    None,
                    Some(8.0),
                    card_style(half_col_w),
                )
                .unwrap();

            tree.container(&[card_left, card_right], row(16.0)).unwrap()
        }
        1 => {
            // --- Tab 1: UI Primitives (2 Grounded Cyber-Glass Workstation Cards) ---
            // Left Card: Workflow & Interactive Primitives
            let left_card_title = tree
                .label("Workflow & Interactive Primitives", leaf(half_col_w - 28.0, 18.0))
                .unwrap();
            let left_card_sub = tree
                .label_muted("Stepper milestones, filter chips, 3D keycaps, multi-progress & ratings", leaf(half_col_w - 28.0, 14.0))
                .unwrap();

            let step_items = vec![
                ui_widgets::StepItem {
                    label: "Init".to_string(),
                    description: Some("Bootstrap".to_string()),
                    state: if state.workflow_step > 0 {
                        ui_widgets::StepState::Completed
                    } else if state.workflow_step == 0 {
                        ui_widgets::StepState::Active
                    } else {
                        ui_widgets::StepState::Pending
                    },
                },
                ui_widgets::StepItem {
                    label: "Config".to_string(),
                    description: Some("Cluster Mesh".to_string()),
                    state: if state.workflow_step > 1 {
                        ui_widgets::StepState::Completed
                    } else if state.workflow_step == 1 {
                        ui_widgets::StepState::Active
                    } else {
                        ui_widgets::StepState::Pending
                    },
                },
                ui_widgets::StepItem {
                    label: "Deploy".to_string(),
                    description: Some("WGPU Engine".to_string()),
                    state: if state.workflow_step > 2 {
                        ui_widgets::StepState::Completed
                    } else if state.workflow_step == 2 {
                        ui_widgets::StepState::Active
                    } else {
                        ui_widgets::StepState::Pending
                    },
                },
                ui_widgets::StepItem {
                    label: "Audit".to_string(),
                    description: Some("Zero-Trust".to_string()),
                    state: if state.workflow_step == 3 {
                        ui_widgets::StepState::Error
                    } else {
                        ui_widgets::StepState::Pending
                    },
                },
            ];
            let stepper_node = tree
                .stepper("pipeline_stepper", step_items, state.workflow_step, leaf(half_col_w - 28.0, 52.0))
                .unwrap();

            let div_p1 = tree.divider(false, leaf(half_col_w - 28.0, 1.0)).unwrap();

            let chip_title = tree
                .label_muted("Interactive Filter Chips (Selectable & Dismissible):", leaf(half_col_w - 28.0, 14.0))
                .unwrap();
            let mut chip_nodes = Vec::new();
            if !state.dismissed_chips.contains("chip_gpu") {
                chip_nodes.push(
                    tree.chip(
                        "chip_gpu",
                        "WGPU Core",
                        Some(ui_widgets::IconKind::Refresh),
                        Some([0.0, 0.85, 1.0, 1.0]),
                        state.selected_chips.contains("chip_gpu"),
                        true,
                        ui_widgets::ChipVariant::Primary,
                        leaf(108.0, 26.0),
                    )
                    .unwrap(),
                );
            }
            if !state.dismissed_chips.contains("chip_shader") {
                chip_nodes.push(
                    tree.chip(
                        "chip_shader",
                        "WGSL Shaders",
                        Some(ui_widgets::IconKind::Settings),
                        Some([0.55, 0.36, 0.96, 1.0]),
                        state.selected_chips.contains("chip_shader"),
                        true,
                        ui_widgets::ChipVariant::Outline,
                        leaf(116.0, 26.0),
                    )
                    .unwrap(),
                );
            }
            if !state.dismissed_chips.contains("chip_shield") {
                chip_nodes.push(
                    tree.chip(
                        "chip_shield",
                        "Zero-Trust",
                        Some(ui_widgets::IconKind::Shield),
                        Some([0.06, 0.72, 0.51, 1.0]),
                        state.selected_chips.contains("chip_shield"),
                        true,
                        ui_widgets::ChipVariant::Success,
                        leaf(104.0, 26.0),
                    )
                    .unwrap(),
                );
            }
            if !state.dismissed_chips.contains("chip_auth") {
                chip_nodes.push(
                    tree.chip(
                        "chip_auth",
                        "TLS 1.3",
                        Some(ui_widgets::IconKind::Lock),
                        Some([0.96, 0.62, 0.04, 1.0]),
                        state.selected_chips.contains("chip_auth"),
                        false,
                        ui_widgets::ChipVariant::Default,
                        leaf(84.0, 26.0),
                    )
                    .unwrap(),
                );
            }
            let chips_row = tree.container(&chip_nodes, row(6.0)).unwrap();

            let kbd_title = tree
                .label_muted("Cybernetic Keyboard Badges (Kbd):", leaf(half_col_w - 28.0, 14.0))
                .unwrap();
            let k1 = tree.kbd("Ctrl", leaf(38.0, 22.0)).unwrap();
            let k_plus1 = tree.label("+", leaf(10.0, 22.0)).unwrap();
            let k2 = tree.kbd("K", leaf(24.0, 22.0)).unwrap();
            let k_sep = tree.label("   ", leaf(12.0, 22.0)).unwrap();
            let k3 = tree.kbd("Shift", leaf(44.0, 22.0)).unwrap();
            let k_plus2 = tree.label("+", leaf(10.0, 22.0)).unwrap();
            let k4 = tree.kbd("Esc", leaf(34.0, 22.0)).unwrap();
            let k_sep2 = tree.label("   ", leaf(12.0, 22.0)).unwrap();
            let k5 = tree.kbd("Enter ↵", leaf(56.0, 22.0)).unwrap();
            let k6 = tree.kbd("Space ␣", leaf(62.0, 22.0)).unwrap();
            let kbd_row = tree
                .container(&[k1, k_plus1, k2, k_sep, k3, k_plus2, k4, k_sep2, k5, k6], row(4.0))
                .unwrap();

            let div_p2 = tree.divider(false, leaf(half_col_w - 28.0, 1.0)).unwrap();

            let mp_title = tree
                .label_muted("Multi-Progress Proportional Resource Split:", leaf(half_col_w - 28.0, 14.0))
                .unwrap();
            let segments = vec![
                ui_widgets::ProgressSegment {
                    label: "VRAM".to_string(),
                    value: 45.0,
                    color: [0.0, 0.85, 1.0, 1.0],
                },
                ui_widgets::ProgressSegment {
                    label: "RAM".to_string(),
                    value: 25.0,
                    color: [0.55, 0.36, 0.96, 1.0],
                },
                ui_widgets::ProgressSegment {
                    label: "Swap".to_string(),
                    value: 15.0,
                    color: [0.96, 0.62, 0.04, 1.0],
                },
                ui_widgets::ProgressSegment {
                    label: "Free".to_string(),
                    value: 15.0,
                    color: [0.25, 0.30, 0.40, 0.7],
                },
            ];
            let multi_p = tree
                .multi_progress("memory_alloc_bar", segments, true, leaf(half_col_w - 28.0, 22.0))
                .unwrap();

            let rating_title = tree
                .label_muted(
                    format!("Interactive Rating Feedback ({}/5 Stars):", state.user_rating),
                    leaf(220.0, 24.0),
                )
                .unwrap();
            let rating_w = tree
                .rating(
                    "demo_star_rating",
                    state.user_rating,
                    5,
                    ui_widgets::RatingGlyph::Star,
                    false,
                    leaf(120.0, 24.0),
                )
                .unwrap();
            let rating_row = tree.container(&[rating_title, rating_w], row(8.0)).unwrap();

            let card_left = tree
                .card(
                    &[
                        left_card_title,
                        left_card_sub,
                        stepper_node,
                        div_p1,
                        chip_title,
                        chips_row,
                        kbd_title,
                        kbd_row,
                        div_p2,
                        mp_title,
                        multi_p,
                        rating_row,
                    ],
                    None,
                    None,
                    Some(8.0),
                    card_style(half_col_w),
                )
                .unwrap();

            // Right Card: Chronological Timeline Feed & Skeleton Loading
            let right_card_title = tree
                .label("Audit Stream & Shimmer Placeholders", leaf(half_col_w - 28.0, 18.0))
                .unwrap();
            let right_card_sub = tree
                .label_muted("Continuous event audit timeline and asynchronous data loading skeletons", leaf(half_col_w - 28.0, 14.0))
                .unwrap();

            let timeline_items = vec![
                ui_widgets::TimelineItem {
                    time: "09:41:22".to_string(),
                    title: "Cluster Initialized".to_string(),
                    description: Some("Bootstrapped 4 regional nodes across Paris, Tokyo, Frankfurt, Sydney".to_string()),
                    status: ui_widgets::TimelineStatus::Success,
                },
                ui_widgets::TimelineItem {
                    time: "09:42:05".to_string(),
                    title: "WGPU Shader Compiled".to_string(),
                    description: Some("Dual-Kawase bloom and glass SDF pipeline initialized with 0 warnings".to_string()),
                    status: ui_widgets::TimelineStatus::Default,
                },
                ui_widgets::TimelineItem {
                    time: "09:44:18".to_string(),
                    title: "Heuristic Shield Triggered".to_string(),
                    description: Some("Anomalous ingress packet neutralized at Edge Gateway #02".to_string()),
                    status: ui_widgets::TimelineStatus::Warning,
                },
                ui_widgets::TimelineItem {
                    time: "09:45:00".to_string(),
                    title: "Zero-Trust Security Audit".to_string(),
                    description: Some("Verifying SHA-256 integrity of active memory buffers and tokens".to_string()),
                    status: ui_widgets::TimelineStatus::Active,
                },
            ];
            let timeline_node = tree
                .timeline(
                    "audit_timeline",
                    timeline_items,
                    leaf(half_col_w - 28.0, 185.0),
                )
                .unwrap();

            let div_p3 = tree.divider(false, leaf(half_col_w - 28.0, 1.0)).unwrap();

            let skel_title = tree
                .label_muted("Async Shimmer Placeholder (Skeleton):", leaf(half_col_w - 28.0, 14.0))
                .unwrap();
            let skel_avatar = tree
                .skeleton(Some(16.0), true, leaf(32.0, 32.0))
                .unwrap();
            let skel_line1 = tree
                .skeleton(Some(4.0), true, leaf(half_col_w - 80.0, 12.0))
                .unwrap();
            let skel_line2 = tree
                .skeleton(Some(4.0), true, leaf(half_col_w - 140.0, 10.0))
                .unwrap();
            let skel_text_col = tree.container(&[skel_line1, skel_line2], column(4.0)).unwrap();
            let skel_header = tree.container(&[skel_avatar, skel_text_col], row(8.0)).unwrap();
            let skel_card = tree
                .skeleton(
                    Some(6.0),
                    true,
                    leaf(half_col_w - 28.0, 40.0),
                )
                .unwrap();
            let skel_container = tree.container(&[skel_header, skel_card], column(6.0)).unwrap();

            let card_right = tree
                .card(
                    &[
                        right_card_title,
                        right_card_sub,
                        timeline_node,
                        div_p3,
                        skel_title,
                        skel_container,
                    ],
                    None,
                    None,
                    Some(8.0),
                    card_style(half_col_w),
                )
                .unwrap();

            tree.container(&[card_left, card_right], row(16.0)).unwrap()
        }
        2 => {
            // --- Tab 2: Security (2 Grounded Cyber-Glass Workstation Cards) ---
            // Left Card: Zero-Trust & Heuristic Shield Controls
            let left_card_title = tree
                .label("Zero-Trust & Heuristic Shield", leaf(half_col_w - 28.0, 18.0))
                .unwrap();
            let left_card_sub = tree
                .label_muted("Live firewall switch, policy selector matrix & cryptographic endpoints", leaf(half_col_w - 28.0, 14.0))
                .unwrap();

            let toggle = tree
                .toggle(
                    WidgetId::new("firewall_toggle"),
                    state.firewall_toggle,
                    leaf(42.0, 22.0),
                )
                .unwrap();
            let toggle_label = tree
                .label("Heuristic Shield & Core Firewall", leaf(half_col_w - 90.0, 20.0))
                .unwrap();
            let toggle_row = tree.container(&[toggle, toggle_label], row(10.0)).unwrap();

            let div_s1 = tree.divider(false, leaf(half_col_w - 28.0, 1.0)).unwrap();

            let radio_title = tree
                .label_muted("Cluster Security Enforcement Mode:", leaf(half_col_w - 28.0, 16.0))
                .unwrap();
            let r1 = tree
                .radio(
                    WidgetId::new("strict"),
                    "sec_policy",
                    "Strict (Zero-Trust)",
                    state.security_policy == "strict",
                    leaf((half_col_w - 44.0) / 3.0, 22.0),
                )
                .unwrap();
            let r2 = tree
                .radio(
                    WidgetId::new("adaptive"),
                    "sec_policy",
                    "Adaptive (AI)",
                    state.security_policy == "adaptive",
                    leaf((half_col_w - 44.0) / 3.0, 22.0),
                )
                .unwrap();
            let r3 = tree
                .radio(
                    WidgetId::new("audit"),
                    "sec_policy",
                    "Audit Only",
                    state.security_policy == "audit",
                    leaf((half_col_w - 44.0) / 3.0, 22.0),
                )
                .unwrap();
            let radio_grid = tree
                .grid(3, 8.0, 0.0, &[r1, r2, r3], leaf(half_col_w - 28.0, 22.0))
                .unwrap();

            let div_s2 = tree.divider(false, leaf(half_col_w - 28.0, 1.0)).unwrap();

            let actions_label = tree
                .label_muted("Rapid Endpoint Controls & Diagnostics:", leaf(half_col_w - 28.0, 14.0))
                .unwrap();

            let lock_btn = tree
                .icon_button(
                    "lock_btn",
                    ui_widgets::IconKind::Lock,
                    true,
                    leaf(30.0, 30.0),
                )
                .unwrap();
            let refresh_btn = tree
                .icon_button(
                    "refresh_btn",
                    ui_widgets::IconKind::Refresh,
                    true,
                    leaf(30.0, 30.0),
                )
                .unwrap();
            let shield_btn = tree
                .icon_button(
                    "shield_btn",
                    ui_widgets::IconKind::Shield,
                    true,
                    leaf(30.0, 30.0),
                )
                .unwrap();
            let action_icons = tree
                .container(&[lock_btn, refresh_btn, shield_btn], row(8.0))
                .unwrap();

            let card_left = tree
                .card(
                    &[
                        left_card_title,
                        left_card_sub,
                        toggle_row,
                        div_s1,
                        radio_title,
                        radio_grid,
                        div_s2,
                        actions_label,
                        action_icons,
                    ],
                    None,
                    None,
                    Some(8.0),
                    card_style(half_col_w),
                )
                .unwrap();

            // Right Card: Active Ingress Rules & Containment Policies
            let right_card_title = tree
                .label("Active Containment & Ingress Rules", leaf(half_col_w - 28.0, 18.0))
                .unwrap();
            let right_card_sub = tree
                .label_muted("Real-time network packet inspection matrix and containment logs", leaf(half_col_w - 28.0, 14.0))
                .unwrap();

            let items = [
                (
                    "TLS 1.3 Certificate valid",
                    ui_widgets::ListItemBadge::Success,
                ),
                (
                    "Unauthorized port scan blocked",
                    ui_widgets::ListItemBadge::Warning,
                ),
                (
                    "Memory analysis active",
                    ui_widgets::ListItemBadge::Active("Active".to_string()),
                ),
                (
                    "Firewall rule #104 allowed",
                    ui_widgets::ListItemBadge::None,
                ),
            ];
            let list = tree
                .rich_list(
                    WidgetId::new("sec_list"),
                    &items,
                    state.selected_item,
                    leaf(half_col_w - 28.0, 28.0),
                    column(4.0),
                )
                .unwrap();

            let acc_content = tree.container(&[list], column(6.0)).unwrap();
            let acc_style = Style {
                size: Size {
                    width: length(half_col_w - 28.0),
                    height: if state.accordion_open {
                        ui_layout::auto()
                    } else {
                        length(46.0)
                    },
                },
                padding: Rect {
                    left: length(0.0),
                    right: length(0.0),
                    top: length(0.0),
                    bottom: if state.accordion_open {
                        length(8.0)
                    } else {
                        length(0.0)
                    },
                },
                ..Default::default()
            };
            let accordion = tree
                .accordion(
                    "firewall_acc",
                    "Heuristic Firewall Policies",
                    Some("4 containment rules"),
                    state.accordion_open,
                    acc_content,
                    acc_style,
                )
                .unwrap();

            let card_right = tree
                .card(
                    &[
                        right_card_title,
                        right_card_sub,
                        accordion,
                    ],
                    None,
                    None,
                    Some(8.0),
                    card_style(half_col_w),
                )
                .unwrap();

            tree.container(&[card_left, card_right], row(16.0)).unwrap()
        }
        3 => {
            // --- Tab 3: Network (Telemetry Metric Cards & Sortable TableView) ---
            // Top Card: Cluster Metrics & Throughput
            let metrics_title = tree
                .label("Cluster Telemetry & Ingress Performance", leaf(content_w - 260.0, 18.0))
                .unwrap();
            let seg_options = ["Real-Time", "24 Hours", "7 Days"];
            let segment_bar = tree
                .segmented_control(
                    WidgetId::new("net_timeframe"),
                    &seg_options,
                    state.selected_segment,
                    leaf(220.0, 24.0),
                    row(4.0),
                )
                .unwrap();
            let metrics_header = tree
                .container(
                    &[metrics_title, segment_bar],
                    Style {
                        size: Size {
                            width: length(content_w - 28.0),
                            height: length(26.0),
                        },
                        flex_direction: FlexDirection::Row,
                        align_items: Some(AlignItems::Center),
                        justify_content: Some(ui_layout::JustifyContent::SpaceBetween),
                        ..Default::default()
                    },
                )
                .unwrap();

            let card_w = (content_w - 28.0 - 36.0) / 4.0;
            let card1 = tree
                .metric_card(
                    "Inbound Bandwidth",
                    "1.24 Gbps",
                    Some(("+18%", true)),
                    leaf(card_w, 56.0),
                )
                .unwrap();
            let card2 = tree
                .metric_card(
                    "Average Latency",
                    "3.8 ms",
                    Some(("-12%", true)),
                    leaf(card_w, 56.0),
                )
                .unwrap();
            let card3 = tree
                .metric_card(
                    "Packet Loss",
                    "0.001%",
                    Some(("-95%", true)),
                    leaf(card_w, 56.0),
                )
                .unwrap();
            let card4 = tree
                .metric_card(
                    "Active Sockets",
                    "14,892",
                    Some(("+4.2%", true)),
                    leaf(card_w, 56.0),
                )
                .unwrap();
            let metrics_grid = tree
                .grid(4, 12.0, 0.0, &[card1, card2, card3, card4], leaf(content_w - 28.0, 56.0))
                .unwrap();

            let top_card = tree
                .card(
                    &[metrics_header, metrics_grid],
                    None,
                    None,
                    Some(8.0),
                    card_style(content_w),
                )
                .unwrap();

            // Bottom Card: Edge Gateway & Regional Node Status
            let table_title = tree
                .label("Regional Node Health & Traffic Routing", leaf(content_w - 28.0, 18.0))
                .unwrap();
            let table_sub = tree
                .label_muted("Multi-column sortable telemetry table with active load distribution and latency metrics", leaf(content_w - 28.0, 14.0))
                .unwrap();

            let col_step = (content_w - 48.0) / 4.0;
            let cols = [
                (
                    "Service / Node",
                    col_step * 1.3,
                    if state.table_sort_col == 0 {
                        Some(state.table_sort_asc)
                    } else {
                        None
                    },
                ),
                (
                    "Status",
                    col_step * 0.8,
                    if state.table_sort_col == 1 {
                        Some(state.table_sort_asc)
                    } else {
                        None
                    },
                ),
                (
                    "Latency",
                    col_step * 0.8,
                    if state.table_sort_col == 2 {
                        Some(state.table_sort_asc)
                    } else {
                        None
                    },
                ),
                (
                    "Traffic",
                    col_step * 1.1,
                    if state.table_sort_col == 3 {
                        Some(state.table_sort_asc)
                    } else {
                        None
                    },
                ),
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
            let data_rows = [row1, row2, row3, row4];
            let table = tree
                .table(
                    "cluster_table",
                    &cols,
                    &data_rows,
                    state.table_selected_row,
                    26.0,
                    leaf(content_w - 28.0, 136.0),
                )
                .unwrap();

            let div_tbl = tree.divider(false, leaf(content_w - 28.0, 1.0)).unwrap();

            let pagination = tree
                .pagination("cluster_pages", state.current_page, 4, leaf(32.0, 24.0), row(6.0))
                .unwrap();
            let page_info = tree
                .label_muted(format!("Page {} of 4 (42 Cluster Nodes)", state.current_page + 1), leaf(180.0, 24.0))
                .unwrap();
            let pagination_row = tree
                .container(
                    &[page_info, pagination],
                    Style {
                        size: Size {
                            width: length(content_w - 28.0),
                            height: length(24.0),
                        },
                        flex_direction: FlexDirection::Row,
                        align_items: Some(AlignItems::Center),
                        justify_content: Some(ui_layout::JustifyContent::SpaceBetween),
                        ..Default::default()
                    },
                )
                .unwrap();

            let bottom_card = tree
                .card(
                    &[
                        table_title,
                        table_sub,
                        table,
                        div_tbl,
                        pagination_row,
                    ],
                    None,
                    None,
                    Some(8.0),
                    card_style(content_w),
                )
                .unwrap();

            tree.container(&[top_card, bottom_card], column(12.0)).unwrap()
        }
        4 => {
            // --- Tab 4: Studio (Tree view, CustomPaint 2D Canvas & SplitView Card) ---
            let studio_title = tree
                .label("Vector Canvas & Workspace Architecture", leaf(content_w - 28.0, 18.0))
                .unwrap();
            let studio_sub = tree
                .label_muted("Workspace module tree inspection and interactive 2D vector spline drawing pipeline", leaf(content_w - 28.0, 14.0))
                .unwrap();

            let mut tree_nodes: Vec<(&str, &str, usize, bool, bool, bool)> = Vec::new();
            let is_crates_open = state.expanded_nodes.contains("crates_dir");
            let is_widgets_open = state.expanded_nodes.contains("widgets_dir");
            let is_examples_open = state.expanded_nodes.contains("examples_dir");

            tree_nodes.push((
                "crates_dir",
                "crates/",
                0,
                true,
                is_crates_open,
                state.selected_tree_node.as_deref() == Some("crates_dir"),
            ));
            if is_crates_open {
                tree_nodes.push((
                    "core_rs",
                    "ui-core",
                    1,
                    false,
                    false,
                    state.selected_tree_node.as_deref() == Some("core_rs"),
                ));
                tree_nodes.push((
                    "layout_rs",
                    "ui-layout",
                    1,
                    false,
                    false,
                    state.selected_tree_node.as_deref() == Some("layout_rs"),
                ));
                tree_nodes.push((
                    "widgets_dir",
                    "ui-widgets/",
                    1,
                    true,
                    is_widgets_open,
                    state.selected_tree_node.as_deref() == Some("widgets_dir"),
                ));
                if is_widgets_open {
                    tree_nodes.push((
                        "tree_rs",
                        "tree.rs",
                        2,
                        false,
                        false,
                        state.selected_tree_node.as_deref() == Some("tree_rs"),
                    ));
                    tree_nodes.push((
                        "frame_rs",
                        "frame.rs",
                        2,
                        false,
                        false,
                        state.selected_tree_node.as_deref() == Some("frame_rs"),
                    ));
                    tree_nodes.push((
                        "kind_rs",
                        "kind.rs",
                        2,
                        false,
                        false,
                        state.selected_tree_node.as_deref() == Some("kind_rs"),
                    ));
                }
                tree_nodes.push((
                    "gpu_rs",
                    "ui-gpu",
                    1,
                    false,
                    false,
                    state.selected_tree_node.as_deref() == Some("gpu_rs"),
                ));
            }
            tree_nodes.push((
                "examples_dir",
                "examples/",
                0,
                true,
                is_examples_open,
                state.selected_tree_node.as_deref() == Some("examples_dir"),
            ));
            if is_examples_open {
                tree_nodes.push((
                    "gallery_rs",
                    "widget_gallery.rs",
                    1,
                    false,
                    false,
                    state.selected_tree_node.as_deref() == Some("gallery_rs"),
                ));
            }
            tree_nodes.push((
                "cargo_toml",
                "Cargo.toml",
                0,
                false,
                false,
                state.selected_tree_node.as_deref() == Some("cargo_toml"),
            ));

            let split_w = content_w - 28.0;
            let left_w = (split_w * state.split_ratio).clamp(140.0, split_w - 200.0);
            let right_w = (split_w - left_w - 6.0).max(200.0);
            let canvas_w = (right_w - 20.0).max(160.0);
            let canvas_h = 210.0;

            let tree_list = tree
                .tree_view(
                    "studio_tree",
                    &tree_nodes,
                    leaf(left_w - 8.0, 22.0),
                    column(2.0),
                )
                .unwrap();

            // Right panel: 2D CustomPaint Canvas Workspace
            let mut painter = ui_widgets::Painter::new();

            // 1. Blueprint Grid Background
            let grid_step = 24.0;
            let mut gx = grid_step;
            while gx < canvas_w {
                painter.line([gx, 0.0], [gx, canvas_h], 0.8, [0.0, 0.85, 1.0, 0.06]);
                gx += grid_step;
            }
            let mut gy = grid_step;
            while gy < canvas_h {
                painter.line([0.0, gy], [canvas_w, gy], 0.8, [0.0, 0.85, 1.0, 0.06]);
                gy += grid_step;
            }

            // 2. Demo Bézier Spline Cable
            painter.bezier(
                [14.0, 36.0],
                [canvas_w * 0.35, 12.0],
                [canvas_w * 0.65, canvas_h - 12.0],
                [canvas_w - 18.0, canvas_h - 40.0],
                2.5,
                [0.0, 0.85, 1.0, 0.7],
            );
            painter.circle([14.0, 36.0], 4.5, Some([0.0, 0.85, 1.0, 1.0]), None);
            painter.circle(
                [canvas_w - 18.0, canvas_h - 40.0],
                4.5,
                Some([0.55, 0.36, 0.96, 1.0]),
                None,
            );

            // 3. Parametric Waveform Polyline
            let wave_pts: Vec<[f32; 2]> = (0..20)
                .map(|i| {
                    let t = i as f32 / 19.0;
                    let x = 14.0 + t * (canvas_w - 28.0);
                    let y = (canvas_h * 0.5) + (t * std::f32::consts::PI * 4.0).sin() * 18.0;
                    [x, y]
                })
                .collect();
            painter.polyline(wave_pts, 1.5, [0.06, 0.72, 0.51, 0.5], false);

            // 4. Stored user freehand vector strokes
            for (pts, col, w) in &state.canvas_strokes {
                if pts.len() >= 2 {
                    painter.polyline(pts.clone(), *w, *col, false);
                } else if !pts.is_empty() {
                    painter.circle(pts[0], *w * 0.5, Some(*col), None);
                }
            }

            // 5. Active drawing stroke
            if !state.canvas_active_stroke.is_empty() {
                let stroke_w = (state.brush_intensity * 0.05).clamp(1.5, 12.0);
                if state.canvas_active_stroke.len() >= 2 {
                    painter.polyline(
                        state.canvas_active_stroke.clone(),
                        stroke_w,
                        state.selected_color,
                        false,
                    );
                } else {
                    painter.circle(
                        state.canvas_active_stroke[0],
                        stroke_w * 0.5,
                        Some(state.selected_color),
                        None,
                    );
                }
            }

            // 6. HUD Text Overlay
            painter.text(
                [8.0, 12.0],
                format!("CANVAS 2D | {} STROKES", state.canvas_strokes.len()),
                10.0,
                [0.0, 0.85, 1.0, 0.8],
            );
            painter.text(
                [8.0, canvas_h - 14.0],
                format!(
                    "Tool: {} | Intensity: {:.0}%",
                    state.active_tool.to_uppercase(),
                    state.brush_intensity
                ),
                10.0,
                [0.6, 0.7, 0.8, 0.7],
            );

            let canvas_widget = tree
                .custom_paint("studio_canvas", painter.finish(), leaf(canvas_w, canvas_h))
                .unwrap();

            let clear_btn = tree
                .button(
                    WidgetId::new("clear_canvas_btn"),
                    "Clear Canvas",
                    true,
                    leaf(96.0, 24.0),
                )
                .unwrap();
            let tool_lbl = tree
                .label_muted(
                    format!("Tool: {} (Draw/Erase)", state.active_tool.to_uppercase()),
                    leaf(canvas_w - 106.0, 24.0),
                )
                .unwrap();
            let toolbar_row = tree.container(&[clear_btn, tool_lbl], row(8.0)).unwrap();

            let inspector_content = tree
                .container(&[canvas_widget, toolbar_row], column(6.0))
                .unwrap();

            let left_panel_style = Style {
                size: Size {
                    width: length(left_w),
                    height: length(256.0),
                },
                padding: Rect {
                    left: length(2.0),
                    right: length(6.0),
                    top: length(0.0),
                    bottom: length(0.0),
                },
                ..Default::default()
            };
            let left_panel = tree.container(&[tree_list], left_panel_style).unwrap();

            let right_panel_style = Style {
                size: Size {
                    width: length(right_w),
                    height: length(256.0),
                },
                padding: Rect {
                    left: length(16.0),
                    right: length(4.0),
                    top: length(0.0),
                    bottom: length(0.0),
                },
                ..Default::default()
            };
            let right_panel = tree
                .container(&[inspector_content], right_panel_style)
                .unwrap();

            let split = tree
                .split_view(
                    "studio_split",
                    ui_widgets::SplitOrientation::Horizontal,
                    state.split_ratio,
                    left_panel,
                    right_panel,
                    leaf(split_w, 256.0),
                )
                .unwrap();

            let div_studio = tree.divider(false, leaf(split_w, 1.0)).unwrap();

            let studio_hint = tree
                .label_muted(
                    "Click & drag to draw on Canvas | Use Tool Palette [Ctrl+P] to switch Pencil/Eraser",
                    leaf(split_w, 16.0),
                )
                .unwrap();

            let studio_card = tree
                .card(
                    &[
                        studio_title,
                        studio_sub,
                        split,
                        div_studio,
                        studio_hint,
                    ],
                    None,
                    None,
                    Some(8.0),
                    card_style(content_w),
                )
                .unwrap();

            tree.container(&[studio_card], column(6.0)).unwrap()
        }
        5 => {
            // --- Tab 5: Media (2 Grounded Cyber-Glass Workstation Cards) ---
            // Left Card: GPU Video Stream & FFT Spectrum
            let vid_badge = tree
                .badge(
                    if state.video_playing {
                        "STREAMING (60 FPS)"
                    } else {
                        "PAUSED"
                    },
                    if state.video_playing {
                        ui_widgets::ListItemBadge::Success
                    } else {
                        ui_widgets::ListItemBadge::Warning
                    },
                    leaf(140.0, 22.0),
                )
                .unwrap();
            let vid_title = tree
                .label("Dynamic Plasma Stream:", leaf(half_col_w - 178.0, 22.0))
                .unwrap();
            let vid_header = tree.container(&[vid_title, vid_badge], row(8.0)).unwrap();

            let video_widget = tree
                .video_player(
                    "stream_video",
                    "stream_video",
                    state.video_playing,
                    state.video_progress,
                    90.0,
                    1.0,
                    leaf(half_col_w - 28.0, 160.0),
                )
                .unwrap();

            let div_m1 = tree.divider(false, leaf(half_col_w - 28.0, 1.0)).unwrap();

            let audio_title = tree
                .label(
                    "Audio Spectrum Visualizer (32 Band FFT Neon Glass):",
                    leaf(half_col_w - 28.0, 18.0),
                )
                .unwrap();
            let visualizer_widget = tree
                .audio_visualizer("audio_bars", &state.audio_spectrum, 1.0, leaf(half_col_w - 28.0, 48.0))
                .unwrap();

            let card_left = tree
                .card(
                    &[
                        vid_header,
                        video_widget,
                        div_m1,
                        audio_title,
                        visualizer_widget,
                    ],
                    None,
                    None,
                    Some(8.0),
                    card_style(half_col_w),
                )
                .unwrap();

            // Right Card: Static Artwork & OS Drag-and-Drop Ingest
            let fit_label = tree
                .label("Artwork Aspect Fitting:", leaf(160.0, 24.0))
                .unwrap();
            let fit_options = ["Cover", "Contain", "Fill"];
            let fit_idx = match state.media_fit_mode {
                MediaFit::Cover => 0,
                MediaFit::Contain => 1,
                MediaFit::Fill => 2,
            };
            let fit_selector = tree
                .segmented_control(
                    WidgetId::new("media_fit_selector"),
                    &fit_options,
                    fit_idx,
                    leaf(half_col_w - 198.0, 24.0),
                    row(4.0),
                )
                .unwrap();
            let fit_header = tree
                .container(&[fit_label, fit_selector], row(8.0))
                .unwrap();

            let image_widget = tree
                .image(
                    "cyber_art",
                    "cyber_art",
                    state.media_fit_mode,
                    leaf(half_col_w - 28.0, 120.0),
                )
                .unwrap();

            let div_m2 = tree.divider(false, leaf(half_col_w - 28.0, 1.0)).unwrap();

            let drop_zone_widget = tree
                .drop_zone(
                    "media_drop_zone",
                    if state.file_hovered {
                        "Release Image or File to Import..."
                    } else if state.dropped_file_info.is_some() {
                        "File Loaded into Pipeline (Drop to Replace)"
                    } else {
                        "Drag & Drop Images / Files Directly onto Window"
                    },
                    Some("Accepts PNG, JPG, JPEG, WebP, BMP, RS, WGSL, JSON"),
                    &["png", "jpg", "webp", "rs", "wgsl"],
                    state.file_hovered,
                    state.dropped_file_info.as_ref().map(|(n, s)| (n.as_str(), *s)),
                    leaf(half_col_w - 28.0, 80.0),
                )
                .unwrap();

            let card_right = tree
                .card(
                    &[
                        fit_header,
                        image_widget,
                        div_m2,
                        drop_zone_widget,
                    ],
                    None,
                    None,
                    Some(8.0),
                    card_style(half_col_w),
                )
                .unwrap();

            tree.container(&[card_left, card_right], row(16.0)).unwrap()
        }
        _ => {
            // --- Tab 6: Nodes & Data (DSP Parameters, GPU Dataviz & Node Graph Cards) ---
            // Top Card: DSP Parameter Matrix & Compute Telemetry
            let top_card_title = tree
                .label("DSP Parameter Matrix & Compute Telemetry", leaf(content_w - 28.0, 18.0))
                .unwrap();
            let top_card_sub = tree
                .label_muted("Rotary faders, dynamic filter tags and realtime GPU compute load dataviz", leaf(content_w - 28.0, 14.0))
                .unwrap();

            let knob1 = tree
                .knob(
                    "knob_gain",
                    state.knob_gain,
                    0.0,
                    100.0,
                    0.5,
                    Some("Gain"),
                    Some("dB"),
                    leaf(70.0, 72.0),
                )
                .unwrap();
            let knob2 = tree
                .knob(
                    "knob_freq",
                    state.knob_freq,
                    20.0,
                    20000.0,
                    10.0,
                    Some("Cutoff"),
                    Some("Hz"),
                    leaf(70.0, 72.0),
                )
                .unwrap();
            let knob3 = tree
                .knob(
                    "knob_mix",
                    state.knob_mix,
                    0.0,
                    100.0,
                    1.0,
                    Some("Mix"),
                    Some("%"),
                    leaf(70.0, 72.0),
                )
                .unwrap();

            let tags_widget = tree
                .tag_input(
                    "gallery_tags",
                    &state.tags_list,
                    "Add filter tag...",
                    None,
                    leaf(content_w - 360.0, 32.0),
                )
                .unwrap();

            let add_tag_btn = tree
                .button(
                    WidgetId::new("add_tag_btn"),
                    "+ Tag",
                    true,
                    leaf(60.0, 30.0),
                )
                .unwrap();

            let tags_group = tree.container(&[tags_widget, add_tag_btn], row(6.0)).unwrap();
            let knobs_group = tree.container(&[knob1, knob2, knob3], row(8.0)).unwrap();
            let top_row = tree.container(&[knobs_group, tags_group], row(16.0)).unwrap();

            let div_n1 = tree.divider(false, leaf(content_w - 28.0, 1.0)).unwrap();

            let series1_pts: Vec<[f32; 2]> = (0..20)
                .map(|i| {
                    let x = i as f32;
                    let y = 35.0 + (x * 0.4).sin() * 25.0 + (x * 0.9).cos() * 12.0;
                    [x, y.clamp(5.0, 95.0)]
                })
                .collect();
            let series2_pts: Vec<[f32; 2]> = (0..20)
                .map(|i| {
                    let x = i as f32;
                    let y = 60.0 + (x * 0.3 + 1.2).cos() * 20.0;
                    [x, y.clamp(5.0, 95.0)]
                })
                .collect();

            let chart_series = vec![
                ui_widgets::ChartSeries {
                    name: "GPU Compute Load".to_string(),
                    color: [0.0, 0.85, 1.0, 1.0],
                    points: series1_pts,
                    filled: true,
                },
                ui_widgets::ChartSeries {
                    name: "Frame Latency (ms)".to_string(),
                    color: [0.75, 0.35, 0.95, 1.0],
                    points: series2_pts,
                    filled: false,
                },
            ];

            let chart_widget = tree
                .time_series_chart(
                    "telemetry_chart",
                    Some("Realtime GPU Compute & Frame Latency (Area / Spline Dataviz)"),
                    chart_series,
                    (0.0, 19.0),
                    (0.0, 100.0),
                    true,
                    true,
                    state.chart_inspected,
                    leaf(content_w - 28.0, 100.0),
                )
                .unwrap();

            let top_card = tree
                .card(
                    &[
                        top_card_title,
                        top_card_sub,
                        top_row,
                        div_n1,
                        chart_widget,
                    ],
                    None,
                    None,
                    Some(8.0),
                    card_style(content_w),
                )
                .unwrap();

            // Bottom Card: Distributed Workflow Node Graph
            let bottom_card_title = tree
                .label("Execution Pipeline & Node Graph Topology", leaf(content_w - 28.0, 18.0))
                .unwrap();
            let bottom_card_sub = tree
                .label_muted("Interactive node graph editor with live connection wires and pin routing", leaf(content_w - 28.0, 14.0))
                .unwrap();

            let node_graph_widget = tree
                .node_graph(
                    "gallery_node_graph",
                    state.node_graph_nodes.clone(),
                    state.node_graph_connections.clone(),
                    [0.0, 0.0],
                    1.0,
                    None,
                    leaf(content_w - 28.0, 140.0),
                )
                .unwrap();

            let div_n2 = tree.divider(false, leaf(content_w - 28.0, 1.0)).unwrap();

            let hint_label = tree
                .label_muted(
                    "Click nodes to select | Click data points on chart to inspect | Rotate knobs",
                    leaf(content_w - 28.0, 14.0),
                )
                .unwrap();

            let bottom_card = tree
                .card(
                    &[
                        bottom_card_title,
                        bottom_card_sub,
                        node_graph_widget,
                        div_n2,
                        hint_label,
                    ],
                    None,
                    None,
                    Some(8.0),
                    card_style(content_w),
                )
                .unwrap();

            tree.container(
                &[top_card, bottom_card],
                column(12.0),
            )
            .unwrap()
        }
    };

    let main_content = tree
        .container(
            &[
                top_header,
                top_divider,
                tabbar,
                tab_content,
            ],
            window_content(8.0),
        )
        .unwrap();
    let win_w = (width - WINDOW_MARGIN * 2.0).max(100.0);
    let win_h = (height - WINDOW_MARGIN * 2.0).max(100.0);
    let win_style = Style {
        position: ui_layout::Position::Absolute,
        inset: Rect {
            top: length(WINDOW_MARGIN),
            left: length(WINDOW_MARGIN),
            right: ui_layout::auto(),
            bottom: ui_layout::auto(),
        },
        size: Size {
            width: length(win_w),
            height: length(win_h),
        },
        ..Default::default()
    };
    let window_card = tree
        .window(
            WidgetId::new("main_window"),
            "AORUI — An Other Rust UI",
            &[main_content],
            win_style,
        )
        .unwrap();
    tree.container(&[window_card], leaf(width, height)).unwrap()
}

fn build_modal_ui(
    tree: &mut WidgetTree,
    state: &DemoState,
    width: f32,
    height: f32,
) -> ui_layout::NodeId {
    let modal_title_div = tree.divider(false, leaf(392.0, 1.0)).unwrap();
    let modal_msg1 = tree
        .label(
            "Do you want to reset cluster network parameters",
            leaf(392.0, 20.0),
        )
        .unwrap();
    let modal_msg2 = tree
        .label_muted(
            "and trigger a full heuristic cluster diagnostic?",
            leaf(392.0, 18.0),
        )
        .unwrap();
    let cancel_btn = tree
        .button(
            WidgetId::new("modal_cancel_btn"),
            "Cancel",
            true,
            leaf(110.0, 32.0),
        )
        .unwrap();
    let confirm_btn = tree
        .button_variant(
            WidgetId::new("modal_confirm_btn"),
            "Confirm",
            ui_widgets::ButtonVariant::Primary,
            true,
            leaf(120.0, 32.0),
        )
        .unwrap();
    let modal_btns = tree
        .container(
            &[cancel_btn, confirm_btn],
            Style {
                flex_direction: FlexDirection::Row,
                justify_content: Some(ui_layout::JustifyContent::FlexEnd),
                size: Size {
                    width: length(392.0),
                    height: length(34.0),
                },
                gap: Size {
                    width: length(10.0),
                    height: length(0.0),
                },
                ..Default::default()
            },
        )
        .unwrap();

    let modal_dialog_content = tree
        .container(
            &[modal_title_div, modal_msg1, modal_msg2, modal_btns],
            Style {
                flex_direction: FlexDirection::Column,
                justify_content: Some(ui_layout::JustifyContent::SpaceBetween),
                size: Size {
                    width: length(392.0),
                    height: length(146.0),
                },
                ..Default::default()
            },
        )
        .unwrap();

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
        size: Size {
            width: length(dialog_w),
            height: length(dialog_h),
        },
        padding: Rect {
            left: length(24.0),
            right: length(24.0),
            top: length(44.0),
            bottom: length(20.0),
        },
        ..Default::default()
    };

    tree.modal_dialog(
        WidgetId::new("demo_modal"),
        "Confirmation Required",
        &[modal_dialog_content],
        dialog_style,
    )
    .unwrap()
}

/// Resolves interactive tooltips and optional keyboard shortcut hints for gallery widgets.
fn get_tooltip_for_widget(widget_id: &str) -> Option<(&'static str, Option<&'static str>)> {
    match widget_id {
        "open_cmd_palette_btn" => Some(("Command Palette & Spotlight Launcher", Some("Ctrl+K"))),
        "open_notifications_btn" => Some(("Notification Center History & Drawer", Some("Ctrl+N"))),
        "user_profile_avatar" => Some(("User Presence Status [Click to Cycle]", None)),
        "open_modal_btn" => Some(("Open Confirmation Modal Dialog", None)),
        "toast_btn" => Some(("Trigger Alert Toast Notification", None)),
        "hello_button" => Some(("Execute Primary System Action", None)),
        "turbo_toggle" => Some(("Toggle Turbo Hardware Acceleration", None)),
        "firewall_toggle" => Some(("Toggle Cybernetic Shield Mesh", None)),
        "spinners_toggle" => Some(("Lock / Unlock Numeric Stepper Inputs", None)),
        "media_drop_zone" => Some(("Drag & Drop image or code files", None)),
        "tool_select" => Some(("Pointer Selection Tool", Some("V"))),
        "tool_brush" => Some(("Digital Paint Brush", Some("B"))),
        "tool_eraser" => Some(("Vector Stroke Eraser", Some("E"))),
        "tool_picker" => Some(("Color Eyedropper Sampler", Some("I"))),
        "tool_gradient" => Some(("Linear Gradient Tool", Some("G"))),
        "concurrency_spin" => Some(("Worker Threads Count (1 to 64)", None)),
        "port_spin" => Some(("Inbound Port (1024-65535)", None)),
        "knob_gain" => Some(("Preamp Input Gain Level", None)),
        "knob_freq" => Some(("Cutoff Frequency (Hz)", None)),
        "knob_mix" => Some(("Dry / Wet DSP Mix Ratio", None)),
        "add_tag_btn" => Some(("Add Telemetry Filter Tag", None)),
        "lock_btn" => Some(("Lock Inbound Network Endpoints", None)),
        "shield_btn" => Some(("Deploy Defensive Security Mesh", None)),
        "refresh_btn" => Some(("Synchronize Cluster Node Telemetry", None)),
        "clear_canvas_btn" => Some(("Clear Vector Drawing Canvas", None)),
        "cyan_swatch" => Some(("Electric Cyan Preset", None)),
        "purple_swatch" => Some(("Neon Violet Preset", None)),
        "green_swatch" => Some(("Emerald Matrix Preset", None)),
        "amber_swatch" => Some(("Amber Warning Preset", None)),
        "master_token_pwd" => Some(("Security Access Token [Click eye to reveal]", None)),
        "global_search" => Some(("Global Module & Metric Search Filter", None)),
        "settings_tabs" => Some(("Switch Workstation Category Tabs", None)),
        "pipeline_stepper" => Some(("Interactive Workflow Progress Stepper", None)),
        "chip_gpu" => Some(("WGPU Graphics & Compute Pipeline Filter", None)),
        "chip_shader" => Some(("WGSL Shader Compilation Pipeline Filter", None)),
        "chip_shield" => Some(("Zero-Trust Containment Filter", None)),
        "chip_auth" => Some(("TLS 1.3 Cryptographic Token Filter", None)),
        "memory_alloc_bar" => Some(("Multi-Segment Proportional Memory Allocation", None)),
        "demo_star_rating" => Some(("Cluster Performance Rating [Click to Score]", None)),
        "audit_timeline" => Some(("Chronological Audit & Incident Event Log", None)),
        _ => None,
    }
}

fn build_popover_ui(
    tree: &mut WidgetTree,
    state: &DemoState,
    width: f32,
    height: f32,
    hovered_widget: Option<&str>,
    cursor_pos: (f32, f32),
) -> Option<ui_layout::NodeId> {
    let mut overlay_nodes = Vec::new();

    // 1. Floating Creative Tools Palette (Photoshop-like floating sub-window)
    if state.show_tools_palette {
        let content_node = if !state.tools_palette_folded {
            let t1 = tree
                .button(
                    WidgetId::new("tool_select"),
                    "↖ Select",
                    state.active_tool == "select",
                    leaf(60.0, 26.0),
                )
                .unwrap();
            let t2 = tree
                .button(
                    WidgetId::new("tool_brush"),
                    "🖌 Brush",
                    state.active_tool == "brush",
                    leaf(60.0, 26.0),
                )
                .unwrap();
            let t3 = tree
                .button(
                    WidgetId::new("tool_eraser"),
                    "⌫ Eraser",
                    state.active_tool == "eraser",
                    leaf(60.0, 26.0),
                )
                .unwrap();
            let t4 = tree
                .button(
                    WidgetId::new("tool_picker"),
                    "⌖ Sample",
                    state.active_tool == "picker",
                    leaf(60.0, 26.0),
                )
                .unwrap();
            let t5 = tree
                .button(
                    WidgetId::new("tool_gradient"),
                    "◩ Grad",
                    state.active_tool == "gradient",
                    leaf(60.0, 26.0),
                )
                .unwrap();
            let t6 = tree
                .button(
                    WidgetId::new("tool_shapes"),
                    "⬡ Shape",
                    state.active_tool == "shapes",
                    leaf(60.0, 26.0),
                )
                .unwrap();
            let t7 = tree
                .button(
                    WidgetId::new("tool_text"),
                    "T Type",
                    state.active_tool == "text",
                    leaf(60.0, 26.0),
                )
                .unwrap();
            let t8 = tree
                .button(
                    WidgetId::new("tool_bucket"),
                    "🪣 Fill",
                    state.active_tool == "bucket",
                    leaf(60.0, 26.0),
                )
                .unwrap();

            let grid_w = (state.tools_palette_size.0 - 12.0).max(120.0);
            let tools_grid = tree
                .grid(
                    2,
                    4.0,
                    4.0,
                    &[t1, t2, t3, t4, t5, t6, t7, t8],
                    leaf(grid_w, 120.0),
                )
                .unwrap();
            Some(tools_grid)
        } else {
            None
        };

        let pal_h = if state.tools_palette_folded {
            28.0
        } else {
            state.tools_palette_size.1
        };
        let tools_palette_node = tree
            .palette(
                "tools_palette",
                "Tools",
                state.tools_palette_folded,
                content_node,
                palette_style(
                    state.tools_palette_pos.0,
                    state.tools_palette_pos.1,
                    state.tools_palette_size.0,
                    pal_h,
                ),
            )
            .unwrap();
        overlay_nodes.push(tools_palette_node);
    }

    // 2. Floating Inspector Palette (Blend mode, Opacity & Properties)
    if state.show_inspector_palette {
        let content_node = if !state.inspector_palette_folded {
            let active_tool_lbl = tree
                .label(
                    format!("Tool: {}", state.active_tool.to_uppercase()),
                    leaf(140.0, 18.0),
                )
                .unwrap();
            let opacity_lbl = tree
                .label_muted(
                    format!("Intensity: {:.0}%", state.brush_intensity),
                    leaf(140.0, 16.0),
                )
                .unwrap();
            let insp_slider = tree
                .slider(
                    WidgetId::new("palette_intensity_slider"),
                    0.0,
                    100.0,
                    state.brush_intensity,
                    leaf(140.0, 16.0),
                )
                .unwrap();
            let insp_btn = tree
                .button(
                    WidgetId::new("palette_preset_btn"),
                    "Apply Blend",
                    true,
                    leaf(140.0, 26.0),
                )
                .unwrap();
            let insp_box = tree
                .container(
                    &[active_tool_lbl, opacity_lbl, insp_slider, insp_btn],
                    column(6.0),
                )
                .unwrap();
            Some(insp_box)
        } else {
            None
        };

        let pal_h = if state.inspector_palette_folded {
            28.0
        } else {
            state.inspector_palette_size.1
        };
        let insp_palette_node = tree
            .palette(
                "inspector_palette",
                "Inspector",
                state.inspector_palette_folded,
                content_node,
                palette_style(
                    state.inspector_palette_pos.0,
                    state.inspector_palette_pos.1,
                    state.inspector_palette_size.0,
                    pal_h,
                ),
            )
            .unwrap();
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
                    (
                        "show_toast_btn",
                        "Trigger Alert Toast",
                        Some("Ctrl+T"),
                        true,
                    ),
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
                    (
                        "toggle_tools_palette",
                        "Toggle Tools Palette",
                        Some("F2"),
                        true,
                    ),
                    (
                        "toggle_insp_palette",
                        "Toggle Inspector Palette",
                        Some("F3"),
                        true,
                    ),
                    ("tab_0", "General Tab", Some("Ctrl+1"), true),
                    ("tab_1", "Security Tab", Some("Ctrl+2"), true),
                    ("tab_2", "Network Tab", Some("Ctrl+3"), true),
                    ("tab_3", "Studio Tab", Some("Ctrl+4"), true),
                    ("tab_4", "Media Tab", Some("Ctrl+5"), true),
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
                popover_style(x + WINDOW_MARGIN, 108.0 + WINDOW_MARGIN, 210.0, popover_h),
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
                popover_style(146.0 + WINDOW_MARGIN, 554.0 + WINDOW_MARGIN, 378.0, 94.0),
            )
            .unwrap();
        overlay_nodes.push(popover);
    }

    // 5. Floating Toast if active
    if let Some((title, msg, kind)) = &state.active_toast {
        let msg_len = msg.len();
        let toast_w = (320.0 + (msg_len as f32 * 1.2)).clamp(320.0, 480.0);
        let toast_h = if msg_len > 70 {
            76.0
        } else if msg_len > 35 {
            68.0
        } else {
            58.0
        };
        let toast_node = tree
            .toast(
                WidgetId::new("active_toast"),
                title,
                msg,
                *kind,
                toast_style(
                    width - toast_w - 20.0 - WINDOW_MARGIN,
                    height - toast_h - 16.0 - WINDOW_MARGIN,
                    toast_w,
                    toast_h,
                ),
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

    // 7. Command Palette Modal Dialog [Ctrl+K]
    if state.show_command_palette {
        let cmd_items = [
            ("cmd_tab_0", "General Dashboard", "Ctrl+1", ui_widgets::IconKind::Cpu),
            ("cmd_tab_1", "UI Primitives & Blocks", "Ctrl+2", ui_widgets::IconKind::FileCode),
            ("cmd_tab_2", "Security & Firewall", "Ctrl+3", ui_widgets::IconKind::Lock),
            ("cmd_tab_3", "Cluster Telemetry", "Ctrl+4", ui_widgets::IconKind::Network),
            ("cmd_tab_4", "Creative Studio", "Ctrl+5", ui_widgets::IconKind::Folder),
            ("cmd_tab_5", "Media & Video Stream", "Ctrl+6", ui_widgets::IconKind::Play),
            ("cmd_tab_6", "Vector Node Graph", "Ctrl+7", ui_widgets::IconKind::Settings),
            ("cmd_turbo", "Toggle Cyber Turbo Mode", "Turbo", ui_widgets::IconKind::Refresh),
            ("cmd_scan", "Run Deep Diagnostic Scan", "F5", ui_widgets::IconKind::Shield),
            ("cmd_notifications", "Toggle Notification Center", "Ctrl+N", ui_widgets::IconKind::Alert),
            ("cmd_status_online", "Set Presence to Online", "Online", ui_widgets::IconKind::Check),
            ("cmd_status_busy", "Set Presence to Busy (DND)", "Busy", ui_widgets::IconKind::Minus),
        ];

        let query = state.command_palette_search.text.to_lowercase();
        let filtered: Vec<_> = cmd_items
            .iter()
            .filter(|(_, label, shortcut, _)| {
                query.is_empty()
                    || label.to_lowercase().contains(&query)
                    || shortcut.to_lowercase().contains(&query)
            })
            .collect();

        let pal_w = 580.0f32;
        let pal_h = 430.0f32;
        let pal_x = ((width - pal_w) * 0.5).max(10.0);
        let pal_y = ((height - pal_h) * 0.32).max(20.0);

        let search_focused = state.focused_input.as_deref() == Some("command_palette_search");
        let search_icon = tree
            .icon(ui_widgets::IconKind::Search, 15.0, Some([0.95, 0.95, 0.95, 1.0]), leaf(22.0, 32.0))
            .unwrap();
        let search_input = tree
            .text_input_with_cursor(
                WidgetId::new("command_palette_search"),
                &state.command_palette_search.text,
                "Type a command or search... [Esc to close]",
                search_focused,
                state.command_palette_search.cursor,
                state.command_palette_search.selection,
                leaf(pal_w - 96.0, 32.0),
            )
            .unwrap();
        let esc_badge = tree.kbd("Esc", leaf(34.0, 22.0)).unwrap();
        let search_bar = tree
            .container(&[search_icon, search_input, esc_badge], row(6.0))
            .unwrap();

        let div = tree.divider(false, leaf(pal_w - 24.0, 1.0)).unwrap();

        let mut item_nodes = Vec::new();
        for (idx, (id, label, shortcut, icon_kind)) in filtered.iter().enumerate() {
            let is_selected = idx == state.command_palette_selected;
            let variant = if is_selected {
                ui_widgets::ButtonVariant::Primary
            } else {
                ui_widgets::ButtonVariant::Default
            };
            let glyph = icon_kind.glyph();
            let item_btn = tree
                .button_variant(
                    WidgetId::new(format!("cmd_action_{}", id)),
                    format!("{}  {:<36}  [{}]", glyph, label, shortcut),
                    variant,
                    true,
                    leaf(pal_w - 24.0, 26.0),
                )
                .unwrap();
            item_nodes.push(item_btn);
        }

        let list_container = tree
            .container(&item_nodes, column(3.0))
            .unwrap();

        let footer_left = tree
            .label_muted("↑↓ to navigate   •   ↵ to execute   •   Esc to dismiss", leaf(pal_w - 180.0, 18.0))
            .unwrap();
        let footer_right = tree
            .label_muted(format!("{} commands", filtered.len()), leaf(140.0, 18.0))
            .unwrap();
        let footer_row = tree
            .container(&[footer_left, footer_right], row(6.0))
            .unwrap();

        let palette_content = tree
            .container(&[search_bar, div, list_container, footer_row], column(6.0))
            .unwrap();

        let palette_dialog = tree
            .modal_dialog(
                WidgetId::new("command_palette_dialog"),
                "",
                &[palette_content],
                popover_style(pal_x, pal_y, pal_w, pal_h),
            )
            .unwrap();

        overlay_nodes.push(palette_dialog);
    }

    // 8. Notifications Center Drawer / Flyout Tray [Ctrl+N]
    if state.show_notifications_drawer {
        let drawer_w = 340.0;
        let drawer_h = (height - 60.0 - WINDOW_MARGIN * 2.0).max(220.0);

        let bell_icon = tree
            .icon(ui_widgets::IconKind::Shield, 14.0, Some([0.95, 0.95, 0.95, 1.0]), leaf(18.0, 22.0))
            .unwrap();
        let hdr_title = tree
            .label("Notification Center", leaf(140.0, 22.0))
            .unwrap();
        let clear_btn = tree
            .button(
                WidgetId::new("clear_notifications_btn"),
                "Clear All",
                true,
                leaf(72.0, 22.0),
            )
            .unwrap();
        let close_drawer_btn = tree
            .button(
                WidgetId::new("close_drawer_btn"),
                "✕",
                true,
                leaf(26.0, 22.0),
            )
            .unwrap();
        let hdr_row = tree
            .container(&[bell_icon, hdr_title, clear_btn, close_drawer_btn], row(6.0))
            .unwrap();
        let hdr_div = tree.divider(false, leaf(drawer_w - 24.0, 1.0)).unwrap();

        let mut notif_nodes = Vec::new();
        if state.notifications_history.is_empty() {
            let empty_lbl = tree
                .label_muted("No notifications. All systems operational.", leaf(drawer_w - 32.0, 30.0))
                .unwrap();
            notif_nodes.push(empty_lbl);
        } else {
            for (title, msg, kind, time) in state.notifications_history.iter().take(6) {
                let badge_text = match kind {
                    ToastKind::Success => "✓ OK",
                    ToastKind::Warning => "▲ WARN",
                    ToastKind::Error => "✕ ERR",
                    ToastKind::Info => "≡ INFO",
                };
                let t_lbl = tree
                    .label(
                        format!("{} [{}] • {}", title, badge_text, time),
                        leaf(drawer_w - 36.0, 16.0),
                    )
                    .unwrap();
                let m_lbl = tree
                    .label_muted(msg.as_str(), leaf(drawer_w - 36.0, 14.0))
                    .unwrap();
                let item_card = tree
                    .container(&[t_lbl, m_lbl], column(2.0))
                    .unwrap();
                notif_nodes.push(item_card);
            }
        }

        let notif_list = tree.container(&notif_nodes, column(6.0)).unwrap();
        let drawer_content = tree
            .container(&[hdr_row, hdr_div, notif_list], column(8.0))
            .unwrap();

        let drawer_dialog = tree
            .modal_dialog(
                WidgetId::new("notifications_drawer"),
                "",
                &[drawer_content],
                popover_style(
                    width - drawer_w - 12.0 - WINDOW_MARGIN,
                    36.0 + WINDOW_MARGIN,
                    drawer_w,
                    drawer_h,
                ),
            )
            .unwrap();
        overlay_nodes.push(drawer_dialog);
    }

    // 9. Floating Hover Tooltip Bubble (Info-bulle cyber-glass)
    if !state.show_command_palette
        && !state.show_notifications_drawer
        && state.active_menu.is_none()
        && !state.dropdown_open
        && state.context_menu.is_none()
    {
        if let Some(w_id) = hovered_widget {
            if let Some((text, shortcut)) = get_tooltip_for_widget(w_id) {
                let tip_w = if shortcut.is_some() {
                    260.0
                } else {
                    (text.len() as f32 * 7.2 + 28.0).clamp(160.0, 320.0)
                };
                let tip_h = 28.0;
                let tip_x = (cursor_pos.0 + 14.0).min(width - tip_w - 12.0);
                let tip_y = (cursor_pos.1 + 18.0).min(height - tip_h - 12.0);

                let tt = tree
                    .tooltip(
                        text,
                        shortcut,
                        ui_widgets::TooltipPlacement::Bottom,
                        popover_style(tip_x, tip_y, tip_w, tip_h),
                    )
                    .unwrap();
                overlay_nodes.push(tt);
            }
        }
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
    resources: ui_gpu::ResourceTable,
    stream_texture: Option<Arc<ui_gpu::GpuTexture>>,
    frame_count: u64,
    state: DemoState,
    tree: WidgetTree,
    root: Option<ui_layout::NodeId>,
    overlay_tree: Option<WidgetTree>,
    overlay_root: Option<ui_layout::NodeId>,
    theme: Arc<RwLock<Theme>>,
    theme_watcher: Option<ThemeWatcher>,
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
    canvas_drag: Option<String>,
    knob_drag: Option<String>,
    node_drag: Option<(String, String, [f32; 2])>,
    prev_cursor_pos: (f32, f32),
    shift_held: bool,
    ctrl_held: bool,
}

impl App {
    fn new() -> Self {
        let mut expanded_nodes = std::collections::HashSet::new();
        expanded_nodes.insert("crates_dir".to_string());
        expanded_nodes.insert("widgets_dir".to_string());

        let initial_theme = Theme::from_file("themes/aether_os.toml")
            .or_else(|_| Theme::from_file("themes/studio_pro.toml"))
            .unwrap_or_else(|_| Theme::aether_os());
        let theme = Arc::new(RwLock::new(initial_theme));

        Self {
            window: None,
            renderer: None,
            resources: ui_gpu::ResourceTable::new(),
            stream_texture: None,
            frame_count: 0,
            theme,
            theme_watcher: None,
            state: DemoState {
                accept_checked: false,
                turbo_toggle: true,
                firewall_toggle: true,
                network_intensity: 72.0,
                brush_intensity: 85.0,
                fader_low: 65.0,
                fader_mid: 82.0,
                fader_high: 48.0,
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
                spinners_enabled: true,
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
                tools_palette_pos: (24.0 + WINDOW_MARGIN, 110.0 + WINDOW_MARGIN),
                tools_palette_size: (150.0, 180.0),
                tools_palette_folded: false,
                active_tool: "brush".to_string(),
                show_inspector_palette: true,
                inspector_palette_pos: (390.0 + WINDOW_MARGIN, 110.0 + WINDOW_MARGIN),
                inspector_palette_size: (166.0, 180.0),
                inspector_palette_folded: false,
                video_playing: true,
                video_progress: 0.35,
                audio_spectrum: (0..32).map(|i| 0.3 + 0.45 * ((i as f32 * 0.4).sin().abs())).collect(),
                media_fit_mode: MediaFit::Cover,
                canvas_strokes: Vec::new(),
                canvas_active_stroke: Vec::new(),
                knob_gain: 74.5,
                knob_freq: 1250.0,
                knob_mix: 85.0,
                tags_list: vec![
                    "GPU-Render".to_string(),
                    "SDF-Pipeline".to_string(),
                    "Realtime-60FPS".to_string(),
                    "DSP-Node".to_string(),
                ],
                chart_inspected: Some((0, 8)),
                node_graph_nodes: vec![
                    ui_widgets::GraphNodeSpec {
                        id: "source_stream".to_string(),
                        title: "Signal Source".to_string(),
                        subtitle: Some("Sine 44.1kHz".to_string()),
                        pos: [20.0, 16.0],
                        size: [130.0, 68.0],
                        inputs: vec![],
                        outputs: vec![ui_widgets::GraphSocket {
                            name: "Wave Out".to_string(),
                            socket_type: ui_widgets::SocketType::Signal,
                            color: None,
                            is_output: true,
                        }],
                        header_color: Some([0.10, 0.25, 0.40, 0.95]),
                        selected: false,
                    },
                    ui_widgets::GraphNodeSpec {
                        id: "dsp_filter".to_string(),
                        title: "DSP LowPass Filter".to_string(),
                        subtitle: Some("Bessel 24dB".to_string()),
                        pos: [175.0, 16.0],
                        size: [140.0, 68.0],
                        inputs: vec![ui_widgets::GraphSocket {
                            name: "Audio In".to_string(),
                            socket_type: ui_widgets::SocketType::Signal,
                            color: None,
                            is_output: false,
                        }],
                        outputs: vec![ui_widgets::GraphSocket {
                            name: "Filtered".to_string(),
                            socket_type: ui_widgets::SocketType::Flow,
                            color: None,
                            is_output: true,
                        }],
                        header_color: Some([0.30, 0.15, 0.45, 0.95]),
                        selected: true,
                    },
                    ui_widgets::GraphNodeSpec {
                        id: "output_sink".to_string(),
                        title: "Audio Sink Output".to_string(),
                        subtitle: Some("DAC Stream".to_string()),
                        pos: [340.0, 16.0],
                        size: [135.0, 68.0],
                        inputs: vec![ui_widgets::GraphSocket {
                            name: "Master In".to_string(),
                            socket_type: ui_widgets::SocketType::Flow,
                            color: None,
                            is_output: false,
                        }],
                        outputs: vec![],
                        header_color: Some([0.12, 0.35, 0.25, 0.95]),
                        selected: false,
                    },
                ],
                node_graph_connections: vec![
                    ui_widgets::GraphConnectionSpec {
                        from_node: "source_stream".to_string(),
                        from_socket: 0,
                        to_node: "dsp_filter".to_string(),
                        to_socket: 0,
                        color: Some([0.0, 0.85, 1.0, 0.9]),
                        flow_active: true,
                    },
                    ui_widgets::GraphConnectionSpec {
                        from_node: "dsp_filter".to_string(),
                        from_socket: 0,
                        to_node: "output_sink".to_string(),
                        to_socket: 0,
                        color: Some([0.65, 0.35, 0.95, 0.9]),
                        flow_active: true,
                    },
                ],
                selected_graph_node: Some("dsp_filter".to_string()),
                file_hovered: false,
                hovered_file: None,
                dropped_file_info: None,
                show_command_palette: false,
                command_palette_search: TextEditorState::new(""),
                command_palette_selected: 0,
                show_notifications_drawer: false,
                notifications_history: vec![
                    (
                        "System Booted".to_string(),
                        "WGPU Render Pipeline active at 60 FPS".to_string(),
                        ToastKind::Success,
                        "2m ago".to_string(),
                    ),
                    (
                        "Cluster Telemetry".to_string(),
                        "4 Node clusters online (Tokyo, Paris, Frankfurt, Sydney)".to_string(),
                        ToastKind::Info,
                        "5m ago".to_string(),
                    ),
                ],
                user_avatar_status: ui_widgets::AvatarStatus::Online,
                workflow_step: 1,
                selected_chips: {
                    let mut s = std::collections::HashSet::new();
                    s.insert("chip_gpu".to_string());
                    s.insert("chip_shader".to_string());
                    s
                },
                dismissed_chips: std::collections::HashSet::new(),
                user_rating: 4,
                selected_timeline_item: Some(1),
            },
            tree: WidgetTree::new(),
            root: None,
            overlay_tree: None,
            overlay_root: None,
            cursor_pos: (0.0, 0.0),
            prev_cursor_pos: (0.0, 0.0),
            pressed: None,
            scrollbar_drag: None,
            slider_drag: None,
            splitter_drag: None,
            modal_drag: None,
            color_picker_drag: None,
            palette_drag: None,
            palette_resize: None,
            text_drag: None,
            canvas_drag: None,
            knob_drag: None,
            node_drag: None,
            shift_held: false,
            ctrl_held: false,
        }
    }

    fn redraw(&mut self) {
        if self.state.video_playing {
            self.state.video_progress = (self.state.video_progress + 0.0015).fract();
            if let (Some(renderer), Some(stream_tex)) = (&self.renderer, &self.stream_texture) {
                let plasma_bytes = generate_plasma_frame(256, 160, self.frame_count as f32 * 0.04);
                renderer.update_texture_rgba(stream_tex, &plasma_bytes);
            }
        }
        let t = self.frame_count as f32 * 0.08;
        for (i, bin) in self.state.audio_spectrum.iter_mut().enumerate() {
            let fi = i as f32;
            let wave1 = (t * 1.5 + fi * 0.5).sin();
            let wave2 = (t * 0.7 - fi * 0.3).cos();
            let amp = (wave1 * 0.5 + wave2 * 0.3 + 0.2).abs().clamp(0.05, 0.98);
            *bin = *bin * 0.7 + amp * 0.3;
        }
        self.frame_count += 1;

        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };

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

        let measure = renderer.text_measure();

        let current_theme = self.theme.read().unwrap().clone();

        let base_hovered = if !self.state.show_modal
            && self.state.active_menu.is_none()
            && !self.state.dropdown_open
        {
            base_tree
                .interaction_key_at(base_root, self.cursor_pos)
                .unwrap_or(None)
        } else {
            None
        };
        let base_interaction = InteractionState {
            hovered: base_hovered.as_ref(),
            pressed: if !self.state.show_modal {
                self.pressed.as_ref()
            } else {
                None
            },
            measure: Some(&measure),
        };

        let base_frame = match base_tree.build_frame(base_root, &current_theme, base_interaction) {
            Ok(f) => f,
            Err(err) => {
                eprintln!("[widget_gallery] base build_frame failed: {err}");
                return;
            }
        };

        let family = font_family(&current_theme.typography.family);
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
                renderer.make_text_run(
                    &spec.text,
                    spec.bounds,
                    spec.font_size,
                    spec.color,
                    align,
                    family,
                    weight,
                    spec.clip,
                )
            })
            .collect();

        let base_layer = ui_gpu::RenderLayer {
            instances: &base_frame.instances,
            texts: &base_text_runs,
        };

        let media: Vec<ui_gpu::MediaInstance> = base_frame
            .media
            .iter()
            .map(|spec| {
                let fit_mode = match spec.fit {
                    ui_widgets::MediaFit::Fill => 0,
                    ui_widgets::MediaFit::Contain => 1,
                    ui_widgets::MediaFit::Cover => 2,
                };
                let clip_bounds = spec.clip;
                ui_gpu::MediaInstance {
                    kind: spec.kind.0.to_string(),
                    resource_id: spec.resource_id.clone(),
                    bounds: spec.bounds,
                    clip_bounds,
                    radius: spec.radius,
                    fit_mode,
                }
            })
            .collect();

        let background = wgpu::Color::TRANSPARENT;

        if self.state.show_modal {
            // --- Layer 1: Modal Overlay ---
            let mut modal_tree = WidgetTree::new();
            let modal_root = build_modal_ui(&mut modal_tree, &self.state, width_f, height_f);
            if modal_tree.compute(modal_root, available).is_err() {
                return;
            }

            let modal_hovered = modal_tree
                .interaction_key_at(modal_root, self.cursor_pos)
                .unwrap_or(None);
            let modal_interaction = InteractionState {
                hovered: modal_hovered.as_ref(),
                pressed: self.pressed.as_ref(),
                measure: Some(&measure),
            };

            let modal_frame =
                match modal_tree.build_frame(modal_root, &current_theme, modal_interaction) {
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
                    renderer.make_text_run(
                        &spec.text,
                        spec.bounds,
                        spec.font_size,
                        spec.color,
                        align,
                        family,
                        weight,
                        spec.clip,
                    )
                })
                .collect();

            let modal_layer = ui_gpu::RenderLayer {
                instances: &modal_frame.instances,
                texts: &modal_text_runs,
            };

            if let Err(err) = renderer.render_layers(
                background,
                &[base_layer, modal_layer],
                &media,
                &self.resources,
            ) {
                eprintln!("[widget_gallery] modal render failed: {err}");
            }

            self.overlay_tree = Some(modal_tree);
            self.overlay_root = Some(modal_root);
        } else {
            // --- Layer 1: Popover / Palette / Toast / Dropdown / Tooltip Overlays (if active) ---
            let mut popover_tree = WidgetTree::new();
            let hovered_key = base_hovered.as_ref().map(|k| k.widget_id.as_str());
            let popover_root = build_popover_ui(
                &mut popover_tree,
                &self.state,
                width_f,
                height_f,
                hovered_key,
                self.cursor_pos,
            );

            if let Some(pop_root) = popover_root {
                if popover_tree.compute(pop_root, available).is_ok() {
                    let pop_hovered = popover_tree
                        .interaction_key_at(pop_root, self.cursor_pos)
                        .unwrap_or(None);
                    let pop_interaction = InteractionState {
                        hovered: pop_hovered.as_ref(),
                        pressed: self.pressed.as_ref(),
                        measure: Some(&measure),
                    };

                    if let Ok(pop_frame) =
                        popover_tree.build_frame(pop_root, &current_theme, pop_interaction)
                    {
                        let pop_text_runs: Vec<_> = pop_frame
                            .texts
                            .iter()
                            .map(|spec| {
                                let align = match spec.align {
                                    ui_widgets::TextAlign::Left => {
                                        glyphon::cosmic_text::Align::Left
                                    }
                                    ui_widgets::TextAlign::Center => {
                                        glyphon::cosmic_text::Align::Center
                                    }
                                    ui_widgets::TextAlign::Right => {
                                        glyphon::cosmic_text::Align::Right
                                    }
                                };
                                let weight = match spec.weight {
                                    ui_widgets::FontWeight::Normal => glyphon::Weight::NORMAL,
                                    ui_widgets::FontWeight::Bold => glyphon::Weight::BOLD,
                                };
                                renderer.make_text_run(
                                    &spec.text,
                                    spec.bounds,
                                    spec.font_size,
                                    spec.color,
                                    align,
                                    family,
                                    weight,
                                    spec.clip,
                                )
                            })
                            .collect();

                        let pop_layer = ui_gpu::RenderLayer {
                            instances: &pop_frame.instances,
                            texts: &pop_text_runs,
                        };

                        if let Err(err) = renderer.render_layers(
                            background,
                            &[base_layer, pop_layer],
                            &media,
                            &self.resources,
                        ) {
                            eprintln!("[widget_gallery] overlay render failed: {err}");
                        }
                    }
                }
                self.overlay_tree = Some(popover_tree);
                self.overlay_root = Some(pop_root);
            } else {
                if let Err(err) =
                    renderer.render_layers(background, &[base_layer], &media, &self.resources)
                {
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
                self.pressed = m_tree
                    .interaction_key_at(m_root, self.cursor_pos)
                    .unwrap_or(None);
                if m_tree
                    .is_modal_title_bar(m_root, self.cursor_pos)
                    .unwrap_or(false)
                {
                    self.modal_drag = Some((
                        self.cursor_pos.0 - self.state.modal_offset.0,
                        self.cursor_pos.1 - self.state.modal_offset.1,
                    ));
                }
            }
            return;
        }

        // Test active overlays first (palettes, dropdowns, popovers, toasts)
        if let (Some(o_tree), Some(o_root)) = (&self.overlay_tree, self.overlay_root) {
            // Check palette drag header
            if let Ok(Some((id, (ax, ay)))) = o_tree.palette_drag_anchor_at(o_root, self.cursor_pos)
            {
                self.palette_drag = Some((id, (ax, ay)));
                return;
            }
            // Check palette resize grip
            if let Ok(Some((id, (cw, ch)))) = o_tree.palette_resize_at(o_root, self.cursor_pos) {
                self.palette_resize = Some((id, self.cursor_pos, (cw, ch)));
                return;
            }

            // Check slider in overlay palette
            if let Ok(Some((id, value))) = o_tree.slider_value_at(o_root, self.cursor_pos) {
                match id.as_str() {
                    "palette_intensity_slider" => self.state.brush_intensity = value,
                    "fader_low" => self.state.fader_low = value,
                    "fader_mid" => self.state.fader_mid = value,
                    "fader_high" => self.state.fader_high = value,
                    _ => self.state.network_intensity = value,
                }
                self.slider_drag = Some(id);
                return;
            }

            if let Ok(Some(key)) = o_tree.interaction_key_at(o_root, self.cursor_pos) {
                self.pressed = Some(key);
                return;
            }
        }

        let Some(root) = self.root else { return };
        self.pressed = self
            .tree
            .interaction_key_at(root, self.cursor_pos)
            .unwrap_or(None);

        let scrollbar_key = ui_widgets::InteractionKey {
            widget_id: WidgetId::new(LIST_SCROLL_ID),
            index: None,
        };
        if self.pressed.as_ref() == Some(&scrollbar_key) {
            self.scrollbar_drag = Some((self.cursor_pos.1, self.state.list_scroll[1]));
        }

        let splitter_key = ui_widgets::InteractionKey {
            widget_id: WidgetId::new("studio_split"),
            index: None,
        };
        if self.pressed.as_ref() == Some(&splitter_key) {
            self.splitter_drag = Some("studio_split".to_string());
        }

        if let Ok(Some((id, value))) = self.tree.slider_value_at(root, self.cursor_pos) {
            match id.as_str() {
                "palette_intensity_slider" => self.state.brush_intensity = value,
                "fader_low" => self.state.fader_low = value,
                "fader_mid" => self.state.fader_mid = value,
                "fader_high" => self.state.fader_high = value,
                _ => self.state.network_intensity = value,
            }
            self.slider_drag = Some(id);
        }

        if let Ok(Some((id, color))) = self.tree.color_picker_hue_at(root, self.cursor_pos) {
            self.state.selected_color = color;
            self.color_picker_drag = Some(id);
        }

        let measure = self.renderer.as_ref().map(|r| r.text_measure());
        let measure_ref = measure.as_ref().map(|m| m as &dyn ui_widgets::TextMeasure);
        let (typo_family, typo_size) = {
            let t = self.theme.read().unwrap();
            (t.typography.family.clone(), t.typography.body_size)
        };
        if let Ok(Some((widget_id, cursor_idx))) = self.tree.text_cursor_at(
            root,
            self.cursor_pos,
            &typo_family,
            typo_size,
            measure_ref,
        ) {
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
        let canvas_key = ui_widgets::InteractionKey {
            widget_id: WidgetId::new("studio_canvas"),
            index: None,
        };
        if self.pressed.as_ref() == Some(&canvas_key) {
            self.canvas_drag = Some("studio_canvas".to_string());
            if let Ok(Some(event)) = self.tree.dispatch_click(root, self.cursor_pos) {
                self.state.apply(event);
            }
        }

        // Check Knob Rotary Drag
        if let Ok(Some(node)) = self.tree.hit_test_effective(root, self.cursor_pos) {
            if let Some(ui_widgets::WidgetKind::Knob { id, .. }) = self.tree.layout().payload(node) {
                let kid = id.to_string();
                if let Ok(Some(value)) = self.tree.knob_drag_value(root, &kid, self.cursor_pos) {
                    match kid.as_str() {
                        "knob_gain" => self.state.knob_gain = value,
                        "knob_freq" => self.state.knob_freq = value,
                        "knob_mix" => self.state.knob_mix = value,
                        _ => {}
                    }
                }
                self.knob_drag = Some(kid);
            }
        }

        // Check NodeGraph Node Drag
        if let Ok(Some((node_id, offset))) = self.tree.node_graph_hit_node(root, "gallery_node_graph", self.cursor_pos) {
            self.state.selected_graph_node = Some(node_id.clone());
            for n in &mut self.state.node_graph_nodes {
                n.selected = n.id == node_id;
            }
            self.node_drag = Some(("gallery_node_graph".to_string(), node_id, offset));
        }
    }

    fn try_start_window_resize(&self) -> bool {
        if self.state.show_modal {
            return false;
        }
        let Some(window) = &self.window else {
            return false;
        };
        let Some(renderer) = &self.renderer else {
            return false;
        };
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
        if self
            .tree
            .is_window_title_bar(root, self.cursor_pos)
            .unwrap_or(false)
        {
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
            self.state.modal_offset = (self.cursor_pos.0 - anchor_x, self.cursor_pos.1 - anchor_y);
        }
    }

    fn update_palette_drag(&mut self) {
        if let Some((ref id, (anchor_x, anchor_y))) = self.palette_drag {
            let new_x = (self.cursor_pos.0 - anchor_x).max(0.0);
            let new_y = (self.cursor_pos.1 - anchor_y).max(0.0);
            self.state.apply(UiEvent::PaletteMoved {
                palette_id: id.clone(),
                x: new_x,
                y: new_y,
            });
        }
    }

    fn update_palette_resize(&mut self) {
        if let Some((ref id, (start_cx, start_cy), (start_w, start_h))) = self.palette_resize {
            let delta_x = self.cursor_pos.0 - start_cx;
            let delta_y = self.cursor_pos.1 - start_cy;
            let new_w = (start_w + delta_x).max(120.0);
            let new_h = (start_h + delta_y).max(120.0);
            self.state.apply(UiEvent::PaletteResized {
                palette_id: id.clone(),
                width: new_w,
                height: new_h,
            });
        }
    }

    fn update_scrollbar_drag(&mut self) {
        if self.state.show_modal {
            return;
        }
        let Some((start_cursor_y, start_scroll_y)) = self.scrollbar_drag else {
            return;
        };
        let delta_cursor = self.cursor_pos.1 - start_cursor_y;
        let scale = LIST_CONTENT_HEIGHT / LIST_VIEWPORT_HEIGHT;
        self.state.list_scroll[1] =
            (start_scroll_y + delta_cursor * scale).clamp(0.0, LIST_MAX_SCROLL_Y.max(0.0));
    }

    fn update_slider_drag(&mut self) {
        if self.state.show_modal {
            return;
        }
        let Some(ref target_id) = self.slider_drag else {
            return;
        };

        // 1. Check overlay tree first (floating palettes)
        if let (Some(o_tree), Some(o_root)) = (&self.overlay_tree, self.overlay_root) {
            if let Ok(Some(value)) = o_tree.slider_drag_value(o_root, target_id, self.cursor_pos) {
                match target_id.as_str() {
                    "palette_intensity_slider" => self.state.brush_intensity = value,
                    "fader_low" => self.state.fader_low = value,
                    "fader_mid" => self.state.fader_mid = value,
                    "fader_high" => self.state.fader_high = value,
                    _ => self.state.network_intensity = value,
                }
                return;
            }
        }

        // 2. Fallback to base tree
        if let Some(root) = self.root {
            if let Ok(Some(value)) = self
                .tree
                .slider_drag_value(root, target_id, self.cursor_pos)
            {
                match target_id.as_str() {
                    "palette_intensity_slider" => self.state.brush_intensity = value,
                    "fader_low" => self.state.fader_low = value,
                    "fader_mid" => self.state.fader_mid = value,
                    "fader_high" => self.state.fader_high = value,
                    _ => self.state.network_intensity = value,
                }
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

    fn update_canvas_drag(&mut self) {
        if self.state.show_modal {
            return;
        }
        let Some(ref target_id) = self.canvas_drag else {
            return;
        };
        let Some(root) = self.root else { return };
        if let Ok(Some(event)) = self.tree.custom_paint_pointer_at(
            root,
            target_id,
            self.cursor_pos,
            self.prev_cursor_pos,
        ) {
            self.state.apply(event);
        }
    }

    fn update_text_drag(&mut self) {
        if self.state.show_modal {
            return;
        }
        let Some((ref drag_id, anchor_idx)) = self.text_drag else {
            return;
        };
        let Some(root) = self.root else { return };
        let measure = self.renderer.as_ref().map(|r| r.text_measure());
        let measure_ref = measure.as_ref().map(|m| m as &dyn ui_widgets::TextMeasure);
        let (typo_family, typo_size) = {
            let t = self.theme.read().unwrap();
            (t.typography.family.clone(), t.typography.body_size)
        };
        if let Ok(Some((widget_id, cur_idx))) = self.tree.text_cursor_at(
            root,
            self.cursor_pos,
            &typo_family,
            typo_size,
            measure_ref,
        ) {
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

    fn update_knob_drag(&mut self) {
        if self.state.show_modal {
            return;
        }
        let Some(ref target_id) = self.knob_drag else {
            return;
        };
        if let Some(root) = self.root {
            if let Ok(Some(value)) = self.tree.knob_drag_value(root, target_id, self.cursor_pos) {
                match target_id.as_str() {
                    "knob_gain" => self.state.knob_gain = value,
                    "knob_freq" => self.state.knob_freq = value,
                    "knob_mix" => self.state.knob_mix = value,
                    _ => {}
                }
            }
        }
    }

    fn update_node_drag(&mut self) {
        if self.state.show_modal {
            return;
        }
        let Some((ref graph_id, ref node_id, grab_offset)) = self.node_drag else {
            return;
        };
        if let Some(root) = self.root {
            if let Ok(effective) = self.tree.effective_bounds(root) {
                for (node, entry) in &effective {
                    if let Some(ui_widgets::WidgetKind::NodeGraph { id, pan, .. }) = self.tree.layout().payload(*node) {
                        if id.as_str() == graph_id {
                            let bounds = entry.visual;
                            let origin_x = bounds[0] + pan[0];
                            let origin_y = bounds[1] + pan[1];
                            let new_x = (self.cursor_pos.0 - origin_x - grab_offset[0]).max(10.0);
                            let new_y = (self.cursor_pos.1 - origin_y - grab_offset[1]).max(10.0);
                            for n in &mut self.state.node_graph_nodes {
                                if &n.id == node_id {
                                    n.pos = [new_x, new_y];
                                    break;
                                }
                            }
                        }
                    }
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
        self.canvas_drag = None;
        self.knob_drag = None;
        self.node_drag = None;
        self.state.apply(UiEvent::PointerUp {
            x: self.cursor_pos.0,
            y: self.cursor_pos.1,
        });

        if self.state.show_modal {
            if let (Some(m_tree), Some(m_root)) = (&self.overlay_tree, self.overlay_root) {
                let released_on = m_tree
                    .interaction_key_at(m_root, self.cursor_pos)
                    .unwrap_or(None);
                if self.pressed.is_some() && self.pressed == released_on {
                    match m_tree.dispatch_click(m_root, self.cursor_pos) {
                        Ok(Some(event)) => self.state.apply(event),
                        Ok(None) => {}
                        Err(err) => {
                            eprintln!("[widget_gallery] modal dispatch_click failed: {err}")
                        }
                    }
                }
            }
            self.pressed = None;
            return;
        }

        // Test overlay first (palettes, menu, or toast)
        if let (Some(o_tree), Some(o_root)) = (&self.overlay_tree, self.overlay_root) {
            let released_on = o_tree
                .interaction_key_at(o_root, self.cursor_pos)
                .unwrap_or(None);
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
        let released_on = self
            .tree
            .interaction_key_at(root, self.cursor_pos)
            .unwrap_or(None);
        if self.pressed.is_some() && self.pressed == released_on {
            match self.tree.dispatch_click(root, self.cursor_pos) {
                Ok(Some(event)) => self.state.apply(event),
                Ok(None) => {
                    // Clicking outside open menu/dropdown/context closes it
                    if self.state.active_menu.is_some()
                        || self.state.dropdown_open
                        || self.state.context_menu.is_some()
                    {
                        self.state.active_menu = None;
                        self.state.dropdown_open = false;
                        self.state.context_menu = None;
                    }
                }
                Err(err) => eprintln!("[widget_gallery] dispatch_click failed: {err}"),
            }
        } else if self.state.active_menu.is_some()
            || self.state.dropdown_open
            || self.state.context_menu.is_some()
        {
            // Click outside active overlay
            self.state.active_menu = None;
            self.state.dropdown_open = false;
            self.state.context_menu = None;
        }
        self.pressed = None;
    }

    fn handle_scroll(&mut self, delta_y: f32) {
        let Some(root) = self.root else { return };
        match self
            .tree
            .dispatch_scroll(root, self.cursor_pos, [0.0, delta_y])
        {
            Ok(Some(UiEvent::ScrollChanged { offset, .. })) => {
                self.state.list_scroll =
                    [offset[0], offset[1].clamp(0.0, LIST_MAX_SCROLL_Y.max(0.0))];
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
            .with_transparent(true)
            .with_inner_size(winit::dpi::PhysicalSize::new(
                WINDOW_WIDTH as u32,
                WINDOW_HEIGHT as u32,
            ));
        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("failed to create OS window"),
        );
        self.window = Some(window.clone());
        let renderer = GpuRenderer::new(window);
        let cyber_art = generate_cyber_artwork(512, 512);
        let static_tex = renderer.create_texture_rgba(512, 512, &cyber_art);
        self.resources.insert("cyber_art", static_tex);

        let plasma_init = generate_plasma_frame(256, 160, 0.0);
        let stream_tex = renderer.create_texture_rgba(256, 160, &plasma_init);
        self.resources.insert("stream_video", stream_tex.clone());
        self.stream_texture = Some(stream_tex);
        self.renderer = Some(renderer);

        let window_clone = self.window.as_ref().unwrap().clone();
        let watcher = ThemeWatcher::watch_file(
            "themes/studio_pro.toml",
            self.theme.clone(),
            move |_| {
                window_clone.request_redraw();
            },
        ).ok();
        self.theme_watcher = watcher;

        event_loop.set_control_flow(ControlFlow::Poll);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
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
            WindowEvent::HoveredFile(path) => {
                self.state.file_hovered = true;
                self.state.hovered_file = Some(path.clone());
                let widget_id = self.root.and_then(|r| {
                    self.tree
                        .hit_test_drop_target(r, self.cursor_pos)
                        .ok()
                        .flatten()
                        .map(|(id, _)| id)
                });
                self.state.apply(UiEvent::FileHovered {
                    widget_id,
                    path,
                    position: [self.cursor_pos.0, self.cursor_pos.1],
                });
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::HoveredFileCancelled => {
                self.state.file_hovered = false;
                self.state.hovered_file = None;
                self.state.apply(UiEvent::FileHoverCancelled);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::DroppedFile(path) => {
                self.state.file_hovered = false;
                self.state.hovered_file = None;
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("file")
                    .to_string();
                let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                self.state.dropped_file_info = Some((file_name.clone(), file_size));

                let widget_id = self.root.and_then(|r| {
                    self.tree
                        .hit_test_drop_target(r, self.cursor_pos)
                        .ok()
                        .flatten()
                        .map(|(id, _)| id)
                });

                let ext = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_lowercase();

                if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp" | "bmp") {
                    if let Some(renderer) = self.renderer.as_ref() {
                        match renderer.create_texture_from_image_file(&path) {
                            Ok(tex) => {
                                self.resources.insert("cyber_art", tex);
                                let disp_name = truncate_middle(&file_name, 28);
                                self.state.active_toast = Some((
                                    "Image Pipeline Updated".to_string(),
                                    format!(
                                        "Decoded '{}' ({:.1} KB) onto GPU texture",
                                        disp_name,
                                        file_size as f32 / 1024.0
                                    ),
                                    ToastKind::Success,
                                ));
                            }
                            Err(e) => {
                                self.state.active_toast = Some((
                                    "Image Import Failed".to_string(),
                                    e,
                                    ToastKind::Error,
                                ));
                            }
                        }
                    }
                } else if matches!(ext.as_str(), "rs" | "wgsl" | "json" | "toml" | "txt" | "md") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        self.state.studio_editor.text = content;
                        self.state.studio_editor.cursor = 0;
                        self.state.studio_editor.selection = None;
                        let disp_name = truncate_middle(&file_name, 28);
                        self.state.active_toast = Some((
                            "Source Code Imported".to_string(),
                            format!("Imported '{}' into Studio Editor", disp_name),
                            ToastKind::Info,
                        ));
                    }
                } else {
                    let disp_name = truncate_middle(&file_name, 28);
                    self.state.active_toast = Some((
                        "File Dropped".to_string(),
                        format!("Imported '{}' ({:.1} KB)", disp_name, file_size as f32 / 1024.0),
                        ToastKind::Info,
                    ));
                }

                self.state.apply(UiEvent::FileDropped {
                    widget_id,
                    path,
                    position: [self.cursor_pos.0, self.cursor_pos.1],
                });

                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.prev_cursor_pos = self.cursor_pos;
                self.cursor_pos = (position.x as f32, position.y as f32);
                self.update_modal_drag();
                self.update_palette_drag();
                self.update_palette_resize();
                self.update_scrollbar_drag();
                self.update_slider_drag();
                self.update_knob_drag();
                self.update_node_drag();
                self.update_color_picker_drag();
                self.update_splitter_drag();
                self.update_text_drag();
                self.update_canvas_drag();
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if !self.try_start_window_resize() && !self.try_start_window_drag() {
                    self.handle_press();
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Right,
                ..
            } => {
                if !self.state.show_modal {
                    self.state.context_menu = Some(self.cursor_pos);
                    self.state.active_menu = None;
                    self.state.dropdown_open = false;
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
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
            WindowEvent::KeyboardInput {
                event: key_event, ..
            } => {
                if key_event.state == ElementState::Pressed {
                    match key_event.logical_key {
                        winit::keyboard::Key::Named(winit::keyboard::NamedKey::Tab) => {
                            if let Some(root) = self.root {
                                let next = self
                                    .tree
                                    .next_focusable(
                                        root,
                                        self.state.focused_input.as_deref(),
                                        self.shift_held,
                                    )
                                    .unwrap_or(None);
                                self.state.focused_input = next;
                            }
                        }
                        winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) => {
                            if self.state.show_command_palette {
                                self.state.show_command_palette = false;
                                self.state.focused_input = None;
                            } else if self.state.show_notifications_drawer {
                                self.state.show_notifications_drawer = false;
                            } else if self.state.show_modal {
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

                            // Global Shortcuts: Ctrl+K (Command Palette) & Ctrl+N (Notification Center)
                            if ctrl {
                                if let winit::keyboard::Key::Character(ref s) = key_event.logical_key {
                                    if s.eq_ignore_ascii_case("k") {
                                        self.state.show_command_palette = !self.state.show_command_palette;
                                        if self.state.show_command_palette {
                                            self.state.focused_input = Some("command_palette_search".to_string());
                                            self.state.command_palette_search.text.clear();
                                            self.state.command_palette_search.cursor = 0;
                                            self.state.command_palette_selected = 0;
                                        }
                                        if let Some(window) = &self.window {
                                            window.request_redraw();
                                        }
                                        return;
                                    } else if s.eq_ignore_ascii_case("n") {
                                        self.state.show_notifications_drawer = !self.state.show_notifications_drawer;
                                        if let Some(window) = &self.window {
                                            window.request_redraw();
                                        }
                                        return;
                                    }
                                }
                            }

                            // Command Palette Arrow Navigation & Enter Execution
                            if self.state.show_command_palette {
                                match key_event.logical_key {
                                    winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowUp) => {
                                        self.state.command_palette_selected = self.state.command_palette_selected.saturating_sub(1);
                                        if let Some(window) = &self.window {
                                            window.request_redraw();
                                        }
                                        return;
                                    }
                                    winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowDown) => {
                                        self.state.command_palette_selected = (self.state.command_palette_selected + 1).min(11);
                                        if let Some(window) = &self.window {
                                            window.request_redraw();
                                        }
                                        return;
                                    }
                                    winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter) => {
                                        let all_commands = [
                                            ("cmd_tab_0", "General Dashboard", "Ctrl+1"),
                                            ("cmd_tab_1", "UI Primitives & Blocks", "Ctrl+2"),
                                            ("cmd_tab_2", "Security & Firewall", "Ctrl+3"),
                                            ("cmd_tab_3", "Cluster Telemetry", "Ctrl+4"),
                                            ("cmd_tab_4", "Creative Studio", "Ctrl+5"),
                                            ("cmd_tab_5", "Media & Video Stream", "Ctrl+6"),
                                            ("cmd_tab_6", "Vector Node Graph", "Ctrl+7"),
                                            ("cmd_turbo", "Toggle Cyber Turbo Mode", "Turbo"),
                                            ("cmd_scan", "Run Deep Diagnostic Scan", "F5"),
                                            ("cmd_notifications", "Toggle Notification Center", "Ctrl+N"),
                                            ("cmd_status_online", "Set Presence to Online", "Online"),
                                            ("cmd_status_busy", "Set Presence to Busy (DND)", "Busy"),
                                        ];
                                        let query = self.state.command_palette_search.text.to_lowercase();
                                        let filtered: Vec<_> = all_commands
                                            .iter()
                                            .filter(|(_, l, sc)| {
                                                query.is_empty()
                                                    || l.to_lowercase().contains(&query)
                                                    || sc.to_lowercase().contains(&query)
                                            })
                                            .collect();
                                        if !filtered.is_empty() {
                                            let sel_idx = self.state.command_palette_selected.min(filtered.len() - 1);
                                            let cmd_id = filtered[sel_idx].0;
                                            self.state.apply(UiEvent::CommandExecuted {
                                                command_id: cmd_id.to_string(),
                                            });
                                        }
                                        if let Some(window) = &self.window {
                                            window.request_redraw();
                                        }
                                        return;
                                    }
                                    _ => {}
                                }
                            }

                            if let Some(focused) = &self.state.focused_input {
                                if self.state.spinners_enabled
                                    && (focused == "concurrency_spin" || focused == "port_spin")
                                {
                                    let is_concurrency = focused == "concurrency_spin";
                                    let (min, max, step) = if is_concurrency {
                                        (1.0, 64.0, 1.0)
                                    } else {
                                        (1024.0, 65535.0, 10.0)
                                    };
                                    let current_val = if is_concurrency {
                                        self.state.concurrency_spin
                                    } else {
                                        self.state.port_spin
                                    };
                                    match key_event.logical_key {
                                        winit::keyboard::Key::Named(
                                            winit::keyboard::NamedKey::ArrowUp,
                                        ) => {
                                            let next = (current_val + step).min(max);
                                            if is_concurrency {
                                                self.state.concurrency_spin = next;
                                            } else {
                                                self.state.port_spin = next;
                                            }
                                        }
                                        winit::keyboard::Key::Named(
                                            winit::keyboard::NamedKey::ArrowDown,
                                        ) => {
                                            let next = (current_val - step).max(min);
                                            if is_concurrency {
                                                self.state.concurrency_spin = next;
                                            } else {
                                                self.state.port_spin = next;
                                            }
                                        }
                                        winit::keyboard::Key::Named(
                                            winit::keyboard::NamedKey::Backspace,
                                        ) => {
                                            let cur_int = current_val as i64;
                                            let next = (cur_int / 10) as f64;
                                            let clamped = if next == 0.0 {
                                                min
                                            } else {
                                                next.clamp(min, max)
                                            };
                                            if is_concurrency {
                                                self.state.concurrency_spin = clamped;
                                            } else {
                                                self.state.port_spin = clamped;
                                            }
                                        }
                                        _ => {
                                            if let Some(text) = &key_event.text {
                                                for c in text.chars() {
                                                    if let Some(d) = c.to_digit(10) {
                                                        let cur_int = current_val as i64;
                                                        let next = (cur_int * 10 + d as i64) as f64;
                                                        let clamped = next.clamp(min, max);
                                                        if is_concurrency {
                                                            self.state.concurrency_spin = clamped;
                                                        } else {
                                                            self.state.port_spin = clamped;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    let (editor, is_multiline) = match focused.as_str() {
                                        "global_search" => {
                                            (Some(&mut self.state.search_editor), false)
                                        }
                                        "command_palette_search" => {
                                            (Some(&mut self.state.command_palette_search), false)
                                        }
                                        "master_token_pwd" => {
                                            (Some(&mut self.state.password_editor), false)
                                        }
                                        "studio_editor" => {
                                            (Some(&mut self.state.studio_editor), true)
                                        }
                                        _ => (None, false),
                                    };

                                    if let Some(ed) = editor {
                                        match key_event.logical_key {
                                            winit::keyboard::Key::Named(
                                                winit::keyboard::NamedKey::ArrowLeft,
                                            ) => {
                                                ed.move_left(ctrl, shift);
                                            }
                                            winit::keyboard::Key::Named(
                                                winit::keyboard::NamedKey::ArrowRight,
                                            ) => {
                                                ed.move_right(ctrl, shift);
                                            }
                                            winit::keyboard::Key::Named(
                                                winit::keyboard::NamedKey::ArrowUp,
                                            ) => {
                                                if is_multiline {
                                                    ed.move_up(shift);
                                                } else {
                                                    ed.move_home(shift);
                                                }
                                            }
                                            winit::keyboard::Key::Named(
                                                winit::keyboard::NamedKey::ArrowDown,
                                            ) => {
                                                if is_multiline {
                                                    ed.move_down(shift);
                                                } else {
                                                    ed.move_end(shift);
                                                }
                                            }
                                            winit::keyboard::Key::Named(
                                                winit::keyboard::NamedKey::Home,
                                            ) => {
                                                ed.move_home(shift);
                                            }
                                            winit::keyboard::Key::Named(
                                                winit::keyboard::NamedKey::End,
                                            ) => {
                                                ed.move_end(shift);
                                            }
                                            winit::keyboard::Key::Named(
                                                winit::keyboard::NamedKey::Backspace,
                                            ) => {
                                                ed.backspace();
                                            }
                                            winit::keyboard::Key::Named(
                                                winit::keyboard::NamedKey::Delete,
                                            ) => {
                                                ed.delete();
                                            }
                                            winit::keyboard::Key::Named(
                                                winit::keyboard::NamedKey::Enter,
                                            ) => {
                                                if is_multiline {
                                                    ed.insert_char('\n');
                                                } else {
                                                    println!("[widget_gallery] input '{focused}' submitted: '{}'", ed.text);
                                                }
                                            }
                                            _ => {
                                                if ctrl {
                                                    if let winit::keyboard::Key::Character(ref s) =
                                                        key_event.logical_key
                                                    {
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
    event_loop
        .run_app(&mut app)
        .expect("failed to run winit event loop");
}
