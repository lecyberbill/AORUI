// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0 | Action: High-grade Autonomous Agent Enterprise Workstation in AORUI
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use serde_json::{json, Value};
use tokio::sync::mpsc;
use ui_core::{StepStatus, UiEvent, UiPatch};
use ui_gpu::GpuRenderer;
use ui_layout::{
    auto, length, AlignItems, AvailableSpace, FlexDirection,
    NodeId, Position, Rect, Size, Style,
};
use ui_widgets::{
    FontFamily, InteractionKey, InteractionState, ListItemBadge, Theme, ThemeWatcher,
    WidgetId, WidgetTree,
};

use agent_runtime::{
    Agent, AgentConfig, BoxFuture, PlanDecision, Planner, Tool, ToolError, ToolRegistry,
};

use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

const WINDOW_WIDTH: f32 = 1120.0;
const WINDOW_HEIGHT: f32 = 780.0;

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

fn rect_margin_b(b: f32) -> Rect<ui_layout::LengthPercentageAuto> {
    Rect {
        left: auto(),
        right: auto(),
        top: auto(),
        bottom: length(b),
    }
}

fn rect_margin_t(t: f32) -> Rect<ui_layout::LengthPercentageAuto> {
    Rect {
        left: auto(),
        right: auto(),
        top: length(t),
        bottom: auto(),
    }
}

// ============================================================================
// 1. Outils Déterministes Exécutables par l'Agent
// ============================================================================

struct SetFieldTool {
    tx_patches: crossbeam_channel::Sender<UiPatch>,
}

impl Tool for SetFieldTool {
    fn name(&self) -> &str {
        "set_field"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["widget_id", "text"],
            "properties": {
                "widget_id": { "type": "string" },
                "text": { "type": "string" }
            }
        })
    }

    fn execute<'a>(&'a self, args: Value) -> BoxFuture<'a, Result<Value, ToolError>> {
        Box::pin(async move {
            let widget_id = args["widget_id"].as_str().ok_or_else(|| ToolError("Missing widget_id".into()))?;
            let text = args["text"].as_str().ok_or_else(|| ToolError("Missing text".into()))?;

            tokio::time::sleep(Duration::from_millis(250)).await;

            let _ = self.tx_patches.send(UiPatch::WidgetTextSet {
                widget_id: widget_id.to_string(),
                text: text.to_string(),
            });

            Ok(json!({ "status": "updated", "widget": widget_id, "new_value": text }))
        })
    }
}

struct SwitchTabTool {
    tx_patches: crossbeam_channel::Sender<UiPatch>,
}

impl Tool for SwitchTabTool {
    fn name(&self) -> &str {
        "switch_tab"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["tab_index"],
            "properties": {
                "tab_index": { "type": "integer" }
            }
        })
    }

    fn execute<'a>(&'a self, args: Value) -> BoxFuture<'a, Result<Value, ToolError>> {
        Box::pin(async move {
            let tab_index = args["tab_index"].as_u64().ok_or_else(|| ToolError("Missing tab_index".into()))? as usize;

            tokio::time::sleep(Duration::from_millis(200)).await;

            let _ = self.tx_patches.send(UiPatch::TabActivated {
                widget_id: "cluster_tabs".to_string(),
                tab_index,
            });

            Ok(json!({ "status": "tab_switched", "tab_index": tab_index }))
        })
    }
}

struct ToggleServiceTool {
    tx_patches: crossbeam_channel::Sender<UiPatch>,
}

impl Tool for ToggleServiceTool {
    fn name(&self) -> &str {
        "toggle_service"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["widget_id", "checked"],
            "properties": {
                "widget_id": { "type": "string" },
                "checked": { "type": "boolean" }
            }
        })
    }

    fn execute<'a>(&'a self, args: Value) -> BoxFuture<'a, Result<Value, ToolError>> {
        Box::pin(async move {
            let widget_id = args["widget_id"].as_str().ok_or_else(|| ToolError("Missing widget_id".into()))?;
            let checked = args["checked"].as_bool().ok_or_else(|| ToolError("Missing checked".into()))?;

            tokio::time::sleep(Duration::from_millis(200)).await;

            let _ = self.tx_patches.send(UiPatch::WidgetCheckedSet {
                widget_id: widget_id.to_string(),
                checked,
            });

            Ok(json!({ "status": "toggled", "widget": widget_id, "checked": checked }))
        })
    }
}

struct UpdateMetricTool {
    tx_patches: crossbeam_channel::Sender<UiPatch>,
}

impl Tool for UpdateMetricTool {
    fn name(&self) -> &str {
        "update_metric"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["key", "value"],
            "properties": {
                "key": { "type": "string" },
                "value": { "type": "number" }
            }
        })
    }

    fn execute<'a>(&'a self, args: Value) -> BoxFuture<'a, Result<Value, ToolError>> {
        Box::pin(async move {
            let key = args["key"].as_str().ok_or_else(|| ToolError("Missing key".into()))?;
            let value = args["value"].as_f64().ok_or_else(|| ToolError("Missing value".into()))? as f32;

            tokio::time::sleep(Duration::from_millis(150)).await;

            let _ = self.tx_patches.send(UiPatch::MetricUpdated {
                key: key.to_string(),
                value,
            });

            Ok(json!({ "status": "metric_updated", "key": key, "value": value }))
        })
    }
}

struct NotifyModalTool {
    tx_patches: crossbeam_channel::Sender<UiPatch>,
}

