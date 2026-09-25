// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0 | Action: AetherOS Deep Atmospheric Glassmorphism Flagship Workstation matching screen.png & code.html
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use serde_json::{json, Value};
use tokio::sync::mpsc;
use ui_core::{StepStatus, UiEvent, UiPatch};
use ui_gpu::GpuRenderer;
use ui_layout::{
    auto, length, AlignItems, AvailableSpace, FlexDirection,
    LengthPercentage, NodeId, Rect, Size, Style,
};
use ui_widgets::{
    IconKind, InteractionKey, InteractionState, ListItemBadge, Painter, Theme,
    ThemeWatcher, WidgetId, WidgetTree,
};

use agent_runtime::{
    Agent, AgentConfig, BoxFuture, PlanDecision, Planner, Tool, ToolError, ToolRegistry,
};

use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

const WINDOW_WIDTH: f32 = 1380.0;
const WINDOW_HEIGHT: f32 = 880.0;

// ============================================================================
// Color & Layout Helpers
// ============================================================================

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn hex_linear(hex: &str) -> [f32; 4] {
    let clean = hex.trim().trim_start_matches('#');
    let srgb_to_lin = |v: u8| -> f32 { srgb_to_linear(v as f32 / 255.0) };
    match clean.len() {
        6 => {
            let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(0);
            [srgb_to_lin(r), srgb_to_lin(g), srgb_to_lin(b), 1.0]
        }
        8 => {
            let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(0);
            let a = u8::from_str_radix(&clean[6..8], 16).unwrap_or(255);
            [srgb_to_lin(r), srgb_to_lin(g), srgb_to_lin(b), a as f32 / 255.0]
        }
        _ => [0.0, 0.0, 0.0, 1.0],
    }
}

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

fn rect_pad(l: f32, r: f32, t: f32, b: f32) -> Rect<LengthPercentage> {
    Rect {
        left: LengthPercentage::Length(l),
        right: LengthPercentage::Length(r),
        top: LengthPercentage::Length(t),
        bottom: LengthPercentage::Length(b),
    }
}

// ============================================================================
// 1. Outils Déterministes Exécutables par l'Agent Tokio
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
            "properties": {
                "widget_id": { "type": "string" },
                "text": { "type": "string" }
            },
            "required": ["widget_id", "text"]
        })
    }

    fn execute<'a>(&'a self, args: Value) -> BoxFuture<'a, Result<Value, ToolError>> {
        Box::pin(async move {
            let widget_id = args.get("widget_id").and_then(|v| v.as_str()).ok_or_else(|| {
                ToolError("Paramètre 'widget_id' manquant".to_string())
            })?;
            let text = args.get("text").and_then(|v| v.as_str()).ok_or_else(|| {
                ToolError("Paramètre 'text' manquant".to_string())
            })?;

            let _ = self.tx_patches.send(UiPatch::WidgetTextSet {
                widget_id: widget_id.to_string(),
                text: text.to_string(),
            });

            Ok(json!({
                "status": "applied",
                "widget_id": widget_id,
                "text": text
            }))
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
            "properties": {
                "tab_index": { "type": "integer", "minimum": 0, "maximum": 5 }
            },
            "required": ["tab_index"]
        })
    }

    fn execute<'a>(&'a self, args: Value) -> BoxFuture<'a, Result<Value, ToolError>> {
        Box::pin(async move {
            let tab_index = args.get("tab_index").and_then(|v| v.as_u64()).ok_or_else(|| {
                ToolError("Paramètre 'tab_index' manquant".to_string())
            })? as usize;

            let _ = self.tx_patches.send(UiPatch::TabActivated {
                widget_id: "main_tabs".to_string(),
                tab_index,
            });

            Ok(json!({
                "status": "tab_switched",
                "index": tab_index
            }))
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
            "properties": {
                "key": { "type": "string" },
                "value": { "type": "number" }
            },
            "required": ["key", "value"]
        })
    }

    fn execute<'a>(&'a self, args: Value) -> BoxFuture<'a, Result<Value, ToolError>> {
        Box::pin(async move {
            let key = args.get("key").and_then(|v| v.as_str()).unwrap_or("cpu");
            let value = args.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;

            let _ = self.tx_patches.send(UiPatch::MetricUpdated {
                key: key.to_string(),
                value,
            });

            Ok(json!({
                "status": "metric_updated",
                "key": key,
                "value": value
            }))
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
            "properties": {
                "title": { "type": "string" },
                "message": { "type": "string" }
            },
            "required": ["title", "message"]
        })
    }

    fn execute<'a>(&'a self, args: Value) -> BoxFuture<'a, Result<Value, ToolError>> {
        Box::pin(async move {
            let title = args.get("title").and_then(|v| v.as_str()).unwrap_or("Alerte");
            let message = args.get("message").and_then(|v| v.as_str()).unwrap_or("");

            let _ = self.tx_patches.send(UiPatch::ModalRequested {
                title: title.to_string(),
                content: message.to_string(),
            });

            Ok(json!({ "status": "modal_shown" }))
        })
    }
}

struct PurgeCachesTool {
    tx_patches: crossbeam_channel::Sender<UiPatch>,
}

impl Tool for PurgeCachesTool {
    fn name(&self) -> &str {
        "purge_caches"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "cache_type": { "type": "string" }
            }
        })
    }

    fn execute<'a>(&'a self, _args: Value) -> BoxFuture<'a, Result<Value, ToolError>> {
        Box::pin(async move {
            let _ = self.tx_patches.send(UiPatch::MetricUpdated {
                key: "memory_used".to_string(),
                value: 14.6,
            });
            let _ = self.tx_patches.send(UiPatch::MetricUpdated {
                key: "cpu_load".to_string(),
                value: 18.2,
            });
            let _ = self.tx_patches.send(UiPatch::MetricUpdated {
                key: "latency_ms".to_string(),
                value: 6.2,
            });
            let _ = self.tx_patches.send(UiPatch::WidgetTextSet {
                widget_id: "diff_preview".to_string(),
                text: "Δ RAM: 18.2G → 14.6G (-3.6 GB) • CPU: 18.2% • Latence: 6.2ms".to_string(),
            });
            let _ = self.tx_patches.send(UiPatch::StepLogged {
                step_id: "purge_step".to_string(),
                tool: "purge_caches".to_string(),
                status: StepStatus::Success,
            });

            Ok(json!({
                "status": "caches_purged",
                "freed_memory_gb": 3.6,
                "latency_improvement_ms": 1.8
            }))
        })
    }
}

struct RestartProcessTool {
    tx_patches: crossbeam_channel::Sender<UiPatch>,
}

impl Tool for RestartProcessTool {
    fn name(&self) -> &str {
        "restart_process"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "process_name": { "type": "string" }
            },
            "required": ["process_name"]
        })
    }

    fn execute<'a>(&'a self, args: Value) -> BoxFuture<'a, Result<Value, ToolError>> {
        Box::pin(async move {
            let process_name = args.get("process_name").and_then(|v| v.as_str()).unwrap_or("ai-inference-engine");

            let pid = match process_name {
                "stratus-kernel" => 1.0,
                "ai-inference-engine" => 4102.0,
                "postgres-primary" => 2889.0,
                _ => 1904.0,
            };

            let _ = self.tx_patches.send(UiPatch::MetricUpdated {
                key: "cpu_load".to_string(),
                value: 21.4,
            });
            let _ = self.tx_patches.send(UiPatch::MetricUpdated {
                key: "restart_proc".to_string(),
                value: pid,
            });
            let _ = self.tx_patches.send(UiPatch::WidgetTextSet {
                widget_id: "diff_preview".to_string(),
                text: format!("Δ MicroVM '{}' redémarrée (PID {}) • CPU stabilisé à 2.4%", process_name, pid as u32),
            });
            let _ = self.tx_patches.send(UiPatch::StepLogged {
                step_id: "restart_step".to_string(),
                tool: "restart_process".to_string(),
                status: StepStatus::Success,
            });

            Ok(json!({
                "status": "restarted",
                "process": process_name,
                "pid": pid as u32
            }))
        })
    }
}

struct KillProcessTool {
    tx_patches: crossbeam_channel::Sender<UiPatch>,
}

impl Tool for KillProcessTool {
    fn name(&self) -> &str {
        "kill_process"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "pid": { "type": "integer" }
            },
            "required": ["pid"]
        })
    }

    fn execute<'a>(&'a self, args: Value) -> BoxFuture<'a, Result<Value, ToolError>> {
        Box::pin(async move {
            let pid = args.get("pid").and_then(|v| v.as_f64()).unwrap_or(1904.0) as f32;

            let _ = self.tx_patches.send(UiPatch::MetricUpdated {
                key: "kill_proc".to_string(),
                value: pid,
            });
            let _ = self.tx_patches.send(UiPatch::WidgetTextSet {
                widget_id: "diff_preview".to_string(),
                text: format!("Δ Processus PID {} arrêté (Terminated) • Ressources libérées", pid as u32),
            });
            let _ = self.tx_patches.send(UiPatch::StepLogged {
                step_id: "kill_step".to_string(),
                tool: "kill_process".to_string(),
                status: StepStatus::Success,
            });

            Ok(json!({ "status": "terminated", "pid": pid as u32 }))
        })
    }
}

struct FilterTableTool {
    tx_patches: crossbeam_channel::Sender<UiPatch>,
}

impl Tool for FilterTableTool {
    fn name(&self) -> &str {
        "filter_table"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" }
            },
            "required": ["query"]
        })
    }

    fn execute<'a>(&'a self, args: Value) -> BoxFuture<'a, Result<Value, ToolError>> {
        Box::pin(async move {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");

            let _ = self.tx_patches.send(UiPatch::WidgetTextSet {
                widget_id: "proc_search".to_string(),
                text: query.to_string(),
            });
            let _ = self.tx_patches.send(UiPatch::WidgetTextSet {
                widget_id: "diff_preview".to_string(),
                text: format!("Δ Filtre table conteneurs appliqué : '{}'", query),
            });
            let _ = self.tx_patches.send(UiPatch::StepLogged {
                step_id: "filter_step".to_string(),
                tool: "filter_table".to_string(),
                status: StepStatus::Success,
            });

            Ok(json!({ "status": "filtered", "query": query }))
        })
    }
}

struct SwitchTimeRangeTool {
    tx_patches: crossbeam_channel::Sender<UiPatch>,
}

impl Tool for SwitchTimeRangeTool {
    fn name(&self) -> &str {
        "switch_time_range"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "range_index": { "type": "integer", "minimum": 0, "maximum": 3 }
            },
            "required": ["range_index"]
        })
    }

    fn execute<'a>(&'a self, args: Value) -> BoxFuture<'a, Result<Value, ToolError>> {
        Box::pin(async move {
            let range_index = args.get("range_index").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32;

            let _ = self.tx_patches.send(UiPatch::MetricUpdated {
                key: "range".to_string(),
                value: range_index,
            });
            let _ = self.tx_patches.send(UiPatch::WidgetTextSet {
                widget_id: "diff_preview".to_string(),
                text: format!("Δ Horizon télémétrique : index {} activé", range_index as usize),
            });
            let _ = self.tx_patches.send(UiPatch::StepLogged {
                step_id: "range_step".to_string(),
                tool: "switch_time_range".to_string(),
                status: StepStatus::Success,
            });

            Ok(json!({ "status": "range_switched", "index": range_index as usize }))
        })
    }
}

