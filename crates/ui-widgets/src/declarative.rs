// [WFGY] Zone: TRANSIT | λ: 0.25 | Fallbacks: 0 | Action: Declarative TOML UI schema, layout deserializer, and WidgetTree generator
use std::collections::HashMap;
use std::path::Path;
use serde::{Deserialize, Serialize};

use ui_layout::{
    auto, length, percent, AlignItems, AlignSelf, Display,
    FlexDirection, FlexWrap, JustifyContent, NodeId, Position, Rect, Size, Style,
};

use crate::kind::{ButtonVariant, ListItemBadge};
use crate::tree::WidgetTree;

/// Serialisable Flex / Grid Layout Style Specification for Declarative UI files.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct LayoutStyleSpec {
    #[serde(default)]
    pub display: Option<String>, // "flex", "grid", "none"
    #[serde(default)]
    pub position: Option<String>, // "relative", "absolute"
    #[serde(default)]
    pub direction: Option<String>, // "row", "column", "row_reverse", "column_reverse"
    #[serde(default)]
    pub wrap: Option<String>, // "no_wrap", "wrap", "wrap_reverse"
    #[serde(default)]
    pub align_items: Option<String>, // "flex_start", "flex_end", "center", "baseline", "stretch"
    #[serde(default)]
    pub align_self: Option<String>,
    #[serde(default)]
    pub align_content: Option<String>,
    #[serde(default)]
    pub justify_content: Option<String>, // "flex_start", "flex_end", "center", "space_between", "space_around", "space_evenly"

    // Dimensions
    #[serde(default)]
    pub width: Option<f32>,
    #[serde(default)]
    pub height: Option<f32>,
    #[serde(default)]
    pub width_percent: Option<f32>,
    #[serde(default)]
    pub height_percent: Option<f32>,
    #[serde(default)]
    pub min_width: Option<f32>,
    #[serde(default)]
    pub min_height: Option<f32>,
    #[serde(default)]
    pub max_width: Option<f32>,
    #[serde(default)]
    pub max_height: Option<f32>,

    // Spacing
    #[serde(default)]
    pub gap: Option<f32>,
    #[serde(default)]
    pub gap_row: Option<f32>,
    #[serde(default)]
    pub gap_col: Option<f32>,

    #[serde(default)]
    pub padding: Option<f32>,
    #[serde(default)]
    pub padding_top: Option<f32>,
    #[serde(default)]
    pub padding_bottom: Option<f32>,
    #[serde(default)]
    pub padding_left: Option<f32>,
    #[serde(default)]
    pub padding_right: Option<f32>,

    #[serde(default)]
    pub margin: Option<f32>,
    #[serde(default)]
    pub margin_top: Option<f32>,
    #[serde(default)]
    pub margin_bottom: Option<f32>,
    #[serde(default)]
    pub margin_left: Option<f32>,
    #[serde(default)]
    pub margin_right: Option<f32>,

    // Absolute Insets
    #[serde(default)]
    pub top: Option<f32>,
    #[serde(default)]
    pub bottom: Option<f32>,
    #[serde(default)]
    pub left: Option<f32>,
    #[serde(default)]
    pub right: Option<f32>,

    // Flex items
    #[serde(default)]
    pub flex_grow: Option<f32>,
    #[serde(default)]
    pub flex_shrink: Option<f32>,
    #[serde(default)]
    pub flex_basis: Option<f32>,
}