impl Tool for NotifyModalTool {
    fn name(&self) -> &str {
        "notify_modal"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["title", "content"],
            "properties": {
                "title": { "type": "string" },
                "content": { "type": "string" }
            }
        })
    }

    fn execute<'a>(&'a self, args: Value) -> BoxFuture<'a, Result<Value, ToolError>> {
        Box::pin(async move {
            let title = args["title"].as_str().ok_or_else(|| ToolError("Missing title".into()))?;
            let content = args["content"].as_str().ok_or_else(|| ToolError("Missing content".into()))?;

            tokio::time::sleep(Duration::from_millis(200)).await;

            let _ = self.tx_patches.send(UiPatch::ModalRequested {
                title: title.to_string(),
                content: content.to_string(),
            });

            Ok(json!({ "status": "modal_shown" }))
        })
    }
}

// ============================================================================
// 2. Planificateur Cognitif de l'Agent
// ============================================================================

struct DynamicAgentPlanner {
    pending_plan: VecDeque<PlanDecision>,
}

impl DynamicAgentPlanner {
    pub fn new() -> Self {
        Self {
            pending_plan: VecDeque::new(),
        }
    }

    fn compile_plan_for_prompt(&mut self, prompt: &str) {
        let p = prompt.to_lowercase();
        self.pending_plan.clear();

        if p.contains("prod") || p.contains("cluster") || p.contains("database") {
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "switch_tab".into(),
                args: json!({ "tab_index": 1 }),
            });
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "set_field".into(),
                args: json!({ "widget_id": "input_host", "text": "db-prod-eu-west-1.aorui.cloud" }),
            });
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "set_field".into(),
                args: json!({ "widget_id": "input_port", "text": "5432" }),
            });
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "toggle_service".into(),
                args: json!({ "widget_id": "chk_ssl", "checked": true }),
            });
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "toggle_service".into(),
                args: json!({ "widget_id": "chk_backup", "checked": true }),
            });
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "update_metric".into(),
                args: json!({ "key": "db_sync", "value": 99.9 }),
            });
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "notify_modal".into(),
                args: json!({
                    "title": "⚡ Cluster Production Configuré",
                    "content": "L'agent a configuré le cluster PostgreSQL, activé le chiffrement TLS 1.3 et validé la réplication à 99.9%."
                }),
            });
        } else if p.contains("audit") || p.contains("securite") || p.contains("security") || p.contains("sante") {
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "switch_tab".into(),
                args: json!({ "tab_index": 0 }),
            });
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "update_metric".into(),
                args: json!({ "key": "cpu_load", "value": 24.5 }),
            });
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "update_metric".into(),
                args: json!({ "key": "memory_used", "value": 42.0 }),
            });
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "update_metric".into(),
                args: json!({ "key": "latency_ms", "value": 8.4 }),
            });
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "toggle_service".into(),
                args: json!({ "widget_id": "chk_firewall", "checked": true }),
            });
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "notify_modal".into(),
                args: json!({
                    "title": "🛡️ Audit de Sécurité Réussi",
                    "content": "Inspection des micro-services terminée : 0 vulnérabilité détectée. Pare-feu WAF actif et latence < 10ms."
                }),
            });
        } else if p.contains("optim") || p.contains("ram") || p.contains("memoire") {
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "switch_tab".into(),
                args: json!({ "tab_index": 0 }),
            });
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "update_metric".into(),
                args: json!({ "key": "memory_used", "value": 18.2 }),
            });
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "switch_tab".into(),
                args: json!({ "tab_index": 2 }),
            });
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "notify_modal".into(),
                args: json!({
                    "title": "🚀 Mémoire RAM Optimisée",
                    "content": "Purge des caches terminée. Utilisation de la mémoire vive réduite à 18.2%."
                }),
            });
        } else {
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "update_metric".into(),
                args: json!({ "key": "cpu_load", "value": 31.0 }),
            });
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "notify_modal".into(),
                args: json!({
                    "title": "🤖 Instruction Traitée",
                    "content": format!("Commande traitée avec succès par l'agent autonome : '{}'", prompt)
                }),
            });
        }

        self.pending_plan.push_back(PlanDecision::Complete);
    }
}

impl Planner for DynamicAgentPlanner {
    fn plan<'a>(&'a mut self, scratchpad: &'a [String]) -> BoxFuture<'a, PlanDecision> {
        Box::pin(async move {
            if self.pending_plan.is_empty() {
                if let Some(last_prompt_entry) = scratchpad.iter().rev().find(|s| s.starts_with("prompt: ")) {
                    let prompt = last_prompt_entry.trim_start_matches("prompt: ");
                    self.compile_plan_for_prompt(prompt);
                }
            }

            self.pending_plan.pop_front().unwrap_or(PlanDecision::Complete)
        })
    }
}

// ============================================================================
// 3. Éditeur de Texte & État Applicatif
// ============================================================================

#[derive(Default, Clone)]
struct TextEditorState {
    text: String,
    cursor: usize,
    selection: Option<(usize, usize)>,
}

impl TextEditorState {
    fn new(initial: &str) -> Self {
        Self {
            text: initial.to_string(),
            cursor: initial.len(),
            selection: None,
        }
    }

    fn insert_char(&mut self, c: char) {
        self.text.insert(self.cursor, c);
        self.cursor += 1;
        self.selection = None;
    }

    fn backspace(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.text.remove(self.cursor);
        }
        self.selection = None;
    }
}

struct WorkstationState {
    active_tab: usize,
    prompt_editor: TextEditorState,
    host_editor: TextEditorState,
    password_editor: TextEditorState,
    password_revealed: bool,

    agent_status: String,
    agent_glow: [f32; 4],
    step_logs: Vec<(String, String, StepStatus)>,
    scratchpad: Vec<String>,

    cpu_load: f32,
    memory_used: f32,
    db_sync: f32,
    latency_ms: f32,

    fader_low: f32,
    fader_mid: f32,
    fader_high: f32,
    network_intensity: f32,

    port_spin: f64,
    chk_ssl: bool,
    chk_backup: bool,
    chk_firewall: bool,
    chk_turbo: bool,
    selected_env: String,
    security_policy: String,
    dropdown_open: bool,