struct AdjustThresholdsTool {
    tx_patches: crossbeam_channel::Sender<UiPatch>,
}

impl Tool for AdjustThresholdsTool {
    fn name(&self) -> &str {
        "adjust_thresholds"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "cpu_max": { "type": "number" },
                "latency_max": { "type": "number" }
            }
        })
    }

    fn execute<'a>(&'a self, args: Value) -> BoxFuture<'a, Result<Value, ToolError>> {
        Box::pin(async move {
            let cpu_max = args.get("cpu_max").and_then(|v| v.as_f64()).unwrap_or(85.0) as f32;
            let latency_max = args.get("latency_max").and_then(|v| v.as_f64()).unwrap_or(15.0) as f32;

            let _ = self.tx_patches.send(UiPatch::MetricUpdated {
                key: "threshold_cpu".to_string(),
                value: cpu_max,
            });
            let _ = self.tx_patches.send(UiPatch::MetricUpdated {
                key: "latency_ms".to_string(),
                value: 6.8,
            });
            let _ = self.tx_patches.send(UiPatch::WidgetTextSet {
                widget_id: "diff_preview".to_string(),
                text: format!("Δ Seuils d'alerte : CPU max {:.0}%, Latence max {:.0}ms", cpu_max, latency_max),
            });
            let _ = self.tx_patches.send(UiPatch::StepLogged {
                step_id: "thresh_step".to_string(),
                tool: "adjust_thresholds".to_string(),
                status: StepStatus::Success,
            });

            Ok(json!({
                "status": "thresholds_adjusted",
                "cpu_max": cpu_max,
                "latency_max": latency_max
            }))
        })
    }
}

// ============================================================================
// 2. Planificateur d'Agent Dynamique
// ============================================================================

struct DynamicAgentPlanner {
    pending_plan: VecDeque<PlanDecision>,
}

impl DynamicAgentPlanner {
    fn new() -> Self {
        Self {
            pending_plan: VecDeque::new(),
        }
    }

    fn compile_plan_for_prompt(&mut self, prompt: &str) {
        let p = prompt.to_lowercase();
        self.pending_plan.clear();

        if p.contains("purge") || p.contains("cache") || p.contains("actualis") {
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "purge_caches".to_string(),
                args: json!({ "cache_type": "all" }),
            });
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "update_metric".to_string(),
                args: json!({ "key": "memory_used", "value": 14.6 }),
            });
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "update_metric".to_string(),
                args: json!({ "key": "cpu_load", "value": 19.8 }),
            });
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "notify_modal".to_string(),
                args: json!({
                    "title": "⚡ Caches Purgés & Télémétrie Optimisée",
                    "message": "Actions automatisées exécutées avec succès :\n• 3.6 GB de RAM libérés sur les caches mémoire Redis\n• Caches réseau vSwitch réinitialisés (pertes: 0.00%)\n• Charge globale CPU stabilisée à 19.8%"
                }),
            });
        } else if p.contains("restart") || p.contains("redémarr") || p.contains("reboot") || p.contains("proc") {
            let target = if p.contains("inference") || p.contains("ai") || p.contains("cuda") {
                "ai-inference-engine"
            } else if p.contains("postgres") || p.contains("db") {
                "postgres-primary"
            } else if p.contains("envoy") || p.contains("proxy") || p.contains("edge") {
                "edge-gateway-proxy"
            } else {
                "ai-inference-engine"
            };
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "restart_process".to_string(),
                args: json!({ "process_name": target }),
            });
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "notify_modal".to_string(),
                args: json!({
                    "title": "🔄 MicroVM Relancée par l'Agent",
                    "message": format!("Le conteneur '{}' a été redémarré proprement.\nLes métriques de latence et de calcul sont revenues au niveau nominal.", target)
                }),
            });
        } else if p.contains("filtr") || p.contains("recherch") || p.contains("search") {
            let query = if p.contains("cuda") || p.contains("ai") {
                "ai"
            } else if p.contains("postgres") || p.contains("db") {
                "postgres"
            } else if p.contains("envoy") || p.contains("proxy") {
                "envoy"
            } else {
                "kernel"
            };
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "filter_table".to_string(),
                args: json!({ "query": query }),
            });
        } else if p.contains("seuil") || p.contains("alert") || p.contains("threshold") {
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "adjust_thresholds".to_string(),
                args: json!({ "cpu_max": 80.0, "latency_max": 15.0 }),
            });
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "switch_time_range".to_string(),
                args: json!({ "range_index": 1 }),
            });
        } else if p.contains("snapshot") || p.contains("rapport") || p.contains("export") {
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "update_metric".to_string(),
                args: json!({ "key": "db_sync", "value": 100.0 }),
            });
        } else if p.contains("fenêtre") || p.contains("range") || p.contains("24h") || p.contains("15m") || p.contains("1h") || p.contains("7j") {
            let idx = if p.contains("7j") || p.contains("24h") || p.contains("3") {
                3
            } else if p.contains("1h") || p.contains("2") {
                2
            } else if p.contains("15m") || p.contains("1") {
                1
            } else {
                0
            };
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "switch_time_range".to_string(),
                args: json!({ "range_index": idx }),
            });
        } else if p.contains("dock") {
            let idx = if p.contains("0") || p.contains("dash") {
                0
            } else if p.contains("1") || p.contains("file") {
                1
            } else if p.contains("2") || p.contains("stat") {
                2
            } else if p.contains("3") || p.contains("net") {
                3
            } else {
                4
            };
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "switch_tab".to_string(),
                args: json!({ "tab_index": idx }),
            });
        } else {
            self.pending_plan.push_back(PlanDecision::CallTool {
                tool: "update_metric".to_string(),
                args: json!({ "key": "cpu_load", "value": 24.0 }),
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
                } else {
                    self.compile_plan_for_prompt("default");
                }
            }
            self.pending_plan.pop_front().unwrap_or(PlanDecision::Complete)
        })
    }
}

// ============================================================================
// 3. Modèle d'État de l'Application AetherOS
// ============================================================================

#[derive(Debug, Clone)]
struct ProcessItem {
    name: String,
    pid: u32,
    user: String,
    cpu_percent: f32,
    memory_mb: u32,
    network_str: String,
    badge: ListItemBadge,
    status: String,
}

struct TextEditorState {
    text: String,
    cursor: usize,
    selection: Option<(usize, usize)>,
}

impl TextEditorState {
    fn new(initial: &str) -> Self {
        Self {
            text: initial.to_string(),
            cursor: initial.chars().count(),
            selection: None,
        }
    }

    fn insert_char(&mut self, c: char) {
        let mut chars: Vec<char> = self.text.chars().collect();
        let idx = self.cursor.min(chars.len());
        chars.insert(idx, c);
        self.cursor = idx + 1;
        self.text = chars.into_iter().collect();
    }

    fn backspace(&mut self) {
        if self.cursor > 0 {
            let mut chars: Vec<char> = self.text.chars().collect();
            let idx = self.cursor - 1;
            if idx < chars.len() {
                chars.remove(idx);
                self.cursor = idx;
                self.text = chars.into_iter().collect();
            }
        }
    }
}

struct AetherWorkstationState {
    active_tab: usize,
    active_range: usize,
    prompt_editor: TextEditorState,
    search_editor: TextEditorState,

    agent_status: String,
    step_logs: Vec<(String, String, StepStatus)>,
    scratchpad: Vec<String>,
    agent_target: Option<String>,
    diff_preview: Option<String>,
    require_human_approval: bool,
    pending_approval_prompt: Option<String>,

    cpu_load: f32,
    memory_used: f32,
    network_speed: f32,
    latency_ms: f32,
    db_sync: f32,

    processes: Vec<ProcessItem>,

    focused_input: Option<String>,
    active_modal: Option<(String, String)>,
    active_dock_app: usize,
}

impl Default for AetherWorkstationState {
    fn default() -> Self {
        let processes = vec![
            ProcessItem {
                name: "● stratus-kernel  •  Core Host • Hypervisor".to_string(),
                pid: 1,
                user: "root".to_string(),
                cpu_percent: 3.8,
                memory_mb: 420,
                network_str: "18.4 KB/s".to_string(),
                badge: ListItemBadge::Success,
                status: "Nominal".to_string(),
            },
            ProcessItem {
                name: "● ai-inference-engine  •  CUDA/TensorRT".to_string(),
                pid: 4102,
                user: "ai-runtime".to_string(),
                cpu_percent: 18.4,
                memory_mb: 12400,
                network_str: "1.2 Gbps (Active)".to_string(),
                badge: ListItemBadge::Warning,
                status: "High Load".to_string(),
            },
            ProcessItem {
                name: "● postgres-primary  •  Cluster Relational DB".to_string(),
                pid: 2889,
                user: "postgres".to_string(),
                cpu_percent: 4.1,
                memory_mb: 3800,
                network_str: "420 Mbps".to_string(),
                badge: ListItemBadge::Success,
                status: "Nominal".to_string(),
            },
            ProcessItem {
                name: "● edge-gateway-proxy  •  Envoy Mesh".to_string(),
                pid: 1904,
                user: "envoy-user".to_string(),
                cpu_percent: 2.1,
                memory_mb: 1600,
                network_str: "1.8 Gbps (Active)".to_string(),
                badge: ListItemBadge::Warning,
                status: "Heavy Traffic".to_string(),
            },
        ];

        Self {
            active_tab: 0,
            active_range: 0,
            prompt_editor: TextEditorState::new("Purger les caches et stabiliser la RAM"),
            search_editor: TextEditorState::new(""),

            agent_status: "IDLE".to_string(),
            step_logs: Vec::new(),
            scratchpad: Vec::new(),
            agent_target: None,
            diff_preview: Some("Δ Statut : Station AetherOS prête • En attente d'instruction".to_string()),
            require_human_approval: false,
            pending_approval_prompt: None,

            cpu_load: 28.4,
            memory_used: 18.2,
            network_speed: 3.40,
            latency_ms: 8.0,
            db_sync: 99.98,

            processes,

            focused_input: Some("prompt_input".to_string()),
            active_modal: None,
            active_dock_app: 0,
        }
    }
}

