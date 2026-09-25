// [WFGY] Zone: SAFE | λ: 0.15 | Fallbacks: 0 | Action: Clean Architecture Stratus Studio (AORUI Modern Visual Form Designer)
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use ui_gpu::GpuRenderer;
use ui_layout::{
    auto, length, percent, AlignItems, AvailableSpace, FlexDirection, LayoutError, NodeId,
    Position, Rect, Size, Style,
};
use ui_widgets::declarative::{DeclarativeUiDoc, LayoutStyleSpec, WidgetNodeSpec};
use ui_widgets::{ButtonVariant, FontFamily, InteractionState, ListItemBadge, Theme, ThemeWatcher, WidgetTree};

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

/// Resize handle position around a selected component
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeHandle {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
}

/// Target property for color modification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorTarget {
    Background,
    Border,
    Text,
}

/// Active drag interaction mode
#[derive(Debug, Clone)]
pub enum DragMode {
    MoveWidget {
        widget_id: String,
        initial_left: f32,
        initial_top: f32,
    },
    ResizeWidget {
        widget_id: String,
        handle: ResizeHandle,
        initial_left: f32,
        initial_top: f32,
        initial_width: f32,
        initial_height: f32,
    },
    ToolboxDrop {
        widget_type: String,
        default_label: String,
        variant: Option<String>,
    },
}

/// Runtime drag session
#[derive(Debug, Clone)]
pub struct DragSession {
    pub mode: DragMode,
    pub start_mouse_pos: (f32, f32),
    pub current_mouse_pos: (f32, f32),
    pub is_active: bool,
}

/// Clean Studio Working State
pub struct DesignerState {
    pub doc: DeclarativeUiDoc,
    pub selected_widget_id: Option<String>,
    pub status: String,
    pub next_id_counter: usize,
    pub active_toolbox_tab: String,
    pub drag_session: Option<DragSession>,
    pub color_target: ColorTarget,

    // Artboard configuration
    pub artboard_width: f32,
    pub artboard_height: f32,
    pub grid_snap_size: f32,
    pub snap_to_grid: bool,
    pub active_preset_name: String,
    pub artboard_node_id: Option<NodeId>,
    pub artboard_screen_rect: [f32; 4],
}

impl DesignerState {
    pub fn snap_val(&self, val: f32) -> f32 {
        if self.snap_to_grid && self.grid_snap_size > 0.0 {
            (val / self.grid_snap_size).round() * self.grid_snap_size
        } else {
            val
        }
    }

    pub fn get_active_color(&self, theme: &Theme) -> [f32; 4] {
        if let Some(sel_id) = &self.selected_widget_id {
            if let Some(node) = self.doc.nodes.iter().find(|n| &n.id == sel_id) {
                let col = match self.color_target {
                    ColorTarget::Background => node.bg,
                    ColorTarget::Border => node.border,
                    ColorTarget::Text => node.color,
                };
                return col.unwrap_or(match self.color_target {
                    ColorTarget::Background => theme.card_bg(),
                    ColorTarget::Border => theme.border_subtle(),
                    ColorTarget::Text => theme.text_color,
                });
            }
        }
        theme.card_bg()
    }

    pub fn set_active_color(&mut self, new_col: Option<[f32; 4]>) {
        if let Some(sel_id) = &self.selected_widget_id {
            if let Some(node) = self.doc.nodes.iter_mut().find(|n| &n.id == sel_id) {
                match self.color_target {
                    ColorTarget::Background => node.bg = new_col,
                    ColorTarget::Border => node.border = new_col,
                    ColorTarget::Text => node.color = new_col,
                }
            }
        }
    }
}

pub struct FormDesignerApp {
    window: Option<Arc<Window>>,
    renderer: Option<GpuRenderer>,
    resources: ui_gpu::ResourceTable,
    theme: Theme,
    theme_shared: Arc<RwLock<Theme>>,
    _theme_watcher: Option<ThemeWatcher>,

    state: DesignerState,
    last_tree: Option<WidgetTree>,
    last_root: Option<NodeId>,
    last_mouse_pos: Option<(f32, f32)>,
}

impl Default for FormDesignerApp {
    fn default() -> Self {
        Self::new()
    }
}

impl FormDesignerApp {
    pub fn new() -> Self {
        let theme_path = PathBuf::from("themes/studio_pro.toml");
        let initial_theme = Theme::from_file(&theme_path).unwrap_or_else(|_| Theme::aether_os());
        let theme_shared = Arc::new(RwLock::new(initial_theme.clone()));

        let watcher_target = theme_shared.clone();
        let _theme_watcher = ThemeWatcher::watch_file(&theme_path, watcher_target, |_new_theme| {
            println!("⚡ [ThemeWatcher] Rechargement à chaud du thème validé.");
        })
        .ok();

        // Initial default document with clean Stratus sample widgets
        let mut initial_doc = DeclarativeUiDoc::default();
        initial_doc.window.title = "Stratus OS Studio Project".to_string();
        initial_doc.window.width = 960.0;
        initial_doc.window.height = 620.0;
        initial_doc.root = "artboard_root".to_string();

        initial_doc.nodes = vec![
            WidgetNodeSpec {
                id: "artboard_root".to_string(),
                widget_type: "Panel".to_string(),
                layout: LayoutStyleSpec {
                    position: Some("relative".to_string()),
                    width_percent: Some(100.0),
                    height_percent: Some(100.0),
                    ..Default::default()
                },
                children: vec![
                    "menubar_top".to_string(),
                    "card_stats_gpu".to_string(),
                    "btn_action_primary".to_string(),
                    "btn_action_secondary".to_string(),
                    "txt_search".to_string(),
                    "slider_volume".to_string(),
                    "toggle_status".to_string(),
                    "modal_dialog".to_string(),
                ],
                text: None,
                label: None,
                placeholder: None,
                variant: None,
                enabled: Some(true),
                checked: None,
                value: None,
                min: None,
                max: None,
                step: None,
                muted: None,
                radius: Some(10.0),
                icon: None,
                title: None,
                subtitle: None,
                delta: None,
                delta_positive: None,
                group_id: None,
                items: None,
                active_index: None,
                bg: None,
                border: None,
                color: None,
                on_click: None,
                on_change: None,
            },
            // 1. MenuBar
            WidgetNodeSpec {
                id: "menubar_top".to_string(),
                widget_type: "MenuBar".to_string(),
                layout: LayoutStyleSpec {
                    position: Some("absolute".to_string()),
                    left: Some(24.0),
                    top: Some(20.0),
                    width: Some(912.0),
                    height: Some(36.0),
                    ..Default::default()
                },
                children: Vec::new(),
                text: None,
                label: None,
                placeholder: None,
                variant: None,
                enabled: Some(true),
                checked: None,
                value: None,
                min: None,
                max: None,
                step: None,
                muted: None,
                radius: Some(6.0),
                icon: None,
                title: None,
                subtitle: None,
                delta: None,
                delta_positive: None,
                group_id: None,
                items: Some(vec![
                    "Fichier".to_string(),
                    "Édition".to_string(),
                    "Affichage".to_string(),
                    "Outils".to_string(),
                    "Aide".to_string(),
                ]),
                active_index: Some(0),
                bg: None,
                border: None,
                color: None,
                on_click: None,
                on_change: None,
            },
            // 2. Metric Stat Card
            WidgetNodeSpec {
                id: "card_stats_gpu".to_string(),
                widget_type: "MetricCard".to_string(),
                layout: LayoutStyleSpec {
                    position: Some("absolute".to_string()),
                    left: Some(24.0),
                    top: Some(72.0),
                    width: Some(260.0),
                    height: Some(96.0),
                    ..Default::default()
                },
                children: Vec::new(),
                text: Some("14.2 GFlops".to_string()),
                label: None,
                placeholder: None,
                variant: None,
                enabled: Some(true),
                checked: None,
                value: None,
                min: None,
                max: None,
                step: None,
                muted: None,
                radius: Some(8.0),
                icon: None,
                title: Some("GPU Compute Engine".to_string()),
                subtitle: None,
                delta: Some("+8.4%".to_string()),
                delta_positive: Some(true),
                group_id: None,
                items: None,
                active_index: None,
                bg: None,
                border: None,
                color: None,
                on_click: None,
                on_change: None,
            },
            // 3. Primary Button
            WidgetNodeSpec {
                id: "btn_action_primary".to_string(),
                widget_type: "Button".to_string(),
                layout: LayoutStyleSpec {
                    position: Some("absolute".to_string()),
                    left: Some(304.0),
                    top: Some(72.0),
                    width: Some(190.0),
                    height: Some(42.0),
                    ..Default::default()
                },
                children: Vec::new(),
                text: None,
                label: Some("Lancer Shader 🚀".to_string()),
                placeholder: None,
                variant: Some("Primary".to_string()),
                enabled: Some(true),
                checked: None,
                value: None,
                min: None,
                max: None,
                step: None,
                muted: None,
                radius: Some(8.0),
                icon: None,
                title: None,
                subtitle: None,
                delta: None,
                delta_positive: None,
                group_id: None,
                items: None,
                active_index: None,
                bg: None,
                border: None,
                color: None,
                on_click: Some("app:on_launch_shader".to_string()),
                on_change: None,
            },
            // 4. Secondary Button
            WidgetNodeSpec {
                id: "btn_action_secondary".to_string(),
                widget_type: "Button".to_string(),
                layout: LayoutStyleSpec {
                    position: Some("absolute".to_string()),
                    left: Some(510.0),
                    top: Some(72.0),
                    width: Some(180.0),
                    height: Some(42.0),
                    ..Default::default()
                },
                children: Vec::new(),
                text: None,
                label: Some("Capture Frame".to_string()),
                placeholder: None,
                variant: Some("Secondary".to_string()),
                enabled: Some(true),
                checked: None,
                value: None,
                min: None,
                max: None,
                step: None,
                muted: None,
                radius: Some(8.0),
                icon: None,
                title: None,
                subtitle: None,
                delta: None,
                delta_positive: None,
                group_id: None,
                items: None,
                active_index: None,
                bg: None,
                border: None,
                color: None,
                on_click: Some("app:on_capture_frame".to_string()),
                on_change: None,
            },
            // 5. Search Text Input
            WidgetNodeSpec {
                id: "txt_search".to_string(),
                widget_type: "TextInput".to_string(),
                layout: LayoutStyleSpec {
                    position: Some("absolute".to_string()),
                    left: Some(304.0),
                    top: Some(128.0),
                    width: Some(386.0),
                    height: Some(38.0),
                    ..Default::default()
                },
                children: Vec::new(),
                text: Some("Filter nodes...".to_string()),
                label: None,
                placeholder: Some("Rechercher un composant...".to_string()),
                variant: None,
                enabled: Some(true),
                checked: None,
                value: None,
                min: None,
                max: None,
                step: None,
                muted: None,
                radius: Some(6.0),
                icon: None,
                title: None,
                subtitle: None,
                delta: None,
                delta_positive: None,
                group_id: None,
                items: None,
                active_index: None,
                bg: None,
                border: None,
                color: None,
                on_click: None,
                on_change: Some("app:on_filter_nodes".to_string()),
            },
            // 6. Slider
            WidgetNodeSpec {
                id: "slider_volume".to_string(),
                widget_type: "Slider".to_string(),
                layout: LayoutStyleSpec {
                    position: Some("absolute".to_string()),
                    left: Some(24.0),
                    top: Some(184.0),
                    width: Some(260.0),
                    height: Some(28.0),
                    ..Default::default()
                },
                children: Vec::new(),
                text: None,
                label: None,
                placeholder: None,
                variant: None,
                enabled: Some(true),
                checked: None,
                value: Some(65.0),
                min: Some(0.0),
                max: Some(100.0),
                step: Some(1.0),
                muted: None,
                radius: None,
                icon: None,
                title: None,
                subtitle: None,
                delta: None,
                delta_positive: None,
                group_id: None,
                items: None,
                active_index: None,
                bg: None,
                border: None,
                color: None,
                on_click: None,
                on_change: Some("app:on_change_volume".to_string()),
            },
            // 7. Toggle Switch
            WidgetNodeSpec {
                id: "toggle_status".to_string(),
                widget_type: "Toggle".to_string(),
                layout: LayoutStyleSpec {
                    position: Some("absolute".to_string()),
                    left: Some(304.0),
                    top: Some(184.0),
                    width: Some(54.0),
                    height: Some(28.0),
                    ..Default::default()
                },
                children: Vec::new(),
                text: None,
                label: None,
                placeholder: None,
                variant: None,
                enabled: Some(true),
                checked: Some(true),
                value: None,
                min: None,
                max: None,
                step: None,
                muted: None,
                radius: None,
                icon: None,
                title: None,
                subtitle: None,
                delta: None,
                delta_positive: None,
                group_id: None,
                items: None,
                active_index: None,
                bg: None,
                border: None,
                color: None,
                on_click: None,
                on_change: Some("app:on_toggle_status".to_string()),
            },
            // 8. Modal Dialog Window
            WidgetNodeSpec {
                id: "modal_dialog".to_string(),
                widget_type: "Window".to_string(),
                layout: LayoutStyleSpec {
                    position: Some("absolute".to_string()),
                    left: Some(380.0),
                    top: Some(230.0),
                    width: Some(360.0),
                    height: Some(200.0),
                    direction: Some("column".to_string()),
                    padding: Some(16.0),
                    gap: Some(10.0),
                    ..Default::default()
                },
                children: vec!["lbl_modal_msg".to_string()],
                text: None,
                label: None,
                placeholder: None,
                variant: None,
                enabled: Some(true),
                checked: None,
                value: None,
                min: None,
                max: None,
                step: None,
                muted: None,
                radius: Some(10.0),
                icon: None,
                title: Some("Fenêtre Dialogue Modale".to_string()),
                subtitle: None,
                delta: None,
                delta_positive: None,
                group_id: None,
                items: None,
                active_index: None,
                bg: None,
                border: None,
                color: None,
                on_click: None,
                on_change: None,
            },
            WidgetNodeSpec {
                id: "lbl_modal_msg".to_string(),
                widget_type: "Label".to_string(),
                layout: LayoutStyleSpec {
                    height: Some(24.0),
                    ..Default::default()
                },
                children: Vec::new(),
                text: Some("Composant modal interactif rendu en GPU.".to_string()),
                label: None,
                placeholder: None,
                variant: None,
                enabled: Some(true),
                checked: None,
                value: None,
                min: None,
                max: None,
                step: None,
                muted: Some(false),
                radius: None,
                icon: None,
                title: None,
                subtitle: None,
                delta: None,
                delta_positive: None,
                group_id: None,
                items: None,
                active_index: None,
                bg: None,
                border: None,
                color: None,
                on_click: None,
                on_change: None,
            },
        ];

        Self {
            window: None,
            renderer: None,
            resources: ui_gpu::ResourceTable::new(),
            theme: initial_theme,
            theme_shared,
            _theme_watcher,
            state: DesignerState {
                doc: initial_doc,
                selected_widget_id: None,
                status: "Studio prêt. Sélectionnez un composant ou glissez depuis la palette.".to_string(),
                next_id_counter: 0,
                active_toolbox_tab: "Controls".to_string(),
                drag_session: None,
                color_target: ColorTarget::Background,
                artboard_width: 960.0,
                artboard_height: 620.0,
                grid_snap_size: 16.0,
                snap_to_grid: true,
                active_preset_name: "Desktop HD (960×620)".to_string(),
                artboard_node_id: None,
                artboard_screen_rect: [0.0, 0.0, 960.0, 620.0],
            },
            last_tree: None,
            last_root: None,
            last_mouse_pos: None,
        }
    }