    focused_input: Option<String>,
    active_modal: Option<(String, String)>,
}

impl Default for WorkstationState {
    fn default() -> Self {
        Self {
            active_tab: 0,
            prompt_editor: TextEditorState::new("Configurer le cluster de production et valider la réplication"),
            host_editor: TextEditorState::new("127.0.0.1"),
            password_editor: TextEditorState::new("sk_prod_live_998348271049"),
            password_revealed: false,

            agent_status: "IDLE".to_string(),
            agent_glow: [0.0, 0.85, 1.0, 1.0],
            step_logs: Vec::new(),
            scratchpad: Vec::new(),

            cpu_load: 24.5,
            memory_used: 42.0,
            db_sync: 99.9,
            latency_ms: 8.4,

            fader_low: 45.0,
            fader_mid: 72.0,
            fader_high: 60.0,
            network_intensity: 75.0,

            port_spin: 5432.0,
            chk_ssl: true,
            chk_backup: true,
            chk_firewall: true,
            chk_turbo: true,
            selected_env: "Cluster: Production (US-East 01)".to_string(),
            security_policy: "strict".to_string(),
            dropdown_open: false,

            focused_input: Some("prompt_input".to_string()),
            active_modal: None,
        }
    }
}

impl WorkstationState {
    fn apply_patch(&mut self, patch: UiPatch) {
        match patch {
            UiPatch::WidgetTextSet { widget_id, text } => match widget_id.as_str() {
                "input_host" => self.host_editor = TextEditorState::new(&text),
                "input_port" => {
                    if let Ok(v) = text.parse::<f64>() {
                        self.port_spin = v;
                    }
                }
                "prompt_input" => self.prompt_editor = TextEditorState::new(&text),
                _ => {}
            },
            UiPatch::WidgetCheckedSet { widget_id, checked } => match widget_id.as_str() {
                "chk_ssl" => self.chk_ssl = checked,
                "chk_backup" => self.chk_backup = checked,
                "chk_firewall" => self.chk_firewall = checked,
                "chk_turbo" => self.chk_turbo = checked,
                _ => {}
            },
            UiPatch::TabActivated { tab_index, .. } => {
                self.active_tab = tab_index.clamp(0, 2);
            }
            UiPatch::StatusChanged { state, glow_hue } => {
                self.agent_status = state.to_uppercase();
                self.agent_glow = glow_hue;
            }
            UiPatch::StepLogged { step_id, tool, status } => {
                if let Some(existing) = self.step_logs.iter_mut().find(|(id, _, _)| id == &step_id) {
                    existing.2 = status;
                } else {
                    self.step_logs.push((step_id, tool, status));
                    if self.step_logs.len() > 18 {
                        self.step_logs.remove(0);
                    }
                }
            }
            UiPatch::MetricUpdated { key, value } => match key.as_str() {
                "cpu_load" => self.cpu_load = value,
                "memory_used" => self.memory_used = value,
                "latency_ms" => self.latency_ms = value,
                "db_sync" => self.db_sync = value,
                _ => {}
            },
            UiPatch::ScratchpadAppended(entry) => {
                self.scratchpad.push(entry);
                if self.scratchpad.len() > 25 {
                    self.scratchpad.remove(0);
                }
            }
            UiPatch::ModalRequested { title, content } => {
                self.active_modal = Some((title, content));
            }
            _ => {}
        }
    }
}

// ============================================================================
// 4. Construction de l'Arbre Graphique (Aesthetics Pro & Cohérent)
// ============================================================================