impl AetherWorkstationState {
    fn apply_patch(&mut self, patch: UiPatch) {
        match patch {
            UiPatch::WidgetTextSet { widget_id, text } => match widget_id.as_str() {
                "prompt_input" => self.prompt_editor = TextEditorState::new(&text),
                "proc_search" => {
                    self.search_editor = TextEditorState::new(&text);
                    self.agent_target = Some("proc_search".to_string());
                }
                "diff_preview" => {
                    self.diff_preview = Some(text);
                }
                _ => {}
            },
            UiPatch::TabActivated { tab_index, .. } => {
                self.active_tab = tab_index;
                self.active_dock_app = tab_index;
            }
            UiPatch::MetricUpdated { key, value } => match key.as_str() {
                "cpu_load" | "cpu" => {
                    self.cpu_load = value;
                    self.agent_target = Some("k1_card".to_string());
                }
                "memory_used" | "ram" => {
                    self.memory_used = value;
                    self.agent_target = Some("k2_card".to_string());
                }
                "network_speed" | "network" => {
                    self.network_speed = value;
                    self.agent_target = Some("k3_card".to_string());
                }
                "latency_ms" | "latency" => {
                    self.latency_ms = value;
                }
                "db_sync" | "sync" => {
                    self.db_sync = value;
                }
                "range" => {
                    self.active_range = (value as usize).min(3);
                    self.agent_target = Some("range_seg".to_string());
                }
                "dock" => {
                    self.active_dock_app = (value as usize).min(5);
                }
                "restart_proc" => {
                    let target_pid = value as u32;
                    if let Some(p) = self.processes.iter_mut().find(|p| p.pid == target_pid) {
                        p.cpu_percent = 2.4;
                        p.badge = ListItemBadge::Success;
                        p.status = "Nominal (Restarted)".to_string();
                        p.network_str = "120 KB/s".to_string();
                    }
                    self.agent_target = Some("proc_table".to_string());
                }
                "kill_proc" => {
                    let target_pid = value as u32;
                    if let Some(p) = self.processes.iter_mut().find(|p| p.pid == target_pid) {
                        p.cpu_percent = 0.0;
                        p.badge = ListItemBadge::None;
                        p.status = "Terminated".to_string();
                        p.network_str = "0 KB/s".to_string();
                    }
                    self.agent_target = Some("proc_table".to_string());
                }
                _ => {}
            },
            UiPatch::ModalRequested { title, content } => {
                self.active_modal = Some((title, content));
            }
            UiPatch::StatusChanged { state, .. } => {
                self.agent_status = state;
            }
            UiPatch::StepLogged { step_id, tool, status } => {
                self.step_logs.push((step_id, tool, status));
            }
            UiPatch::ScratchpadAppended(entry) => {
                self.scratchpad.push(entry);
            }
            _ => {}
        }
    }
}

// ============================================================================
// 4. Construction Visuelle Pixel-Perfect de l'Interface AetherOS
// ============================================================================