impl LayoutStyleSpec {
    pub fn to_style(&self) -> Style {
        let display = match self.display.as_deref() {
            Some("grid") => Display::Grid,
            Some("none") => Display::None,
            _ => Display::Flex,
        };

        let position = match self.position.as_deref() {
            Some("absolute") => Position::Absolute,
            _ => Position::Relative,
        };

        let flex_direction = match self.direction.as_deref() {
            Some("column") => FlexDirection::Column,
            Some("row_reverse") => FlexDirection::RowReverse,
            Some("column_reverse") => FlexDirection::ColumnReverse,
            _ => FlexDirection::Row,
        };

        let flex_wrap = match self.wrap.as_deref() {
            Some("wrap") => FlexWrap::Wrap,
            Some("wrap_reverse") => FlexWrap::WrapReverse,
            _ => FlexWrap::NoWrap,
        };

        let align_items = self.align_items.as_deref().and_then(|s| match s {
            "flex_start" | "start" => Some(AlignItems::FlexStart),
            "flex_end" | "end" => Some(AlignItems::FlexEnd),
            "center" => Some(AlignItems::Center),
            "baseline" => Some(AlignItems::Baseline),
            "stretch" => Some(AlignItems::Stretch),
            _ => None,
        });

        let align_self = self.align_self.as_deref().and_then(|s| match s {
            "flex_start" | "start" => Some(AlignSelf::FlexStart),
            "flex_end" | "end" => Some(AlignSelf::FlexEnd),
            "center" => Some(AlignSelf::Center),
            "baseline" => Some(AlignSelf::Baseline),
            "stretch" => Some(AlignSelf::Stretch),
            _ => None,
        });

        let justify_content = self.justify_content.as_deref().and_then(|s| match s {
            "flex_start" | "start" => Some(JustifyContent::FlexStart),
            "flex_end" | "end" => Some(JustifyContent::FlexEnd),
            "center" => Some(JustifyContent::Center),
            "space_between" => Some(JustifyContent::SpaceBetween),
            "space_around" => Some(JustifyContent::SpaceAround),
            "space_evenly" => Some(JustifyContent::SpaceEvenly),
            _ => None,
        });

        let size = Size {
            width: match (self.width, self.width_percent) {
                (Some(w), _) => length(w),
                (_, Some(p)) => percent(p / 100.0),
                _ => auto(),
            },
            height: match (self.height, self.height_percent) {
                (Some(h), _) => length(h),
                (_, Some(p)) => percent(p / 100.0),
                _ => auto(),
            },
        };

        let gap = Size {
            width: length(self.gap_col.or(self.gap).unwrap_or(0.0)),
            height: length(self.gap_row.or(self.gap).unwrap_or(0.0)),
        };

        let p_top = self.padding_top.or(self.padding).unwrap_or(0.0);
        let p_bottom = self.padding_bottom.or(self.padding).unwrap_or(0.0);
        let p_left = self.padding_left.or(self.padding).unwrap_or(0.0);
        let p_right = self.padding_right.or(self.padding).unwrap_or(0.0);

        let padding = Rect {
            top: length(p_top),
            bottom: length(p_bottom),
            left: length(p_left),
            right: length(p_right),
        };

        let m_top = self.margin_top.or(self.margin).unwrap_or(0.0);
        let m_bottom = self.margin_bottom.or(self.margin).unwrap_or(0.0);
        let m_left = self.margin_left.or(self.margin).unwrap_or(0.0);
        let m_right = self.margin_right.or(self.margin).unwrap_or(0.0);

        let margin = Rect {
            top: length(m_top),
            bottom: length(m_bottom),
            left: length(m_left),
            right: length(m_right),
        };

        let inset = Rect {
            top: self.top.map(length).unwrap_or_else(auto),
            bottom: self.bottom.map(length).unwrap_or_else(auto),
            left: self.left.map(length).unwrap_or_else(auto),
            right: self.right.map(length).unwrap_or_else(auto),
        };

        Style {
            display,
            position,
            flex_direction,
            flex_wrap,
            align_items,
            align_self,
            justify_content,
            size,
            gap,
            padding,
            margin,
            inset,
            flex_grow: self.flex_grow.unwrap_or(0.0),
            flex_shrink: self.flex_shrink.unwrap_or(1.0),
            flex_basis: self.flex_basis.map(length).unwrap_or_else(auto),
            ..Default::default()
        }
    }
}