fn build_workstation_ui(tree: &mut WidgetTree, state: &WorkstationState) -> NodeId {
    // 1. Top Window Header
    let app_icon = tree.icon(ui_widgets::IconKind::Cpu, 18.0, Some([0.0, 0.85, 1.0, 1.0]), leaf(20.0, 20.0)).unwrap();
    let title_label = tree.label("AORUI — Enterprise Autonomous Agent Workstation", leaf(420.0, 22.0)).unwrap();
    let header_left = tree.container(&[app_icon, title_label], row(8.0)).unwrap();

    let crumbs = [
        ("nav_hub", "AORUI Hub"),
        ("nav_cluster", "US-East-01"),
        ("nav_agent", "Autonomous Runtime"),
    ];
    let breadcrumb_node = tree.breadcrumb("main_breadcrumb", &crumbs, leaf(90.0, 20.0), row(2.0)).unwrap();

    let status_badge = tree.badge(
        &format!("AGENT: {}", state.agent_status),
        if state.agent_status == "IDLE" {
            ListItemBadge::Success
        } else {
            ListItemBadge::Warning
        },
        leaf(140.0, 26.0),
    ).unwrap();

    let top_header = tree.container(
        &[header_left, breadcrumb_node, status_badge],
        Style {
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::Center),
            justify_content: Some(ui_layout::JustifyContent::SpaceBetween),
            size: Size { width: length(1072.0), height: length(32.0) },
            margin: rect_margin_b(8.0),
            ..Default::default()
        },
    ).unwrap();

    // 2. Master Command Hub (Input + Execution + Preset Badges)
    let prompt_focused = state.focused_input.as_deref() == Some("prompt_input");
    let prompt_input = tree.text_input_with_cursor(
        WidgetId::new("prompt_input"),
        &state.prompt_editor.text,
        "Enter natural language instruction for the autonomous agent...",
        prompt_focused,
        state.prompt_editor.cursor,
        state.prompt_editor.selection,
        leaf(860.0, 34.0),
    ).unwrap();

    let execute_btn = tree.button_variant(
        WidgetId::new("btn_send_prompt"),
        "⚡ EXECUTE INTENT",
        ui_widgets::ButtonVariant::Primary,
        true,
        leaf(188.0, 34.0),
    ).unwrap();

    let prompt_row = tree.container(&[prompt_input, execute_btn], row(8.0)).unwrap();

    let p_lbl = tree.label_muted("Scenario Presets:", leaf(120.0, 24.0)).unwrap();
    let btn_sc_prod = tree.button_variant(
        WidgetId::new("btn_sc_prod"),
        "⚡ Auto-Config Prod DB",
        ui_widgets::ButtonVariant::Secondary,
        true,
        leaf(190.0, 26.0),
    ).unwrap();
    let btn_sc_sec = tree.button_variant(
        WidgetId::new("btn_sc_sec"),
        "🛡️ Zero-Trust Security Audit",
        ui_widgets::ButtonVariant::Secondary,
        true,
        leaf(210.0, 26.0),
    ).unwrap();
    let btn_sc_opt = tree.button_variant(
        WidgetId::new("btn_sc_opt"),
        "🚀 RAM Cache Optimizer",
        ui_widgets::ButtonVariant::Secondary,
        true,
        leaf(190.0, 26.0),
    ).unwrap();

    let presets_row = tree.container(&[p_lbl, btn_sc_prod, btn_sc_sec, btn_sc_opt], row(8.0)).unwrap();

    let command_card = tree.card(
        &[prompt_row, presets_row],
        None,
        None,
        Some(6.0),
        Style {
            size: Size { width: length(1072.0), height: length(100.0) },
            flex_direction: FlexDirection::Column,
            gap: Size { width: length(0.0), height: length(8.0) },
            padding: Rect { left: length(14.0), right: length(14.0), top: length(10.0), bottom: length(10.0) },
            margin: rect_margin_b(12.0),
            ..Default::default()
        },
    ).unwrap();

    // 3. Left Panel : Cognitive Execution Stream & Memory
    let timeline_title = tree.label("Cognitive Tool Timeline (Live Tokio Loop):", leaf(480.0, 20.0)).unwrap();

    let mut step_nodes = Vec::new();
    for (_idx, (id, tool, status)) in state.step_logs.iter().enumerate() {
        let status_badge = match status {
            StepStatus::Pending => ListItemBadge::None,
            StepStatus::Running => ListItemBadge::Warning,
            StepStatus::Success => ListItemBadge::Success,
            StepStatus::Failed(_) => ListItemBadge::Warning,
        };
        let status_str = match status {
            StepStatus::Pending => "PENDING",
            StepStatus::Running => "RUNNING",
            StepStatus::Success => "SUCCESS",
            StepStatus::Failed(e) => e.as_str(),
        };
        let b = tree.badge(&format!("[{}] {} -> {}", id, tool, status_str), status_badge, leaf(480.0, 22.0)).unwrap();
        step_nodes.push(b);
    }

    if step_nodes.is_empty() {
        let empty_lbl = tree.label_muted("No actions executed yet. Submit a prompt or click a preset.", leaf(480.0, 20.0)).unwrap();
        step_nodes.push(empty_lbl);
    }

    let timeline_box = tree.container(&step_nodes, column(4.0)).unwrap();

    let mem_title = tree.label("Agent Scratchpad Deductions:", leaf(480.0, 20.0)).unwrap();
    let mut mem_nodes = Vec::new();
    for (_idx, mem) in state.scratchpad.iter().rev().take(6).enumerate() {
        let m = tree.label_muted(&format!("• {}", mem), leaf(480.0, 18.0)).unwrap();
        mem_nodes.push(m);
    }
    if mem_nodes.is_empty() {
        let m_empty = tree.label_muted("• Agent FSM is initialized in IDLE state.", leaf(480.0, 18.0)).unwrap();
        mem_nodes.push(m_empty);
    }
    let mem_box = tree.container(&mem_nodes, column(2.0)).unwrap();

    let left_panel = tree.card(
        &[timeline_title, timeline_box, mem_title, mem_box],
        None,
        None,
        Some(6.0),
        Style {
            size: Size { width: length(516.0), height: length(550.0) },
            flex_direction: FlexDirection::Column,
            gap: Size { width: length(0.0), height: length(8.0) },
            padding: Rect { left: length(14.0), right: length(14.0), top: length(10.0), bottom: length(10.0) },
            ..Default::default()
        },
    ).unwrap();

    // 4. Right Panel : Multi-Tab Target Cluster System
    let tabs = ["📊 Telemetry", "⚙️ Service Config", "🛡️ Security Policy"];
    let tabbar = tree.tabbar(
        WidgetId::new("cluster_tabs"),
        &tabs,
        state.active_tab,
        leaf(160.0, 30.0),
        row(4.0),
    ).unwrap();

    let tab_content = match state.active_tab {
        0 => {
            // Tab 0: Telemetry & Gauges
            let c1 = tree.metric_card(
                "CPU Core Load",
                &format!("{:.1}%", state.cpu_load),
                Some((if state.cpu_load > 75.0 { "- Surcharge" } else { "+ Nominal" }, state.cpu_load <= 75.0)),
                leaf(164.0, 80.0),
            ).unwrap();

            let c2 = tree.metric_card(
                "Memory RAM Used",
                &format!("{:.1}%", state.memory_used),
                Some((if state.memory_used > 70.0 { "High Load" } else { "+ Optimal" }, state.memory_used <= 70.0)),
                leaf(164.0, 80.0),
            ).unwrap();

            let c3 = tree.metric_card(
                "Database Sync",
                &format!("{:.1}%", state.db_sync),
                Some(("+ Realtime", true)),
                leaf(164.0, 80.0),
            ).unwrap();

            let metrics_grid = tree.grid(3, 8.0, 0.0, &[c1, c2, c3], leaf(516.0, 80.0)).unwrap();

            let f1 = tree.slider_vertical(WidgetId::new("fader_low"), 0.0, 100.0, state.fader_low, leaf(20.0, 48.0)).unwrap();
            let f1_lbl = tree.label_muted("Lo", leaf(20.0, 12.0)).unwrap();
            let f1_box = tree.container(&[f1, f1_lbl], column(2.0)).unwrap();

            let f2 = tree.slider_vertical(WidgetId::new("fader_mid"), 0.0, 100.0, state.fader_mid, leaf(20.0, 48.0)).unwrap();
            let f2_lbl = tree.label_muted("Mid", leaf(20.0, 12.0)).unwrap();
            let f2_box = tree.container(&[f2, f2_lbl], column(2.0)).unwrap();

            let f3 = tree.slider_vertical(WidgetId::new("fader_high"), 0.0, 100.0, state.fader_high, leaf(20.0, 48.0)).unwrap();
            let f3_lbl = tree.label_muted("Hi", leaf(20.0, 12.0)).unwrap();
            let f3_box = tree.container(&[f3, f3_lbl], column(2.0)).unwrap();

            let faders_row = tree.container(&[f1_box, f2_box, f3_box], row(6.0)).unwrap();

            let ring_gauge = tree.progress_ring(state.cpu_load / 100.0, Some("CPU"), leaf(52.0, 52.0)).unwrap();
            let pie_disc = tree.progress_pie(state.memory_used / 100.0, Some("RAM"), leaf(52.0, 52.0)).unwrap();

            let slider_h = tree.slider(WidgetId::new("net_slider"), 0.0, 100.0, state.network_intensity, leaf(190.0, 20.0)).unwrap();
            let slider_lbl = tree.label_muted("Network Bandwidth Throttle (Mbps)", leaf(190.0, 16.0)).unwrap();
            let slider_box = tree.container(&[slider_lbl, slider_h], column(4.0)).unwrap();

            let gauges_row = tree.container(&[faders_row, ring_gauge, pie_disc, slider_box], row(14.0)).unwrap();

            let t1 = tree.toggle(WidgetId::new("chk_firewall"), state.chk_firewall, leaf(42.0, 22.0)).unwrap();
            let t1_lbl = tree.label("Core WAF Firewall & Anti-DDoS Filter", leaf(440.0, 22.0)).unwrap();
            let t1_row = tree.container(&[t1, t1_lbl], row(8.0)).unwrap();

            let t2 = tree.toggle(WidgetId::new("chk_ssl"), state.chk_ssl, leaf(42.0, 22.0)).unwrap();
            let t2_lbl = tree.label("TLS 1.3 Strict End-to-End Encryption", leaf(440.0, 22.0)).unwrap();
            let t2_row = tree.container(&[t2, t2_lbl], row(8.0)).unwrap();

            let t3 = tree.toggle(WidgetId::new("chk_backup"), state.chk_backup, leaf(42.0, 22.0)).unwrap();
            let t3_lbl = tree.label("Continuous Cluster Snapshot Replication", leaf(440.0, 22.0)).unwrap();
            let t3_row = tree.container(&[t3, t3_lbl], row(8.0)).unwrap();

            let t4 = tree.toggle(WidgetId::new("chk_turbo"), state.chk_turbo, leaf(42.0, 22.0)).unwrap();
            let t4_lbl = tree.label("GPU Turbo Hardware Graphics Pipeline Acceleration", leaf(440.0, 22.0)).unwrap();
            let t4_row = tree.container(&[t4, t4_lbl], row(8.0)).unwrap();

            tree.container(
                &[metrics_grid, gauges_row, t1_row, t2_row, t3_row, t4_row],
                Style {
                    flex_direction: FlexDirection::Column,
                    gap: Size { width: length(0.0), height: length(12.0) },
                    margin: rect_margin_t(8.0),
                    ..Default::default()
                },
            ).unwrap()
        }
        1 => {
            // Tab 1: Configuration Form
            let env_lbl = tree.label_muted("Target Cluster Environment:", leaf(516.0, 18.0)).unwrap();
            let env_dropdown = tree.dropdown(
                WidgetId::new("env_dropdown"),
                "Cluster",
                &state.selected_env,
                state.dropdown_open,
                leaf(516.0, 30.0),
            ).unwrap();

            let host_lbl = tree.label_muted("Database Connection Hostname:", leaf(516.0, 18.0)).unwrap();
            let host_focused = state.focused_input.as_deref() == Some("input_host");
            let host_input = tree.text_input_with_cursor(
                WidgetId::new("input_host"),
                &state.host_editor.text,
                "db.prod.internal...",
                host_focused,
                state.host_editor.cursor,
                state.host_editor.selection,
                leaf(516.0, 30.0),
            ).unwrap();

            let port_lbl = tree.label_muted("Service Port (1024-65535):", leaf(250.0, 18.0)).unwrap();
            let port_spin = tree.number_input_state(
                WidgetId::new("input_port"),
                state.port_spin,
                1024.0,
                65535.0,
                1.0,
                0,
                false,
                true,
                leaf(250.0, 30.0),
            ).unwrap();
            let port_box = tree.container(&[port_lbl, port_spin], column(4.0)).unwrap();

            let pwd_lbl = tree.label_muted("Master Cluster Access Token:", leaf(250.0, 18.0)).unwrap();
            let pwd_focused = state.focused_input.as_deref() == Some("master_token_pwd");
            let pwd_input = tree.password_input_with_cursor(
                WidgetId::new("master_token_pwd"),
                &state.password_editor.text,
                "Master Token...",
                pwd_focused,
                state.password_revealed,
                state.password_editor.cursor,
                leaf(250.0, 30.0),
            ).unwrap();
            let pwd_box = tree.container(&[pwd_lbl, pwd_input], column(4.0)).unwrap();

            let creds_row = tree.container(&[port_box, pwd_box], row(16.0)).unwrap();

            let save_btn = tree.button_variant(
                WidgetId::new("btn_save_config"),
                "💾 Save Cluster Configuration",
                ui_widgets::ButtonVariant::Primary,
                true,
                leaf(260.0, 34.0),
            ).unwrap();

            tree.container(
                &[env_lbl, env_dropdown, host_lbl, host_input, creds_row, save_btn],
                Style {
                    flex_direction: FlexDirection::Column,
                    gap: Size { width: length(0.0), height: length(12.0) },
                    margin: rect_margin_t(8.0),
                    ..Default::default()
                },
            ).unwrap()
        }
        _ => {
            // Tab 2: Security Policies
            let pol_lbl = tree.label_muted("Cluster Security Enforcement Mode:", leaf(516.0, 18.0)).unwrap();
            let r1 = tree.radio(WidgetId::new("strict"), "sec_policy", "Strict (Zero-Trust)", state.security_policy == "strict", leaf(164.0, 22.0)).unwrap();
            let r2 = tree.radio(WidgetId::new("adaptive"), "sec_policy", "Adaptive (AI Shield)", state.security_policy == "adaptive", leaf(164.0, 22.0)).unwrap();
            let r3 = tree.radio(WidgetId::new("audit"), "sec_policy", "Audit Logging Only", state.security_policy == "audit", leaf(164.0, 22.0)).unwrap();
            let radio_grid = tree.grid(3, 8.0, 0.0, &[r1, r2, r3], leaf(516.0, 24.0)).unwrap();

            let divider = tree.divider(false, leaf(516.0, 1.0)).unwrap();

            let lock_btn = tree.icon_button("lock_btn", ui_widgets::IconKind::Lock, true, leaf(36.0, 36.0)).unwrap();
            let shield_btn = tree.icon_button("shield_btn", ui_widgets::IconKind::Eye, true, leaf(36.0, 36.0)).unwrap();
            let term_btn = tree.icon_button("term_btn", ui_widgets::IconKind::Terminal, true, leaf(36.0, 36.0)).unwrap();
            let icons_row = tree.container(&[lock_btn, shield_btn, term_btn], row(10.0)).unwrap();

            let sec_desc = tree.label_muted("All ingress and egress traffic is inspected by the neural filter.", leaf(516.0, 20.0)).unwrap();

            tree.container(
                &[pol_lbl, radio_grid, divider, icons_row, sec_desc],
                Style {
                    flex_direction: FlexDirection::Column,
                    gap: Size { width: length(0.0), height: length(12.0) },
                    margin: rect_margin_t(8.0),
                    ..Default::default()
                },
            ).unwrap()
        }
    };

    let right_panel = tree.card(
        &[tabbar, tab_content],
        None,
        None,
        Some(6.0),
        Style {
            size: Size { width: length(540.0), height: length(550.0) },
            flex_direction: FlexDirection::Column,
            padding: Rect { left: length(14.0), right: length(14.0), top: length(10.0), bottom: length(10.0) },
            ..Default::default()
        },
    ).unwrap();

    let split_workstation = tree.container(
        &[left_panel, right_panel],
        Style {
            flex_direction: FlexDirection::Row,
            gap: Size { width: length(16.0), height: length(0.0) },
            ..Default::default()
        },
    ).unwrap();

    // Outer Window Wrapper
    tree.container(
        &[top_header, command_card, split_workstation],
        Style {
            flex_direction: FlexDirection::Column,
            size: Size { width: length(WINDOW_WIDTH), height: length(WINDOW_HEIGHT) },
            padding: Rect {
                left: length(24.0),
                right: length(24.0),
                top: length(18.0),
                bottom: length(18.0),
            },
            ..Default::default()
        },
    ).unwrap()
}