fn build_aether_ui(tree: &mut WidgetTree, state: &AetherWorkstationState) -> NodeId {
    let mut root_children = Vec::new();

    // ------------------------------------------------------------------------
    // A. TOP GLOBAL BAR (36px)
    // ------------------------------------------------------------------------
    let logo_icon = tree.icon(IconKind::Network, 18.0, Some(hex_linear("#7bd0ff")), leaf(22.0, 22.0)).unwrap();
    let logo_lbl = tree.label("Stratus OS", leaf(80.0, 22.0)).unwrap();
    let logo_group = tree.container(&[logo_icon, logo_lbl], row(6.0)).unwrap();

    let nav_btn_0 = tree.button(WidgetId::new("nav_file"), "Fichier", true, leaf(54.0, 26.0)).unwrap();
    let nav_btn_1 = tree.button(WidgetId::new("nav_view"), "Affichage", true, leaf(68.0, 26.0)).unwrap();
    let nav_btn_2 = tree.button(WidgetId::new("nav_spaces"), "Espaces Cloud", true, leaf(96.0, 26.0)).unwrap();
    let nav_btn_3 = tree.button(WidgetId::new("nav_tools"), "Outils", true, leaf(54.0, 26.0)).unwrap();
    let nav_btn_4 = tree.button(WidgetId::new("nav_help"), "Aide", true, leaf(48.0, 26.0)).unwrap();
    let nav_group = tree.container(&[nav_btn_0, nav_btn_1, nav_btn_2, nav_btn_3, nav_btn_4], row(4.0)).unwrap();

    let top_left = tree.container(&[logo_group, nav_group], row(20.0)).unwrap();

    // Top Right Pill Capsule
    let mut pill_painter = Painter::new();
    pill_painter
        .rect([0.0, 0.0, 260.0, 26.0], 13.0, Some(hex_linear("#131a2aec")), Some((hex_linear("#2a385480"), 1.0)))
        .circle([12.0, 13.0], 3.0, Some(hex_linear("#4edea3")), None)
        .text([20.0, 6.0], "99.98%", 11.0, hex_linear("#8d99b2"))
        .text([76.0, 6.0], "12ms", 11.0, hex_linear("#7bd0ff"))
        .text([108.0, 6.0], "FRA", 11.0, hex_linear("#8d99b2"))
        .text([140.0, 6.0], "RAM", 11.0, hex_linear("#8d99b2"))
        .text([168.0, 6.0], "4.8/32G", 11.0, hex_linear("#dfe2f1"))
        .text([216.0, 6.0], "CPU", 11.0, hex_linear("#8d99b2"))
        .text([240.0, 6.0], "14%", 11.0, hex_linear("#c0c1ff"));

    let telemetry_pill = tree.custom_paint("telemetry_pill", pill_painter.finish(), leaf(260.0, 26.0)).unwrap();
    let btn_tune = tree.button(WidgetId::new("btn_tune"), "⚙", true, leaf(28.0, 26.0)).unwrap();
    let clock_lbl = tree.label("Ven 24 Oct 14:32", leaf(105.0, 22.0)).unwrap();

    let mut avatar_p = Painter::new();
    avatar_p.circle([14.0, 13.0], 12.0, Some(hex_linear("#1e293b")), Some((hex_linear("#7bd0ff"), 1.0)));
    let user_avatar = tree.custom_paint("user_avatar", avatar_p.finish(), leaf(28.0, 26.0)).unwrap();

    let top_right = tree.container(&[telemetry_pill, btn_tune, clock_lbl, user_avatar], row(10.0)).unwrap();

    let global_topbar = tree.panel(
        &[top_left, top_right],
        Some(hex_linear("#0a0e18e6")),
        Some(hex_linear("#1f2a3ef0")),
        Style {
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::Center),
            justify_content: Some(ui_layout::JustifyContent::SpaceBetween),
            size: Size { width: length(1340.0), height: length(36.0) },
            padding: rect_pad(12.0, 12.0, 4.0, 4.0),
            margin: rect_margin_b(6.0),
            ..Default::default()
        },
    ).unwrap();
    root_children.push(global_topbar);

    // ------------------------------------------------------------------------
    // B. INTERACTIVE AGENT COPILOT COMMAND HUD & PROMPT BAR (68px)
    // ------------------------------------------------------------------------
    let agent_status_badge = match state.agent_status.as_str() {
        "RUNNING" => tree.badge("⚡ AGENT : EXÉCUTION...", ListItemBadge::Warning, leaf(160.0, 26.0)).unwrap(),
        "PLANNING" => tree.badge("🧠 AGENT : PLANIFICATION", ListItemBadge::Warning, leaf(160.0, 26.0)).unwrap(),
        _ => tree.badge("🤖 COPILOT ACTIF", ListItemBadge::Success, leaf(125.0, 26.0)).unwrap(),
    };

    let prompt_focused = state.focused_input.as_deref() == Some("prompt_input");
    let prompt_box = tree.text_input_with_cursor(
        WidgetId::new("prompt_input"),
        &state.prompt_editor.text,
        "Demander une action : purger caches, relancer microVM, filtrer...",
        prompt_focused,
        state.prompt_editor.cursor,
        state.prompt_editor.selection,
        leaf(490.0, 28.0),
    ).unwrap();

    let btn_exec = tree.button_variant(
        WidgetId::new("btn_send_prompt"),
        "⚡ Exécuter",
        ui_widgets::ButtonVariant::Primary,
        true,
        leaf(95.0, 28.0),
    ).unwrap();

    let approval_label = if state.require_human_approval {
        "🛡️ Mode Sécurisé : ON"
    } else {
        "⚡ Mode Direct : ACTIF"
    };
    let btn_approval = tree.button(
        WidgetId::new("btn_toggle_approval"),
        approval_label,
        true,
        leaf(160.0, 28.0),
    ).unwrap();

    let diff_text = state.diff_preview.as_deref().unwrap_or("Δ Prêt • Aucun changement en attente");
    let diff_lbl = tree.label_muted(diff_text, leaf(370.0, 24.0)).unwrap();

    let copilot_row_1 = tree.container(
        &[agent_status_badge, prompt_box, btn_exec, btn_approval, diff_lbl],
        Style {
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::Center),
            gap: Size { width: length(8.0), height: length(0.0) },
            size: Size { width: length(1316.0), height: length(28.0) },
            ..Default::default()
        },
    ).unwrap();

    // Quick Action Prompt Pills
    let q1 = tree.button(WidgetId::new("quick_purge"), "⚡ Purger Caches & RAM", true, leaf(165.0, 22.0)).unwrap();
    let q2 = tree.button(WidgetId::new("quick_cuda"), "🔄 Relancer CUDA MicroVM", true, leaf(180.0, 22.0)).unwrap();
    let q3 = tree.button(WidgetId::new("quick_envoy"), "🔍 Filtrer Envoy Proxy", true, leaf(150.0, 22.0)).unwrap();
    let q4 = tree.button(WidgetId::new("quick_thresh"), "⚙️ Calibrer Seuils (80%)", true, leaf(165.0, 22.0)).unwrap();
    let q5 = tree.button(WidgetId::new("quick_snap"), "📥 Snapshot & Horizon 24h", true, leaf(180.0, 22.0)).unwrap();

    let copilot_row_2 = tree.container(
        &[q1, q2, q3, q4, q5],
        Style {
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::Center),
            gap: Size { width: length(8.0), height: length(0.0) },
            size: Size { width: length(1316.0), height: length(24.0) },
            ..Default::default()
        },
    ).unwrap();

    let copilot_hud = tree.panel(
        &[copilot_row_1, copilot_row_2],
        Some(hex_linear("#0e1524f2")),
        Some(hex_linear("#243550f0")),
        Style {
            flex_direction: FlexDirection::Column,
            justify_content: Some(ui_layout::JustifyContent::SpaceBetween),
            size: Size { width: length(1340.0), height: length(66.0) },
            padding: rect_pad(12.0, 12.0, 6.0, 6.0),
            margin: rect_margin_b(8.0),
            ..Default::default()
        },
    ).unwrap();
    root_children.push(copilot_hud);

    // ------------------------------------------------------------------------
    // C. HEADER CONTEXT BAND (36px)
    // ------------------------------------------------------------------------
    let ctx_badge = tree.label("● TÉLÉMÉTRIE ACTIVE  /  Cluster-EU-West-04  /  Node-Alpha-92", leaf(440.0, 16.0)).unwrap();
    let ctx_title = tree.label("Moniteur de Performance Système & Cloud", leaf(520.0, 22.0)).unwrap();
    let ctx_left = tree.container(&[ctx_badge, ctx_title], column(2.0)).unwrap();

    let range_items = ["Direct (1s)", "15m", "1h", "24h"];
    let range_seg = tree.segmented_control(
        WidgetId::new("range_seg"),
        &range_items,
        state.active_range,
        leaf(68.0, 26.0),
        row(2.0),
    ).unwrap();
    let btn_thresh = tree.button(WidgetId::new("btn_thresholds"), "⚙ Seuils", true, leaf(85.0, 26.0)).unwrap();
    let btn_snap = tree.button(WidgetId::new("btn_snapshot"), "📥 Rapport Snapshot", true, leaf(150.0, 26.0)).unwrap();
    let ctx_right = tree.container(&[range_seg, btn_thresh, btn_snap], row(8.0)).unwrap();

    let context_band = tree.container(
        &[ctx_left, ctx_right],
        Style {
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::Center),
            justify_content: Some(ui_layout::JustifyContent::SpaceBetween),
            size: Size { width: length(1340.0), height: length(36.0) },
            margin: rect_margin_b(8.0),
            ..Default::default()
        },
    ).unwrap();
    root_children.push(context_band);

    // ------------------------------------------------------------------------
    // D. 4 KEY KPI TELEMETRY CARDS GRID (322px each) with Target Glow Highlighting
    // ------------------------------------------------------------------------
    let k1_highlight = state.agent_target.as_deref() == Some("k1_card");
    let k2_highlight = state.agent_target.as_deref() == Some("k2_card");
    let k3_highlight = state.agent_target.as_deref() == Some("k3_card");

    // Card 1 : Charge vCPU
    let mut spark1 = Painter::new();
    spark1
        .bezier([0.0, 22.0], [20.0, 18.0], [35.0, 14.0], [70.0, 16.0], 2.0, hex_linear("#c0c1ff"))
        .bezier([70.0, 16.0], [95.0, 6.0], [110.0, 10.0], [120.0, 12.0], 2.0, hex_linear("#c0c1ff"))
        .polyline(vec![[0.0, 22.0], [35.0, 14.0], [70.0, 16.0], [95.0, 6.0], [120.0, 12.0], [120.0, 30.0], [0.0, 30.0]], 0.0, hex_linear("#c0c1ff1f"), true);
    let spark1_canvas = tree.custom_paint("spark1", spark1.finish(), leaf(120.0, 30.0)).unwrap();

    let k1_lbl = tree.label("⚙ CHARGE VCPU", leaf(160.0, 16.0)).unwrap();
    let k1_badge = if k1_highlight {
        tree.badge("🤖 Agent Target", ListItemBadge::Warning, leaf(95.0, 18.0)).unwrap()
    } else {
        tree.badge("Nominal", ListItemBadge::Success, leaf(72.0, 18.0)).unwrap()
    };
    let k1_head = tree.container(&[k1_lbl, k1_badge], row(4.0)).unwrap();
    let k1_val = tree.label(&format!("{:.1}%  @ 3.8 GHz", state.cpu_load), leaf(280.0, 24.0)).unwrap();
    let k1_sub = tree.label("32 Cores virtuels • AMD EPYC Cloud", leaf(280.0, 14.0)).unwrap();
    let k1_bot = tree.label("Min 14.1%                                  Pic 42.0%", leaf(280.0, 14.0)).unwrap();
    let k1_card = tree.card(
        &[k1_head, k1_val, k1_sub, spark1_canvas, k1_bot],
        Some(hex_linear("#111624f2")),
        Some(if k1_highlight { hex_linear("#00e0fa") } else { hex_linear("#25324af0") }),
        Some(10.0),
        Style {
            size: Size { width: length(322.0), height: length(116.0) },
            flex_direction: FlexDirection::Column,
            gap: Size { width: length(0.0), height: length(2.0) },
            padding: rect_pad(12.0, 12.0, 8.0, 8.0),
            ..Default::default()
        },
    ).unwrap();

    // Card 2 : Mémoire Vive (RAM)
    let mut bar2 = Painter::new();
    bar2
        .rect([0.0, 4.0, 290.0, 6.0], 3.0, Some(hex_linear("#1b2438")), None)
        .rect([0.0, 4.0, (state.memory_used / 64.0) * 290.0, 6.0], 3.0, Some(hex_linear("#7bd0ff")), None);
    let bar2_canvas = tree.custom_paint("bar2", bar2.finish(), leaf(290.0, 14.0)).unwrap();

    let k2_lbl = tree.label("🗄 MÉMOIRE VIVE (RAM)", leaf(160.0, 16.0)).unwrap();
    let k2_badge = if k2_highlight {
        tree.badge("🤖 Agent Target", ListItemBadge::Warning, leaf(95.0, 18.0)).unwrap()
    } else {
        tree.badge(&format!("{:.1}% utilisé", (state.memory_used / 64.0) * 100.0), ListItemBadge::Warning, leaf(96.0, 18.0)).unwrap()
    };
    let k2_head = tree.container(&[k2_lbl, k2_badge], row(4.0)).unwrap();
    let k2_val = tree.label(&format!("{:.1}  / 64 GB", state.memory_used), leaf(280.0, 24.0)).unwrap();
    let k2_sub = tree.label("DDR5 ECC • Peak: 22.4 GB", leaf(280.0, 14.0)).unwrap();
    let k2_bot = tree.label("Disponible: 45.8 GB               Cache: 6.2 GB", leaf(280.0, 14.0)).unwrap();
    let k2_card = tree.card(
        &[k2_head, k2_val, k2_sub, bar2_canvas, k2_bot],
        Some(hex_linear("#111624f2")),
        Some(if k2_highlight { hex_linear("#00e0fa") } else { hex_linear("#25324af0") }),
        Some(10.0),
        Style {
            size: Size { width: length(322.0), height: length(116.0) },
            flex_direction: FlexDirection::Column,
            gap: Size { width: length(0.0), height: length(2.0) },
            padding: rect_pad(12.0, 12.0, 8.0, 8.0),
            ..Default::default()
        },
    ).unwrap();

    // Card 3 : I/O Réseau Cloud
    let mut spark3 = Painter::new();
    spark3
        .bezier([0.0, 18.0], [25.0, 16.0], [45.0, 8.0], [65.0, 20.0], 2.0, hex_linear("#4edea3"))
        .bezier([65.0, 20.0], [90.0, 6.0], [105.0, 14.0], [120.0, 13.0], 2.0, hex_linear("#4edea3"))
        .polyline(vec![[0.0, 18.0], [45.0, 8.0], [65.0, 20.0], [90.0, 6.0], [120.0, 13.0], [120.0, 30.0], [0.0, 30.0]], 0.0, hex_linear("#4edea31f"), true);
    let spark3_canvas = tree.custom_paint("spark3", spark3.finish(), leaf(120.0, 30.0)).unwrap();

    let k3_lbl = tree.label("📡 I/O RÉSEAU CLOUD", leaf(160.0, 16.0)).unwrap();
    let k3_badge = if k3_highlight {
        tree.badge("🤖 Agent Target", ListItemBadge::Warning, leaf(95.0, 18.0)).unwrap()
    } else {
        tree.badge("● 8ms", ListItemBadge::Success, leaf(64.0, 18.0)).unwrap()
    };
    let k3_head = tree.container(&[k3_lbl, k3_badge], row(4.0)).unwrap();
    let k3_val = tree.label(&format!("{:.1} Gbps In  / 1.8 Out", state.network_speed), leaf(280.0, 24.0)).unwrap();
    let k3_sub = tree.label("Dual 25 GbE vSwitch Fabric", leaf(280.0, 14.0)).unwrap();
    let k3_bot = tree.label("Paquets: 410k pps               Pertes: 0.00%", leaf(280.0, 14.0)).unwrap();
    let k3_card = tree.card(
        &[k3_head, k3_val, k3_sub, spark3_canvas, k3_bot],
        Some(hex_linear("#111624f2")),
        Some(if k3_highlight { hex_linear("#4edea3") } else { hex_linear("#25324af0") }),
        Some(10.0),
        Style {
            size: Size { width: length(322.0), height: length(116.0) },
            flex_direction: FlexDirection::Column,
            gap: Size { width: length(0.0), height: length(2.0) },
            padding: rect_pad(12.0, 12.0, 8.0, 8.0),
            ..Default::default()
        },
    ).unwrap();

    // Card 4 : Stockage NVMe Débit
    let mut spark4 = Painter::new();
    spark4
        .bezier([0.0, 24.0], [30.0, 18.0], [55.0, 11.0], [80.0, 13.0], 2.0, hex_linear("#c0c1ff"))
        .bezier([80.0, 13.0], [100.0, 5.0], [110.0, 8.0], [120.0, 9.0], 2.0, hex_linear("#c0c1ff"))
        .polyline(vec![[0.0, 24.0], [55.0, 11.0], [80.0, 13.0], [100.0, 5.0], [120.0, 9.0], [120.0, 30.0], [0.0, 30.0]], 0.0, hex_linear("#c0c1ff1f"), true);
    let spark4_canvas = tree.custom_paint("spark4", spark4.finish(), leaf(120.0, 30.0)).unwrap();

    let k4_lbl = tree.label("🗄 STOCKAGE NVME DÉBIT", leaf(160.0, 16.0)).unwrap();
    let k4_badge = tree.badge("Tier-0 SAN", ListItemBadge::None, leaf(78.0, 18.0)).unwrap();
    let k4_head = tree.container(&[k4_lbl, k4_badge], row(4.0)).unwrap();
    let k4_val = tree.label("2,450 Mo/s", leaf(280.0, 24.0)).unwrap();
    let k4_sub = tree.label("Écriture: 1,120 Mo/s • IOPS: 184,200", leaf(280.0, 14.0)).unwrap();
    let k4_bot = tree.label("Latence R/W: 0.22ms                    Queue: 4", leaf(280.0, 14.0)).unwrap();
    let k4_card = tree.card(
        &[k4_head, k4_val, k4_sub, spark4_canvas, k4_bot],
        Some(hex_linear("#111624f2")),
        Some(hex_linear("#25324af0")),
        Some(10.0),
        Style {
            size: Size { width: length(322.0), height: length(116.0) },
            flex_direction: FlexDirection::Column,
            gap: Size { width: length(0.0), height: length(2.0) },
            padding: rect_pad(12.0, 12.0, 8.0, 8.0),
            ..Default::default()
        },
    ).unwrap();

    let kpi_row = tree.container(
        &[k1_card, k2_card, k3_card, k4_card],
        Style {
            flex_direction: FlexDirection::Row,
            justify_content: Some(ui_layout::JustifyContent::SpaceBetween),
            size: Size { width: length(1340.0), height: length(116.0) },
            margin: rect_margin_b(8.0),
            ..Default::default()
        },
    ).unwrap();
    root_children.push(kpi_row);

    // ------------------------------------------------------------------------
    // E. MAIN MIDDLE SECTION (Spline Telemetry Scrubber + Global Geo Topology)
    // ------------------------------------------------------------------------
    let chart_highlight = state.agent_target.as_deref() == Some("range_seg") || state.agent_target.as_deref() == Some("scrub_chart");

    // Left: Télémétrie Temporelle Croisée
    let chart_title = tree.label("Télémétrie Temporelle Croisée ●", leaf(300.0, 20.0)).unwrap();
    let chart_sub = tree.label("Surveillance combinée bande passante réseau et cycle CPU hôte", leaf(400.0, 14.0)).unwrap();
    let chart_head_left = tree.container(&[chart_title, chart_sub], column(2.0)).unwrap();
    let chart_leg_net = tree.badge("■ Réseau Ingress/Egress", ListItemBadge::None, leaf(150.0, 20.0)).unwrap();
    let chart_leg_cpu = tree.badge("■ Calcul vCPU", ListItemBadge::None, leaf(100.0, 20.0)).unwrap();
    let chart_head_right = tree.container(&[chart_leg_net, chart_leg_cpu], row(6.0)).unwrap();
    let chart_head = tree.container(
        &[chart_head_left, chart_head_right],
        Style {
            flex_direction: FlexDirection::Row,
            justify_content: Some(ui_layout::JustifyContent::SpaceBetween),
            size: Size { width: length(850.0), height: length(32.0) },
            ..Default::default()
        },
    ).unwrap();

    let mut scrub_p = Painter::new();
    // 4 subtle grid lines
    scrub_p
        .line([0.0, 30.0], [850.0, 30.0], 1.0, hex_linear("#1e2a3e80"))
        .line([0.0, 75.0], [850.0, 75.0], 1.0, hex_linear("#1e2a3e80"))
        .line([0.0, 120.0], [850.0, 120.0], 1.0, hex_linear("#1e2a3e80"))
        .line([0.0, 165.0], [850.0, 165.0], 1.0, hex_linear("#1e2a3e80"))
        // Cyan Ingress/Egress Area & Curve
        .bezier([0.0, 120.0], [90.0, 95.0], [170.0, 110.0], [340.0, 70.0], 2.5, hex_linear("#7bd0ff"))
        .bezier([340.0, 70.0], [510.0, 85.0], [680.0, 40.0], [850.0, 55.0], 2.5, hex_linear("#7bd0ff"))
        .polyline(vec![[0.0, 120.0], [170.0, 110.0], [340.0, 70.0], [510.0, 85.0], [680.0, 40.0], [850.0, 55.0], [850.0, 170.0], [0.0, 170.0]], 0.0, hex_linear("#7bd0ff1f"), true)
        // Indigo CPU Curve
        .bezier([0.0, 140.0], [110.0, 125.0], [220.0, 135.0], [420.0, 100.0], 2.0, hex_linear("#c0c1ff"))
        .bezier([420.0, 100.0], [590.0, 115.0], [730.0, 75.0], [850.0, 90.0], 2.0, hex_linear("#c0c1ff"))
        .polyline(vec![[0.0, 140.0], [220.0, 135.0], [420.0, 100.0], [590.0, 115.0], [730.0, 75.0], [850.0, 90.0], [850.0, 170.0], [0.0, 170.0]], 0.0, hex_linear("#c0c1ff1a"), true)
        // Vertical Crosshair Line at x = 680
        .line([680.0, 0.0], [680.0, 170.0], 1.5, hex_linear("#dfe2f166"))
        .circle([680.0, 40.0], 5.0, Some(hex_linear("#7bd0ff")), Some((hex_linear("#0b0f19"), 2.0)))
        .circle([680.0, 80.0], 4.0, Some(hex_linear("#c0c1ff")), Some((hex_linear("#0b0f19"), 2.0)))
        // Floating Scrubber Tooltip Popup at x = 600, y = 8
        .rect([600.0, 8.0, 150.0, 56.0], 6.0, Some(hex_linear("#172033f8")), Some((hex_linear("#2d3f5ff0"), 1.0)))
        .text([610.0, 14.0], "Index T-04:22:18", 10.0, hex_linear("#8d99b2"))
        .text([610.0, 28.0], "Bande passante: 4.62 Gbps", 11.0, hex_linear("#7bd0ff"))
        .text([610.0, 44.0], "Charge vCPU:    38.7%", 11.0, hex_linear("#c0c1ff"));

    let scrub_chart = tree.custom_paint("scrub_chart", scrub_p.finish(), leaf(850.0, 150.0)).unwrap();
    let scrub_timeline = tree.label("14:15:00                       14:20:00                       14:25:00                       14:30:00                       Maintenant (Live)", leaf(850.0, 16.0)).unwrap();

    let chart_card = tree.card(
        &[chart_head, scrub_chart, scrub_timeline],
        Some(hex_linear("#111624f2")),
        Some(if chart_highlight { hex_linear("#7bd0ff") } else { hex_linear("#25324af0") }),
        Some(10.0),
        Style {
            size: Size { width: length(874.0), height: length(218.0) },
            flex_direction: FlexDirection::Column,
            gap: Size { width: length(0.0), height: length(2.0) },
            padding: rect_pad(12.0, 12.0, 6.0, 6.0),
            ..Default::default()
        },
    ).unwrap();

    // Right: Topologie Globale
    let geo_title = tree.label("Topologie Globale", leaf(180.0, 20.0)).unwrap();
    let geo_sub = tree.label("Disponibilité maillée inter-régions", leaf(220.0, 14.0)).unwrap();
    let geo_head_left = tree.container(&[geo_title, geo_sub], column(2.0)).unwrap();
    let geo_badge = tree.badge("4 / 4 Opérationnels", ListItemBadge::Success, leaf(130.0, 20.0)).unwrap();
    let geo_head = tree.container(
        &[geo_head_left, geo_badge],
        Style {
            flex_direction: FlexDirection::Row,
            justify_content: Some(ui_layout::JustifyContent::SpaceBetween),
            size: Size { width: length(430.0), height: length(32.0) },
            ..Default::default()
        },
    ).unwrap();

    let mut map_p = Painter::new();
    map_p
        .rect([0.0, 0.0, 430.0, 95.0], 6.0, Some(hex_linear("#0c111df2")), Some((hex_linear("#1e283cf0"), 1.0)))
        // Inter-region dotted links
        .line([90.0, 25.0], [310.0, 25.0], 1.0, hex_linear("#4edea366"))
        .line([90.0, 25.0], [90.0, 75.0], 1.0, hex_linear("#4edea366"))
        .line([310.0, 25.0], [310.0, 75.0], 1.0, hex_linear("#4edea366"))
        // Node 1: Frankfurt (FRA-1)
        .rect([16.0, 12.0, 180.0, 26.0], 4.0, Some(hex_linear("#151c2cf8")), Some((hex_linear("#293854f0"), 1.0)))
        .circle([28.0, 25.0], 3.0, Some(hex_linear("#4edea3")), None)
        .text([38.0, 18.0], "Frankfurt (FRA-1)", 11.0, hex_linear("#dfe2f1"))
        // Node 2: Paris (CDG-2)
        .rect([230.0, 12.0, 180.0, 26.0], 4.0, Some(hex_linear("#151c2cf8")), Some((hex_linear("#293854f0"), 1.0)))
        .circle([242.0, 25.0], 3.0, Some(hex_linear("#4edea3")), None)
        .text([252.0, 18.0], "Paris (CDG-2)", 11.0, hex_linear("#dfe2f1"))
        // Node 3: N. Virginia (US-E1)
        .rect([16.0, 58.0, 180.0, 26.0], 4.0, Some(hex_linear("#151c2cf8")), Some((hex_linear("#293854f0"), 1.0)))
        .circle([28.0, 71.0], 3.0, Some(hex_linear("#4edea3")), None)
        .text([38.0, 64.0], "N. Virginia (US-E1)", 11.0, hex_linear("#dfe2f1"))
        // Node 4: Tokyo (HND-1)
        .rect([230.0, 58.0, 180.0, 26.0], 4.0, Some(hex_linear("#151c2cf8")), Some((hex_linear("#293854f0"), 1.0)))
        .circle([242.0, 71.0], 3.0, Some(hex_linear("#4edea3")), None)
        .text([252.0, 64.0], "Tokyo (HND-1)", 11.0, hex_linear("#dfe2f1"));

    let geo_map_canvas = tree.custom_paint("geo_map_canvas", map_p.finish(), leaf(430.0, 95.0)).unwrap();

    let lat1 = tree.label("FRA ↔ CDG (Intra-EU)                               4.2 ms", leaf(430.0, 14.0)).unwrap();
    let lat2 = tree.label("FRA ↔ US-E1 (Transatlantique)                  68.4 ms", leaf(430.0, 14.0)).unwrap();
    let lat3 = tree.label("CDG ↔ Tokyo (Transpacifique)                 182.1 ms", leaf(430.0, 14.0)).unwrap();

    let geo_card = tree.card(
        &[geo_head, geo_map_canvas, lat1, lat2, lat3],
        Some(hex_linear("#111624f2")),
        Some(hex_linear("#25324af0")),
        Some(10.0),
        Style {
            size: Size { width: length(454.0), height: length(218.0) },
            flex_direction: FlexDirection::Column,
            gap: Size { width: length(0.0), height: length(3.0) },
            padding: rect_pad(12.0, 12.0, 6.0, 6.0),
            ..Default::default()
        },
    ).unwrap();

    let visual_row = tree.container(
        &[chart_card, geo_card],
        Style {
            flex_direction: FlexDirection::Row,
            justify_content: Some(ui_layout::JustifyContent::SpaceBetween),
            size: Size { width: length(1340.0), height: length(218.0) },
            margin: rect_margin_b(8.0),
            ..Default::default()
        },
    ).unwrap();
    root_children.push(visual_row);

    // ------------------------------------------------------------------------
    // F. BOTTOM SECTION: PROCESSUS & CONTENEURS ACTIFS TABLE (174px)
    // ------------------------------------------------------------------------
    let table_highlight = state.agent_target.as_deref() == Some("proc_table") || state.agent_target.as_deref() == Some("proc_search");

    let tbl_title = tree.label("Processus & Conteneurs Actifs", leaf(280.0, 20.0)).unwrap();
    let tbl_sub = tree.label("Orchestrateur MicroVM Firecracker v2.9", leaf(300.0, 14.0)).unwrap();
    let tbl_head_left = tree.container(&[tbl_title, tbl_sub], column(2.0)).unwrap();

    let search_focused = state.focused_input.as_deref() == Some("proc_search");
    let search_box = tree.text_input_with_cursor(
        WidgetId::new("proc_search"),
        &state.search_editor.text,
        "Filtrer un conteneur / PID...",
        search_focused,
        state.search_editor.cursor,
        state.search_editor.selection,
        leaf(260.0, 26.0),
    ).unwrap();
    let btn_refresh = tree.button(WidgetId::new("btn_refresh"), "🔄", true, leaf(32.0, 26.0)).unwrap();
    let tbl_head_right = tree.container(&[search_box, btn_refresh], row(6.0)).unwrap();

    let tbl_header = tree.container(
        &[tbl_head_left, tbl_head_right],
        Style {
            flex_direction: FlexDirection::Row,
            justify_content: Some(ui_layout::JustifyContent::SpaceBetween),
            size: Size { width: length(1316.0), height: length(30.0) },
            ..Default::default()
        },
    ).unwrap();

    // Table Columns & Dynamic Rows
    let table_cols = [
        ("PROCESSUS / NAMESPACE", 340.0, None),
        ("PID", 80.0, None),
        ("UTILISATEUR", 110.0, None),
        ("CHARGE VCPU", 140.0, None),
        ("MÉMOIRE (RAM)", 140.0, None),
        ("I/O RÉSEAU", 160.0, None),
        ("ACTIONS", 240.0, None),
    ];

    let filter_query = state.search_editor.text.trim().to_lowercase();
    let filtered_processes: Vec<&ProcessItem> = state.processes
        .iter()
        .filter(|p| {
            if filter_query.is_empty() {
                true
            } else {
                p.name.to_lowercase().contains(&filter_query)
                    || p.user.to_lowercase().contains(&filter_query)
                    || p.pid.to_string().contains(&filter_query)
                    || p.status.to_lowercase().contains(&filter_query)
            }
        })
        .collect();

    let table_rows: Vec<Vec<(String, ListItemBadge)>> = filtered_processes
        .iter()
        .map(|p| {
            vec![
                (p.name.clone(), p.badge.clone()),
                (p.pid.to_string(), ListItemBadge::None),
                (p.user.clone(), ListItemBadge::None),
                (format!("{:.1}%", p.cpu_percent), if p.cpu_percent > 15.0 { ListItemBadge::Warning } else { ListItemBadge::None }),
                (if p.memory_mb >= 1024 { format!("{:.1} GB", p.memory_mb as f32 / 1024.0) } else { format!("{} MB", p.memory_mb) }, ListItemBadge::None),
                (p.network_str.clone(), if p.network_str.contains("Gbps") { ListItemBadge::Warning } else { ListItemBadge::None }),
                (if p.status == "Terminated" { "[ Relancer ]".to_string() } else { "[ Logs ]  [ Redémarrer ]  [ Terminer ]".to_string() }, if p.cpu_percent > 15.0 { ListItemBadge::Warning } else { ListItemBadge::None }),
            ]
        })
        .collect();

    let row_refs: Vec<Vec<(&str, ListItemBadge)>> = table_rows
        .iter()
        .map(|r| r.iter().map(|(s, b)| (s.as_str(), b.clone())).collect())
        .collect();

    let proc_table = tree.table("proc_table", &table_cols, &row_refs, Some(0), 22.0, leaf(1316.0, 116.0)).unwrap();

    let tbl_footer_left = tree.label("Total: 84 processus actifs • Zombies: 0 • Threads alloués: 640", leaf(500.0, 16.0)).unwrap();
    let tbl_footer_right = tree.label("Charge moyenne (Load Avg): 1.12, 1.05, 0.98", leaf(340.0, 16.0)).unwrap();
    let tbl_footer = tree.container(
        &[tbl_footer_left, tbl_footer_right],
        Style {
            flex_direction: FlexDirection::Row,
            justify_content: Some(ui_layout::JustifyContent::SpaceBetween),
            size: Size { width: length(1316.0), height: length(18.0) },
            margin: Rect {
                left: auto(),
                right: auto(),
                top: length(6.0),
                bottom: length(2.0),
            },
            ..Default::default()
        },
    ).unwrap();

    let proc_card = tree.card(
        &[tbl_header, proc_table, tbl_footer],
        Some(hex_linear("#111624f2")),
        Some(if table_highlight { hex_linear("#7bd0ff") } else { hex_linear("#25324af0") }),
        Some(10.0),
        Style {
            size: Size { width: length(1340.0), height: length(196.0) },
            flex_direction: FlexDirection::Column,
            gap: Size { width: length(0.0), height: length(4.0) },
            padding: rect_pad(12.0, 12.0, 6.0, 6.0),
            margin: rect_margin_b(6.0),
            ..Default::default()
        },
    ).unwrap();
    root_children.push(proc_card);

    // ------------------------------------------------------------------------
    // G. FLOATING DOCK AT BOTTOM (Monochrome White on Midnight Navy with White Hairline)
    // ------------------------------------------------------------------------
    let dock_d1 = tree.button(WidgetId::new("dock_0"), "⊞", true, leaf(36.0, 28.0)).unwrap();
    let dock_d2 = tree.button(WidgetId::new("dock_1"), "▤", true, leaf(36.0, 28.0)).unwrap();
    let dock_d3 = tree.button(WidgetId::new("dock_2"), "◈", true, leaf(36.0, 28.0)).unwrap();
    let dock_d4 = tree.button(WidgetId::new("dock_3"), "◎", true, leaf(36.0, 28.0)).unwrap();
    let dock_d5 = tree.button(WidgetId::new("dock_4"), "⚙", true, leaf(36.0, 28.0)).unwrap();
    let dock_d6 = tree.button(WidgetId::new("dock_5"), "✕", true, leaf(36.0, 28.0)).unwrap();

    let dock_capsule = tree.card(
        &[dock_d1, dock_d2, dock_d3, dock_d4, dock_d5, dock_d6],
        Some(hex_linear("#0b1220f8")),
        Some(hex_linear("#ffffffcc")),
        Some(17.0),
        Style {
            flex_direction: FlexDirection::Row,
            align_items: Some(AlignItems::Center),
            justify_content: Some(ui_layout::JustifyContent::Center),
            gap: Size { width: length(4.0), height: length(0.0) },
            size: Size { width: length(270.0), height: length(34.0) },
            padding: rect_pad(6.0, 6.0, 2.0, 2.0),
            ..Default::default()
        },
    ).unwrap();

    let dock_wrapper = tree.container(
        &[dock_capsule],
        Style {
            flex_direction: FlexDirection::Row,
            justify_content: Some(ui_layout::JustifyContent::Center),
            size: Size { width: length(1340.0), height: length(36.0) },
            ..Default::default()
        },
    ).unwrap();
    root_children.push(dock_wrapper);

    // Root Container
    tree.container(
        &root_children,
        Style {
            flex_direction: FlexDirection::Column,
            size: Size { width: length(WINDOW_WIDTH), height: length(WINDOW_HEIGHT) },
            padding: rect_pad(20.0, 20.0, 6.0, 6.0),
            ..Default::default()
        },
    ).unwrap()
}