    fn font_family(family: &FontFamily) -> glyphon::Family<'_> {
        match family {
            FontFamily::SansSerif => glyphon::Family::SansSerif,
            FontFamily::Serif => glyphon::Family::Serif,
            FontFamily::Monospace => glyphon::Family::Monospace,
            FontFamily::Named(name) => glyphon::Family::Name(name.as_str()),
        }
    }

    pub fn add_widget_at_coords(
        state: &mut DesignerState,
        widget_type: &str,
        default_label: &str,
        variant: Option<&str>,
        art_x: f32,
        art_y: f32,
    ) {
        state.next_id_counter += 1;
        let prefix = match widget_type.to_lowercase().as_str() {
            "button" => "btn",
            "label" => "lbl",
            "textinput" => "txt",
            "slider" => "slider",
            "toggle" => "toggle",
            "card" => "card",
            "panel" => "panel",
            "menubar" => "menu",
            "metriccard" => "metric",
            "badge" => "badge",
            "window" | "modal" => "modal",
            "dropdown" => "drop",
            "toast" => "toast",
            _ => "widget",
        };
        let new_id = format!("{}_{}", prefix, state.next_id_counter);
        let snapped_x = state.snap_val(art_x);
        let snapped_y = state.snap_val(art_y);

        let mut new_node = WidgetNodeSpec {
            id: new_id.clone(),
            widget_type: widget_type.to_string(),
            layout: LayoutStyleSpec {
                position: Some("absolute".to_string()),
                left: Some(snapped_x),
                top: Some(snapped_y),
                width: Some(180.0),
                height: Some(38.0),
                ..Default::default()
            },
            children: Vec::new(),
            text: None,
            label: Some(default_label.to_string()),
            placeholder: None,
            variant: variant.map(|v| v.to_string()),
            enabled: Some(true),
            checked: None,
            value: None,
            min: None,
            max: None,
            step: None,
            muted: None,
            radius: Some(8.0),
            icon: None,
            title: None,
            subtitle: None,
            delta: None,
            delta_positive: None,
            group_id: None,
            items: None,
            active_index: None,
            bg: None,
            border: None,
            color: None,
            on_click: None,
            on_change: None,
        };

        match widget_type.to_lowercase().as_str() {
            "menubar" => {
                new_node.items = Some(vec![
                    "Fichier".to_string(),
                    "Édition".to_string(),
                    "Affichage".to_string(),
                    "Outils".to_string(),
                    "Aide".to_string(),
                ]);
                new_node.active_index = Some(0);
                new_node.layout.width = Some(540.0);
                new_node.layout.height = Some(36.0);
            }
            "window" | "modal" => {
                new_node.title = Some(default_label.to_string());
                new_node.layout.width = Some(360.0);
                new_node.layout.height = Some(200.0);
                new_node.layout.padding = Some(16.0);
            }
            "textinput" => {
                new_node.placeholder = Some("Saisir du texte...".to_string());
                new_node.text = Some(String::new());
                new_node.layout.width = Some(260.0);
                new_node.layout.height = Some(38.0);
            }
            "slider" => {
                new_node.value = Some(50.0);
                new_node.min = Some(0.0);
                new_node.max = Some(100.0);
                new_node.layout.width = Some(220.0);
                new_node.layout.height = Some(28.0);
            }
            "metriccard" => {
                new_node.title = Some("GPU Metric".to_string());
                new_node.text = Some("98.4 %".to_string());
                new_node.delta = Some("+4.2%".to_string());
                new_node.delta_positive = Some(true);
                new_node.layout.width = Some(240.0);
                new_node.layout.height = Some(84.0);
            }
            "card" => {
                new_node.layout.width = Some(320.0);
                new_node.layout.height = Some(180.0);
            }
            "panel" => {
                new_node.layout.width = Some(340.0);
                new_node.layout.height = Some(200.0);
            }
            "toggle" => {
                new_node.checked = Some(true);
                new_node.layout.width = Some(54.0);
                new_node.layout.height = Some(28.0);
            }
            "badge" => {
                new_node.text = Some(default_label.to_string());
                new_node.variant = Some("Success".to_string());
                new_node.layout.width = Some(140.0);
                new_node.layout.height = Some(24.0);
            }
            "dropdown" => {
                new_node.label = Some("Sélectionner".to_string());
                new_node.text = Some("Option Active".to_string());
                new_node.layout.width = Some(220.0);
                new_node.layout.height = Some(34.0);
            }
            "toast" => {
                new_node.title = Some("Notification Système".to_string());
                new_node.text = Some("Opération effectuée avec succès.".to_string());
                new_node.layout.width = Some(300.0);
                new_node.layout.height = Some(60.0);
            }
            _ => {}
        }

        if let Some(root_node) = state.doc.nodes.iter_mut().find(|n| n.id == state.doc.root) {
            root_node.children.push(new_id.clone());
        }

        state.doc.nodes.push(new_node);
        state.selected_widget_id = Some(new_id.clone());
        state.status = format!("➕ Composant '{}' ({}) placé à X: {:.0}, Y: {:.0} !", new_id, widget_type, snapped_x, snapped_y);
    }

    /// Assembles the interactive artboard canvas
    fn build_artboard_canvas(&mut self, tree: &mut WidgetTree) -> Result<NodeId, LayoutError> {
        let mut artboard_child_nodes = Vec::new();

        let nodes_snapshot = self.state.doc.nodes.clone();
        for child in &nodes_snapshot {
            if child.id != self.state.doc.root {
                let is_selected = self.state.selected_widget_id.as_deref() == Some(&child.id);

                let child_style = Style {
                    position: match child.layout.position.as_deref() {
                        Some("absolute") => Position::Absolute,
                        _ => Position::Relative,
                    },
                    inset: Rect {
                        left: child.layout.left.map(length).unwrap_or(auto()),
                        top: child.layout.top.map(length).unwrap_or(auto()),
                        right: child.layout.right.map(length).unwrap_or(auto()),
                        bottom: child.layout.bottom.map(length).unwrap_or(auto()),
                    },
                    size: Size {
                        width: child.layout.width.map(length).unwrap_or(auto()),
                        height: child.layout.height.map(length).unwrap_or(auto()),
                    },
                    flex_direction: match child.layout.direction.as_deref() {
                        Some("row") => FlexDirection::Row,
                        _ => FlexDirection::Column,
                    },
                    padding: Rect {
                        top: child.layout.padding.map(length).unwrap_or(length(0.0)),
                        bottom: child.layout.padding.map(length).unwrap_or(length(0.0)),
                        left: child.layout.padding.map(length).unwrap_or(length(0.0)),
                        right: child.layout.padding.map(length).unwrap_or(length(0.0)),
                    },
                    gap: Size {
                        width: child.layout.gap.map(length).unwrap_or(length(0.0)),
                        height: child.layout.gap.map(length).unwrap_or(length(0.0)),
                    },
                    ..Default::default()
                };

                let widget_inner = match child.widget_type.to_lowercase().as_str() {
                    "menubar" => {
                        let items_vec = child.items.clone().unwrap_or_default();
                        let active = child.active_index.unwrap_or(0);
                        let mut item_nodes = Vec::new();
                        for (idx, itm) in items_vec.iter().enumerate() {
                            let item_btn = tree.button_variant(
                                format!("{}_itm_{}", child.id, idx),
                                itm,
                                if idx == active { ButtonVariant::Primary } else { ButtonVariant::Ghost },
                                true,
                                Style { size: Size { width: length(70.0), height: length(26.0) }, ..Default::default() },
                            )?;
                            item_nodes.push(item_btn);
                        }
                        tree.panel(
                            &item_nodes,
                            child.bg.or(Some(self.theme.card_bg())),
                            if is_selected { Some(self.theme.border_highlight()) } else { child.border.or(Some(self.theme.border_subtle())) },
                            Style {
                                flex_direction: FlexDirection::Row,
                                align_items: Some(AlignItems::Center),
                                gap: Size { width: length(4.0), height: length(0.0) },
                                padding: Rect { top: length(2.0), bottom: length(2.0), left: length(6.0), right: length(6.0) },
                                ..child_style
                            },
                        )?
                    }
                    "window" | "modal" => {
                        let title = child.title.as_deref().unwrap_or("Window");
                        let title_lbl = tree.label(
                            title,
                            Style { size: Size { width: auto(), height: length(22.0) }, ..Default::default() },
                        )?;
                        let close_btn = tree.button_variant(
                            format!("{}_close", child.id),
                            "✕",
                            ButtonVariant::Ghost,
                            true,
                            Style { size: Size { width: length(22.0), height: length(22.0) }, ..Default::default() },
                        )?;
                        let win_header = tree.container(
                            &[title_lbl, close_btn],
                            Style {
                                flex_direction: FlexDirection::Row,
                                justify_content: Some(ui_layout::JustifyContent::SpaceBetween),
                                align_items: Some(AlignItems::Center),
                                size: Size { width: percent(1.0), height: length(26.0) },
                                ..Default::default()
                            },
                        )?;

                        let content_lbl = tree.label_muted(
                            "Contenu interne de la fenêtre modale.",
                            Style { size: Size { width: percent(1.0), height: length(40.0) }, ..Default::default() },
                        )?;

                        tree.panel(
                            &[win_header, content_lbl],
                            child.bg.or(Some(self.theme.card_bg())),
                            if is_selected { Some(self.theme.border_highlight()) } else { child.border.or(Some(self.theme.border_subtle())) },
                            child_style,
                        )?
                    }
                    "card" => {
                        tree.card(
                            &[],
                            child.bg.or(Some(self.theme.card_bg())),
                            if is_selected { Some(self.theme.border_highlight()) } else { child.border.or(Some(self.theme.border_subtle())) },
                            child.radius,
                            child_style,
                        )?
                    }
                    "panel" => {
                        tree.panel(
                            &[],
                            child.bg.or(Some(self.theme.card_bg())),
                            if is_selected { Some(self.theme.border_highlight()) } else { child.border.or(Some(self.theme.border_subtle())) },
                            child_style,
                        )?
                    }
                    "button" => {
                        let label = child.label.as_deref().unwrap_or(&child.id);
                        let var = match child.variant.as_deref() {
                            Some("Secondary") => ButtonVariant::Secondary,
                            Some("Danger") => ButtonVariant::Danger,
                            Some("Ghost") => ButtonVariant::Ghost,
                            _ => ButtonVariant::Primary,
                        };
                        tree.button_variant(child.id.as_str(), label, var, child.enabled.unwrap_or(true), child_style)?
                    }
                    "label" => {
                        let text = child.text.as_deref().or(child.label.as_deref()).unwrap_or(&child.id);
                        if child.muted.unwrap_or(false) {
                            tree.label_muted(text, child_style)?
                        } else {
                            tree.label(text, child_style)?
                        }
                    }
                    "textinput" | "text_input" => {
                        let val = child.text.as_deref().unwrap_or("");
                        let placeholder = child.placeholder.as_deref().unwrap_or("");
                        tree.text_input(child.id.as_str(), val, placeholder, false, child_style)?
                    }
                    "slider" => {
                        let val = child.value.unwrap_or(50.0) as f32;
                        let min = child.min.unwrap_or(0.0) as f32;
                        let max = child.max.unwrap_or(100.0) as f32;
                        tree.slider(child.id.as_str(), min, max, val, child_style)?
                    }
                    "toggle" => {
                        tree.toggle(child.id.as_str(), child.checked.unwrap_or(true), child_style)?
                    }
                    "badge" => {
                        let text = child.text.as_deref().or(child.label.as_deref()).unwrap_or(&child.id);
                        tree.badge(text, ListItemBadge::Success, child_style)?
                    }
                    "metriccard" => {
                        let title = child.title.as_deref().unwrap_or("Metric");
                        let val = child.text.as_deref().unwrap_or("0");
                        let delta = child.delta.as_deref().map(|d| (d, child.delta_positive.unwrap_or(true)));
                        tree.metric_card(title, val, delta, child_style)?
                    }
                    "dropdown" => {
                        let label = child.label.as_deref().unwrap_or("Sélectionner");
                        let sel = child.text.as_deref().unwrap_or("");
                        tree.dropdown(child.id.as_str(), label, sel, child.checked.unwrap_or(false), child_style)?
                    }
                    "toast" => {
                        let title = child.title.as_deref().unwrap_or("Notification");
                        let msg = child.text.as_deref().unwrap_or("");
                        tree.toast(child.id.as_str(), title, msg, ui_widgets::ToastKind::Success, child_style)?
                    }
                    _ => tree.container(&[], child_style)?,
                };

                artboard_child_nodes.push(widget_inner);

                // Render 8 Resize Handles if selected
                if is_selected {
                    let left_x = child.layout.left.unwrap_or(0.0);
                    let top_y = child.layout.top.unwrap_or(0.0);
                    let width_w = child.layout.width.unwrap_or(180.0);
                    let height_h = child.layout.height.unwrap_or(38.0);
                    let handle_size = 8.0;
                    let half_handle = handle_size * 0.5;

                    let handle_positions = [
                        ("handle_tl", left_x - half_handle, top_y - half_handle),
                        ("handle_t", left_x + width_w * 0.5 - half_handle, top_y - half_handle),
                        ("handle_tr", left_x + width_w - half_handle, top_y - half_handle),
                        ("handle_r", left_x + width_w - half_handle, top_y + height_h * 0.5 - half_handle),
                        ("handle_br", left_x + width_w - half_handle, top_y + height_h - half_handle),
                        ("handle_b", left_x + width_w * 0.5 - half_handle, top_y + height_h - half_handle),
                        ("handle_bl", left_x - half_handle, top_y + height_h - half_handle),
                        ("handle_l", left_x - half_handle, top_y + height_h * 0.5 - half_handle),
                    ];

                    for (hid, hx, hy) in handle_positions {
                        let handle_btn = tree.button_variant(
                            hid,
                            "",
                            ButtonVariant::Primary,
                            true,
                            Style {
                                position: Position::Absolute,
                                inset: Rect { left: length(hx), top: length(hy), right: auto(), bottom: auto() },
                                size: Size { width: length(handle_size), height: length(handle_size) },
                                ..Default::default()
                            },
                        )?;
                        artboard_child_nodes.push(handle_btn);
                    }

                    // Floating dimension tooltip badge
                    let geo_str = format!("📍 X: {:.0}  Y: {:.0} | 📏 W: {:.0}  H: {:.0}", left_x, top_y, width_w, height_h);
                    let geo_label = tree.label(
                        &geo_str,
                        Style { size: Size { width: auto(), height: length(16.0) }, ..Default::default() },
                    )?;
                    let geo_badge = tree.panel(
                        &[geo_label],
                        Some(self.theme.card_bg()),
                        Some(self.theme.border_highlight()),
                        Style {
                            position: Position::Absolute,
                            inset: Rect { left: length(left_x), top: length((top_y - 22.0).max(4.0)), right: auto(), bottom: auto() },
                            padding: Rect { top: length(2.0), bottom: length(2.0), left: length(6.0), right: length(6.0) },
                            size: Size { width: auto(), height: length(20.0) },
                            ..Default::default()
                        },
                    )?;
                    artboard_child_nodes.push(geo_badge);
                }
            }
        }

        // Render Artboard Surface Box
        let artboard_surface = tree.panel(
            &artboard_child_nodes,
            Some(self.theme.card_bg()),
            Some(self.theme.border_subtle()),
            Style {
                position: Position::Relative,
                size: Size { width: length(self.state.artboard_width), height: length(self.state.artboard_height) },
                ..Default::default()
            },
        )?;
        self.state.artboard_node_id = Some(artboard_surface);

        let viewport_container = tree.container(
            &[artboard_surface],
            Style {
                align_items: Some(AlignItems::Center),
                size: Size { width: percent(1.0), height: percent(1.0) },
                padding: Rect { top: length(16.0), bottom: length(16.0), left: length(16.0), right: length(16.0) },
                ..Default::default()
            },
        )?;

        Ok(viewport_container)
    }

    /// Assembles the complete Stratus OS / Aether Visual Studio Form Designer
    fn build_studio_tree(&mut self, tree: &mut WidgetTree) -> Result<NodeId, LayoutError> {
        let panel_bg = Some(self.theme.glass_bg);
        let border_subtle = Some(self.theme.border_subtle());

        // --- 1. TOP HEADER TOOLBAR ---
        let header_title = tree.label(
            "📐 Stratus Studio (AORUI Visual Designer)",
            Style {
                size: Size { width: auto(), height: length(28.0) },
                margin: Rect { top: length(2.0), bottom: length(0.0), left: length(4.0), right: length(16.0) },
                ..Default::default()
            },
        )?;

        let btn_pre_desk = tree.button_variant("btn_preset_desktop", "🖥️ Desktop HD", if self.state.active_preset_name.contains("Desktop") { ButtonVariant::Primary } else { ButtonVariant::Ghost }, true, Style { size: Size { width: length(110.0), height: length(26.0) }, ..Default::default() })?;
        let btn_pre_comp = tree.button_variant("btn_preset_compact", "💻 800×500", if self.state.active_preset_name.contains("Compact") { ButtonVariant::Primary } else { ButtonVariant::Ghost }, true, Style { size: Size { width: length(85.0), height: length(26.0) }, ..Default::default() })?;
        let btn_pre_mob = tree.button_variant("btn_preset_mobile", "📱 Mobile", if self.state.active_preset_name.contains("Mobile") { ButtonVariant::Primary } else { ButtonVariant::Ghost }, true, Style { size: Size { width: length(80.0), height: length(26.0) }, ..Default::default() })?;

        let btn_snap = tree.button_variant("btn_toggle_snap", if self.state.snap_to_grid { "🧲 Aimantation: 16px" } else { "🧲 Aimantation: OFF" }, if self.state.snap_to_grid { ButtonVariant::Secondary } else { ButtonVariant::Default }, true, Style { size: Size { width: length(145.0), height: length(26.0) }, ..Default::default() })?;
        let btn_dup = tree.button_variant("btn_dup_node", "📋 Dupliquer", ButtonVariant::Default, self.state.selected_widget_id.is_some(), Style { size: Size { width: length(85.0), height: length(26.0) }, ..Default::default() })?;
        let btn_del = tree.button_variant("btn_del_node", "🗑️ Supprimer", ButtonVariant::Danger, self.state.selected_widget_id.is_some(), Style { size: Size { width: length(95.0), height: length(26.0) }, ..Default::default() })?;

        let btn_save = tree.button_variant("btn_export_toml", "💾 Export TOML", ButtonVariant::Primary, true, Style { size: Size { width: length(110.0), height: length(26.0) }, ..Default::default() })?;
        let btn_rust = tree.button_variant("btn_export_rust", "📄 Code Rust", ButtonVariant::Secondary, true, Style { size: Size { width: length(95.0), height: length(26.0) }, ..Default::default() })?;

        let sync_badge = tree.badge("● Synchro Active", ListItemBadge::Success, Style { size: Size { width: length(120.0), height: length(24.0) }, margin: Rect { top: length(0.0), bottom: length(0.0), left: length(12.0), right: length(4.0) }, ..Default::default() })?;

        let top_toolbar = tree.panel(
            &[
                header_title,
                btn_pre_desk, btn_pre_comp, btn_pre_mob,
                btn_snap,
                btn_dup, btn_del,
                btn_save, btn_rust,
                sync_badge,
            ],
            panel_bg,
            border_subtle,
            Style {
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                gap: Size { width: length(6.0), height: length(0.0) },
                padding: Rect { top: length(6.0), bottom: length(6.0), left: length(12.0), right: length(12.0) },
                size: Size { width: percent(1.0), height: length(42.0) },
                ..Default::default()
            },
        )?;

        // --- 2. LEFT TOOLBOX PALETTE (PALETTE DE COMPOSANTS) ---
        let tool_header = tree.label(
            "🧰 COMPOSANTS DISPONIBLES",
            Style {
                size: Size { width: percent(1.0), height: length(20.0) },
                ..Default::default()
            },
        )?;

        let tab_ctl = tree.button_variant("tab_controls", "Boutons", if self.state.active_toolbox_tab == "Controls" { ButtonVariant::Primary } else { ButtonVariant::Ghost }, true, Style { size: Size { width: length(58.0), height: length(24.0) }, ..Default::default() })?;
        let tab_inp = tree.button_variant("tab_inputs", "Inputs", if self.state.active_toolbox_tab == "Inputs" { ButtonVariant::Primary } else { ButtonVariant::Ghost }, true, Style { size: Size { width: length(48.0), height: length(24.0) }, ..Default::default() })?;
        let tab_nav = tree.button_variant("tab_nav", "Menus", if self.state.active_toolbox_tab == "Navigation" { ButtonVariant::Primary } else { ButtonVariant::Ghost }, true, Style { size: Size { width: length(48.0), height: length(24.0) }, ..Default::default() })?;
        let tab_pop = tree.button_variant("tab_popups", "Popups", if self.state.active_toolbox_tab == "Popups" { ButtonVariant::Primary } else { ButtonVariant::Ghost }, true, Style { size: Size { width: length(52.0), height: length(24.0) }, ..Default::default() })?;
        let tab_srf = tree.button_variant("tab_surfaces", "Surfaces", if self.state.active_toolbox_tab == "Surfaces" { ButtonVariant::Primary } else { ButtonVariant::Ghost }, true, Style { size: Size { width: length(58.0), height: length(24.0) }, ..Default::default() })?;

        let toolbox_tabs_row = tree.container(
            &[tab_ctl, tab_inp, tab_nav, tab_pop, tab_srf],
            Style {
                flex_direction: FlexDirection::Row,
                gap: Size { width: length(2.0), height: length(0.0) },
                size: Size { width: percent(1.0), height: length(26.0) },
                ..Default::default()
            },
        )?;

        let mut tool_items = vec![tool_header, toolbox_tabs_row];

        match self.state.active_toolbox_tab.as_str() {
            "Navigation" => {
                let btn_menu = tree.button_variant("tool_add_menubar", "📦 MenuBar (Barre Menus)", ButtonVariant::Primary, true, Style { size: Size { width: percent(1.0), height: length(28.0) }, ..Default::default() })?;
                let btn_drop = tree.button_variant("tool_add_dropdown", "📦 Dropdown Select", ButtonVariant::Default, true, Style { size: Size { width: percent(1.0), height: length(28.0) }, ..Default::default() })?;
                tool_items.extend(vec![btn_menu, btn_drop]);
            }
            "Popups" => {
                let btn_modal = tree.button_variant("tool_add_modal", "📦 Modal Dialog (Window)", ButtonVariant::Primary, true, Style { size: Size { width: percent(1.0), height: length(28.0) }, ..Default::default() })?;
                let btn_toast = tree.button_variant("tool_add_toast", "📦 Toast Notification", ButtonVariant::Default, true, Style { size: Size { width: percent(1.0), height: length(28.0) }, ..Default::default() })?;
                tool_items.extend(vec![btn_modal, btn_toast]);
            }
            "Inputs" => {
                let btn_inp = tree.button_variant("tool_add_input", "📦 TextInput Saisie", ButtonVariant::Default, true, Style { size: Size { width: percent(1.0), height: length(28.0) }, ..Default::default() })?;
                let btn_sli = tree.button_variant("tool_add_slider", "📦 Slider Fader", ButtonVariant::Default, true, Style { size: Size { width: percent(1.0), height: length(28.0) }, ..Default::default() })?;
                let btn_tog = tree.button_variant("tool_add_toggle", "📦 Toggle Switch", ButtonVariant::Default, true, Style { size: Size { width: percent(1.0), height: length(28.0) }, ..Default::default() })?;
                tool_items.extend(vec![btn_inp, btn_sli, btn_tog]);
            }
            "Surfaces" => {
                let btn_card = tree.button_variant("tool_add_card", "📦 Surface Card", ButtonVariant::Default, true, Style { size: Size { width: percent(1.0), height: length(28.0) }, ..Default::default() })?;
                let btn_pan = tree.button_variant("tool_add_panel", "📦 Surface Panel", ButtonVariant::Default, true, Style { size: Size { width: percent(1.0), height: length(28.0) }, ..Default::default() })?;
                let btn_met = tree.button_variant("tool_add_metric", "📦 Metric Stat Card", ButtonVariant::Default, true, Style { size: Size { width: percent(1.0), height: length(28.0) }, ..Default::default() })?;
                tool_items.extend(vec![btn_card, btn_pan, btn_met]);
            }
            _ => {
                let btn_pri = tree.button_variant("tool_add_btn_primary", "📦 Button Primary", ButtonVariant::Primary, true, Style { size: Size { width: percent(1.0), height: length(28.0) }, ..Default::default() })?;
                let btn_sec = tree.button_variant("tool_add_btn_secondary", "📦 Button Secondary", ButtonVariant::Secondary, true, Style { size: Size { width: percent(1.0), height: length(28.0) }, ..Default::default() })?;
                let btn_dan = tree.button_variant("tool_add_btn_danger", "📦 Button Danger", ButtonVariant::Danger, true, Style { size: Size { width: percent(1.0), height: length(28.0) }, ..Default::default() })?;
                let btn_lbl = tree.button_variant("tool_add_label", "📦 Label Texte", ButtonVariant::Default, true, Style { size: Size { width: percent(1.0), height: length(28.0) }, ..Default::default() })?;
                let btn_badge = tree.button_variant("tool_add_badge", "📦 Badge Status Pill", ButtonVariant::Default, true, Style { size: Size { width: percent(1.0), height: length(28.0) }, ..Default::default() })?;
                tool_items.extend(vec![btn_pri, btn_sec, btn_dan, btn_lbl, btn_badge]);
            }
        }

        let toolbox_panel = tree.panel(
            &tool_items,
            panel_bg,
            border_subtle,
            Style {
                flex_direction: FlexDirection::Column,
                gap: Size { width: length(0.0), height: length(6.0) },
                padding: Rect { top: length(12.0), bottom: length(12.0), left: length(12.0), right: length(12.0) },
                size: Size { width: length(240.0), height: percent(1.0) },
                ..Default::default()
            },
        )?;

        // --- 3. CENTER WORKSPACE CANVAS (ARTBOARD PAGE) ---
        let artboard_viewport = self.build_artboard_canvas(tree).unwrap_or_else(|_| {
            tree.container(&[], Style::default()).unwrap_or(NodeId::from(0usize))
        });

        let canvas_header = tree.label(
            format!("🖼️ Planche Active : \"{}\" [Gabarit: {}] (Grille: 16px)", self.state.doc.window.title, self.state.active_preset_name),
            Style {
                size: Size { width: percent(1.0), height: length(20.0) },
                ..Default::default()
            },
        )?;

        let center_canvas_wrapper = tree.panel(
            &[canvas_header, artboard_viewport],
            Some(self.theme.window_bg()),
            border_subtle,
            Style {
                flex_direction: FlexDirection::Column,
                flex_grow: 1.0,
                gap: Size { width: length(0.0), height: length(10.0) },
                padding: Rect { top: length(10.0), bottom: length(10.0), left: length(12.0), right: length(12.0) },
                size: Size { width: auto(), height: percent(1.0) },
                ..Default::default()
            },
        )?;

        // --- 4. RIGHT PROPERTY GRID (STRATUS-STYLE INSPECTOR) ---
        let prop_header = tree.label(
            "📋 INSPECTEUR D'OBJET",
            Style {
                size: Size { width: percent(1.0), height: length(20.0) },
                ..Default::default()
            },
        )?;

        let mut prop_items = vec![prop_header];

        if let Some(sel_id) = &self.state.selected_widget_id {
            if let Some(node) = self.state.doc.nodes.iter().find(|n| &n.id == sel_id) {
                let lbl_id = tree.label(format!("ID: {}  [{}]", node.id, node.widget_type), Style { size: Size { width: percent(1.0), height: length(20.0) }, ..Default::default() })?;

                // Section 1: Color Picker & Nuancier
                let lbl_sec_col = tree.label("━━ Apparence & Couleurs ━━", Style { size: Size { width: percent(1.0), height: length(18.0) }, ..Default::default() })?;

                let btn_t_bg = tree.button_variant("prop_target_bg", "🎨 Fond", if self.state.color_target == ColorTarget::Background { ButtonVariant::Primary } else { ButtonVariant::Ghost }, true, Style { size: Size { width: length(85.0), height: length(22.0) }, ..Default::default() })?;
                let btn_t_br = tree.button_variant("prop_target_border", "🔲 Bordure", if self.state.color_target == ColorTarget::Border { ButtonVariant::Primary } else { ButtonVariant::Ghost }, true, Style { size: Size { width: length(90.0), height: length(22.0) }, ..Default::default() })?;
                let btn_t_tx = tree.button_variant("prop_target_text", "✍️ Texte", if self.state.color_target == ColorTarget::Text { ButtonVariant::Primary } else { ButtonVariant::Ghost }, true, Style { size: Size { width: length(85.0), height: length(22.0) }, ..Default::default() })?;
                let row_col_targets = tree.container(&[btn_t_bg, btn_t_br, btn_t_tx], Style {
                    flex_direction: FlexDirection::Row,
                    gap: Size { width: length(4.0), height: length(0.0) },
                    size: Size { width: percent(1.0), height: length(24.0) },
                    ..Default::default()
                })?;

                let cur_rgba = self.state.get_active_color(&self.theme);
                let rgba_str = format!("RGBA: {:.2}, {:.2}, {:.2}, {:.2}", cur_rgba[0], cur_rgba[1], cur_rgba[2], cur_rgba[3]);
                let lbl_rgba_val = tree.label_muted(rgba_str, Style { size: Size { width: auto(), height: length(18.0) }, ..Default::default() })?;
                let col_preview_swatch = tree.panel(&[], Some(cur_rgba), Some(self.theme.border_highlight()), Style {
                    size: Size { width: length(26.0), height: length(18.0) },
                    ..Default::default()
                })?;
                let row_preview = tree.container(&[col_preview_swatch, lbl_rgba_val], Style {
                    flex_direction: FlexDirection::Row,
                    align_items: Some(AlignItems::Center),
                    gap: Size { width: length(8.0), height: length(0.0) },
                    size: Size { width: percent(1.0), height: length(20.0) },
                    ..Default::default()
                })?;

                // Swatches Row 1
                let btn_c_cyan = tree.button_variant("prop_col_cyan", "🔵 Cyan", ButtonVariant::Default, true, Style { size: Size { width: length(62.0), height: length(22.0) }, ..Default::default() })?;
                let btn_c_pur = tree.button_variant("prop_col_purple", "🟣 Violet", ButtonVariant::Default, true, Style { size: Size { width: length(65.0), height: length(22.0) }, ..Default::default() })?;
                let btn_c_mag = tree.button_variant("prop_col_magenta", "🌸 Rose", ButtonVariant::Default, true, Style { size: Size { width: length(60.0), height: length(22.0) }, ..Default::default() })?;
                let btn_c_eme = tree.button_variant("prop_col_emerald", "🟢 Vert", ButtonVariant::Default, true, Style { size: Size { width: length(60.0), height: length(22.0) }, ..Default::default() })?;
                let row_swatch_1 = tree.container(&[btn_c_cyan, btn_c_pur, btn_c_mag, btn_c_eme], Style {
                    flex_direction: FlexDirection::Row,
                    gap: Size { width: length(4.0), height: length(0.0) },
                    size: Size { width: percent(1.0), height: length(24.0) },
                    ..Default::default()
                })?;

                // Swatches Row 2
                let btn_c_gld = tree.button_variant("prop_col_gold", "🟡 Or", ButtonVariant::Default, true, Style { size: Size { width: length(56.0), height: length(22.0) }, ..Default::default() })?;
                let btn_c_crim = tree.button_variant("prop_col_crimson", "🔴 Rouge", ButtonVariant::Default, true, Style { size: Size { width: length(65.0), height: length(22.0) }, ..Default::default() })?;
                let btn_c_obs = tree.button_variant("prop_col_obsidian", "🌌 Dark", ButtonVariant::Default, true, Style { size: Size { width: length(62.0), height: length(22.0) }, ..Default::default() })?;
                let btn_c_gls = tree.button_variant("prop_col_glass", "🌫️ Verre", ButtonVariant::Default, true, Style { size: Size { width: length(62.0), height: length(22.0) }, ..Default::default() })?;
                let row_swatch_2 = tree.container(&[btn_c_gld, btn_c_crim, btn_c_obs, btn_c_gls], Style {
                    flex_direction: FlexDirection::Row,
                    gap: Size { width: length(4.0), height: length(0.0) },
                    size: Size { width: percent(1.0), height: length(24.0) },
                    ..Default::default()
                })?;

                // Section 2: Géométrie 2D (Position & Dimensions)
                let lbl_sec_geo = tree.label("━━ Géométrie & Position ━━", Style { size: Size { width: percent(1.0), height: length(18.0) }, ..Default::default() })?;
                let pos_x = node.layout.left.unwrap_or(0.0);
                let pos_y = node.layout.top.unwrap_or(0.0);
                let w_val = node.layout.width.unwrap_or(180.0);
                let h_val = node.layout.height.unwrap_or(38.0);

                let lbl_coords = tree.label_muted(format!("X: {:.0}px | Y: {:.0}px | W: {:.0}px | H: {:.0}px", pos_x, pos_y, w_val, h_val), Style { size: Size { width: percent(1.0), height: length(18.0) }, ..Default::default() })?;

                let btn_xm = tree.button_variant("prop_btn_x_minus", "X -16", ButtonVariant::Default, true, Style { size: Size { width: length(60.0), height: length(22.0) }, ..Default::default() })?;
                let btn_xp = tree.button_variant("prop_btn_x_plus", "X +16", ButtonVariant::Default, true, Style { size: Size { width: length(60.0), height: length(22.0) }, ..Default::default() })?;
                let btn_ym = tree.button_variant("prop_btn_y_minus", "Y -16", ButtonVariant::Default, true, Style { size: Size { width: length(60.0), height: length(22.0) }, ..Default::default() })?;
                let btn_yp = tree.button_variant("prop_btn_y_plus", "Y +16", ButtonVariant::Default, true, Style { size: Size { width: length(60.0), height: length(22.0) }, ..Default::default() })?;
                let row_xy = tree.container(&[btn_xm, btn_xp, btn_ym, btn_yp], Style {
                    flex_direction: FlexDirection::Row,
                    gap: Size { width: length(4.0), height: length(0.0) },
                    size: Size { width: percent(1.0), height: length(24.0) },
                    ..Default::default()
                })?;

                let btn_wm = tree.button_variant("prop_btn_w_minus", "W -16", ButtonVariant::Default, true, Style { size: Size { width: length(60.0), height: length(22.0) }, ..Default::default() })?;
                let btn_wp = tree.button_variant("prop_btn_w_plus", "W +16", ButtonVariant::Default, true, Style { size: Size { width: length(60.0), height: length(22.0) }, ..Default::default() })?;
                let btn_hm = tree.button_variant("prop_btn_h_minus", "H -8", ButtonVariant::Default, true, Style { size: Size { width: length(60.0), height: length(22.0) }, ..Default::default() })?;
                let btn_hp = tree.button_variant("prop_btn_h_plus", "H +8", ButtonVariant::Default, true, Style { size: Size { width: length(60.0), height: length(22.0) }, ..Default::default() })?;
                let row_wh = tree.container(&[btn_wm, btn_wp, btn_hm, btn_hp], Style {
                    flex_direction: FlexDirection::Row,
                    gap: Size { width: length(4.0), height: length(0.0) },
                    size: Size { width: percent(1.0), height: length(24.0) },
                    ..Default::default()
                })?;

                // Section 3: Variantes & Radius
                let lbl_sec_sty = tree.label("━━ Style & Variantes ━━", Style { size: Size { width: percent(1.0), height: length(18.0) }, ..Default::default() })?;
                let btn_var_pri = tree.button_variant("prop_btn_primary", "Primary", ButtonVariant::Primary, true, Style { size: Size { width: length(62.0), height: length(22.0) }, ..Default::default() })?;
                let btn_var_sec = tree.button_variant("prop_btn_secondary", "Secondary", ButtonVariant::Secondary, true, Style { size: Size { width: length(72.0), height: length(22.0) }, ..Default::default() })?;
                let btn_var_dan = tree.button_variant("prop_btn_danger", "Danger", ButtonVariant::Danger, true, Style { size: Size { width: length(58.0), height: length(22.0) }, ..Default::default() })?;
                let btn_var_gho = tree.button_variant("prop_btn_ghost", "Ghost", ButtonVariant::Ghost, true, Style { size: Size { width: length(52.0), height: length(22.0) }, ..Default::default() })?;
                let row_vars = tree.container(&[btn_var_pri, btn_var_sec, btn_var_dan, btn_var_gho], Style {
                    flex_direction: FlexDirection::Row,
                    gap: Size { width: length(4.0), height: length(0.0) },
                    size: Size { width: percent(1.0), height: length(24.0) },
                    ..Default::default()
                })?;

                let btn_rad_0 = tree.button_variant("prop_btn_rad_0", "Rad 0", ButtonVariant::Default, true, Style { size: Size { width: length(52.0), height: length(20.0) }, ..Default::default() })?;
                let btn_rad_4 = tree.button_variant("prop_btn_rad_4", "Rad 4", ButtonVariant::Default, true, Style { size: Size { width: length(52.0), height: length(20.0) }, ..Default::default() })?;
                let btn_rad_8 = tree.button_variant("prop_btn_rad_8", "Rad 8", ButtonVariant::Default, true, Style { size: Size { width: length(52.0), height: length(20.0) }, ..Default::default() })?;
                let btn_rad_16 = tree.button_variant("prop_btn_rad_16", "Rad 16", ButtonVariant::Default, true, Style { size: Size { width: length(52.0), height: length(20.0) }, ..Default::default() })?;
                let row_rads = tree.container(&[btn_rad_0, btn_rad_4, btn_rad_8, btn_rad_16], Style {
                    flex_direction: FlexDirection::Row,
                    gap: Size { width: length(4.0), height: length(0.0) },
                    size: Size { width: percent(1.0), height: length(22.0) },
                    ..Default::default()
                })?;

                // Section 4: Liaisons d'Événements
                let lbl_sec_ev = tree.label("━━ Handlers & Événements ━━", Style { size: Size { width: percent(1.0), height: length(18.0) }, ..Default::default() })?;
                let event_str = node.on_click.as_deref().or(node.on_change.as_deref()).unwrap_or("<aucun>");
                let lbl_ev = tree.label_muted(format!("Action: {}", event_str), Style { size: Size { width: percent(1.0), height: length(18.0) }, ..Default::default() })?;

                let btn_bind_c = tree.button_variant("prop_btn_bind_click", "Lier onClick", ButtonVariant::Default, true, Style { size: Size { width: length(120.0), height: length(22.0) }, ..Default::default() })?;
                let btn_bind_ch = tree.button_variant("prop_btn_bind_change", "Lier onChange", ButtonVariant::Default, true, Style { size: Size { width: length(120.0), height: length(22.0) }, ..Default::default() })?;
                let row_bind = tree.container(&[btn_bind_c, btn_bind_ch], Style {
                    flex_direction: FlexDirection::Row,
                    gap: Size { width: length(6.0), height: length(0.0) },
                    size: Size { width: percent(1.0), height: length(24.0) },
                    ..Default::default()
                })?;

                prop_items.extend(vec![
                    lbl_id,
                    lbl_sec_col, row_col_targets, row_preview, row_swatch_1, row_swatch_2,
                    lbl_sec_geo, lbl_coords, row_xy, row_wh,
                    lbl_sec_sty, row_vars, row_rads,
                    lbl_sec_ev, lbl_ev, row_bind,
                ]);
            }
        } else {
            let lbl_none = tree.label_muted(
                "Sélectionnez un composant sur la planche active pour modifier ses dimensions, couleurs et liaisons d'actions.",
                Style { size: Size { width: percent(1.0), height: length(40.0) }, ..Default::default() },
            )?;
            prop_items.push(lbl_none);
        }

        let inspector_panel = tree.panel(
            &prop_items,
            panel_bg,
            border_subtle,
            Style {
                flex_direction: FlexDirection::Column,
                gap: Size { width: length(0.0), height: length(4.0) },
                padding: Rect { top: length(12.0), bottom: length(12.0), left: length(12.0), right: length(12.0) },
                size: Size { width: length(305.0), height: percent(1.0) },
                ..Default::default()
            },
        )?;

        // --- 5. WORKSPACE MIDDLE ROW ---
        let workspace_row = tree.container(
            &[toolbox_panel, center_canvas_wrapper, inspector_panel],
            Style {
                flex_direction: FlexDirection::Row,
                flex_grow: 1.0,
                gap: Size { width: length(6.0), height: length(0.0) },
                size: Size { width: percent(1.0), height: auto() },
                ..Default::default()
            },
        )?;

        // --- 6. BOTTOM STATUS / FLOATING DOCK ---
        let status_lbl = tree.label_muted(
            format!("⚡ {}", self.state.status),
            Style {
                size: Size { width: percent(1.0), height: length(20.0) },
                ..Default::default()
            },
        )?;

        let status_bar = tree.panel(
            &[status_lbl],
            Some(self.theme.floating_dock_bg()),
            border_subtle,
            Style {
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                padding: Rect { top: length(4.0), bottom: length(4.0), left: length(12.0), right: length(12.0) },
                size: Size { width: percent(1.0), height: length(28.0) },
                ..Default::default()
            },
        )?;

        // --- 7. ROOT CONTAINER ---
        let root = tree.panel(
            &[top_toolbar, workspace_row, status_bar],
            Some(self.theme.window_bg()),
            None,
            Style {
                position: Position::Absolute,
                inset: Rect { top: length(0.0), bottom: length(0.0), left: length(0.0), right: length(0.0) },
                flex_direction: FlexDirection::Column,
                gap: Size { width: length(0.0), height: length(4.0) },
                size: Size { width: percent(1.0), height: percent(1.0) },
                ..Default::default()
            },
        )?;

        Ok(root)
    }

    fn redraw(&mut self) {
        if let Ok(lock) = self.theme_shared.read() {
            self.theme = lock.clone();
        }

        let (width_f, height_f) = if let Some(r) = &self.renderer {
            let (w, h) = r.window_size();
            (w as f32, h as f32)
        } else {
            return;
        };

        let mut tree = WidgetTree::new();
        let root = match self.build_studio_tree(&mut tree) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Erreur de construction de l'arbre Studio: {}", e);
                return;
            }
        };

        let Some(renderer) = self.renderer.as_mut() else { return; };

        let available = Size {
            width: AvailableSpace::Definite(width_f),
            height: AvailableSpace::Definite(height_f),
        };
        let _ = tree.compute(root, available);

        if let Ok(bounds_map) = tree.resolved_bounds(root) {
            if let Some(art_node) = self.state.artboard_node_id {
                if let Some(bounds) = bounds_map.get(&art_node) {
                    self.state.artboard_screen_rect = *bounds;
                }
            }
        }

        let measure = renderer.text_measure();
        let interaction = InteractionState {
            hovered: None,
            pressed: None,
            measure: Some(&measure),
        };

        if let Ok(frame) = tree.build_frame(root, &self.theme, interaction) {
            let family = Self::font_family(&self.theme.typography.family);
            let text_runs: Vec<_> = frame
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

            let layer = ui_gpu::RenderLayer {
                instances: &frame.instances,
                texts: &text_runs,
            };

            let (cr, cg, cb) = self.theme.window_clear_color();

            let _ = renderer.render_layers(
                wgpu::Color { r: cr, g: cg, b: cb, a: 1.0 },
                &[layer],
                &[],
                &self.resources,
            );
        }

        self.last_tree = Some(tree);
        self.last_root = Some(root);
    }
}