fn build_modal_ui(tree: &mut WidgetTree, title: &str, content: &str) -> NodeId {
    let title_lbl = tree.label(title, leaf(440.0, 26.0)).unwrap();
    let content_lbl = tree.label(content, leaf(440.0, 60.0)).unwrap();
    let dismiss_btn = tree.button_variant(
        WidgetId::new("modal_dismiss_btn"),
        "COMPRIS / DISMISS",
        ui_widgets::ButtonVariant::Primary,
        true,
        leaf(180.0, 34.0),
    ).unwrap();

    let dialog_style = Style {
        position: Position::Absolute,
        inset: Rect {
            left: length((WINDOW_WIDTH - 480.0) * 0.5),
            top: length((WINDOW_HEIGHT - 220.0) * 0.5),
            right: auto(),
            bottom: auto(),
        },
        size: Size { width: length(480.0), height: length(220.0) },
        flex_direction: FlexDirection::Column,
        align_items: Some(AlignItems::Center),
        justify_content: Some(ui_layout::JustifyContent::SpaceBetween),
        padding: Rect { left: length(20.0), right: length(20.0), top: length(18.0), bottom: length(18.0) },
        ..Default::default()
    };

    let backdrop_style = Style {
        position: Position::Absolute,
        inset: Rect { left: length(0.0), top: length(0.0), right: length(0.0), bottom: length(0.0) },
        size: Size { width: length(WINDOW_WIDTH), height: length(WINDOW_HEIGHT) },
        ..Default::default()
    };

    tree.modal(
        WidgetId::new("modal_dialog"),
        title,
        &[title_lbl, content_lbl, dismiss_btn],
        dialog_style,
        backdrop_style,
    ).unwrap()
}

