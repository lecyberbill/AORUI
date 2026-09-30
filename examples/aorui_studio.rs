// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0 | Action: Stratus GUI Studio - High-End AetherOS Interface Designer & WYSIWYG Canvas
use std::sync::{Arc, RwLock};
use std::time::Instant;

use ui_core::UiEvent;
use ui_gpu::{BackgroundParams, GpuRenderer, MediaInstance, RenderLayer, ResourceTable};
use ui_layout::{
    auto, length, AlignItems, AvailableSpace, FlexDirection, JustifyContent, NodeId, Rect, Size,
    Style,
};
use ui_widgets::{
    ButtonVariant, FocusManager, IconKind, InteractionKey, InteractionState, KeyChord, KeyCode,
    KeyMap, ListItemBadge, Modifiers, Painter, Theme, ToastKind, WidgetId, WidgetTree,
};

use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

const WINDOW_WIDTH: f32 = 1560.0;
const WINDOW_HEIGHT: f32 = 960.0;

// ============================================================================
// Layout Helpers
// ============================================================================

fn leaf(width: f32, height: f32) -> Style {
    Style {
        size: Size {
            width: length(width),
            height: length(height),
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

fn font_family<'a>(family: &'a ui_widgets::FontFamily) -> glyphon::Family<'a> {
    match family {
        ui_widgets::FontFamily::SansSerif => glyphon::Family::SansSerif,
        ui_widgets::FontFamily::Serif => glyphon::Family::Serif,
        ui_widgets::FontFamily::Monospace => glyphon::Family::Monospace,
        ui_widgets::FontFamily::Named(name) => glyphon::Family::Name(name.as_str()),
    }
}

fn card_style(w: f32) -> Style {
    Style {
        size: Size {
            width: length(w),
            height: auto(),
        },
        flex_direction: FlexDirection::Column,
        gap: Size {
            width: length(0.0),
            height: length(8.0),
        },
        padding: Rect {
            left: length(12.0),
            right: length(12.0),
            top: length(10.0),
            bottom: length(12.0),
        },
        ..Default::default()
    }
}

// ============================================================================
// Studio State & Data Models
// ============================================================================

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StudioTool {
    Select,    // V
    Hand,      // H
    Frame,     // F
    Component, // C
    Text,      // T
    Asset,     // A
}

impl StudioTool {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Select => "V",
            Self::Hand => "H",
            Self::Frame => "F",
            Self::Component => "C",
            Self::Text => "T",
            Self::Asset => "A",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StudioMode {
    Editor,
    Preview,
    Jsx,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LeftTab {
    Components,
    Layers,
}

#[derive(Clone, Debug)]
pub struct LayerItem {
    pub id: String,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    pub is_selected: bool,
}

#[derive(Clone, Debug)]
pub struct MicroVmRow {
    pub id: String,
    pub role: String,
    pub vcpu: String,
    pub memory: String,
    pub status: String,
}

pub struct StudioState {
    pub active_tool: StudioTool,
    pub active_mode: StudioMode,
    pub left_tab: LeftTab,
    pub zoom_level: f32,
    pub grid_snap_enabled: bool,
    pub snap_grid_size: f32,

    // Component Geometry & Properties
    pub selected_component_id: String,
    pub comp_w: f32,
    pub comp_h: f32,
    pub comp_x: f32,
    pub comp_y: f32,
    pub flex_direction_row: bool,
    pub gap_size: f32,

    // Cloud Data Binding
    pub ws_source: String,
    pub refresh_rate_ms: u32,
    pub fallback_mode: String,
    pub tls_streaming: bool,

    // Search filter
    pub search_query: String,
    pub focused_input: Option<String>,

    // Scene & Canvas Data
    pub layers: Vec<LayerItem>,
    pub micro_vms: Vec<MicroVmRow>,
    pub sparkline_wave_phase: f32,
    pub ring_gauge_val: f32,

    // Feedback
    pub active_toast: Option<(String, String, ToastKind)>,
    pub toast_timer: Option<Instant>,
    pub start_time: Instant,
    pub focus_manager: FocusManager,
    pub key_map: KeyMap,
    pub close_requested: bool,
}

impl Default for StudioState {
    fn default() -> Self {
        Self::new()
    }
}

impl StudioState {
    pub fn new() -> Self {
        let layers = vec![
            LayerItem {
                id: "TelemetrySparkCard".to_string(),
                name: "TelemetrySparkCard (GPU H100)".to_string(),
                visible: true,
                locked: false,
                is_selected: true,
            },
            LayerItem {
                id: "RingGaugeNvME".to_string(),
                name: "RingGaugeNvME (Storage)".to_string(),
                visible: true,
                locked: true,
                is_selected: false,
            },
            LayerItem {
                id: "FirecrackerMicroVMList".to_string(),
                name: "FirecrackerMicroVMList".to_string(),
                visible: true,
                locked: true,
                is_selected: false,
            },
            LayerItem {
                id: "GlobalWindowHeader".to_string(),
                name: "GlobalWindowHeader".to_string(),
                visible: true,
                locked: true,
                is_selected: false,
            },
        ];

        let micro_vms = vec![
            MicroVmRow {
                id: "vm-ai-fra-02".to_string(),
                role: "Stratus-AI-Alpine v3".to_string(),
                vcpu: "8 Cores".to_string(),
                memory: "16.0 GB".to_string(),
                status: "Actif".to_string(),
            },
            MicroVmRow {
                id: "vm-gw-fra-01".to_string(),
                role: "Stratus-Gateway-Rust".to_string(),
                vcpu: "4 Cores".to_string(),
                memory: "4.0 GB".to_string(),
                status: "Actif".to_string(),
            },
            MicroVmRow {
                id: "vm-sec-hsm".to_string(),
                role: "Kernel-Enclave-AES".to_string(),
                vcpu: "2 Cores".to_string(),
                memory: "2.0 GB".to_string(),
                status: "Actif".to_string(),
            },
        ];

        let mut key_map = KeyMap::new();
        key_map.bind(
            KeyChord::new(Modifiers::NONE, KeyCode::Char('v')),
            "tool_select",
        );
        key_map.bind(
            KeyChord::new(Modifiers::NONE, KeyCode::Char('h')),
            "tool_hand",
        );
        key_map.bind(
            KeyChord::new(Modifiers::NONE, KeyCode::Char('f')),
            "tool_frame",
        );
        key_map.bind(
            KeyChord::new(Modifiers::NONE, KeyCode::Char('c')),
            "tool_component",
        );
        key_map.bind(
            KeyChord::new(Modifiers::NONE, KeyCode::Char('t')),
            "tool_text",
        );
        key_map.bind(
            KeyChord::new(Modifiers::NONE, KeyCode::Char('a')),
            "tool_asset",
        );

        Self {
            active_tool: StudioTool::Select,
            active_mode: StudioMode::Editor,
            left_tab: LeftTab::Components,
            zoom_level: 0.75,
            grid_snap_enabled: true,
            snap_grid_size: 8.0,

            selected_component_id: "charge-cluster-gpu".to_string(),
            comp_w: 520.0,
            comp_h: 228.0,
            comp_x: 340.0,
            comp_y: 180.0,
            flex_direction_row: true,
            gap_size: 12.0,

            ws_source: "stratus.cluster.telemetry.gpu_vram".to_string(),
            refresh_rate_ms: 15,
            fallback_mode: "Mock Auto-Gen".to_string(),
            tls_streaming: true,

            search_query: String::new(),
            focused_input: None,

            layers,
            micro_vms,
            sparkline_wave_phase: 0.0,
            ring_gauge_val: 0.84,

            active_toast: None,
            toast_timer: None,
            start_time: Instant::now(),
            focus_manager: FocusManager::new(),
            key_map,
            close_requested: false,
        }
    }

    pub fn trigger_toast(
        &mut self,
        title: impl Into<String>,
        msg: impl Into<String>,
        kind: ToastKind,
    ) {
        self.active_toast = Some((title.into(), msg.into(), kind));
        self.toast_timer = Some(Instant::now());
    }

    pub fn update_animations(&mut self) {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        self.sparkline_wave_phase = elapsed * 2.5;

        if let Some(timer) = self.toast_timer {
            if timer.elapsed().as_secs_f32() > 4.0 {
                self.active_toast = None;
                self.toast_timer = None;
            }
        }
    }
}

// ============================================================================
// UI Construction Pipeline
// ============================================================================

fn build_studio_ui(tree: &mut WidgetTree, state: &StudioState, width: f32, height: f32) -> NodeId {
    // ------------------------------------------------------------------------
    // TOP BAR 1: Global OS Header
    // ------------------------------------------------------------------------
    let top_os_bar = {
        let logo_icon = tree
            .icon(IconKind::Cpu, 16.0, Some([0.0, 0.85, 1.0, 1.0]), leaf(18.0, 18.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let logo_txt = tree
            .label("Stratus OS", leaf(88.0, 18.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let menu_items = [
            ("menu_file", "Fichier"),
            ("menu_view", "Affichage"),
            ("menu_cloud", "Espaces Cloud"),
            ("menu_studio", "Éditeur GUI Studio"),
            ("menu_tools", "Outils"),
            ("menu_help", "Aide"),
        ];

        let mut menu_nodes = Vec::new();
        for (id, label) in menu_items {
            let item = tree
                .button_variant(
                    WidgetId::new(id),
                    label,
                    ButtonVariant::Ghost,
                    true,
                    leaf(label.len() as f32 * 8.0 + 16.0, 24.0),
                )
                .unwrap_or_else(|_| NodeId::from(0usize));
            menu_nodes.push(item);
        }

        let left_part_nodes = [vec![logo_icon, logo_txt], menu_nodes].concat();
        let left_container = tree
            .container(&left_part_nodes, row(8.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        // Right Telemetry & Status Pill
        let uptime_badge = tree
            .badge(
                "99.98%",
                ListItemBadge::Success,
                leaf(72.0, 20.0),
            )
            .unwrap_or_else(|_| NodeId::from(0usize));

        let ping_lbl = tree
            .label("12ms FRA", leaf(62.0, 16.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let ram_lbl = tree
            .label_muted("RAM 4.8/32G", leaf(80.0, 16.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let cpu_lbl = tree
            .label("CPU 14%", leaf(58.0, 16.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let clock_lbl = tree
            .label("Ven 24 Oct 14:32", leaf(110.0, 16.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let user_avatar = tree
            .avatar(
                WidgetId::new("user_avatar"),
                None::<String>,
                Some("CB"),
                ui_widgets::AvatarStatus::Online,
                24.0,
                Some([0.0, 0.85, 1.0, 1.0]),
                true,
                leaf(24.0, 24.0),
            )
            .unwrap_or_else(|_| NodeId::from(0usize));

        let right_container = tree
            .container(
                &[uptime_badge, ping_lbl, ram_lbl, cpu_lbl, clock_lbl, user_avatar],
                row(12.0),
            )
            .unwrap_or_else(|_| NodeId::from(0usize));

        let mut bar_style = row(0.0);
        bar_style.size = Size {
            width: length(width),
            height: length(36.0),
        };
        bar_style.justify_content = Some(JustifyContent::SpaceBetween);
        bar_style.padding = Rect {
            left: length(14.0),
            right: length(14.0),
            top: length(4.0),
            bottom: length(4.0),
        };

        tree.container(&[left_container, right_container], bar_style)
            .unwrap_or_else(|_| NodeId::from(0usize))
    };

    // ------------------------------------------------------------------------
    // TOP BAR 2: Studio Navigation Ribbon & Tool Selector
    // ------------------------------------------------------------------------
    let studio_ribbon = {
        let studio_icon = tree
            .icon(IconKind::Cpu, 16.0, Some([0.0, 0.85, 1.0, 1.0]), leaf(18.0, 18.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let studio_title = tree
            .label("Stratus GUI Studio", leaf(135.0, 18.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let ver_badge = tree
            .badge(
                "v2.4",
                ListItemBadge::Active("v2.4".to_string()),
                leaf(38.0, 18.0),
            )
            .unwrap_or_else(|_| NodeId::from(0usize));

        let breadcrumb = tree
            .button_variant(
                WidgetId::new("breadcrumb_btn"),
                "⬡ ClusterMonitoringDashboard",
                ButtonVariant::Ghost,
                true,
                leaf(210.0, 26.0),
            )
            .unwrap_or_else(|_| NodeId::from(0usize));

        // Tool buttons: V, H, F, C, T, A
        let tools = [
            (StudioTool::Select, "tool_v"),
            (StudioTool::Hand, "tool_h"),
            (StudioTool::Frame, "tool_f"),
            (StudioTool::Component, "tool_c"),
            (StudioTool::Text, "tool_t"),
            (StudioTool::Asset, "tool_a"),
        ];

        let mut tool_nodes = Vec::new();
        for (tool, id) in tools {
            let is_active = state.active_tool == tool;
            let variant = if is_active {
                ButtonVariant::Primary
            } else {
                ButtonVariant::Ghost
            };
            let btn = tree
                .button_variant(WidgetId::new(id), tool.label(), variant, true, leaf(28.0, 26.0))
                .unwrap_or_else(|_| NodeId::from(0usize));
            tool_nodes.push(btn);
        }

        let tools_pill = tree
            .container(&tool_nodes, row(2.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let left_ribbon = tree
            .container(
                &[studio_icon, studio_title, ver_badge, breadcrumb, tools_pill],
                row(10.0),
            )
            .unwrap_or_else(|_| NodeId::from(0usize));

        // Center Viewport Controls (Desktop, Resolution dropdown, Zoom, Undo/Redo)
        let desk_icon = tree
            .icon(IconKind::Cpu, 14.0, Some([0.6, 0.7, 0.8, 0.9]), leaf(16.0, 16.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let res_btn = tree
            .button_variant(
                WidgetId::new("res_dropdown"),
                "1920×1080 ▾",
                ButtonVariant::Ghost,
                true,
                leaf(95.0, 26.0),
            )
            .unwrap_or_else(|_| NodeId::from(0usize));
        let tablet_icon = tree
            .icon(IconKind::Settings, 14.0, Some([0.6, 0.7, 0.8, 0.9]), leaf(16.0, 16.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let zoom_lbl = tree
            .button_variant(
                WidgetId::new("zoom_btn"),
                format!("{:.0}%", state.zoom_level * 100.0),
                ButtonVariant::Ghost,
                true,
                leaf(48.0, 26.0),
            )
            .unwrap_or_else(|_| NodeId::from(0usize));
        let undo_btn = tree
            .button_variant(WidgetId::new("undo_btn"), "↶", ButtonVariant::Ghost, true, leaf(24.0, 26.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let redo_btn = tree
            .button_variant(WidgetId::new("redo_btn"), "↷", ButtonVariant::Ghost, true, leaf(24.0, 26.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let center_ribbon = tree
            .container(
                &[desk_icon, res_btn, tablet_icon, zoom_lbl, undo_btn, redo_btn],
                row(6.0),
            )
            .unwrap_or_else(|_| NodeId::from(0usize));

        // Right Mode Switcher (Éditeur, Aperçu, <> JSX, Publier OS)
        let ed_var = if state.active_mode == StudioMode::Editor {
            ButtonVariant::Primary
        } else {
            ButtonVariant::Ghost
        };
        let ed_btn = tree
            .button_variant(WidgetId::new("mode_editor"), "Éditeur", ed_var, true, leaf(65.0, 26.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let prev_var = if state.active_mode == StudioMode::Preview {
            ButtonVariant::Primary
        } else {
            ButtonVariant::Ghost
        };
        let prev_btn = tree
            .button_variant(WidgetId::new("mode_preview"), "Aperçu", prev_var, true, leaf(65.0, 26.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let jsx_var = if state.active_mode == StudioMode::Jsx {
            ButtonVariant::Primary
        } else {
            ButtonVariant::Ghost
        };
        let jsx_btn = tree
            .button_variant(WidgetId::new("mode_jsx"), "<> JSX", jsx_var, true, leaf(65.0, 26.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let publish_btn = tree
            .button_variant(WidgetId::new("publish_btn"), "⚡ Publier OS", ButtonVariant::Primary, true, leaf(110.0, 28.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let right_ribbon = tree
            .container(&[ed_btn, prev_btn, jsx_btn, publish_btn], row(8.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let mut ribbon_style = row(0.0);
        ribbon_style.size = Size {
            width: length(width),
            height: length(44.0),
        };
        ribbon_style.justify_content = Some(JustifyContent::SpaceBetween);
        ribbon_style.padding = Rect {
            left: length(14.0),
            right: length(14.0),
            top: length(6.0),
            bottom: length(6.0),
        };

        tree.container(&[left_ribbon, center_ribbon, right_ribbon], ribbon_style)
            .unwrap_or_else(|_| NodeId::from(0usize))
    };

    // ------------------------------------------------------------------------
    // LEFT SIDEBAR: Composants OS & Arborescence
    // ------------------------------------------------------------------------
    let left_sidebar_w = 310.0;
    let main_height = height - 36.0 - 44.0 - 32.0;

    let left_sidebar = {
        // Tabs
        let comp_var = if state.left_tab == LeftTab::Components {
            ButtonVariant::Primary
        } else {
            ButtonVariant::Ghost
        };
        let comp_tab_btn = tree
            .button_variant(WidgetId::new("tab_comp"), "🎛️ Composants OS", comp_var, true, leaf(140.0, 28.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let layers_var = if state.left_tab == LeftTab::Layers {
            ButtonVariant::Primary
        } else {
            ButtonVariant::Ghost
        };
        let layers_tab_btn = tree
            .button_variant(WidgetId::new("tab_layers"), "🗂️ Calques (14)", layers_var, true, leaf(120.0, 28.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let tabs_row = tree
            .container(&[comp_tab_btn, layers_tab_btn], row(6.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        // Search Input
        let search_input = tree
            .text_input(
                WidgetId::new("search_bricks"),
                &state.search_query,
                "🔍 Filtrer brique (⌘K)...",
                false,
                leaf(left_sidebar_w - 28.0, 32.0),
            )
            .unwrap_or_else(|_| NodeId::from(0usize));

        // Section 1: Primitives Système
        let sec1_title = tree
            .label("PRIMITIVES SYSTÈME", leaf(left_sidebar_w - 60.0, 16.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let sec1_count = tree
            .label_muted("4", leaf(20.0, 16.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let sec1_hdr = tree
            .container(&[sec1_title, sec1_count], row(0.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let prim_b1 = tree
            .button_variant(WidgetId::new("prim_window"), "Fenêtre Glass [Org]", ButtonVariant::Ghost, true, leaf(128.0, 34.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let prim_b2 = tree
            .button_variant(WidgetId::new("prim_bar"), "Global Bar [Atom]", ButtonVariant::Ghost, true, leaf(128.0, 34.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let prim_r1 = tree.container(&[prim_b1, prim_b2], row(8.0)).unwrap_or_else(|_| NodeId::from(0usize));

        let prim_b3 = tree
            .button_variant(WidgetId::new("prim_dock"), "Aether Dock [Mol]", ButtonVariant::Ghost, true, leaf(128.0, 34.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let prim_b4 = tree
            .button_variant(WidgetId::new("prim_palette"), "Palette ⌘K [Atom]", ButtonVariant::Ghost, true, leaf(128.0, 34.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let prim_r2 = tree.container(&[prim_b3, prim_b4], row(8.0)).unwrap_or_else(|_| NodeId::from(0usize));

        // Section 2: Widgets & Métriques
        let sec2_title = tree
            .label("WIDGETS & MÉTRIQUES", leaf(left_sidebar_w - 60.0, 16.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let sec2_count = tree
            .label_muted("4", leaf(20.0, 16.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let sec2_hdr = tree
            .container(&[sec2_title, sec2_count], row(0.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        // Featured Card: TelemetrySparkCard
        let card_lead = tree
            .label("📈 TelemetrySparkCard", leaf(left_sidebar_w - 75.0, 18.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let card_badge = tree
            .badge(
                "Prêt",
                ListItemBadge::Success,
                leaf(40.0, 18.0),
            )
            .unwrap_or_else(|_| NodeId::from(0usize));
        let card_lead_row = tree
            .container(&[card_lead, card_badge], row(4.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let card_desc = tree
            .label_muted("Carte d'observabilité GPU/vCPU avec flux SVG...", leaf(left_sidebar_w - 50.0, 14.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let card_stat = tree
            .label("H100 • 87%", leaf(left_sidebar_w - 50.0, 16.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let spark_preview_card = tree
            .card(
                &[card_lead_row, card_desc, card_stat],
                None,
                None,
                Some(8.0),
                card_style(left_sidebar_w - 28.0),
            )
            .unwrap_or_else(|_| NodeId::from(0usize));

        let w_gauge = tree
            .button_variant(
                WidgetId::new("w_gauge"),
                "RingGaugeNvME (84% POSIX)",
                ButtonVariant::Ghost,
                true,
                leaf(left_sidebar_w - 28.0, 30.0),
            )
            .unwrap_or_else(|_| NodeId::from(0usize));
        let w_k8s = tree
            .button_variant(
                WidgetId::new("w_k8s"),
                "K8sClusterNodeBadge (fra-04b)",
                ButtonVariant::Ghost,
                true,
                leaf(left_sidebar_w - 28.0, 30.0),
            )
            .unwrap_or_else(|_| NodeId::from(0usize));

        // Section 3: Contrôles & Saisie
        let sec3_title = tree
            .label("CONTRÔLES & SAISIE", leaf(left_sidebar_w - 60.0, 16.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let sec3_count = tree
            .label_muted("3", leaf(20.0, 16.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let sec3_hdr = tree
            .container(&[sec3_title, sec3_count], row(0.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let c_btn1 = tree
            .button_variant(WidgetId::new("c_btn1"), "Bouton Glass (Glow)", ButtonVariant::Ghost, true, leaf(128.0, 30.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let c_btn2 = tree
            .button_variant(WidgetId::new("c_btn2"), "HyperSlider IA", ButtonVariant::Ghost, true, leaf(128.0, 30.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let sec3_row = tree.container(&[c_btn1, c_btn2], row(8.0)).unwrap_or_else(|_| NodeId::from(0usize));

        // Section 4: Tree / Arborescence
        let tree_title = tree
            .label("ARBORESCENCE DU CANVAS", leaf(left_sidebar_w - 28.0, 16.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let mut layer_nodes = Vec::new();
        for (i, layer) in state.layers.iter().enumerate() {
            let vis_icon = if layer.visible { "👁" } else { "⊘" };
            let lock_icon = if layer.locked { "🔒" } else { "🔓" };
            let label = format!("{} {} {}", vis_icon, lock_icon, layer.name);

            let l_var = if layer.is_selected {
                ButtonVariant::Primary
            } else {
                ButtonVariant::Ghost
            };
            let item_btn = tree
                .button_variant(WidgetId::new(format!("layer_{}", i)), label, l_var, true, leaf(left_sidebar_w - 28.0, 26.0))
                .unwrap_or_else(|_| NodeId::from(0usize));
            layer_nodes.push(item_btn);
        }

        let layers_container = tree
            .container(&layer_nodes, column(4.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let mut sidebar_style = column(8.0);
        sidebar_style.size = Size {
            width: length(left_sidebar_w),
            height: length(main_height),
        };
        sidebar_style.padding = Rect {
            left: length(12.0),
            right: length(12.0),
            top: length(10.0),
            bottom: length(10.0),
        };

        tree.container(
            &[
                tabs_row,
                search_input,
                sec1_hdr,
                prim_r1,
                prim_r2,
                sec2_hdr,
                spark_preview_card,
                w_gauge,
                w_k8s,
                sec3_hdr,
                sec3_row,
                tree_title,
                layers_container,
            ],
            sidebar_style,
        )
        .unwrap_or_else(|_| NodeId::from(0usize))
    };

    // ------------------------------------------------------------------------
    // RIGHT SIDEBAR: Inspecteur de Propriétés & Événements
    // ------------------------------------------------------------------------
    let right_sidebar_w = 340.0;
    let right_sidebar = {
        // Selected Component Header
        let comp_badge_title = tree
            .label("COMPOSANT SÉLECTIONNÉ", leaf(180.0, 16.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let conf_badge = tree
            .badge(
                "Conforme",
                ListItemBadge::Success,
                leaf(75.0, 20.0),
            )
            .unwrap_or_else(|_| NodeId::from(0usize));
        let comp_header_row = tree
            .container(&[comp_badge_title, conf_badge], row(0.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let comp_tag = tree
            .label("<AetherCard.telemetry>", leaf(right_sidebar_w - 28.0, 20.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let comp_meta = tree
            .label_muted("#charge-cluster-gpu • Variant: \"sparkline\"", leaf(right_sidebar_w - 28.0, 14.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        // Section 1: Mise en page & Géométrie
        let geom_title = tree
            .label("MISE EN PAGE & GÉOMÉTRIE ⚙", leaf(right_sidebar_w - 28.0, 16.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let w_lbl = tree.label("W", leaf(20.0, 24.0)).unwrap_or_else(|_| NodeId::from(0usize));
        let w_inp = tree
            .text_input(
                WidgetId::new("geom_w"),
                format!("{:.0}px", state.comp_w),
                "520px",
                false,
                leaf(120.0, 26.0),
            )
            .unwrap_or_else(|_| NodeId::from(0usize));
        let h_lbl = tree.label("H", leaf(20.0, 24.0)).unwrap_or_else(|_| NodeId::from(0usize));
        let h_inp = tree
            .text_input(
                WidgetId::new("geom_h"),
                format!("{:.0}px", state.comp_h),
                "228px",
                false,
                leaf(120.0, 26.0),
            )
            .unwrap_or_else(|_| NodeId::from(0usize));

        let geom_row1 = tree
            .container(&[w_lbl, w_inp, h_lbl, h_inp], row(6.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let x_lbl = tree.label("X", leaf(20.0, 24.0)).unwrap_or_else(|_| NodeId::from(0usize));
        let x_inp = tree
            .text_input(
                WidgetId::new("geom_x"),
                format!("{:.0}", state.comp_x),
                "340",
                false,
                leaf(120.0, 26.0),
            )
            .unwrap_or_else(|_| NodeId::from(0usize));
        let y_lbl = tree.label("Y", leaf(20.0, 24.0)).unwrap_or_else(|_| NodeId::from(0usize));
        let y_inp = tree
            .text_input(
                WidgetId::new("geom_y"),
                format!("{:.0}", state.comp_y),
                "180",
                false,
                leaf(120.0, 26.0),
            )
            .unwrap_or_else(|_| NodeId::from(0usize));

        let geom_row2 = tree
            .container(&[x_lbl, x_inp, y_lbl, y_inp], row(6.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let flex_lbl = tree
            .label_muted("Flex Direction / Gap", leaf(130.0, 24.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let flex_h_btn = tree
            .button_variant(WidgetId::new("flex_h"), "⬌ Row", ButtonVariant::Primary, true, leaf(65.0, 24.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let flex_v_btn = tree
            .button_variant(WidgetId::new("flex_v"), "⬍ Col", ButtonVariant::Ghost, true, leaf(65.0, 24.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let flex_row = tree
            .container(&[flex_lbl, flex_h_btn, flex_v_btn], row(6.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        // Section 2: Matériau & Finition
        let mat_title = tree
            .label("MATÉRIAU & FINITION (Aether Glass L2)", leaf(right_sidebar_w - 28.0, 16.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let mat_l1 = tree
            .label_muted("Fond Glass: #0f131d (70% Blur)", leaf(right_sidebar_w - 28.0, 14.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let mat_l2 = tree
            .label_muted("Bordure: 1px white/10 | Rayon: 12px", leaf(right_sidebar_w - 28.0, 14.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let mat_l3 = tree
            .label("Glow Néon: Cyan-Indigo 12px", leaf(right_sidebar_w - 28.0, 14.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        // Section 3: Liaison Données Cloud
        let cloud_title = tree
            .label("LIAISON DONNÉES CLOUD 🟢", leaf(right_sidebar_w - 28.0, 16.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let ws_inp = tree
            .text_input(
                WidgetId::new("ws_inp"),
                &state.ws_source,
                "stratus.cluster.telemetry.gpu_vram",
                false,
                leaf(right_sidebar_w - 28.0, 28.0),
            )
            .unwrap_or_else(|_| NodeId::from(0usize));

        let cloud_info1 = tree
            .label_muted("Rafraîchissement: 15ms (Temps réel)", leaf(right_sidebar_w - 28.0, 14.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let cloud_info2 = tree
            .label_muted("Fallback: Mock Auto-Gen | TLS: Actif", leaf(right_sidebar_w - 28.0, 14.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        // Section 4: Événements & Triggers
        let ev_title = tree
            .label("ÉVÉNEMENTS & TRIGGERS", leaf(200.0, 16.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let ev_add_btn = tree
            .button_variant(WidgetId::new("ev_add"), "+ Lier", ButtonVariant::Ghost, true, leaf(60.0, 20.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let ev_hdr = tree
            .container(&[ev_title, ev_add_btn], row(0.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let ev1 = tree
            .button_variant(WidgetId::new("ev_click"), "onClick ➔ ouvrirInspecteurLogs() ✕", ButtonVariant::Ghost, true, leaf(right_sidebar_w - 28.0, 26.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let ev2 = tree
            .button_variant(WidgetId::new("ev_alert"), "onAlert ➔ triggerToastH100Warning() ✕", ButtonVariant::Ghost, true, leaf(right_sidebar_w - 28.0, 26.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        // Section 5: Aether JSX Output
        let jsx_title = tree
            .label("Aether JSX Output", leaf(180.0, 16.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let copy_btn = tree
            .button_variant(WidgetId::new("copy_jsx"), "Copier", ButtonVariant::Primary, true, leaf(65.0, 20.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let jsx_hdr = tree
            .container(&[jsx_title, copy_btn], row(0.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let jsx_snippet = tree
            .label_muted("<AetherCard metric=\"gpu.load\" value={87.4} glow=\"cyan\" />", leaf(right_sidebar_w - 28.0, 16.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let mut right_style = column(8.0);
        right_style.size = Size {
            width: length(right_sidebar_w),
            height: length(main_height),
        };
        right_style.padding = Rect {
            left: length(12.0),
            right: length(12.0),
            top: length(10.0),
            bottom: length(10.0),
        };

        tree.container(
            &[
                comp_header_row,
                comp_tag,
                comp_meta,
                geom_title,
                geom_row1,
                geom_row2,
                flex_row,
                mat_title,
                mat_l1,
                mat_l2,
                mat_l3,
                cloud_title,
                ws_inp,
                cloud_info1,
                cloud_info2,
                ev_hdr,
                ev1,
                ev2,
                jsx_hdr,
                jsx_snippet,
            ],
            right_style,
        )
        .unwrap_or_else(|_| NodeId::from(0usize))
    };

    // ------------------------------------------------------------------------
    // CENTER CANVAS: WYSIWYG Workspace Surface (CustomPaint 2D Vector Surface)
    // ------------------------------------------------------------------------
    let canvas_w = width - left_sidebar_w - right_sidebar_w;
    let canvas_h = main_height;

    let center_canvas_node = {
        let mut p = Painter::new();

        // 1. Precise Snapped Dot Grid Background (Cyan subtle dots)
        let dot_spacing = 16.0;
        let mut x = 8.0;
        while x < canvas_w {
            let mut y = 8.0;
            while y < canvas_h {
                p.circle([x, y], 1.0, Some([0.0, 0.85, 1.0, 0.18]), None);
                y += dot_spacing;
            }
            x += dot_spacing;
        }

        // 2. Canvas Watermark Label
        p.text(
            [canvas_w * 0.5 - 45.0, 18.0],
            "Edge FRA-1",
            12.0,
            [0.6, 0.7, 0.8, 0.4],
        );

        // 3. Main Live Canvas Container Border
        let surface_x = 20.0;
        let surface_y = 36.0;
        let surface_w = canvas_w - 40.0;
        let surface_h = canvas_h - 52.0;

        p.rect(
            [surface_x, surface_y, surface_w, surface_h],
            8.0,
            Some([0.05, 0.08, 0.14, 0.85]),
            Some(([0.0, 0.85, 1.0, 0.2], 1.0)),
        );

        // Header text inside surface
        p.text(
            [surface_x + 16.0, surface_y + 22.0],
            "Cluster Monitoring Dashboard",
            14.0,
            [0.9, 0.95, 1.0, 0.9],
        );
        p.rect(
            [surface_x + 230.0, surface_y + 10.0, 48.0, 18.0],
            4.0,
            Some([0.06, 0.72, 0.51, 0.25]),
            Some(([0.06, 0.72, 0.51, 0.8], 1.0)),
        );
        p.text(
            [surface_x + 240.0, surface_y + 22.0],
            "LIVE",
            10.0,
            [0.06, 0.72, 0.51, 1.0],
        );

        // --------------------------------------------------------------------
        // Live Widget 1: TelemetrySparkCard (GPU H100) - Active Selected Box
        // --------------------------------------------------------------------
        let card1_x = surface_x + 16.0;
        let card1_y = surface_y + 40.0;
        let card1_w = surface_w * 0.56;
        let card1_h = 220.0;

        // Glass background with neon cyan glow
        p.rect(
            [card1_x, card1_y, card1_w, card1_h],
            12.0,
            Some([0.06, 0.08, 0.12, 0.95]),
            Some(([0.0, 0.85, 1.0, 0.3], 1.0)),
        );

        // Title & Node metadata
        p.text(
            [card1_x + 16.0, card1_y + 24.0],
            "Cluster GPU H100",
            13.0,
            [0.9, 0.95, 1.0, 0.95],
        );
        p.text(
            [card1_x + 16.0, card1_y + 42.0],
            "fra-cloud-01 • 8 Nœuds",
            11.0,
            [0.6, 0.7, 0.8, 0.7],
        );

        // Chip Nominal
        p.rect(
            [card1_x + card1_w - 95.0, card1_y + 14.0, 80.0, 20.0],
            10.0,
            Some([0.06, 0.72, 0.51, 0.2]),
            Some(([0.06, 0.72, 0.51, 0.8], 1.0)),
        );
        p.text(
            [card1_x + card1_w - 86.0, card1_y + 28.0],
            "• Nominal",
            11.0,
            [0.06, 0.72, 0.51, 1.0],
        );

        // Glowing Spline Wave Curve (Cyan Gradient Spline)
        let graph_x = card1_x + 16.0;
        let graph_y = card1_y + 60.0;
        let graph_w = card1_w - 32.0;

        let phase = state.sparkline_wave_phase;
        let p0 = [graph_x, graph_y + 70.0];
        let p1 = [graph_x + graph_w * 0.25, graph_y + 30.0 + (phase).sin() * 20.0];
        let p2 = [graph_x + graph_w * 0.55, graph_y + 80.0 + (phase * 1.3).cos() * 18.0];
        let p3 = [graph_x + graph_w * 0.8, graph_y + 20.0 + (phase * 0.8).sin() * 22.0];
        let p4 = [graph_x + graph_w, graph_y + 50.0];

        // Bezier segments
        p.bezier(p0, p1, p2, p3, 2.5, [0.0, 0.85, 1.0, 0.9]);
        p.bezier(p3, [p3[0] + 20.0, p3[1] + 30.0], [p4[0] - 20.0, p4[1] - 20.0], p4, 2.5, [0.0, 0.85, 1.0, 0.9]);

        // Subtext
        p.text(
            [card1_x + 16.0, card1_y + card1_h - 14.0],
            "Temp: 58°C",
            11.0,
            [0.0, 0.85, 1.0, 0.9],
        );
        p.text(
            [card1_x + card1_w - 150.0, card1_y + card1_h - 14.0],
            "Power: 680W / Node",
            11.0,
            [0.06, 0.72, 0.51, 0.9],
        );

        // Selection Bounding Box with Cyan Grab Handles
        if state.active_mode == StudioMode::Editor {
            let sel_pad = 4.0;
            let bx = card1_x - sel_pad;
            let by = card1_y - sel_pad;
            let bw = card1_w + sel_pad * 2.0;
            let bh = card1_h + sel_pad * 2.0;

            // Cyan selection outline
            p.rect([bx, by, bw, bh], 14.0, None, Some(([0.0, 0.85, 1.0, 1.0], 1.5)));

            // 8 Grab Handles (Corners & Edge midpoints)
            let handle_size = 7.0;
            let handles = [
                [bx - handle_size * 0.5, by - handle_size * 0.5],
                [bx + bw * 0.5 - handle_size * 0.5, by - handle_size * 0.5],
                [bx + bw - handle_size * 0.5, by - handle_size * 0.5],
                [bx - handle_size * 0.5, by + bh * 0.5 - handle_size * 0.5],
                [bx + bw - handle_size * 0.5, by + bh * 0.5 - handle_size * 0.5],
                [bx - handle_size * 0.5, by + bh - handle_size * 0.5],
                [bx + bw * 0.5 - handle_size * 0.5, by + bh - handle_size * 0.5],
                [bx + bw - handle_size * 0.5, by + bh - handle_size * 0.5],
            ];

            for h in handles {
                p.rect(
                    [h[0], h[1], handle_size, handle_size],
                    1.5,
                    Some([0.08, 0.12, 0.2, 1.0]),
                    Some(([0.0, 0.85, 1.0, 1.0], 1.5)),
                );
            }
        }

        // --------------------------------------------------------------------
        // Live Widget 2: Réplication POSIX (Radial Storage Gauge Card)
        // --------------------------------------------------------------------
        let card2_x = card1_x + card1_w + 16.0;
        let card2_y = card1_y;
        let card2_w = surface_w - card1_w - 48.0;
        let card2_h = card1_h;

        p.rect(
            [card2_x, card2_y, card2_w, card2_h],
            12.0,
            Some([0.06, 0.08, 0.12, 0.95]),
            Some(([1.0, 1.0, 1.0, 0.08], 1.0)),
        );

        p.text(
            [card2_x + 16.0, card2_y + 24.0],
            "Réplication POSIX",
            13.0,
            [0.9, 0.95, 1.0, 0.95],
        );
        p.text(
            [card2_x + 16.0, card2_y + 42.0],
            "nvme-pool-stratus-zfs",
            11.0,
            [0.6, 0.7, 0.8, 0.7],
        );

        // Circular Ring Gauge 84%
        let g_cx = card2_x + 60.0;
        let g_cy = card2_y + 115.0;
        let g_r = 38.0;

        // Background ring
        p.circle([g_cx, g_cy], g_r, None, Some(([0.2, 0.3, 0.4, 0.3], 5.0)));
        // Active glowing emerald ring
        p.circle([g_cx, g_cy], g_r, None, Some(([0.06, 0.72, 0.51, 0.9], 5.0)));
        p.text([g_cx - 14.0, g_cy + 5.0], "84%", 14.0, [1.0, 1.0, 1.0, 0.95]);

        // POSIX Stats text
        p.text(
            [card2_x + 115.0, card2_y + 95.0],
            "18.4 TB / 22 TB",
            12.0,
            [0.9, 0.95, 1.0, 0.9],
        );
        p.text(
            [card2_x + 115.0, card2_y + 115.0],
            "I/O: 2.4 GB/s",
            11.0,
            [0.0, 0.85, 1.0, 0.9],
        );
        p.text(
            [card2_x + 115.0, card2_y + 135.0],
            "🟢 AETHER SYNC",
            10.0,
            [0.06, 0.72, 0.51, 1.0],
        );

        // --------------------------------------------------------------------
        // Live Widget 3: MicroVMs Firecracker Actives Table
        // --------------------------------------------------------------------
        let table_x = card1_x;
        let table_y = card1_y + card1_h + 16.0;
        let table_w = surface_w - 32.0;
        let table_h = surface_h - card1_h - 70.0;

        p.rect(
            [table_x, table_y, table_w, table_h],
            12.0,
            Some([0.06, 0.08, 0.12, 0.95]),
            Some(([1.0, 1.0, 1.0, 0.08], 1.0)),
        );

        // Table Title & Badge
        p.text(
            [table_x + 16.0, table_y + 24.0],
            "MicroVMs Firecracker Actives",
            13.0,
            [0.9, 0.95, 1.0, 0.95],
        );
        p.rect(
            [table_x + 220.0, table_y + 12.0, 85.0, 18.0],
            4.0,
            Some([0.0, 0.85, 1.0, 0.15]),
            Some(([0.0, 0.85, 1.0, 0.6], 1.0)),
        );
        p.text(
            [table_x + 226.0, table_y + 24.0],
            "3/3 En ligne",
            10.0,
            [0.0, 0.85, 1.0, 1.0],
        );

        // Table Header row
        let col_y = table_y + 48.0;
        p.text([table_x + 16.0, col_y], "VM ID", 11.0, [0.6, 0.7, 0.8, 0.7]);
        p.text([table_x + 140.0, col_y], "RÔLE / IMAGE OS", 11.0, [0.6, 0.7, 0.8, 0.7]);
        p.text([table_x + 360.0, col_y], "VCPU EPYC", 11.0, [0.6, 0.7, 0.8, 0.7]);
        p.text([table_x + 480.0, col_y], "MÉMOIRE", 11.0, [0.6, 0.7, 0.8, 0.7]);
        p.text([table_x + 600.0, col_y], "STATUT", 11.0, [0.6, 0.7, 0.8, 0.7]);

        p.line(
            [table_x + 16.0, col_y + 8.0],
            [table_x + table_w - 16.0, col_y + 8.0],
            1.0,
            [1.0, 1.0, 1.0, 0.06],
        );

        // Rows
        for (idx, vm) in state.micro_vms.iter().enumerate() {
            let row_y = col_y + 28.0 + idx as f32 * 26.0;
            p.text([table_x + 16.0, row_y], &vm.id, 11.0, [0.0, 0.85, 1.0, 0.95]);
            p.text([table_x + 140.0, row_y], &vm.role, 11.0, [0.9, 0.95, 1.0, 0.9]);
            p.text([table_x + 360.0, row_y], &vm.vcpu, 11.0, [0.6, 0.7, 0.8, 0.9]);
            p.text([table_x + 480.0, row_y], &vm.memory, 11.0, [0.6, 0.7, 0.8, 0.9]);
            p.text([table_x + 600.0, row_y], &format!("🟢 {}", vm.status), 11.0, [0.06, 0.72, 0.51, 1.0]);
        }

        tree.custom_paint(
            WidgetId::new("center_canvas_paint"),
            p.finish(),
            leaf(canvas_w, canvas_h),
        )
        .unwrap_or_else(|_| NodeId::from(0usize))
    };

    // Main 3-Column Horizontal Layout
    let center_row = tree
        .container(&[left_sidebar, center_canvas_node, right_sidebar], row(0.0))
        .unwrap_or_else(|_| NodeId::from(0usize));

    // ------------------------------------------------------------------------
    // BOTTOM STATUS BAR & FLOATING DOCK
    // ------------------------------------------------------------------------
    let bottom_status_bar = {
        // Left Status Indicators
        let dsys_lbl = tree
            .label("🟢 Design System AetherOS Conforme", leaf(230.0, 18.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let lint_lbl = tree
            .label("🟢 0 erreurs de linting", leaf(140.0, 18.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let comp_count_lbl = tree
            .label_muted("14 composants dans la scène", leaf(170.0, 18.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let left_status = tree
            .container(&[dsys_lbl, lint_lbl, comp_count_lbl], row(10.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        // Center Floating Dock (macOS-style pill container)
        let dock_items = [
            ("dock_grid", "⊞"),
            ("dock_folder", "📁"),
            ("dock_cam", "📷"),
            ("dock_analytics", "📊"),
            ("dock_nodes", "🕸️"),
            ("dock_settings", "⚙️"),
            ("dock_trash", "🗑️"),
        ];

        let mut dock_nodes = Vec::new();
        for (id, icon) in dock_items {
            let btn = tree
                .button_variant(WidgetId::new(id), icon, ButtonVariant::Ghost, true, leaf(28.0, 24.0))
                .unwrap_or_else(|_| NodeId::from(0usize));
            dock_nodes.push(btn);
        }

        let center_dock = tree
            .container(&dock_nodes, row(4.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        // Right Grid & Zoom Status
        let grid_lbl = tree
            .label_muted("Magnétisme Grille: Activé (8px)", leaf(190.0, 18.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let zoom_stat_lbl = tree
            .label_muted("Zoom: 75%", leaf(75.0, 18.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let utf_lbl = tree
            .label_muted("UTF-8", leaf(50.0, 18.0))
            .unwrap_or_else(|_| NodeId::from(0usize));
        let vmm_lbl = tree
            .label("Stratus Cloud VMM", leaf(120.0, 18.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let right_status = tree
            .container(&[grid_lbl, zoom_stat_lbl, utf_lbl, vmm_lbl], row(8.0))
            .unwrap_or_else(|_| NodeId::from(0usize));

        let mut bar_style = row(0.0);
        bar_style.size = Size {
            width: length(width),
            height: length(32.0),
        };
        bar_style.justify_content = Some(JustifyContent::SpaceBetween);
        bar_style.padding = Rect {
            left: length(14.0),
            right: length(14.0),
            top: length(4.0),
            bottom: length(4.0),
        };

        tree.container(&[left_status, center_dock, right_status], bar_style)
            .unwrap_or_else(|_| NodeId::from(0usize))
    };

    // Root Master Container
    let mut root_style = column(0.0);
    root_style.size = Size {
        width: length(width),
        height: length(height),
    };

    tree.container(&[top_os_bar, studio_ribbon, center_row, bottom_status_bar], root_style)
        .unwrap_or_else(|_| NodeId::from(0usize))
}

// ============================================================================
// Application & Event Loop Handler
// ============================================================================

struct StudioApp {
    state: StudioState,
    theme: Arc<RwLock<Theme>>,
    window: Option<Arc<Window>>,
    renderer: Option<GpuRenderer>,
    tree: WidgetTree,
    root: Option<NodeId>,
    cursor_pos: (f32, f32),
    pressed: Option<InteractionKey>,
    resources: ResourceTable,
}

impl StudioApp {
    fn new() -> Self {
        let theme = Theme::cyber_glass();
        let state = StudioState::new();

        Self {
            state,
            theme: Arc::new(RwLock::new(theme)),
            window: None,
            renderer: None,
            tree: WidgetTree::new(),
            root: None,
            cursor_pos: (0.0, 0.0),
            pressed: None,
            resources: ResourceTable::new(),
        }
    }

    fn handle_ui_action(&mut self, id: &str) {
        match id {
            "tool_v" => self.state.active_tool = StudioTool::Select,
            "tool_h" => self.state.active_tool = StudioTool::Hand,
            "tool_f" => self.state.active_tool = StudioTool::Frame,
            "tool_c" => self.state.active_tool = StudioTool::Component,
            "tool_t" => self.state.active_tool = StudioTool::Text,
            "tool_a" => self.state.active_tool = StudioTool::Asset,

            "mode_editor" => self.state.active_mode = StudioMode::Editor,
            "mode_preview" => self.state.active_mode = StudioMode::Preview,
            "mode_jsx" => self.state.active_mode = StudioMode::Jsx,

            "tab_comp" => self.state.left_tab = LeftTab::Components,
            "tab_layers" => self.state.left_tab = LeftTab::Layers,

            "publish_btn" => {
                self.state.trigger_toast(
                    "Déploiement OS",
                    "ClusterMonitoringDashboard publié avec succès sur Stratus Cloud Edge FRA-1 !",
                    ToastKind::Success,
                );
            }
            "copy_jsx" => {
                self.state.trigger_toast(
                    "Code JSX Copié",
                    "Le composant AetherCard a été copié dans le presse-papier.",
                    ToastKind::Info,
                );
            }
            "undo_btn" => {
                self.state.trigger_toast("Undo", "Action précédente annulée.", ToastKind::Info);
            }
            "redo_btn" => {
                self.state.trigger_toast("Redo", "Action rétablie.", ToastKind::Info);
            }
            "zoom_btn" => {
                self.state.zoom_level = if self.state.zoom_level >= 1.0 { 0.5 } else { self.state.zoom_level + 0.25 };
            }
            "flex_h" => self.state.flex_direction_row = true,
            "flex_v" => self.state.flex_direction_row = false,
            _ => {
                if id.starts_with("layer_") {
                    if let Ok(idx) = id.trim_start_matches("layer_").parse::<usize>() {
                        for (i, layer) in self.state.layers.iter_mut().enumerate() {
                            layer.is_selected = i == idx;
                        }
                    }
                }
            }
        }
    }

    fn handle_press(&mut self) {
        let Some(root) = self.root else { return };
        self.pressed = self
            .tree
            .interaction_key_at(root, self.cursor_pos)
            .unwrap_or(None);
    }

    fn handle_release(&mut self) {
        if let Some(root) = self.root {
            let released_on = self
                .tree
                .interaction_key_at(root, self.cursor_pos)
                .unwrap_or(None);
            if self.pressed.is_some() && self.pressed == released_on {
                if let Ok(Some(ev)) = self.tree.dispatch_click(root, self.cursor_pos) {
                    if let UiEvent::ButtonClicked { widget_id } = ev {
                        self.handle_ui_action(&widget_id);
                    }
                }
            }
        }
        self.pressed = None;
    }

    fn redraw(&mut self) {
        let Some(window) = &self.window else { return };
        let Some(renderer) = &mut self.renderer else {
            return;
        };

        self.state.update_animations();

        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        let w = size.width as f32;
        let h = size.height as f32;
        renderer.background_params_mut().screen_size = [w, h];

        let available = Size {
            width: AvailableSpace::Definite(w),
            height: AvailableSpace::Definite(h),
        };

        let current_theme = self.theme.read().unwrap().clone();
        let measure = renderer.text_measure();

        // Build UI tree
        let mut base_tree = WidgetTree::new();
        let base_root = build_studio_ui(&mut base_tree, &self.state, w, h);
        if let Err(err) = base_tree.compute(base_root, available) {
            eprintln!("[aorui_studio] base compute failed: {err}");
            return;
        }

        let base_hovered = base_tree
            .interaction_key_at(base_root, self.cursor_pos)
            .unwrap_or(None);
        let base_interaction = InteractionState {
            hovered: base_hovered.as_ref(),
            pressed: self.pressed.as_ref(),
            measure: Some(&measure),
        };

        let base_frame = match base_tree.build_frame(base_root, &current_theme, base_interaction) {
            Ok(f) => f,
            Err(err) => {
                eprintln!("[aorui_studio] base build_frame failed: {err}");
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

        let base_layer = RenderLayer {
            instances: &base_frame.instances,
            texts: &base_text_runs,
        };

        let media: Vec<MediaInstance> = base_frame
            .media
            .iter()
            .map(|spec| {
                let fit_mode = match spec.fit {
                    ui_widgets::MediaFit::Fill => 0,
                    ui_widgets::MediaFit::Contain => 1,
                    ui_widgets::MediaFit::Cover => 2,
                };
                MediaInstance {
                    kind: spec.kind.0.to_string(),
                    resource_id: spec.resource_id.clone(),
                    bounds: spec.bounds,
                    clip_bounds: spec.clip,
                    radius: spec.radius,
                    fit_mode,
                }
            })
            .collect();

        let background = wgpu::Color {
            r: 0.02,
            g: 0.04,
            b: 0.07,
            a: 1.0,
        };

        let render_res = renderer.render_layers(background, &[base_layer], &media, &self.resources);
        if let Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) = render_res {
            renderer.resize(size);
        }

        self.tree = base_tree;
        self.root = Some(base_root);
    }
}

impl ApplicationHandler for StudioApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = WindowAttributes::default()
            .with_title("Stratus GUI Studio - AetherOS Interface Designer")
            .with_inner_size(winit::dpi::PhysicalSize::new(
                WINDOW_WIDTH as u32,
                WINDOW_HEIGHT as u32,
            ));

        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        let mut renderer = GpuRenderer::new(window.clone());
        let mut bg_params = BackgroundParams::aether_os(WINDOW_WIDTH, WINDOW_HEIGHT);
        bg_params.grid_dot_size = 0.0;
        bg_params.grid_opacity = 0.0;
        bg_params.base_color = [0.005, 0.008, 0.016, 1.0];
        bg_params.grad1_color = [0.096, 0.106, 0.718, 0.20];
        bg_params.grad2_color = [0.005, 0.035, 0.140, 0.16];
        renderer.set_background_params(bg_params);

        self.window = Some(window);
        self.renderer = Some(renderer);
        event_loop.set_control_flow(ControlFlow::Poll);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if self.state.close_requested {
            event_loop.exit();
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    if let Some(renderer) = &mut self.renderer {
                        renderer.resize(size);
                    }
                    if let Some(w) = &self.window {
                        w.request_redraw();
                    }
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(w) = &self.window {
                    let size = w.inner_size();
                    if size.width > 0 && size.height > 0 {
                        if let Some(renderer) = &mut self.renderer {
                            renderer.resize(size);
                        }
                        w.request_redraw();
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = (position.x as f32, position.y as f32);
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                self.handle_press();
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                self.handle_release();
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        ref logical_key,
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                if let Key::Named(NamedKey::Escape) = logical_key {
                    event_loop.exit();
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                self.redraw();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }
}

fn main() {
    println!("[aorui_studio] Starting Stratus GUI Studio Interface Designer...");
    let event_loop = EventLoop::new().unwrap();
    let mut app = StudioApp::new();
    event_loop.run_app(&mut app).unwrap();
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_studio_initial_state() {
        let state = StudioState::new();
        assert_eq!(state.active_tool, StudioTool::Select);
        assert_eq!(state.active_mode, StudioMode::Editor);
        assert_eq!(state.layers.len(), 4);
        assert_eq!(state.micro_vms.len(), 3);
    }

    #[test]
    fn test_studio_ui_tree_generation() {
        let mut tree = WidgetTree::new();
        let state = StudioState::new();
        let root = build_studio_ui(&mut tree, &state, WINDOW_WIDTH, WINDOW_HEIGHT);
        let available = Size {
            width: AvailableSpace::Definite(WINDOW_WIDTH),
            height: AvailableSpace::Definite(WINDOW_HEIGHT),
        };
        assert!(tree.compute(root, available).is_ok());
    }
}