fn build_modal_ui(tree: &mut WidgetTree, title: &str, message: &str, is_approval: bool) -> NodeId {
    let title_lbl = tree.label(title, leaf(500.0, 24.0)).unwrap();
    let msg_lbl = tree.label(message, leaf(500.0, 56.0)).unwrap();
    let btn_row = if is_approval {
        let btn_cancel = tree.button(
            WidgetId::new("modal_cancel_btn"),
            "Annuler",
            true,
            leaf(110.0, 32.0),
        ).unwrap();
        let btn_confirm = tree.button_variant(
            WidgetId::new("modal_confirm_btn"),
            "Confirmer & Appliquer",
            ui_widgets::ButtonVariant::Primary,
            true,
            leaf(180.0, 32.0),
        ).unwrap();
        tree.container(
            &[btn_cancel, btn_confirm],
            Style {
                flex_direction: FlexDirection::Row,
                justify_content: Some(ui_layout::JustifyContent::FlexEnd),
                gap: Size { width: length(10.0), height: length(0.0) },
                size: Size { width: length(500.0), height: length(34.0) },
                ..Default::default()
            },
        ).unwrap()
    } else {
        let btn_ok = tree.button_variant(
            WidgetId::new("modal_dismiss_btn"),
            "Fermer",
            ui_widgets::ButtonVariant::Primary,
            true,
            leaf(120.0, 32.0),
        ).unwrap();
        tree.container(
            &[btn_ok],
            Style {
                flex_direction: FlexDirection::Row,
                justify_content: Some(ui_layout::JustifyContent::FlexEnd),
                size: Size { width: length(500.0), height: length(34.0) },
                ..Default::default()
            },
        ).unwrap()
    };

    let dialog = tree.card(
        &[title_lbl, msg_lbl, btn_row],
        Some(hex_linear("#0f1422fa")),
        Some(hex_linear("#2a3854f0")),
        Some(12.0),
        Style {
            size: Size { width: length(540.0), height: length(176.0) },
            flex_direction: FlexDirection::Column,
            justify_content: Some(ui_layout::JustifyContent::SpaceBetween),
            padding: rect_pad(20.0, 20.0, 16.0, 16.0),
            ..Default::default()
        },
    ).unwrap();

    tree.container(
        &[dialog],
        Style {
            size: Size { width: length(WINDOW_WIDTH), height: length(WINDOW_HEIGHT) },
            align_items: Some(AlignItems::Center),
            justify_content: Some(ui_layout::JustifyContent::Center),
            ..Default::default()
        },
    ).unwrap()
}