// ============================================================================
// 5. Application Principale Winit / Wgpu
// ============================================================================

struct AgentControlCenterApp {
    window: Option<Arc<Window>>,
    renderer: Option<GpuRenderer>,
    resources: ui_gpu::ResourceTable,
    theme: Arc<RwLock<Theme>>,
    _theme_watcher: Option<ThemeWatcher>,

    state: WorkstationState,
    tx_events: mpsc::Sender<UiEvent>,
    rx_patches: crossbeam_channel::Receiver<UiPatch>,

    cursor_pos: (f32, f32),
    pressed: Option<InteractionKey>,
    root: Option<NodeId>,
    overlay_root: Option<NodeId>,
}

impl AgentControlCenterApp {
    pub fn new(tx_events: mpsc::Sender<UiEvent>, rx_patches: crossbeam_channel::Receiver<UiPatch>) -> Self {
        let theme_path = "themes/enterprise_dark.toml";
        let initial_theme = Theme::from_file(theme_path).unwrap_or_else(|_| Theme::studio_pro());
        let theme = Arc::new(RwLock::new(initial_theme));

        Self {
            window: None,
            renderer: None,
            resources: ui_gpu::ResourceTable::new(),
            theme,
            _theme_watcher: None,
            state: WorkstationState::default(),
            tx_events,
            rx_patches,
            cursor_pos: (0.0, 0.0),
            pressed: None,
            root: None,
            overlay_root: None,
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

    fn redraw(&mut self) {
        // 1. Applique tous les patches reçus de l'Agent
        while let Ok(patch) = self.rx_patches.try_recv() {
            self.state.apply_patch(patch);
        }

        let Some(renderer) = self.renderer.as_mut() else { return };

        let (width_f, height_f) = (WINDOW_WIDTH, WINDOW_HEIGHT);
        let available = Size {
            width: AvailableSpace::Definite(width_f),
            height: AvailableSpace::Definite(height_f),
        };

        let current_theme = self.theme.read().unwrap().clone();
        let measure = renderer.text_measure();

        // Layer 0: Base Workstation
        let mut base_tree = WidgetTree::new();
        let base_root = build_workstation_ui(&mut base_tree, &self.state);
        let _ = base_tree.compute(base_root, available);

        let base_hovered = if self.state.active_modal.is_none() {
            base_tree.interaction_key_at(base_root, self.cursor_pos).unwrap_or(None)
        } else {
            None
        };

        let base_interaction = InteractionState {
            hovered: base_hovered.as_ref(),
            pressed: if self.state.active_modal.is_none() { self.pressed.as_ref() } else { None },
            measure: Some(&measure),
        };

        let base_frame = match base_tree.build_frame(base_root, &current_theme, base_interaction) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("Error build_frame: {:?}", e);
                return;
            }
        };

        let family = Self::font_family(&current_theme.typography.family);
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

        let background = wgpu::Color::BLACK;

        if let Some((title, content)) = &self.state.active_modal {
            // Layer 1: Modal Dialog
            let mut modal_tree = WidgetTree::new();
            let modal_root = build_modal_ui(&mut modal_tree, title, content);
            let _ = modal_tree.compute(modal_root, available);

            let modal_hovered = modal_tree.interaction_key_at(modal_root, self.cursor_pos).unwrap_or(None);
            let modal_interaction = InteractionState {
                hovered: modal_hovered.as_ref(),
                pressed: self.pressed.as_ref(),
                measure: Some(&measure),
            };

            if let Ok(modal_frame) = modal_tree.build_frame(modal_root, &current_theme, modal_interaction) {
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

                let _ = renderer.render_layers(background, &[base_layer, modal_layer], &[], &self.resources);
            }
            self.overlay_root = Some(modal_root);
        } else {
            let _ = renderer.render_layers(background, &[base_layer], &[], &self.resources);
            self.overlay_root = None;
        }

        self.root = Some(base_root);
    }