/// Metadata header for a declarative UI document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WindowMetaSpec {
    #[serde(default = "default_title")]
    pub title: String,
    #[serde(default = "default_width")]
    pub width: f32,
    #[serde(default = "default_height")]
    pub height: f32,
    #[serde(default)]
    pub theme: Option<String>,
}

fn default_title() -> String { "AORUI App".to_string() }
fn default_width() -> f32 { 1280.0 }
fn default_height() -> f32 { 800.0 }

/// Declarative specification of a single Widget node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WidgetNodeSpec {
    pub id: String,
    #[serde(rename = "type")]
    pub widget_type: String,

    #[serde(default)]
    pub layout: LayoutStyleSpec,

    // Hierarchical children node IDs
    #[serde(default)]
    pub children: Vec<String>,

    // Widget specific attributes
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub variant: Option<String>, // "Default", "Primary", "Secondary", "Ghost", "Danger"
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub checked: Option<bool>,
    #[serde(default)]
    pub value: Option<f64>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub step: Option<f64>,
    #[serde(default)]
    pub muted: Option<bool>,
    #[serde(default)]
    pub radius: Option<f32>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub subtitle: Option<String>,
    #[serde(default)]
    pub delta: Option<String>,
    #[serde(default)]
    pub delta_positive: Option<bool>,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub items: Option<Vec<String>>,
    #[serde(default)]
    pub active_index: Option<usize>,

    // Custom Glass / Panel overrides
    #[serde(default, with = "color_serde_opt")]
    pub bg: Option<[f32; 4]>,
    #[serde(default, with = "color_serde_opt")]
    pub border: Option<[f32; 4]>,
    #[serde(default, with = "color_serde_opt")]
    pub color: Option<[f32; 4]>,

    // Event binding action names (e.g. "app:save_file")
    #[serde(default)]
    pub on_click: Option<String>,
    #[serde(default)]
    pub on_change: Option<String>,
}

pub mod color_serde_opt {
    use serde::{de::Error, Deserialize, Deserializer, Serializer};
    use crate::theme::color_serde::parse_hex_color;

    pub fn serialize<S>(color: &Option<[f32; 4]>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match color {
            Some(col) => crate::theme::color_serde::serialize(col, serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<[f32; 4]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum OptColorValue {
            FloatArray([f32; 4]),
            Hex(String),
        }

        let opt = Option::<OptColorValue>::deserialize(deserializer)?;
        match opt {
            Some(OptColorValue::FloatArray(arr)) => Ok(Some(arr)),
            Some(OptColorValue::Hex(s)) => parse_hex_color(&s).map(Some).map_err(D::Error::custom),
            None => Ok(None),
        }
    }
}

/// Root document representation of a complete declarative UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DeclarativeUiDoc {
    #[serde(default)]
    pub window: WindowMetaSpec,
    #[serde(default)]
    pub root: String, // ID of the root node
    #[serde(default)]
    pub nodes: Vec<WidgetNodeSpec>,
}

impl DeclarativeUiDoc {
    /// Parses a Declarative UI Document from a TOML string.
    pub fn from_toml(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }

    /// Serializes the UI document to a TOML string.
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    /// Loads a Declarative UI Document from a disk file.
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let doc = Self::from_toml(&content)?;
        Ok(doc)
    }