// ============================================================================
// 5. Application Winit & Boucle d'Événements
// ============================================================================

struct AetherWorkstationApp {
    window: Option<Arc<Window>>,
    renderer: Option<GpuRenderer>,
    theme: Arc<RwLock<Theme>>,
    _theme_watcher: Option<ThemeWatcher>,
    state: AetherWorkstationState,
    cursor_pos: (f32, f32),
    pressed: Option<InteractionKey>,
    root: Option<NodeId>,

    tx_events: mpsc::Sender<UiEvent>,
    rx_patches: crossbeam_channel::Receiver<UiPatch>,
    resources: ui_gpu::ResourceTable,
}

impl AetherWorkstationApp {
    fn new(
        tx_events: mpsc::Sender<UiEvent>,
        rx_patches: crossbeam_channel::Receiver<UiPatch>,
    ) -> Self {
        let theme = Theme::from_file("themes/aether_os.toml").unwrap_or_else(|_| Theme::aether_os());
        Self {
            window: None,
            renderer: None,
            theme: Arc::new(RwLock::new(theme)),
            _theme_watcher: None,
            state: AetherWorkstationState::default(),
            cursor_pos: (0.0, 0.0),
            pressed: None,
            root: None,
            tx_events,
            rx_patches,
            resources: ui_gpu::ResourceTable::new(),
        }
    }

    fn redraw(&mut self) {
        while let Ok(patch) = self.rx_patches.try_recv() {
            self.state.apply_patch(patch);
        }

        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };

        let current_theme = self.theme.read().unwrap().clone();
        let measure = renderer.text_measure();

        let mut tree = WidgetTree::new();
        let base_root = build_aether_ui(&mut tree, &self.state);

        let available = Size {
            width: AvailableSpace::Definite(WINDOW_WIDTH),
            height: AvailableSpace::Definite(WINDOW_HEIGHT),
        };
        if let Err(e) = tree.compute(base_root, available) {
            eprintln!("⚠️ [AetherWorkstation] Layout error: {:?}", e);
            return;
        }

        let hovered = tree.interaction_key_at(base_root, self.cursor_pos).unwrap_or(None);
        let interaction = InteractionState {
            hovered: hovered.as_ref(),
            pressed: self.pressed.as_ref(),
            measure: Some(&measure),
        };