    fn submit_agent_prompt(&mut self, prompt: String) {
        println!("🤖 [AgentControlCenter] Transmission de l'instruction à l'Agent : '{}'", prompt);
        let _ = self.tx_events.try_send(UiEvent::UserPromptSubmitted(prompt));
    }
}

impl ApplicationHandler for AgentControlCenterApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attrs = WindowAttributes::default()
            .with_title("AORUI — Autonomous Agent Enterprise Workstation")
            .with_inner_size(winit::dpi::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
            .with_resizable(false);

        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .expect("Failed to create agent control center window"),
        );

        let renderer = GpuRenderer::new(window.clone());
        self.window = Some(window.clone());
        self.renderer = Some(renderer);

        let window_clone = window.clone();
        let watcher = ThemeWatcher::watch_file(
            "themes/enterprise_dark.toml",
            self.theme.clone(),
            move |_| {
                window_clone.request_redraw();
            },
        ).ok();
        self._theme_watcher = watcher;

        event_loop.set_control_flow(ControlFlow::Poll);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(r) = self.renderer.as_mut() {
                    r.resize(size);
                }
                self.redraw();
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_pos = (position.x as f32, position.y as f32);
                self.redraw();
            }
            WindowEvent::KeyboardInput {
                event: KeyEvent { state: ElementState::Pressed, logical_key, .. },
                ..
            } => {
                match logical_key {
                    Key::Named(NamedKey::Enter) => {
                        let prompt = self.state.prompt_editor.text.clone();
                        if !prompt.trim().is_empty() {
                            self.submit_agent_prompt(prompt);
                        }
                    }
                    Key::Named(NamedKey::Backspace) => {
                        if self.state.focused_input.as_deref() == Some("prompt_input") {
                            self.state.prompt_editor.backspace();
                        } else if self.state.focused_input.as_deref() == Some("input_host") {
                            self.state.host_editor.backspace();
                        }
                        self.redraw();
                    }
                    Key::Named(NamedKey::Space) => {
                        if self.state.focused_input.as_deref() == Some("prompt_input") {
                            self.state.prompt_editor.insert_char(' ');
                        } else if self.state.focused_input.as_deref() == Some("input_host") {
                            self.state.host_editor.insert_char(' ');
                        }
                        self.redraw();
                    }
                    Key::Character(s) => {
                        for c in s.chars() {
                            if self.state.focused_input.as_deref() == Some("prompt_input") {
                                self.state.prompt_editor.insert_char(c);
                            } else if self.state.focused_input.as_deref() == Some("input_host") {
                                self.state.host_editor.insert_char(c);
                            }
                        }
                        self.redraw();
                    }
                    _ => {}
                }
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                if self.state.active_modal.is_some() {
                    let mut modal_tree = WidgetTree::new();
                    let (title, content) = self.state.active_modal.clone().unwrap();
                    let m_root = build_modal_ui(&mut modal_tree, &title, &content);
                    let _ = modal_tree.compute(m_root, Size { width: AvailableSpace::Definite(WINDOW_WIDTH), height: AvailableSpace::Definite(WINDOW_HEIGHT) });
                    if let Ok(Some(hit)) = modal_tree.interaction_key_at(m_root, self.cursor_pos) {
                        if hit.widget_id.as_str() == "modal_dismiss_btn" || hit.widget_id.as_str() == "modal_dialog" {
                            self.state.active_modal = None;
                        }
                    }
                    self.redraw();
                    return;
                }

                let mut tree = WidgetTree::new();
                let root = build_workstation_ui(&mut tree, &self.state);
                let _ = tree.compute(root, Size { width: AvailableSpace::Definite(WINDOW_WIDTH), height: AvailableSpace::Definite(WINDOW_HEIGHT) });

                if let Ok(Some(hit_key)) = tree.interaction_key_at(root, self.cursor_pos) {
                    let hit_str = hit_key.widget_id.as_str();
                    println!("🖱️ Clic sur widget : {}", hit_str);

                    if hit_str == "cluster_tabs" {
                        if let Some(idx) = hit_key.index {
                            self.state.active_tab = idx;
                        }
                    } else if hit_str == "btn_send_prompt" {
                        let prompt = self.state.prompt_editor.text.clone();
                        self.submit_agent_prompt(prompt);
                    } else if hit_str == "btn_sc_prod" {
                        self.state.prompt_editor = TextEditorState::new("Configurer le cluster de production et valider la réplication");
                        self.submit_agent_prompt(self.state.prompt_editor.text.clone());
                    } else if hit_str == "btn_sc_sec" {
                        self.state.prompt_editor = TextEditorState::new("Lancer un audit de sécurité complet et vérifier le pare-feu");
                        self.submit_agent_prompt(self.state.prompt_editor.text.clone());
                    } else if hit_str == "btn_sc_opt" {
                        self.state.prompt_editor = TextEditorState::new("Optimiser l'utilisation de la mémoire RAM et purger les caches");
                        self.submit_agent_prompt(self.state.prompt_editor.text.clone());
                    } else if hit_str == "chk_ssl" {
                        self.state.chk_ssl = !self.state.chk_ssl;
                    } else if hit_str == "chk_firewall" {
                        self.state.chk_firewall = !self.state.chk_firewall;
                    } else if hit_str == "chk_backup" {
                        self.state.chk_backup = !self.state.chk_backup;
                    } else if hit_str == "chk_turbo" {
                        self.state.chk_turbo = !self.state.chk_turbo;
                    } else if hit_str == "prompt_input" {
                        self.state.focused_input = Some("prompt_input".to_string());
                    } else if hit_str == "input_host" {
                        self.state.focused_input = Some("input_host".to_string());
                    } else if hit_str == "btn_save_config" {
                        self.state.active_modal = Some((
                            "💾 Cluster Configuration Saved".to_string(),
                            format!("Host: {} | Port: {} | TLS 1.3: Active", self.state.host_editor.text, self.state.port_spin as u32)
                        ));
                    } else if hit_str == "strict" {
                        self.state.security_policy = "strict".to_string();
                    } else if hit_str == "adaptive" {
                        self.state.security_policy = "adaptive".to_string();
                    } else if hit_str == "audit" {
                        self.state.security_policy = "audit".to_string();
                    } else if hit_str == "master_token_pwd" {
                        self.state.password_revealed = !self.state.password_revealed;
                    }

                    self.redraw();
                }
            }
            WindowEvent::RedrawRequested => self.redraw(),
            _ => {}
        }
    }
}