    /// Generates Rust boilerplate code with an AppState struct and event handlers
    /// for all `on_click` and `on_change` actions defined in this document.
    pub fn generate_rust_handlers(&self) -> String {
        let mut out = String::new();
        out.push_str("// [WFGY] Zone: SAFE | λ: 0.1 | Action: Auto-generated Rust event handlers for AORUI\n");
        out.push_str("// Generated automatically by AORUI Form Designer\n\n");
        out.push_str("use ui_widgets::declarative::EventRouter;\n\n");
        out.push_str("/// Application state manipulated by the business logic.\n");
        out.push_str("#[derive(Debug, Default)]\n");
        out.push_str("pub struct AppState {\n");
        out.push_str("    pub status_message: String,\n");
        out.push_str("    pub is_busy: bool,\n");
        out.push_str("}\n\n");
        out.push_str("/// Registers all business logic handlers for actions bound in the TOML layout.\n");
        out.push_str("pub fn register_app_handlers(router: &mut EventRouter<AppState>) {\n");

        let mut registered_actions = std::collections::HashSet::new();
        for node in &self.nodes {
            if let Some(action) = &node.on_click {
                if registered_actions.insert(action.clone()) {
                    let fn_name = action.replace(':', "_").replace('-', "_").replace('.', "_");
                    out.push_str(&format!("    // Action triggered by widget '{}' (on_click)\n", node.id));
                    out.push_str(&format!("    router.on(\"{}\", |state: &mut AppState, widget_id: &str| {{\n", action));
                    out.push_str(&format!("        println!(\"🚀 Action '{}' triggered by widget: '{{}}'\", widget_id);\n", action));
                    out.push_str(&format!("        state.status_message = format!(\"Action {} exécutée !\");\n", fn_name));
                    out.push_str("        // TODO: Implémenter la logique métier ici (LLM / Agent / Dev)\n");
                    out.push_str("    });\n\n");
                }
            }
            if let Some(action) = &node.on_change {
                if registered_actions.insert(action.clone()) {
                    let fn_name = action.replace(':', "_").replace('-', "_").replace('.', "_");
                    out.push_str(&format!("    // Action triggered by widget '{}' (on_change)\n", node.id));
                    out.push_str(&format!("    router.on(\"{}\", |state: &mut AppState, widget_id: &str| {{\n", action));
                    out.push_str(&format!("        println!(\"🔄 Action '{}' triggered by widget: '{{}}'\", widget_id);\n", action));
                    out.push_str(&format!("        state.status_message = format!(\"Valeur modifiée dans {}\");\n", fn_name));
                    out.push_str("        // TODO: Implémenter la logique métier ici\n");
                    out.push_str("    });\n\n");
                }
            }
        }

        out.push_str("}\n");
        out
    }

    /// Compiles and instantiates the complete [`WidgetTree`] from this document.
    pub fn build_tree(&self, tree: &mut WidgetTree) -> anyhow::Result<NodeId> {
        let mut node_map = HashMap::new();
        for node in &self.nodes {
            node_map.insert(node.id.clone(), node);
        }

        let root_id = if !self.root.is_empty() {
            &self.root
        } else if let Some(first) = self.nodes.first() {
            &first.id
        } else {
            anyhow::bail!("Declarative UI document contains no nodes.");
        };

        self.instantiate_node(root_id, &node_map, tree)
    }

    fn parse_icon(icon_str: Option<&str>) -> crate::kind::IconKind {
        match icon_str.unwrap_or("file").to_lowercase().as_str() {
            "settings" | "gear" => crate::kind::IconKind::Settings,
            "search" => crate::kind::IconKind::Search,
            "folder" => crate::kind::IconKind::Folder,
            "folder_open" => crate::kind::IconKind::FolderOpen,
            "terminal" => crate::kind::IconKind::Terminal,
            "cpu" => crate::kind::IconKind::Cpu,
            "network" => crate::kind::IconKind::Network,
            "play" => crate::kind::IconKind::Play,
            "pause" => crate::kind::IconKind::Pause,
            "refresh" => crate::kind::IconKind::Refresh,
            "copy" => crate::kind::IconKind::Copy,
            "trash" | "delete" => crate::kind::IconKind::Trash,
            "eye" => crate::kind::IconKind::Eye,
            "lock" => crate::kind::IconKind::Lock,
            "plus" | "add" => crate::kind::IconKind::Plus,
            "minus" => crate::kind::IconKind::Minus,
            "check" => crate::kind::IconKind::Check,
            "close" => crate::kind::IconKind::Close,
            _ => crate::kind::IconKind::File,
        }
    }