        let base_frame = match tree.build_frame(base_root, &current_theme, interaction) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("⚠️ [AetherWorkstation] Frame build error: {:?}", e);
                return;
            }
        };

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
                    glyphon::cosmic_text::Family::SansSerif,
                    weight,
                    spec.clip,
                )
            })
            .collect();

        let base_layer = ui_gpu::RenderLayer {
            instances: &base_frame.instances,
            texts: &base_text_runs,
        };

        // Dark deep atmospheric cosmic background (#05080f in linear color space)
        let background = wgpu::Color {
            r: 0.0011,
            g: 0.0020,
            b: 0.0044,
            a: 1.0,
        };

        if let Some((title, content)) = &self.state.active_modal {
            let mut modal_tree = WidgetTree::new();
            let is_approval = self.state.pending_approval_prompt.is_some();
            let modal_root = build_modal_ui(&mut modal_tree, title, content, is_approval);
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
                            glyphon::cosmic_text::Family::SansSerif,
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
        } else {
            let _ = renderer.render_layers(background, &[base_layer], &[], &self.resources);
        }

        self.root = Some(base_root);
    }

    fn submit_agent_prompt(&mut self, prompt: String) {
        println!("🤖 [AetherWorkstation] Action transmise à l'Agent : '{}'", prompt);
        let _ = self.tx_events.try_send(UiEvent::UserPromptSubmitted(prompt));
    }

    fn submit_agent_instruction(&mut self, prompt: String) {
        if self.state.require_human_approval {
            self.state.pending_approval_prompt = Some(prompt.clone());
            self.state.active_modal = Some((
                "🛡️ Confirmation du Plan Copilote".to_string(),
                format!("L'Agent Copilote a préparé le plan pour :\n\"{}\"\n\nSouhaitez-vous confirmer et appliquer ces modifications ?", prompt),
            ));
        } else {
            self.submit_agent_prompt(prompt);
        }
    }
}

impl ApplicationHandler for AetherWorkstationApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attrs = WindowAttributes::default()
            .with_title("AetherOS — Moniteur de Performance Système & Cloud")
            .with_inner_size(winit::dpi::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
            .with_resizable(false);

        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .expect("Failed to create AetherOS workstation window"),
        );

        let renderer = GpuRenderer::new(window.clone());
        self.window = Some(window.clone());
        self.renderer = Some(renderer);

        let window_clone = window.clone();
        let watcher = ThemeWatcher::watch_file(
            "themes/aether_os.toml",
            self.theme.clone(),
            move |_| {
                window_clone.request_redraw();
            },
        ).ok();
        self._theme_watcher = watcher;

        event_loop.set_control_flow(ControlFlow::Poll);
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let mut has_patches = false;
        while let Ok(patch) = self.rx_patches.try_recv() {
            self.state.apply_patch(patch);
            has_patches = true;
        }
        if has_patches {
            if let Some(w) = self.window.as_ref() {
                w.request_redraw();
            }
            self.redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                self.redraw();
            }
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
                        if self.state.focused_input.as_deref() == Some("proc_search") {
                            let q = self.state.search_editor.text.clone();
                            self.submit_agent_instruction(format!("Filtrer les conteneurs et analyser: {}", q));
                        } else {
                            let prompt = self.state.prompt_editor.text.clone();
                            if !prompt.trim().is_empty() {
                                self.submit_agent_instruction(prompt);
                            }
                        }
                    }
                    Key::Named(NamedKey::Backspace) => {
                        if self.state.focused_input.as_deref() == Some("proc_search") {
                            self.state.search_editor.backspace();
                        } else if self.state.focused_input.as_deref() == Some("prompt_input") {
                            self.state.prompt_editor.backspace();
                        }
                        self.redraw();
                    }
                    Key::Named(NamedKey::Space) => {
                        if self.state.focused_input.as_deref() == Some("proc_search") {
                            self.state.search_editor.insert_char(' ');
                        } else if self.state.focused_input.as_deref() == Some("prompt_input") {
                            self.state.prompt_editor.insert_char(' ');
                        }
                        self.redraw();
                    }
                    Key::Character(s) => {
                        for c in s.chars() {
                            if self.state.focused_input.as_deref() == Some("proc_search") {
                                self.state.search_editor.insert_char(c);
                            } else if self.state.focused_input.as_deref() == Some("prompt_input") {
                                self.state.prompt_editor.insert_char(c);
                            }
                        }
                        self.redraw();
                    }
                    _ => {}
                }
            }
            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                if let Some((_title, _content)) = &self.state.active_modal {
                    let mut modal_tree = WidgetTree::new();
                    let is_approval = self.state.pending_approval_prompt.is_some();
                    let m_root = build_modal_ui(&mut modal_tree, "Modal", "Content", is_approval);
                    let _ = modal_tree.compute(m_root, Size { width: AvailableSpace::Definite(WINDOW_WIDTH), height: AvailableSpace::Definite(WINDOW_HEIGHT) });
                    if let Ok(Some(hit)) = modal_tree.interaction_key_at(m_root, self.cursor_pos) {
                        let hit_id = hit.widget_id.as_str();
                        if hit_id == "modal_dismiss_btn" || hit_id == "modal_cancel_btn" {
                            self.state.active_modal = None;
                            self.state.pending_approval_prompt = None;
                        } else if hit_id == "modal_confirm_btn" {
                            if let Some(prompt) = self.state.pending_approval_prompt.take() {
                                self.submit_agent_prompt(prompt);
                            }
                            self.state.active_modal = None;
                        }
                    }
                    self.redraw();
                    return;
                }

                let mut tree = WidgetTree::new();
                let root = build_aether_ui(&mut tree, &self.state);
                let _ = tree.compute(root, Size { width: AvailableSpace::Definite(WINDOW_WIDTH), height: AvailableSpace::Definite(WINDOW_HEIGHT) });

                if let Ok(Some(hit_key)) = tree.interaction_key_at(root, self.cursor_pos) {
                    let hit_str = hit_key.widget_id.as_str();
                    println!("🖱️ Clic sur widget AetherOS : {}", hit_str);

                    if hit_str == "prompt_input" {
                        self.state.focused_input = Some("prompt_input".to_string());
                    } else if hit_str == "proc_search" {
                        self.state.focused_input = Some("proc_search".to_string());
                    } else if hit_str == "btn_send_prompt" {
                        let prompt = self.state.prompt_editor.text.clone();
                        if !prompt.trim().is_empty() {
                            self.submit_agent_instruction(prompt);
                        }
                    } else if hit_str == "btn_toggle_approval" {
                        self.state.require_human_approval = !self.state.require_human_approval;
                        let mode_str = if self.state.require_human_approval { "Sécurisé (Validation requise)" } else { "Direct (Exécution immédiate)" };
                        self.state.diff_preview = Some(format!("Δ Mode Copilote : {}", mode_str));
                    } else if hit_str == "quick_purge" {
                        self.state.prompt_editor = TextEditorState::new("Purger les caches et stabiliser la RAM");
                        self.submit_agent_instruction("Purger les caches et actualiser les flux télémétriques".to_string());
                    } else if hit_str == "quick_cuda" {
                        self.state.prompt_editor = TextEditorState::new("Inspecter et redémarrer le processus: ai-inference-engine");
                        self.submit_agent_instruction("Inspecter et redémarrer le processus: ai-inference-engine".to_string());
                    } else if hit_str == "quick_envoy" {
                        self.state.prompt_editor = TextEditorState::new("Filtrer les conteneurs et analyser: envoy");
                        self.submit_agent_instruction("Filtrer les conteneurs et analyser: envoy".to_string());
                    } else if hit_str == "quick_thresh" {
                        self.state.prompt_editor = TextEditorState::new("Ajuster les seuils de performance et vérifier les alertes");
                        self.submit_agent_instruction("Ajuster les seuils de performance et vérifier les alertes".to_string());
                    } else if hit_str == "quick_snap" {
                        self.state.prompt_editor = TextEditorState::new("Générer le rapport snapshot et valider la réplication multi-région");
                        self.submit_agent_instruction("Générer le rapport snapshot et valider la réplication multi-région".to_string());
                    } else if hit_str == "btn_snapshot" {
                        self.submit_agent_instruction("Générer le rapport snapshot et valider la réplication multi-région".to_string());
                    } else if hit_str == "btn_thresholds" {
                        self.submit_agent_instruction("Ajuster les seuils de performance et vérifier les alertes".to_string());
                    } else if hit_str == "btn_refresh" {
                        self.submit_agent_instruction("Purger les caches et actualiser les flux télémétriques".to_string());
                    } else if hit_str == "proc_table" {
                        if let Some(row_idx) = hit_key.index {
                            let p_name = self.state.processes.get(row_idx).map(|p| p.name.clone()).unwrap_or_default();
                            self.submit_agent_instruction(format!("Inspecter et redémarrer le processus: {}", p_name));
                        }
                    } else if hit_str.starts_with("dock_") {
                        if let Ok(idx) = hit_str.trim_start_matches("dock_").parse::<usize>() {
                            self.state.active_dock_app = idx;
                            self.submit_agent_instruction(format!("Basculer vers l'espace dock {}", idx));
                        }
                    } else if hit_str == "range_seg" {
                        if let Some(idx) = hit_key.index {
                            self.state.active_range = idx;
                            let range_label = match idx {
                                1 => "15m",
                                2 => "1h",
                                3 => "24h",
                                _ => "Direct (1s)",
                            };
                            self.submit_agent_instruction(format!("Ajuster la fenêtre télémétrique sur {}", range_label));
                        }
                    }

                    self.redraw();
                }
            }
            _ => {}
        }
    }
}

// ============================================================================
// 6. Démarrage de l'Application AetherOS & HTTP Bridge d'Agent
// ============================================================================

async fn start_agent_http_bridge(tx_events: mpsc::Sender<UiEvent>, base_port: u16) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    let mut listener_opt = None;
    let mut bound_port = base_port;

    for p in base_port..=(base_port + 5) {
        let addr = format!("127.0.0.1:{}", p);
        if let Ok(l) = TcpListener::bind(&addr).await {
            println!("🌐 [AgentBridge] Serveur HTTP d'instructions agent distant actif sur http://{}", addr);
            println!("🛡️ [Sandbox Mode] Exécution sandboxée étanche : Aucun impact sur les processus réels de l'OS hôte.");
            bound_port = p;
            listener_opt = Some(l);
            break;
        }
    }

    let listener = match listener_opt {
        Some(l) => l,
        None => {
            println!("ℹ️ [AgentBridge] Mode Sandbox local actif (contrôle direct par l'UI et le runtime mémoire).");
            return;
        }
    };

    loop {
        if let Ok((mut socket, _)) = listener.accept().await {
            let tx = tx_events.clone();
            tokio::spawn(async move {
                let mut full_req = Vec::new();
                let mut buf = [0u8; 2048];
                let mut content_length: Option<usize> = None;
                let mut header_len = 0;

                loop {
                    let n = match socket.read(&mut buf).await {
                        Ok(n) if n > 0 => n,
                        _ => break,
                    };
                    full_req.extend_from_slice(&buf[..n]);

                    if content_length.is_none() {
                        if let Some(pos) = full_req.windows(4).position(|w| w == b"\r\n\r\n") {
                            header_len = pos + 4;
                            let header_str = String::from_utf8_lossy(&full_req[..pos]);
                            for line in header_str.lines() {
                                if line.to_ascii_lowercase().starts_with("content-length:") {
                                    if let Some(val) = line.split(':').nth(1) {
                                        content_length = val.trim().parse::<usize>().ok();
                                    }
                                }
                            }
                            if content_length.is_none() {
                                content_length = Some(0);
                            }
                        }
                    }

                    if let Some(cl) = content_length {
                        if full_req.len() >= header_len + cl {
                            break;
                        }
                    }
                }

                if full_req.is_empty() { return; }
                let req = String::from_utf8_lossy(&full_req);

                let (status_code, resp_json) = if req.starts_with("POST /prompt") || req.starts_with("POST /action") {
                    if let Some(body_start) = req.find("\r\n\r\n") {
                        let body = req[body_start + 4..].trim();
                        let prompt_opt = if let Ok(parsed) = serde_json::from_str::<Value>(body) {
                            parsed.get("prompt").and_then(|v| v.as_str()).map(|s| s.to_string())
                        } else if !body.is_empty() {
                            Some(body.to_string())
                        } else {
                            None
                        };

                        if let Some(prompt) = prompt_opt {
                            println!("🌐 [AgentBridge] Ordre reçu via HTTP : '{}'", prompt);
                            let _ = tx.send(UiEvent::UserPromptSubmitted(prompt.clone())).await;
                            ("200 OK", json!({
                                "status": "dispatched",
                                "prompt": prompt,
                                "source": "http_bridge"
                            }))
                        } else {
                            ("400 Bad Request", json!({"error": "Champ 'prompt' manquant dans le corps JSON"}))
                        }
                    } else {
                        ("400 Bad Request", json!({"error": "Headers malformés"}))
                    }
                } else if req.starts_with("GET /health") || req.starts_with("GET /status") {
                    ("200 OK", json!({
                        "status": "healthy",
                        "application": "AetherOS Workstation",
                        "agent_runtime": "active",
                        "port": bound_port,
                    }))
                    } else if req.starts_with("OPTIONS") {
                        ("200 OK", json!({"status": "allowed"}))
                    } else {
                        ("404 Not Found", json!({"error": "Route introuvable"}))
                    };

                    let body_str = resp_json.to_string();
                    let response = format!(
                        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: *\r\nAccess-Control-Allow-Methods: POST, GET, OPTIONS\r\nConnection: close\r\n\r\n{}",
                        status_code,
                        body_str.len(),
                        body_str
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                    let _ = socket.flush().await;
                    let _ = socket.shutdown().await;
            });
        }
    }
}