// ============================================================================
// 6. Démarrage de l'Application
// ============================================================================

fn main() {
    println!("============================================================");
    println!("⚡ AORUI ENTERPRISE AUTONOMOUS AGENT WORKSTATION");
    println!("🚀 Dual-Kawase Blur + Instanced GPU SDF + Tokio Agent Runtime");
    println!("============================================================");

    let (tx_events, rx_events) = mpsc::channel::<UiEvent>(64);
    let (tx_patches, rx_patches) = crossbeam_channel::unbounded::<UiPatch>();

    let mut tools = ToolRegistry::new();
    tools.register(Arc::new(SetFieldTool { tx_patches: tx_patches.clone() }));
    tools.register(Arc::new(SwitchTabTool { tx_patches: tx_patches.clone() }));
    tools.register(Arc::new(ToggleServiceTool { tx_patches: tx_patches.clone() }));
    tools.register(Arc::new(UpdateMetricTool { tx_patches: tx_patches.clone() }));
    tools.register(Arc::new(NotifyModalTool { tx_patches: tx_patches.clone() }));

    let agent_config = AgentConfig {
        max_steps: 16,
        tool_timeout: Duration::from_secs(10),
        max_scratchpad_entries: 50,
    };
    let agent = Agent::new(agent_config, tools, tx_patches);

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Échec de création du runtime Tokio");

        rt.block_on(async move {
            let planner = Box::new(DynamicAgentPlanner::new());
            println!("🤖 [AgentRuntime] Agent initialisé en attente d'instructions...");
            if let Err(e) = agent.run(rx_events, planner).await {
                eprintln!("⚠️ [AgentRuntime] Erreur d'exécution de l'agent : {:?}", e);
            }
        });
    });

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = AgentControlCenterApp::new(tx_events, rx_patches);
    event_loop.run_app(&mut app).unwrap();
}