    fn instantiate_node(
        &self,
        node_id: &str,
        node_map: &HashMap<String, &WidgetNodeSpec>,
        tree: &mut WidgetTree,
    ) -> anyhow::Result<NodeId> {
        let node = node_map
            .get(node_id)
            .ok_or_else(|| anyhow::anyhow!("Node reference '{}' not found in document", node_id))?;

        let style = node.layout.to_style();

        // Recursively build children
        let mut child_node_ids = Vec::new();
        for child_id in &node.children {
            let child_nid = self.instantiate_node(child_id, node_map, tree)?;
            child_node_ids.push(child_nid);
        }

        let nid = match node.widget_type.to_lowercase().as_str() {
            "container" | "hbox" | "vbox" | "row" | "col" => tree.container(&child_node_ids, style)?,
            "panel" => tree.panel(&child_node_ids, node.bg, node.border, style)?,
            "card" => tree.card(&child_node_ids, node.bg, node.border, node.radius, style)?,
            "window" => {
                let title = node.title.as_deref().or(node.label.as_deref()).unwrap_or("Fenêtre");
                tree.window(node.id.as_str(), title, &child_node_ids, style)?
            }
            "label" => {
                let text = node.text.as_deref().or(node.label.as_deref()).unwrap_or("");
                if node.muted.unwrap_or(false) {
                    tree.label_muted(text, style)?
                } else {
                    tree.label(text, style)?
                }
            }
            "button" => {
                let label = node.label.as_deref().or(node.text.as_deref()).unwrap_or(&node.id);
                let variant = match node.variant.as_deref() {
                    Some("Primary") | Some("primary") => ButtonVariant::Primary,
                    Some("Secondary") | Some("secondary") => ButtonVariant::Secondary,
                    Some("Ghost") | Some("ghost") => ButtonVariant::Ghost,
                    Some("Danger") | Some("danger") => ButtonVariant::Danger,
                    _ => ButtonVariant::Default,
                };
                tree.button_variant(node.id.as_str(), label, variant, node.enabled.unwrap_or(true), style)?
            }
            "iconbutton" | "icon_button" => {
                let icon_kind = Self::parse_icon(node.icon.as_deref());
                tree.icon_button(node.id.as_str(), icon_kind, node.enabled.unwrap_or(true), style)?
            }
            "icon" => {
                let icon_kind = Self::parse_icon(node.icon.as_deref());
                let size = node.radius.unwrap_or(16.0);
                tree.icon(icon_kind, size, node.color, style)?
            }
            "badge" => {
                let text = node.text.as_deref().or(node.label.as_deref()).unwrap_or(&node.id);
                let badge_type = match node.variant.as_deref() {
                    Some("Success") | Some("success") => ListItemBadge::Success,
                    Some("Warning") | Some("warning") => ListItemBadge::Warning,
                    _ => ListItemBadge::None,
                };
                tree.badge(text, badge_type, style)?
            }
            "checkbox" => {
                tree.checkbox(node.id.as_str(), node.checked.unwrap_or(false), style)?
            }
            "textinput" | "text_input" => {
                let val = node.text.as_deref().unwrap_or("");
                let placeholder = node.placeholder.as_deref().unwrap_or("");
                tree.text_input(node.id.as_str(), val, placeholder, false, style)?
            }
            "textarea" | "text_area" => {
                let val = node.text.as_deref().unwrap_or("");
                let placeholder = node.placeholder.as_deref().unwrap_or("");
                tree.text_area(node.id.as_str(), val, placeholder, false, true, style)?
            }
            "passwordinput" | "password_input" => {
                let val = node.text.as_deref().unwrap_or("");
                let placeholder = node.placeholder.as_deref().unwrap_or("");
                tree.password_input(node.id.as_str(), val, placeholder, false, false, style)?
            }
            "numberinput" | "number_input" => {
                let val = node.value.unwrap_or(0.0);
                let min = node.min.unwrap_or(0.0);
                let max = node.max.unwrap_or(100.0);
                let step = node.step.unwrap_or(1.0);
                tree.number_input(node.id.as_str(), val, min, max, step, 0, false, style)?
            }
            "slider" => {
                let val = node.value.unwrap_or(0.0) as f32;
                let min = node.min.unwrap_or(0.0) as f32;
                let max = node.max.unwrap_or(1.0) as f32;
                tree.slider(node.id.as_str(), min, max, val, style)?
            }
            "toggle" => {
                tree.toggle(node.id.as_str(), node.checked.unwrap_or(false), style)?
            }
            "radio" | "radiobutton" | "radio_button" => {
                let group = node.group_id.as_deref().unwrap_or("default");
                let label = node.label.as_deref().or(node.text.as_deref()).unwrap_or(&node.id);
                tree.radio(node.id.as_str(), group, label, node.checked.unwrap_or(false), style)?
            }
            "progressbar" | "progress_bar" => {
                let progress = node.value.unwrap_or(0.5) as f32;
                tree.progress_bar(progress, style)?
            }
            "metriccard" | "metric_card" => {
                let title = node.title.as_deref().or(node.label.as_deref()).unwrap_or("Metric");
                let value = node.text.as_deref().unwrap_or("0");
                let delta = node.delta.as_deref().map(|d| (d, node.delta_positive.unwrap_or(true)));
                tree.metric_card(title, value, delta, style)?
            }
            "tabbar" | "tabs" => {
                let default_tabs = vec!["Général".to_string(), "Paramètres".to_string(), "Avancé".to_string()];
                let tabs_list = node.items.as_ref().unwrap_or(&default_tabs);
                let active = node.active_index.unwrap_or(0);
                let tab_style = Style {
                    size: Size { width: length(90.0), height: length(28.0) },
                    ..Default::default()
                };
                tree.tabbar(node.id.as_str(), tabs_list, active, tab_style, style)?
            }
            "segmentedcontrol" | "segmented_control" => {
                let default_opts = vec!["Option 1".to_string(), "Option 2".to_string()];
                let opts = node.items.as_ref().unwrap_or(&default_opts);
                let sel = node.active_index.unwrap_or(0);
                let opt_style = Style {
                    size: Size { width: length(80.0), height: length(26.0) },
                    ..Default::default()
                };
                tree.segmented_control(node.id.as_str(), opts, sel, opt_style, style)?
            }
            "dropdown" => {
                let label = node.label.as_deref().unwrap_or("Sélectionnez");
                let selected = node.text.as_deref().unwrap_or("Option Active");
                tree.dropdown(node.id.as_str(), label, selected, node.checked.unwrap_or(false), style)?
            }
            "colorswatch" | "color_swatch" => {
                let col = node.color.or(node.bg).unwrap_or([0.2, 0.6, 0.9, 1.0]);
                tree.color_swatch(node.id.as_str(), col, node.label.as_deref(), style)?
            }
            "colorpicker" | "color_picker" => {
                let col = node.color.or(node.bg).unwrap_or([0.2, 0.6, 0.9, 1.0]);
                tree.color_picker(node.id.as_str(), col, crate::color::ColorSpace::Rgb, style)?
            }
            "toast" => {
                let title = node.title.as_deref().unwrap_or("Notification");
                let message = node.text.as_deref().unwrap_or("Message");
                tree.toast(node.id.as_str(), title, message, crate::kind::ToastKind::Info, style)?
            }
            "tooltip" => {
                let text = node.text.as_deref().unwrap_or("Info-bulle");
                tree.tooltip_simple(text, style)?
            }
            "table" => {
                let cols = vec![("ID", 60.0, None), ("Nom", 140.0, Some(true)), ("Statut", 90.0, None)];
                let rows: Vec<Vec<(&str, ListItemBadge)>> = vec![
                    vec![("001", ListItemBadge::None), ("Formulaire Client", ListItemBadge::None), ("Actif", ListItemBadge::Success)],
                    vec![("002", ListItemBadge::None), ("Rapport Ventes", ListItemBadge::None), ("En attente", ListItemBadge::Warning)],
                ];
                tree.table(node.id.as_str(), &cols, &rows, Some(0), 24.0, style)?
            }
            "treeview" | "tree_view" => {
                let nodes_data = vec![
                    ("n1", "📁 Racine Projet", 0, true, true, false),
                    ("n2", "📄 Form1.toml", 1, false, false, true),
                    ("n3", "⚙️ Config.toml", 1, false, false, false),
                ];
                let item_style = Style {
                    size: Size { width: percent(1.0), height: length(22.0) },
                    ..Default::default()
                };
                tree.tree_view(node.id.as_str(), &nodes_data, item_style, style)?
            }
            "divider" => {
                tree.divider(false, style)?
            }
            other => {
                tracing::warn!("Unknown declarative widget type '{}' for node '{}', falling back to container", other, node.id);
                tree.container(&child_node_ids, style)?
            }
        };

        Ok(nid)
    }