impl ApplicationHandler for FormDesignerApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() { return; }

        let window_attrs = WindowAttributes::default()
            .with_title("Stratus Studio - AORUI Modern Visual Form Designer")
            .with_inner_size(winit::dpi::LogicalSize::new(1440.0, 880.0))
            .with_min_inner_size(winit::dpi::LogicalSize::new(1000.0, 600.0));

        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .expect("Failed to create visual form designer window"),
        );

        let size = window.inner_size();
        let mut renderer = GpuRenderer::new(window.clone());
        renderer.resize(size);

        self.renderer = Some(renderer);
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(new_size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(new_size);
                }
                if let Some(win) = &self.window {
                    win.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                self.redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                let pos = (position.x as f32, position.y as f32);
                self.last_mouse_pos = Some(pos);

                let snap_to_grid = self.state.snap_to_grid;
                let grid_snap_size = self.state.grid_snap_size;
                let snap_val = |val: f32| {
                    if snap_to_grid && grid_snap_size > 0.0 {
                        (val / grid_snap_size).round() * grid_snap_size
                    } else {
                        val
                    }
                };

                let mut drag_mode_clone = None;
                let mut is_active_now = false;

                if let Some(session) = &mut self.state.drag_session {
                    session.current_mouse_pos = pos;
                    let dx = pos.0 - session.start_mouse_pos.0;
                    let dy = pos.1 - session.start_mouse_pos.1;

                    if (dx * dx + dy * dy).sqrt() > 3.0 {
                        session.is_active = true;
                    }
                    if session.is_active {
                        drag_mode_clone = Some((session.mode.clone(), dx, dy));
                        is_active_now = true;
                    }
                }

                if is_active_now {
                    if let Some((mode, dx, dy)) = drag_mode_clone {
                        match mode {
                            DragMode::MoveWidget { widget_id, initial_left, initial_top } => {
                                if let Some(node) = self.state.doc.nodes.iter_mut().find(|n| n.id == widget_id) {
                                    let new_x = snap_val((initial_left + dx).max(0.0));
                                    let new_y = snap_val((initial_top + dy).max(0.0));
                                    node.layout.left = Some(new_x);
                                    node.layout.top = Some(new_y);
                                    self.state.status = format!("📍 Déplacement de '{}' ➔ X: {:.0}px, Y: {:.0}px", widget_id, new_x, new_y);
                                }
                            }
                            DragMode::ResizeWidget { widget_id, handle, initial_left, initial_top, initial_width, initial_height } => {
                                if let Some(node) = self.state.doc.nodes.iter_mut().find(|n| n.id == widget_id) {
                                    match handle {
                                        ResizeHandle::Right => {
                                            let new_w = snap_val((initial_width + dx).max(32.0));
                                            node.layout.width = Some(new_w);
                                        }
                                        ResizeHandle::Bottom => {
                                            let new_h = snap_val((initial_height + dy).max(20.0));
                                            node.layout.height = Some(new_h);
                                        }
                                        ResizeHandle::BottomRight => {
                                            let new_w = snap_val((initial_width + dx).max(32.0));
                                            let new_h = snap_val((initial_height + dy).max(20.0));
                                            node.layout.width = Some(new_w);
                                            node.layout.height = Some(new_h);
                                        }
                                        ResizeHandle::TopLeft => {
                                            let new_x = snap_val((initial_left + dx).max(0.0));
                                            let new_y = snap_val((initial_top + dy).max(0.0));
                                            let new_w = snap_val((initial_width - dx).max(32.0));
                                            let new_h = snap_val((initial_height - dy).max(20.0));
                                            node.layout.left = Some(new_x);
                                            node.layout.top = Some(new_y);
                                            node.layout.width = Some(new_w);
                                            node.layout.height = Some(new_h);
                                        }
                                        ResizeHandle::Top => {
                                            let new_y = snap_val((initial_top + dy).max(0.0));
                                            let new_h = snap_val((initial_height - dy).max(20.0));
                                            node.layout.top = Some(new_y);
                                            node.layout.height = Some(new_h);
                                        }
                                        ResizeHandle::TopRight => {
                                            let new_y = snap_val((initial_top + dy).max(0.0));
                                            let new_w = snap_val((initial_width + dx).max(32.0));
                                            let new_h = snap_val((initial_height - dy).max(20.0));
                                            node.layout.top = Some(new_y);
                                            node.layout.width = Some(new_w);
                                            node.layout.height = Some(new_h);
                                        }
                                        ResizeHandle::BottomLeft => {
                                            let new_x = snap_val((initial_left + dx).max(0.0));
                                            let new_w = snap_val((initial_width - dx).max(32.0));
                                            let new_h = snap_val((initial_height + dy).max(20.0));
                                            node.layout.left = Some(new_x);
                                            node.layout.width = Some(new_w);
                                            node.layout.height = Some(new_h);
                                        }
                                        ResizeHandle::Left => {
                                            let new_x = snap_val((initial_left + dx).max(0.0));
                                            let new_w = snap_val((initial_width - dx).max(32.0));
                                            node.layout.left = Some(new_x);
                                            node.layout.width = Some(new_w);
                                        }
                                    }
                                    self.state.status = format!("📐 Redimensionnement de '{}' ➔ W: {:.0}px, H: {:.0}px", widget_id, node.layout.width.unwrap_or(0.0), node.layout.height.unwrap_or(0.0));
                                }
                            }
                            DragMode::ToolboxDrop { .. } => {}
                        }
                    }
                }

                if let Some(win) = &self.window {
                    win.request_redraw();
                }
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                if let Some(pos) = self.last_mouse_pos {
                    let rect = self.state.artboard_screen_rect;
                    let art_x = pos.0 - rect[0];
                    let art_y = pos.1 - rect[1];

                    // Check clicking on artboard
                    if art_x >= 0.0 && art_x <= rect[2] && art_y >= 0.0 && art_y <= rect[3] {
                        let mut handle_clicked = None;
                        if let Some(sel_id) = &self.state.selected_widget_id {
                            if let Some(node) = self.state.doc.nodes.iter().find(|n| &n.id == sel_id) {
                                let lx = node.layout.left.unwrap_or(0.0);
                                let ty = node.layout.top.unwrap_or(0.0);
                                let w = node.layout.width.unwrap_or(180.0);
                                let h = node.layout.height.unwrap_or(38.0);
                                let half = 8.0;

                                let handles = [
                                    (ResizeHandle::TopLeft, lx, ty),
                                    (ResizeHandle::Top, lx + w * 0.5, ty),
                                    (ResizeHandle::TopRight, lx + w, ty),
                                    (ResizeHandle::Right, lx + w, ty + h * 0.5),
                                    (ResizeHandle::BottomRight, lx + w, ty + h),
                                    (ResizeHandle::Bottom, lx + w * 0.5, ty + h),
                                    (ResizeHandle::BottomLeft, lx, ty + h),
                                    (ResizeHandle::Left, lx, ty + h * 0.5),
                                ];

                                for (h_type, hx, hy) in handles {
                                    if (art_x - hx).abs() <= half && (art_y - hy).abs() <= half {
                                        handle_clicked = Some((h_type, lx, ty, w, h));
                                        break;
                                    }
                                }
                            }
                        }

                        if let Some((handle, lx, ty, w, h)) = handle_clicked {
                            if let Some(sel_id) = self.state.selected_widget_id.clone() {
                                self.state.drag_session = Some(DragSession {
                                    mode: DragMode::ResizeWidget {
                                        widget_id: sel_id,
                                        handle,
                                        initial_left: lx,
                                        initial_top: ty,
                                        initial_width: w,
                                        initial_height: h,
                                    },
                                    start_mouse_pos: pos,
                                    current_mouse_pos: pos,
                                    is_active: false,
                                });
                                if let Some(win) = &self.window { win.request_redraw(); }
                                return;
                            }
                        }

                        let mut found_node = None;
                        for node in self.state.doc.nodes.iter().rev() {
                            if node.id == self.state.doc.root { continue; }
                            let left = node.layout.left.unwrap_or(0.0);
                            let top = node.layout.top.unwrap_or(0.0);
                            let width = node.layout.width.unwrap_or(180.0);
                            let height = node.layout.height.unwrap_or(38.0);

                            if art_x >= left && art_x <= left + width && art_y >= top && art_y <= top + height {
                                found_node = Some((node.id.clone(), left, top));
                                break;
                            }
                        }

                        if let Some((nid, left, top)) = found_node {
                            self.state.selected_widget_id = Some(nid.clone());
                            self.state.status = format!("🎯 Sélectionné : '{}'", nid);
                            self.state.drag_session = Some(DragSession {
                                mode: DragMode::MoveWidget {
                                    widget_id: nid,
                                    initial_left: left,
                                    initial_top: top,
                                },
                                start_mouse_pos: pos,
                                current_mouse_pos: pos,
                                is_active: false,
                            });
                            if let Some(win) = &self.window { win.request_redraw(); }
                            return;
                        } else {
                            self.state.selected_widget_id = None;
                            self.state.status = "Artboard sélectionné".to_string();
                        }
                    }

                    // Check clicking on UI controls
                    let mut hit_id = None;
                    if let Some(tree) = &self.last_tree {
                        if let Some(root) = self.last_root {
                            if let Ok(Some(hit_node)) = tree.hit_test(root, pos.0, pos.1) {
                                if let Some(wid) = tree.widget_id(hit_node) {
                                    hit_id = Some(wid.as_str().to_string());
                                }
                            }
                        }
                    }

                    if let Some(widget_id) = hit_id {
                        let toolbox_info = match widget_id.as_str() {
                            "tool_add_btn_primary" => Some(("Button", "Button Primary", Some("Primary"))),
                            "tool_add_btn_secondary" => Some(("Button", "Button Secondary", Some("Secondary"))),
                            "tool_add_btn_danger" => Some(("Button", "Button Danger", Some("Danger"))),
                            "tool_add_label" => Some(("Label", "Nouveau Libellé", None)),
                            "tool_add_menubar" => Some(("MenuBar", "Barre de Menus", None)),
                            "tool_add_modal" => Some(("Window", "Dialogue Modale", None)),
                            "tool_add_dropdown" => Some(("Dropdown", "Menu Déroulant", None)),
                            "tool_add_toast" => Some(("Toast", "Notification Toast", None)),
                            "tool_add_input" => Some(("TextInput", "Saisie...", None)),
                            "tool_add_slider" => Some(("Slider", "Curseur Fader", None)),
                            "tool_add_toggle" => Some(("Toggle", "Switch", None)),
                            "tool_add_card" => Some(("Card", "Surface Card", None)),
                            "tool_add_panel" => Some(("Panel", "Surface Panel", None)),
                            "tool_add_metric" => Some(("MetricCard", "Statistiques", None)),
                            "tool_add_badge" => Some(("Badge", "Status OK", None)),
                            _ => None,
                        };

                        if let Some((w_type, w_label, w_var)) = toolbox_info {
                            self.state.drag_session = Some(DragSession {
                                mode: DragMode::ToolboxDrop {
                                    widget_type: w_type.to_string(),
                                    default_label: w_label.to_string(),
                                    variant: w_var.map(|v| v.to_string()),
                                },
                                start_mouse_pos: pos,
                                current_mouse_pos: pos,
                                is_active: false,
                            });
                        }

                        match widget_id.as_str() {
                            "btn_export_toml" => {
                                match self.state.doc.to_toml() {
                                    Ok(toml_text) => {
                                        let out_path = "ui/exported_form.toml";
                                        let _ = std::fs::write(out_path, &toml_text);
                                        self.state.status = format!("💾 Formulaire exporté dans '{}' !", out_path);
                                    }
                                    Err(e) => {
                                        self.state.status = format!("❌ Erreur TOML: {}", e);
                                    }
                                }
                            }
                            "btn_export_rust" => {
                                let code = self.state.doc.generate_rust_handlers();
                                let out_path = "ui/exported_handlers.rs";
                                let _ = std::fs::write(out_path, &code);
                                self.state.status = format!("📄 Handlers Rust générés dans '{}' !", out_path);
                            }
                            "btn_toggle_snap" => {
                                self.state.snap_to_grid = !self.state.snap_to_grid;
                                self.state.status = format!("Aimantation : {}", if self.state.snap_to_grid { "ACTIVÉE (16px)" } else { "DÉSACTIVÉE" });
                            }
                            "btn_preset_desktop" => {
                                self.state.artboard_width = 960.0;
                                self.state.artboard_height = 620.0;
                                self.state.active_preset_name = "Desktop HD (960×620)".to_string();
                                self.state.status = "Gabarit: Desktop HD (960×620)".to_string();
                            }
                            "btn_preset_compact" => {
                                self.state.artboard_width = 800.0;
                                self.state.artboard_height = 500.0;
                                self.state.active_preset_name = "Compact (800×500)".to_string();
                                self.state.status = "Gabarit: Compact (800×500)".to_string();
                            }
                            "btn_preset_mobile" => {
                                self.state.artboard_width = 390.0;
                                self.state.artboard_height = 700.0;
                                self.state.active_preset_name = "Mobile (390×700)".to_string();
                                self.state.status = "Gabarit: Mobile (390×700)".to_string();
                            }
                            "btn_dup_node" => {
                                if let Some(sel_id) = self.state.selected_widget_id.clone() {
                                    if sel_id != self.state.doc.root {
                                        if let Some(node) = self.state.doc.nodes.iter().find(|n| n.id == sel_id).cloned() {
                                            self.state.next_id_counter += 1;
                                            let new_id = format!("{}_copy_{}", node.id, self.state.next_id_counter);
                                            let mut dup = node.clone();
                                            dup.id = new_id.clone();
                                            let cur_x = dup.layout.left.unwrap_or(40.0);
                                            let cur_y = dup.layout.top.unwrap_or(40.0);
                                            dup.layout.left = Some(cur_x + 24.0);
                                            dup.layout.top = Some(cur_y + 24.0);

                                            if let Some(root_node) = self.state.doc.nodes.iter_mut().find(|n| n.id == self.state.doc.root) {
                                                root_node.children.push(new_id.clone());
                                            }
                                            self.state.doc.nodes.push(dup);
                                            self.state.selected_widget_id = Some(new_id.clone());
                                            self.state.status = format!("📋 Widget '{}' dupliqué !", sel_id);
                                        }
                                    }
                                }
                            }
                            "btn_del_node" => {
                                if let Some(sel_id) = self.state.selected_widget_id.take() {
                                    if sel_id != self.state.doc.root {
                                        for node in &mut self.state.doc.nodes {
                                            node.children.retain(|c| c != &sel_id);
                                        }
                                        self.state.doc.nodes.retain(|n| n.id != sel_id);
                                        self.state.status = format!("🗑️ Widget '{}' supprimé.", sel_id);
                                    }
                                }
                            }
                            // Category tabs
                            "tab_controls" => { self.state.active_toolbox_tab = "Controls".to_string(); }
                            "tab_inputs" => { self.state.active_toolbox_tab = "Inputs".to_string(); }
                            "tab_nav" => { self.state.active_toolbox_tab = "Navigation".to_string(); }
                            "tab_popups" => { self.state.active_toolbox_tab = "Popups".to_string(); }
                            "tab_surfaces" => { self.state.active_toolbox_tab = "Surfaces".to_string(); }

                            // Color Target & Swatches
                            "prop_target_bg" => { self.state.color_target = ColorTarget::Background; }
                            "prop_target_border" => { self.state.color_target = ColorTarget::Border; }
                            "prop_target_text" => { self.state.color_target = ColorTarget::Text; }
                            "prop_col_cyan" => { self.state.set_active_color(Some([0.0, 0.95, 1.0, 1.0])); }
                            "prop_col_purple" => { self.state.set_active_color(Some([0.75, 0.76, 1.0, 1.0])); }
                            "prop_col_magenta" => { self.state.set_active_color(Some([0.93, 0.28, 0.60, 1.0])); }
                            "prop_col_emerald" => { self.state.set_active_color(Some([0.31, 0.87, 0.64, 1.0])); }
                            "prop_col_gold" => { self.state.set_active_color(Some([0.96, 0.62, 0.04, 1.0])); }
                            "prop_col_crimson" => { self.state.set_active_color(Some([1.0, 0.71, 0.67, 1.0])); }
                            "prop_col_obsidian" => { self.state.set_active_color(Some(self.theme.window_bg())); }
                            "prop_col_glass" => { self.state.set_active_color(Some(self.theme.glass_bg)); }

                            // Geometry fine steppers
                            "prop_btn_x_minus" => {
                                if let Some(sel_id) = &self.state.selected_widget_id {
                                    if let Some(node) = self.state.doc.nodes.iter_mut().find(|n| &n.id == sel_id) {
                                        let cur = node.layout.left.unwrap_or(0.0);
                                        node.layout.left = Some((cur - 16.0).max(0.0));
                                    }
                                }
                            }
                            "prop_btn_x_plus" => {
                                if let Some(sel_id) = &self.state.selected_widget_id {
                                    if let Some(node) = self.state.doc.nodes.iter_mut().find(|n| &n.id == sel_id) {
                                        let cur = node.layout.left.unwrap_or(0.0);
                                        node.layout.left = Some(cur + 16.0);
                                    }
                                }
                            }
                            "prop_btn_y_minus" => {
                                if let Some(sel_id) = &self.state.selected_widget_id {
                                    if let Some(node) = self.state.doc.nodes.iter_mut().find(|n| &n.id == sel_id) {
                                        let cur = node.layout.top.unwrap_or(0.0);
                                        node.layout.top = Some((cur - 16.0).max(0.0));
                                    }
                                }
                            }
                            "prop_btn_y_plus" => {
                                if let Some(sel_id) = &self.state.selected_widget_id {
                                    if let Some(node) = self.state.doc.nodes.iter_mut().find(|n| &n.id == sel_id) {
                                        let cur = node.layout.top.unwrap_or(0.0);
                                        node.layout.top = Some(cur + 16.0);
                                    }
                                }
                            }
                            "prop_btn_w_minus" => {
                                if let Some(sel_id) = &self.state.selected_widget_id {
                                    if let Some(node) = self.state.doc.nodes.iter_mut().find(|n| &n.id == sel_id) {
                                        let cur = node.layout.width.unwrap_or(180.0);
                                        node.layout.width = Some((cur - 16.0).max(32.0));
                                    }
                                }
                            }
                            "prop_btn_w_plus" => {
                                if let Some(sel_id) = &self.state.selected_widget_id {
                                    if let Some(node) = self.state.doc.nodes.iter_mut().find(|n| &n.id == sel_id) {
                                        let cur = node.layout.width.unwrap_or(180.0);
                                        node.layout.width = Some(cur + 16.0);
                                    }
                                }
                            }
                            "prop_btn_h_minus" => {
                                if let Some(sel_id) = &self.state.selected_widget_id {
                                    if let Some(node) = self.state.doc.nodes.iter_mut().find(|n| &n.id == sel_id) {
                                        let cur = node.layout.height.unwrap_or(38.0);
                                        node.layout.height = Some((cur - 8.0).max(20.0));
                                    }
                                }
                            }
                            "prop_btn_h_plus" => {
                                if let Some(sel_id) = &self.state.selected_widget_id {
                                    if let Some(node) = self.state.doc.nodes.iter_mut().find(|n| &n.id == sel_id) {
                                        let cur = node.layout.height.unwrap_or(38.0);
                                        node.layout.height = Some(cur + 8.0);
                                    }
                                }
                            }

                            // Style variants
                            "prop_btn_primary" => {
                                if let Some(sel_id) = &self.state.selected_widget_id {
                                    if let Some(node) = self.state.doc.nodes.iter_mut().find(|n| &n.id == sel_id) {
                                        node.variant = Some("Primary".to_string());
                                    }
                                }
                            }
                            "prop_btn_secondary" => {
                                if let Some(sel_id) = &self.state.selected_widget_id {
                                    if let Some(node) = self.state.doc.nodes.iter_mut().find(|n| &n.id == sel_id) {
                                        node.variant = Some("Secondary".to_string());
                                    }
                                }
                            }
                            "prop_btn_danger" => {
                                if let Some(sel_id) = &self.state.selected_widget_id {
                                    if let Some(node) = self.state.doc.nodes.iter_mut().find(|n| &n.id == sel_id) {
                                        node.variant = Some("Danger".to_string());
                                    }
                                }
                            }
                            "prop_btn_ghost" => {
                                if let Some(sel_id) = &self.state.selected_widget_id {
                                    if let Some(node) = self.state.doc.nodes.iter_mut().find(|n| &n.id == sel_id) {
                                        node.variant = Some("Ghost".to_string());
                                    }
                                }
                            }
                            "prop_btn_rad_0" => {
                                if let Some(sel_id) = &self.state.selected_widget_id {
                                    if let Some(node) = self.state.doc.nodes.iter_mut().find(|n| &n.id == sel_id) {
                                        node.radius = Some(0.0);
                                    }
                                }
                            }
                            "prop_btn_rad_4" => {
                                if let Some(sel_id) = &self.state.selected_widget_id {
                                    if let Some(node) = self.state.doc.nodes.iter_mut().find(|n| &n.id == sel_id) {
                                        node.radius = Some(4.0);
                                    }
                                }
                            }
                            "prop_btn_rad_8" => {
                                if let Some(sel_id) = &self.state.selected_widget_id {
                                    if let Some(node) = self.state.doc.nodes.iter_mut().find(|n| &n.id == sel_id) {
                                        node.radius = Some(8.0);
                                    }
                                }
                            }
                            "prop_btn_rad_16" => {
                                if let Some(sel_id) = &self.state.selected_widget_id {
                                    if let Some(node) = self.state.doc.nodes.iter_mut().find(|n| &n.id == sel_id) {
                                        node.radius = Some(16.0);
                                    }
                                }
                            }

                            // Event bindings
                            "prop_btn_bind_click" => {
                                if let Some(sel_id) = &self.state.selected_widget_id {
                                    if let Some(node) = self.state.doc.nodes.iter_mut().find(|n| &n.id == sel_id) {
                                        node.on_click = Some(format!("app:on_{}_click", sel_id));
                                        self.state.status = format!("⚡ Liaison onClick générée : 'app:on_{}_click'", sel_id);
                                    }
                                }
                            }
                            "prop_btn_bind_change" => {
                                if let Some(sel_id) = &self.state.selected_widget_id {
                                    if let Some(node) = self.state.doc.nodes.iter_mut().find(|n| &n.id == sel_id) {
                                        node.on_change = Some(format!("app:on_{}_change", sel_id));
                                        self.state.status = format!("⚡ Liaison onChange générée : 'app:on_{}_change'", sel_id);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    if let Some(win) = &self.window {
                        win.request_redraw();
                    }
                }
            }
            WindowEvent::MouseInput { state: ElementState::Released, button: MouseButton::Left, .. } => {
                if let Some(session) = self.state.drag_session.take() {
                    if let DragMode::ToolboxDrop { widget_type, default_label, variant } = session.mode {
                        if let Some(mouse_pos) = self.last_mouse_pos {
                            let rect = self.state.artboard_screen_rect;
                            let art_x = (mouse_pos.0 - rect[0]).max(16.0);
                            let art_y = (mouse_pos.1 - rect[1]).max(16.0);
                            Self::add_widget_at_coords(
                                &mut self.state,
                                &widget_type,
                                &default_label,
                                variant.as_deref(),
                                art_x,
                                art_y,
                            );
                        }
                    }

                    if let Some(win) = &self.window {
                        win.request_redraw();
                    }
                }
            }
            _ => {}
        }
    }
}

fn main() {
    println!("============================================================");
    println!("📐 STRATUS STUDIO — ATELIER VISUEL DE COMPOSITION FRONT-END");
    println!("🚀 AORUI Modern Vector Artboard & Clean Architecture Studio");
    println!("============================================================");

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = FormDesignerApp::new();
    let _ = event_loop.run_app(&mut app);
}