fn main() {
    println!("============================================================");
    println!("🌌 AETHEROS — MONITEUR DE PERFORMANCE SYSTÈME & CLOUD");
    println!("🚀 Deep Atmospheric Glassmorphism & Autonomous Agent Control");
    println!("============================================================");

    let (tx_events, rx_events) = mpsc::channel::<UiEvent>(64);
    let (tx_patches, rx_patches) = crossbeam_channel::unbounded::<UiPatch>();

    let mut tools = ToolRegistry::new();
    tools.register(Arc::new(SetFieldTool { tx_patches: tx_patches.clone() }));
    tools.register(Arc::new(SwitchTabTool { tx_patches: tx_patches.clone() }));
    tools.register(Arc::new(UpdateMetricTool { tx_patches: tx_patches.clone() }));
    tools.register(Arc::new(NotifyModalTool { tx_patches: tx_patches.clone() }));
    tools.register(Arc::new(PurgeCachesTool { tx_patches: tx_patches.clone() }));
    tools.register(Arc::new(RestartProcessTool { tx_patches: tx_patches.clone() }));
    tools.register(Arc::new(KillProcessTool { tx_patches: tx_patches.clone() }));
    tools.register(Arc::new(FilterTableTool { tx_patches: tx_patches.clone() }));
    tools.register(Arc::new(SwitchTimeRangeTool { tx_patches: tx_patches.clone() }));
    tools.register(Arc::new(AdjustThresholdsTool { tx_patches: tx_patches.clone() }));

    let agent_config = AgentConfig {
        max_steps: 16,
        tool_timeout: Duration::from_secs(10),
        max_scratchpad_entries: 50,
    };
    let agent = Agent::new(agent_config, tools, tx_patches);

    let tx_http = tx_events.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Échec de création du runtime Tokio");

        rt.block_on(async move {
            tokio::spawn(start_agent_http_bridge(tx_http, 8765));
            let planner = Box::new(DynamicAgentPlanner::new());
            println!("🤖 [AetherRuntime] Agent cognitif initialisé avec 10 outils de contrôle UI.");
            if let Err(e) = agent.run(rx_events, planner).await {
                eprintln!("⚠️ [AetherRuntime] Erreur d'exécution de l'agent : {:?}", e);
            }
        });
    });

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = AetherWorkstationApp::new(tx_events, rx_patches);
    event_loop.run_app(&mut app).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_agent() -> (
        mpsc::Sender<UiEvent>,
        crossbeam_channel::Receiver<UiPatch>,
        tokio::task::JoinHandle<()>,
    ) {
        let (tx_events, rx_events) = mpsc::channel::<UiEvent>(64);
        let (tx_patches, rx_patches) = crossbeam_channel::unbounded::<UiPatch>();

        let mut tools = ToolRegistry::new();
        tools.register(Arc::new(SetFieldTool { tx_patches: tx_patches.clone() }));
        tools.register(Arc::new(SwitchTabTool { tx_patches: tx_patches.clone() }));
        tools.register(Arc::new(UpdateMetricTool { tx_patches: tx_patches.clone() }));
        tools.register(Arc::new(NotifyModalTool { tx_patches: tx_patches.clone() }));
        tools.register(Arc::new(PurgeCachesTool { tx_patches: tx_patches.clone() }));
        tools.register(Arc::new(RestartProcessTool { tx_patches: tx_patches.clone() }));
        tools.register(Arc::new(KillProcessTool { tx_patches: tx_patches.clone() }));
        tools.register(Arc::new(FilterTableTool { tx_patches: tx_patches.clone() }));
        tools.register(Arc::new(SwitchTimeRangeTool { tx_patches: tx_patches.clone() }));
        tools.register(Arc::new(AdjustThresholdsTool { tx_patches: tx_patches.clone() }));

        let agent_config = AgentConfig {
            max_steps: 16,
            tool_timeout: Duration::from_secs(5),
            max_scratchpad_entries: 50,
        };
        let agent = Agent::new(agent_config, tools, tx_patches);

        let handle = tokio::spawn(async move {
            let planner = Box::new(DynamicAgentPlanner::new());
            let _ = agent.run(rx_events, planner).await;
        });

        (tx_events, rx_patches, handle)
    }

    #[tokio::test]
    async fn test_agent_purge_caches_and_telemetry_flow() {
        let (tx_events, rx_patches, _handle) = setup_test_agent();
        let mut state = AetherWorkstationState::default();

        assert_eq!(state.memory_used, 18.2);
        assert!(state.active_modal.is_none());

        tx_events
            .send(UiEvent::UserPromptSubmitted(
                "Purger les caches et actualiser les flux télémétriques".to_string(),
            ))
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(250)).await;

        while let Ok(patch) = rx_patches.try_recv() {
            state.apply_patch(patch);
        }

        assert_eq!(state.memory_used, 14.6);
        assert_eq!(state.cpu_load, 19.8);
        assert_eq!(state.agent_target.as_deref(), Some("k1_card"));
        assert!(state.diff_preview.as_ref().unwrap().contains("RAM: 18.2G → 14.6G"));
        assert!(state.active_modal.is_some());
        let (title, content) = state.active_modal.unwrap();
        assert_eq!(title, "⚡ Caches Purgés & Télémétrie Optimisée");
        assert!(content.contains("3.6 GB de RAM"));
    }

    #[tokio::test]
    async fn test_agent_restart_process_flow() {
        let (tx_events, rx_patches, _handle) = setup_test_agent();
        let mut state = AetherWorkstationState::default();

        let p_before = state.processes.iter().find(|p| p.pid == 4102).unwrap();
        assert_eq!(p_before.cpu_percent, 18.4);
        assert_eq!(p_before.badge, ListItemBadge::Warning);

        tx_events
            .send(UiEvent::UserPromptSubmitted(
                "Inspecter et redémarrer le processus: ai-inference-engine".to_string(),
            ))
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(250)).await;

        while let Ok(patch) = rx_patches.try_recv() {
            state.apply_patch(patch);
        }

        let p_after = state.processes.iter().find(|p| p.pid == 4102).unwrap();
        assert_eq!(p_after.cpu_percent, 2.4);
        assert_eq!(p_after.badge, ListItemBadge::Success);
        assert_eq!(p_after.status, "Nominal (Restarted)");
        assert_eq!(state.agent_target.as_deref(), Some("proc_table"));
        assert!(state.diff_preview.as_ref().unwrap().contains("ai-inference-engine"));
        assert!(state.active_modal.is_some());
    }

    #[tokio::test]
    async fn test_agent_filter_table_flow() {
        let (tx_events, rx_patches, _handle) = setup_test_agent();
        let mut state = AetherWorkstationState::default();

        assert_eq!(state.search_editor.text, "");

        tx_events
            .send(UiEvent::UserPromptSubmitted(
                "Filtrer les conteneurs et analyser: postgres".to_string(),
            ))
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(250)).await;

        while let Ok(patch) = rx_patches.try_recv() {
            state.apply_patch(patch);
        }

        assert_eq!(state.search_editor.text, "postgres");
        assert_eq!(state.agent_target.as_deref(), Some("proc_search"));
        assert!(state.diff_preview.as_ref().unwrap().contains("postgres"));
    }

    #[tokio::test]
    async fn test_agent_switch_time_range_flow() {
        let (tx_events, rx_patches, _handle) = setup_test_agent();
        let mut state = AetherWorkstationState::default();

        assert_eq!(state.active_range, 0);

        tx_events
            .send(UiEvent::UserPromptSubmitted(
                "Ajuster la fenêtre télémétrique sur 24h".to_string(),
            ))
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(250)).await;

        while let Ok(patch) = rx_patches.try_recv() {
            state.apply_patch(patch);
        }

        assert_eq!(state.active_range, 3);
        assert_eq!(state.agent_target.as_deref(), Some("range_seg"));
    }

    #[tokio::test]
    async fn test_copilot_human_approval_flow() {
        let (tx_events, rx_patches, _handle) = setup_test_agent();
        let mut state = AetherWorkstationState::default();

        // Enable secure human-in-the-loop mode
        state.require_human_approval = true;
        assert!(state.require_human_approval);
        assert!(state.pending_approval_prompt.is_none());

        // When prompt is submitted in secure mode, the prompt is prepared and modal requested
        let prompt_text = "Purger les caches et actualiser les flux télémétriques".to_string();
        state.pending_approval_prompt = Some(prompt_text.clone());
        state.active_modal = Some((
            "🛡️ Confirmation du Plan Copilote".to_string(),
            format!("Plan pour : {}", prompt_text),
        ));

        assert!(state.active_modal.is_some());
        assert_eq!(state.pending_approval_prompt.as_deref(), Some(prompt_text.as_str()));

        // Confirming the modal triggers actual execution
        let confirmed_prompt = state.pending_approval_prompt.take().unwrap();
        state.active_modal = None;
        tx_events.send(UiEvent::UserPromptSubmitted(confirmed_prompt)).await.unwrap();

        tokio::time::sleep(Duration::from_millis(250)).await;

        while let Ok(patch) = rx_patches.try_recv() {
            state.apply_patch(patch);
        }

        assert_eq!(state.memory_used, 14.6);
        assert_eq!(state.cpu_load, 19.8);
        assert!(state.active_modal.is_some());
    }
}