    /// Finds the action associated with a clicked widget_id, and dispatches it via the [`EventRouter`].
    pub fn dispatch_click<T>(&self, event_router: &EventRouter<T>, state: &mut T, clicked_widget_id: &str) -> bool {
        for node in &self.nodes {
            if node.id == clicked_widget_id {
                if let Some(action) = &node.on_click {
                    return event_router.dispatch(state, action, clicked_widget_id);
                }
            }
        }
        false
    }
}

/// Action and event callback router for declarative UI elements.
#[derive(Default)]
pub struct EventRouter<T> {
    handlers: HashMap<String, Box<dyn Fn(&mut T, &str) + Send + Sync>>,
}

impl<T> EventRouter<T> {
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Registers an event handler action by key name (e.g. "app:save_file").
    pub fn on<F>(&mut self, action_name: impl Into<String>, handler: F)
    where
        F: Fn(&mut T, &str) + Send + Sync + 'static,
    {
        self.handlers.insert(action_name.into(), Box::new(handler));
    }

    /// Dispatches an event string to the registered handler.
    pub fn dispatch(&self, state: &mut T, action_name: &str, widget_id: &str) -> bool {
        if let Some(handler) = self.handlers.get(action_name) {
            handler(state, widget_id);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_layout::AvailableSpace;

    #[test]
    fn test_declarative_doc_toml_roundtrip() {
        let toml_str = r#"
            [window]
            title = "Studio Pro"
            width = 1920.0
            height = 1080.0
            theme = "themes/studio_pro.toml"

            root = "main_panel"

            [[nodes]]
            id = "main_panel"
            type = "Panel"
            layout = { position = "absolute", top = 0.0, left = 0.0, width = 1920.0, height = 1080.0, direction = "column", gap = 8.0 }
            children = ["top_header", "btn_start"]

            [[nodes]]
            id = "top_header"
            type = "Label"
            text = "🌌 AORUI Declarative Studio"
            layout = { width = 400.0, height = 30.0 }

            [[nodes]]
            id = "btn_start"
            type = "Button"
            label = "🚀 Démarrer"
            variant = "Primary"
            layout = { width = 160.0, height = 32.0 }
            on_click = "app:start"
        "#;

        let doc = DeclarativeUiDoc::from_toml(toml_str).expect("parse declarative toml");
        assert_eq!(doc.window.title, "Studio Pro");
        assert_eq!(doc.nodes.len(), 3);
        assert_eq!(doc.nodes[0].id, "main_panel");
        assert_eq!(doc.nodes[2].on_click.as_deref(), Some("app:start"));

        let mut tree = WidgetTree::new();
        let root_nid = doc.build_tree(&mut tree).expect("build tree from doc");
        
        let space = Size {
            width: AvailableSpace::Definite(1920.0),
            height: AvailableSpace::Definite(1080.0),
        };
        assert!(tree.compute(root_nid, space).is_ok());
    }
}
