// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0 | Action: Blade Runner / Cyber-Ops Autonomous Sentinel Workstation (Frameless, Grounded Cards & Precise Alignment)
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use ui_core::UiEvent;
use ui_gpu::{GpuRenderer, MediaInstance, RenderLayer, ResourceTable};
use ui_layout::{
    auto, length, AlignItems, AvailableSpace, FlexDirection, JustifyContent, NodeId, Rect, Size, Style,
};
use ui_widgets::{
    IconKind, InteractionKey, InteractionState, ListItemBadge, Painter, Theme, ToastKind, WidgetId, WidgetTree,
};

use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowAttributes, WindowId};

const WINDOW_WIDTH: f32 = 1380.0;
const WINDOW_HEIGHT: f32 = 880.0;

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
            left: length(14.0),
            right: length(14.0),
            top: length(12.0),
            bottom: length(14.0),
        },
        ..Default::default()
    }
}

fn window_content(gap: f32) -> Style {
    Style {
        flex_direction: FlexDirection::Column,
        gap: Size {
            width: length(0.0),
            height: length(gap),
        },
        padding: Rect {
            left: length(20.0),
            right: length(20.0),
            top: length(12.0),
            bottom: length(16.0),
        },
        ..Default::default()
    }
}

// ============================================================================
// Data Models & State
// ============================================================================

#[derive(Clone, Debug, PartialEq)]
pub enum DefconLevel {
    Defcon5, // Normal
    Defcon4, // Elevated
    Defcon3, // Airborne Alert
    Defcon2, // Red Heuristic
    Defcon1, // Max Alert
}

impl DefconLevel {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Defcon5 => "DEFCON 5 // ROUTINE",
            Self::Defcon4 => "DEFCON 4 // ELEVATED",
            Self::Defcon3 => "DEFCON 3 // AIRBORNE ALERT",
            Self::Defcon2 => "DEFCON 2 // RED HEURISTIC",
            Self::Defcon1 => "DEFCON 1 // IMMINENT BREACH",
        }
    }

    pub fn color(&self) -> [f32; 4] {
        match self {
            Self::Defcon5 => [0.06, 0.72, 0.51, 1.0], // Green
            Self::Defcon4 => [0.0, 0.85, 1.0, 1.0],   // Cyan
            Self::Defcon3 => [0.96, 0.62, 0.04, 1.0], // Amber
            Self::Defcon2 => [0.95, 0.35, 0.20, 1.0], // Orange-Red
            Self::Defcon1 => [1.0, 0.15, 0.25, 1.0],  // Crimson
        }
    }
}

#[derive(Clone, Debug)]
pub struct ThreatIncident {
    pub id: String,
    pub timestamp: String,
    pub source_ip: String,
    pub target_node: String,
    pub attack_type: String,
    pub severity: &'static str,
    pub quarantined: bool,
}

#[derive(Clone, Debug)]
pub struct SentinelAgent {
    pub id: String,
    pub codename: String,
    pub role: String,
    pub model: String,
    pub cpu_usage: f32,
    pub vram_mb: u32,
    pub active: bool,
    pub tasks_executed: u32,
}

pub struct BladeRunnerState {
    pub active_tab: usize,
    pub defcon: DefconLevel,
    pub incidents: Vec<ThreatIncident>,
    pub selected_incident: Option<usize>,
    pub agents: Vec<SentinelAgent>,
    pub table_sort_col: usize,
    pub table_sort_asc: bool,
    pub table_page: usize,
    pub auto_quarantine: bool,
    pub zero_trust_enforced: bool,
    pub anomaly_score: f32,
    pub heuristic_threshold: f32,
    pub radar_sweep_angle: f32,
    pub terminal_input: String,
    pub terminal_cursor: usize,
    pub terminal_logs: VecDeque<(String, [f32; 4])>,
    pub audio_spectrum: Vec<f32>,
    pub gpu_telemetry_series: Vec<f32>,
    pub active_toast: Option<(String, String, ToastKind)>,
    pub show_command_palette: bool,
    pub command_palette_search: String,
    pub command_palette_selected: usize,
    pub show_notifications_drawer: bool,
    pub notifications_history: Vec<(String, String, ToastKind, String)>,
    pub user_avatar_status: ui_widgets::AvatarStatus,
    pub focused_input: Option<String>,
    pub close_requested: bool,
    pub start_time: Instant,
}

impl BladeRunnerState {
    pub fn new() -> Self {
        let mut logs = VecDeque::new();
        logs.push_back(("[SYSTEM] Blade Runner Cyber-Ops Initialized.".to_string(), [0.0, 0.85, 1.0, 1.0]));
        logs.push_back(("[SENTINEL] Nexus-06 Sentinel online. Neural weights verified.".to_string(), [0.06, 0.72, 0.51, 1.0]));
        logs.push_back(("[INGRESS] Ingress filter active on Sector 4 (Tokyo Gateway).".to_string(), [0.6, 0.7, 0.8, 1.0]));
        logs.push_back(("[THREAT-FEED] 4 anomalies detected in last 60 seconds.".to_string(), [0.96, 0.62, 0.04, 1.0]));

        let incidents = vec![
            ThreatIncident {
                id: "INC-8891".to_string(),
                timestamp: "13:24:02".to_string(),
                source_ip: "198.51.100.42".to_string(),
                target_node: "Nexus-Core-01".to_string(),
                attack_type: "Zero-Day Memory Injection".to_string(),
                severity: "CRITICAL",
                quarantined: false,
            },
            ThreatIncident {
                id: "INC-8892".to_string(),
                timestamp: "13:24:19".to_string(),
                source_ip: "203.0.113.88".to_string(),
                target_node: "Auth-Gateway-Paris".to_string(),
                attack_type: "Credential Stuffing / TLS Flood".to_string(),
                severity: "HIGH",
                quarantined: false,
            },
            ThreatIncident {
                id: "INC-8893".to_string(),
                timestamp: "13:24:45".to_string(),
                source_ip: "192.0.2.155".to_string(),
                target_node: "CDN-Relay-Frankfurt".to_string(),
                attack_type: "BGP Route Poisoning Probe".to_string(),
                severity: "WARNING",
                quarantined: true,
            },
            ThreatIncident {
                id: "INC-8894".to_string(),
                timestamp: "13:25:10".to_string(),
                source_ip: "198.18.0.23".to_string(),
                target_node: "Edge-Router-Sydney".to_string(),
                attack_type: "Port Knocking & Banner Grabbing".to_string(),
                severity: "AUDIT",
                quarantined: false,
            },
            ThreatIncident {
                id: "INC-8895".to_string(),
                timestamp: "13:25:31".to_string(),
                source_ip: "203.0.113.99".to_string(),
                target_node: "VRAM-Compute-Grid".to_string(),
                attack_type: "Side-Channel Speculative Leak".to_string(),
                severity: "CRITICAL",
                quarantined: false,
            },
        ];

        let agents = vec![
            SentinelAgent {
                id: "sentinel_deckard".to_string(),
                codename: "Deckard-07".to_string(),
                role: "Heuristic Threat Hunter".to_string(),
                model: "Nexus-9 Autonomous Core".to_string(),
                cpu_usage: 42.5,
                vram_mb: 2048,
                active: true,
                tasks_executed: 1420,
            },
            SentinelAgent {
                id: "sentinel_rachael".to_string(),
                codename: "Rachael-01".to_string(),
                role: "TLS Cryptographic Sentinel".to_string(),
                model: "Tyrell Deep Vault v4".to_string(),
                cpu_usage: 18.2,
                vram_mb: 1024,
                active: true,
                tasks_executed: 893,
            },
            SentinelAgent {
                id: "sentinel_roy".to_string(),
                codename: "Roy-Batty".to_string(),
                role: "Autonomous Counter-Infiltration".to_string(),
                model: "Combat / Ingress Model A".to_string(),
                cpu_usage: 74.8,
                vram_mb: 4096,
                active: true,
                tasks_executed: 3105,
            },
            SentinelAgent {
                id: "sentinel_pris".to_string(),
                codename: "Pris-04".to_string(),
                role: "Vulnerability & Fuzzing Probe".to_string(),
                model: "Adaptive Heuristic SDK".to_string(),
                cpu_usage: 12.0,
                vram_mb: 512,
                active: false,
                tasks_executed: 440,
            },
        ];

        let notifications_history = vec![
            (
                "Zero-Day Neutralized".to_string(),
                "Memory injection attempt blocked at Nexus-Core-01.".to_string(),
                ToastKind::Success,
                "13:24:05".to_string(),
            ),
            (
                "Ingress Spike".to_string(),
                "4.8 Gbps burst detected on Edge Gateway Paris.".to_string(),
                ToastKind::Warning,
                "13:24:22".to_string(),
            ),
            (
                "Sentinel Re-Allocated".to_string(),
                "Roy-Batty assigned to anomalous subnet quarantine.".to_string(),
                ToastKind::Info,
                "13:25:01".to_string(),
            ),
        ];

        let audio_spectrum = (0..32)
            .map(|i| {
                let f = i as f32 / 32.0;
                (f * std::f32::consts::PI * 2.5).sin().abs() * 0.75 + 0.15
            })
            .collect();

        let gpu_telemetry_series = (0..20)
            .map(|i| {
                let t = i as f32;
                30.0 + (t * 0.4).sin() * 20.0 + (t * 0.8).cos() * 10.0
            })
            .collect();

        Self {
            active_tab: 0,
            defcon: DefconLevel::Defcon2,
            incidents,
            selected_incident: Some(0),
            agents,
            table_sort_col: 0,
            table_sort_asc: true,
            table_page: 0,
            auto_quarantine: true,
            zero_trust_enforced: true,
            anomaly_score: 68.4,
            heuristic_threshold: 75.0,
            radar_sweep_angle: 0.0,
            terminal_input: String::new(),
            terminal_cursor: 0,
            terminal_logs: logs,
            audio_spectrum,
            gpu_telemetry_series,
            active_toast: None,
            show_command_palette: false,
            command_palette_search: String::new(),
            command_palette_selected: 0,
            show_notifications_drawer: false,
            notifications_history,
            user_avatar_status: ui_widgets::AvatarStatus::Online,
            focused_input: None,
            close_requested: false,
            start_time: Instant::now(),
        }
    }

    pub fn execute_terminal_cmd(&mut self, cmd: &str) {
        let trimmed = cmd.trim();
        if trimmed.is_empty() {
            return;
        }

        self.terminal_logs.push_back((
            format!("root@blade-runner:~# {}", trimmed),
            [0.0, 0.85, 1.0, 1.0],
        ));

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        match parts[0] {
            "help" => {
                self.terminal_logs.push_back((
                    "Available Commands:".to_string(),
                    [0.6, 0.7, 0.8, 1.0],
                ));
                self.terminal_logs.push_back((
                    "  scan               - Trigger full cluster anomaly diagnostic".to_string(),
                    [0.6, 0.7, 0.8, 1.0],
                ));
                self.terminal_logs.push_back((
                    "  quarantine <id>    - Isolate targeted incident or node".to_string(),
                    [0.6, 0.7, 0.8, 1.0],
                ));
                self.terminal_logs.push_back((
                    "  defcon <1-5>       - Alter workstation defense posture".to_string(),
                    [0.6, 0.7, 0.8, 1.0],
                ));
                self.terminal_logs.push_back((
                    "  mitigate all       - Apply auto-remediation to all open alerts".to_string(),
                    [0.6, 0.7, 0.8, 1.0],
                ));
                self.terminal_logs.push_back((
                    "  status             - Output real-time sentinel fleet telemetry".to_string(),
                    [0.6, 0.7, 0.8, 1.0],
                ));
                self.terminal_logs.push_back((
                    "  clear              - Purge terminal buffer output".to_string(),
                    [0.6, 0.7, 0.8, 1.0],
                ));
            }
            "scan" => {
                self.terminal_logs.push_back((
                    "[SCAN] Scanning 48 edge gateways and 16 compute clusters...".to_string(),
                    [0.0, 0.85, 1.0, 1.0],
                ));
                self.terminal_logs.push_back((
                    "[SCAN] SHA-256 Memory hashes verified. 0 rootkits detected.".to_string(),
                    [0.06, 0.72, 0.51, 1.0],
                ));
                self.active_toast = Some((
                    "Deep Diagnostic".to_string(),
                    "Cluster heuristic scan complete (48 Nodes Nominal).".to_string(),
                    ToastKind::Success,
                ));
            }
            "quarantine" => {
                if parts.len() > 1 {
                    let target = parts[1];
                    let mut found = false;
                    for inc in &mut self.incidents {
                        if inc.id.eq_ignore_ascii_case(target) || inc.source_ip == target {
                            inc.quarantined = true;
                            found = true;
                        }
                    }
                    if found {
                        self.terminal_logs.push_back((
                            format!("[QUARANTINE] Target '{}' successfully isolated.", target),
                            [0.06, 0.72, 0.51, 1.0],
                        ));
                        self.active_toast = Some((
                            "Node Quarantined".to_string(),
                            format!("Ingress traffic for '{}' dropped.", target),
                            ToastKind::Warning,
                        ));
                    } else {
                        self.terminal_logs.push_back((
                            format!("[ERROR] Incident or IP '{}' not found in active table.", target),
                            [1.0, 0.25, 0.25, 1.0],
                        ));
                    }
                } else {
                    self.terminal_logs.push_back((
                        "Usage: quarantine <INC-XXXX | IP_ADDR>".to_string(),
                        [0.96, 0.62, 0.04, 1.0],
                    ));
                }
            }
            "defcon" => {
                if parts.len() > 1 {
                    match parts[1] {
                        "1" => self.defcon = DefconLevel::Defcon1,
                        "2" => self.defcon = DefconLevel::Defcon2,
                        "3" => self.defcon = DefconLevel::Defcon3,
                        "4" => self.defcon = DefconLevel::Defcon4,
                        "5" => self.defcon = DefconLevel::Defcon5,
                        _ => {}
                    }
                    self.terminal_logs.push_back((
                        format!("[DEFCON] Posture switched to {}", self.defcon.name()),
                        self.defcon.color(),
                    ));
                    self.active_toast = Some((
                        "Posture Updated".to_string(),
                        format!("Defense state is now {}", self.defcon.name()),
                        ToastKind::Info,
                    ));
                }
            }
            "mitigate" => {
                for inc in &mut self.incidents {
                    inc.quarantined = true;
                }
                self.terminal_logs.push_back((
                    "[MITIGATION] All active threat vectors neutralized.".to_string(),
                    [0.06, 0.72, 0.51, 1.0],
                ));
                self.active_toast = Some((
                    "Mitigation Complete".to_string(),
                    "All pending threats marked as quarantined.".to_string(),
                    ToastKind::Success,
                ));
            }
            "status" => {
                self.terminal_logs.push_back((
                    format!(
                        "[FLEET STATUS] {} Agents Online | Threat Score: {:.1}% | Defcon: {}",
                        self.agents.iter().filter(|a| a.active).count(),
                        self.anomaly_score,
                        self.defcon.name()
                    ),
                    [0.0, 0.85, 1.0, 1.0],
                ));
            }
            "clear" => {
                self.terminal_logs.clear();
            }
            _ => {
                self.terminal_logs.push_back((
                    format!("[ERROR] Command '{}' unrecognized. Type 'help' for manual.", parts[0]),
                    [1.0, 0.25, 0.25, 1.0],
                ));
            }
        }

        while self.terminal_logs.len() > 100 {
            self.terminal_logs.pop_front();
        }
    }
}

// ============================================================================
// UI Layout & Widget Composition
// ============================================================================

fn build_blade_runner_ui(
    tree: &mut WidgetTree,
    state: &BladeRunnerState,
    width: f32,
    height: f32,
) -> NodeId {
    let content_w = (width - 40.0).max(460.0);
    let half_col_w = ((content_w - 16.0) * 0.5).max(200.0);

    // 0. Top Cyber Titlebar (App Name & Window Trio Controls: Minimize, Maximize, Close)
    let app_title_lbl = tree
        .label("❖ BLADE RUNNER // SENTINEL CYBER-OPS WORKSTATION", leaf(440.0, 24.0))
        .unwrap();

    let btn_min = tree
        .button(
            WidgetId::new("btn_win_minimize"),
            "─",
            true,
            leaf(28.0, 22.0),
        )
        .unwrap();

    let btn_max = tree
        .button(
            WidgetId::new("btn_win_maximize"),
            "□",
            true,
            leaf(28.0, 22.0),
        )
        .unwrap();

    let btn_close = tree
        .button(
            WidgetId::new("btn_win_close"),
            "✕",
            true,
            leaf(28.0, 22.0),
        )
        .unwrap();

    let win_controls = tree.container(&[btn_min, btn_max, btn_close], row(4.0)).unwrap();

    let titlebar = tree
        .container(
            &[app_title_lbl, win_controls],
            Style {
                size: Size {
                    width: length(content_w),
                    height: length(24.0),
                },
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                justify_content: Some(JustifyContent::SpaceBetween),
                ..Default::default()
            },
        )
        .unwrap();

    let title_divider = tree.divider(false, leaf(content_w, 1.0)).unwrap();

    // 1. Top Cyber-Ops Command Bar (Logo, DEFCON indicator, Quick Search, Bell, User Profile)
    let defcon_badge = tree
        .badge(
            state.defcon.name(),
            match state.defcon {
                DefconLevel::Defcon1 => ListItemBadge::Active("CRITICAL".to_string()),
                DefconLevel::Defcon2 => ListItemBadge::Warning,
                DefconLevel::Defcon3 => ListItemBadge::Warning,
                DefconLevel::Defcon4 => ListItemBadge::Active("NOMINAL".to_string()),
                DefconLevel::Defcon5 => ListItemBadge::Success,
            },
            leaf(170.0, 24.0),
        )
        .unwrap();

    let tabs = ["Threat Matrix", "Sentinel Fleet", "Tactical Radar", "Forensic Signals", "Cyber Terminal"];
    let current_tab_name = tabs.get(state.active_tab).unwrap_or(&"Threat Matrix");
    let crumbs = [
        ("nav_ops", "Operations"),
        ("nav_nexus", "Nexus-09"),
        ("nav_sec", *current_tab_name),
    ];
    let breadcrumb_node = tree
        .breadcrumb("blade_breadcrumbs", &crumbs, leaf(72.0, 20.0), row(2.0))
        .unwrap();

    let cmd_btn = tree
        .button(
            WidgetId::new("quick_cmd_btn"),
            "⌕ Commands [Ctrl+K]",
            true,
            leaf(142.0, 24.0),
        )
        .unwrap();

    let bell_btn = tree
        .icon_button(
            "bell_notifications_btn",
            IconKind::Alert,
            true,
            leaf(24.0, 24.0),
        )
        .unwrap();

    let avatar_widget = tree
        .avatar(
            "operator_avatar",
            None::<&str>,
            Some("BR"),
            state.user_avatar_status,
            24.0,
            Some(state.defcon.color()),
            true,
            leaf(24.0, 24.0),
        )
        .unwrap();

    let user_lbl = tree
        .label("Deckard // Op-07", leaf(98.0, 20.0))
        .unwrap();

    let right_header = tree
        .container(&[cmd_btn, bell_btn, avatar_widget, user_lbl], row(8.0))
        .unwrap();

    let top_header = tree
        .container(
            &[defcon_badge, breadcrumb_node, right_header],
            Style {
                size: Size {
                    width: length(content_w),
                    height: length(32.0),
                },
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                justify_content: Some(JustifyContent::SpaceBetween),
                ..Default::default()
            },
        )
        .unwrap();

    let top_divider = tree.divider(false, leaf(content_w, 1.0)).unwrap();

    let tabbar = tree
        .tabbar(
            WidgetId::new("blade_tabs"),
            &tabs,
            state.active_tab,
            leaf(136.0, 28.0),
            row(4.0),
        )
        .unwrap();

    // 2. Tab Content Matrix
    let tab_content = match state.active_tab {
        0 => {
            // --- Tab 0: Threat Matrix (Live Incident Table & Mitigation Surface) ---
            let m_card_w = (content_w - 36.0) / 4.0;
            let m1 = tree
                .metric_card(
                    "Active Threats",
                    format!("{}", state.incidents.iter().filter(|i| !i.quarantined).count()),
                    Some(("+2 New", false)),
                    leaf(m_card_w, 56.0),
                )
                .unwrap();
            let m2 = tree
                .metric_card(
                    "Quarantined Nodes",
                    format!("{}", state.incidents.iter().filter(|i| i.quarantined).count()),
                    Some(("100% Isolated", true)),
                    leaf(m_card_w, 56.0),
                )
                .unwrap();
            let m3 = tree
                .metric_card(
                    "Ingress Anomaly",
                    format!("{:.1}%", state.anomaly_score),
                    Some(("-4.2%", true)),
                    leaf(m_card_w, 56.0),
                )
                .unwrap();
            let m4 = tree
                .metric_card(
                    "Zero-Trust Shield",
                    if state.zero_trust_enforced { "STRICT" } else { "PERMISSIVE" },
                    Some(("Active", true)),
                    leaf(m_card_w, 56.0),
                )
                .unwrap();
            let metrics_grid = tree
                .grid(4, 12.0, 0.0, &[m1, m2, m3, m4], leaf(content_w, 56.0))
                .unwrap();

            // Table of Ingress Incidents
            let col_step = (content_w - 28.0 - 48.0) / 5.0;
            let cols = [
                ("Incident ID", col_step * 0.9, if state.table_sort_col == 0 { Some(state.table_sort_asc) } else { None }),
                ("Timestamp", col_step * 0.8, if state.table_sort_col == 1 { Some(state.table_sort_asc) } else { None }),
                ("Source IP", col_step * 1.1, if state.table_sort_col == 2 { Some(state.table_sort_asc) } else { None }),
                ("Target Node", col_step * 1.1, if state.table_sort_col == 3 { Some(state.table_sort_asc) } else { None }),
                ("Threat Classification", col_step * 1.5, if state.table_sort_col == 4 { Some(state.table_sort_asc) } else { None }),
            ];

            let row_data: Vec<[(&str, ListItemBadge); 5]> = state
                .incidents
                .iter()
                .map(|inc| {
                    let badge = if inc.quarantined {
                        ListItemBadge::Success
                    } else if inc.severity == "CRITICAL" {
                        ListItemBadge::Active("CRITICAL".to_string())
                    } else if inc.severity == "HIGH" {
                        ListItemBadge::Warning
                    } else {
                        ListItemBadge::None
                    };
                    [
                        (inc.id.as_str(), ListItemBadge::None),
                        (inc.timestamp.as_str(), ListItemBadge::None),
                        (inc.source_ip.as_str(), ListItemBadge::None),
                        (inc.target_node.as_str(), ListItemBadge::None),
                        (inc.attack_type.as_str(), badge),
                    ]
                })
                .collect();

            let table = tree
                .table(
                    "incidents_table",
                    &cols,
                    &row_data,
                    state.selected_incident,
                    26.0,
                    leaf(content_w - 28.0, 160.0),
                )
                .unwrap();

            let div_t = tree.divider(false, leaf(content_w - 28.0, 1.0)).unwrap();

            // Actions row: Quarantine Selected, Scan Cluster, Switch Defcon
            let btn_quarantine = tree
                .button(
                    WidgetId::new("btn_quarantine_selected"),
                    "Quarantine Target Node",
                    state.selected_incident.is_some(),
                    leaf(160.0, 28.0),
                )
                .unwrap();

            let btn_mitigate_all = tree
                .button(
                    WidgetId::new("btn_mitigate_all"),
                    "Mitigate All Threats",
                    true,
                    leaf(150.0, 28.0),
                )
                .unwrap();

            let btn_deep_scan = tree
                .button(
                    WidgetId::new("btn_deep_scan"),
                    "Run Neural Diagnostic",
                    true,
                    leaf(150.0, 28.0),
                )
                .unwrap();

            let actions_group = tree
                .container(&[btn_quarantine, btn_mitigate_all, btn_deep_scan], row(8.0))
                .unwrap();

            let pagination = tree
                .pagination("incidents_pages", state.table_page, 3, leaf(32.0, 26.0), row(4.0))
                .unwrap();

            let table_footer = tree
                .container(
                    &[actions_group, pagination],
                    Style {
                        size: Size {
                            width: length(content_w - 28.0),
                            height: length(28.0),
                        },
                        flex_direction: FlexDirection::Row,
                        align_items: Some(AlignItems::Center),
                        justify_content: Some(JustifyContent::SpaceBetween),
                        ..Default::default()
                    },
                )
                .unwrap();

            let table_card = tree
                .card(
                    &[table, div_t, table_footer],
                    None,
                    None,
                    Some(8.0),
                    card_style(content_w),
                )
                .unwrap();

            tree.container(&[metrics_grid, table_card], column(12.0)).unwrap()
        }
        1 => {
            // --- Tab 1: Sentinel Fleet (Autonomous Agents Matrix) ---
            let mut agent_cards = Vec::new();
            let card_w = (content_w - 16.0) * 0.5;

            for agent in &state.agents {
                let title = tree
                    .label(format!("⬡ {} [{}]", agent.codename, agent.model), leaf(card_w - 28.0, 18.0))
                    .unwrap();
                let sub = tree
                    .label_muted(format!("Role: {} | Executed: {} tasks", agent.role, agent.tasks_executed), leaf(card_w - 28.0, 14.0))
                    .unwrap();

                let cpu_p = tree
                    .progress_bar(agent.cpu_usage / 100.0, leaf(card_w - 28.0, 6.0))
                    .unwrap();
                let cpu_lbl = tree
                    .label_muted(format!("CPU Utilization: {:.1}% | VRAM: {} MB", agent.cpu_usage, agent.vram_mb), leaf(card_w - 28.0, 14.0))
                    .unwrap();

                let toggle = tree
                    .toggle(
                        WidgetId::new(format!("toggle_{}", agent.id)),
                        agent.active,
                        leaf(36.0, 18.0),
                    )
                    .unwrap();
                let toggle_lbl = tree
                    .label(if agent.active { "Sentinel Operational (Active Ingress Defense)" } else { "Sentinel Offline (Standby Mode)" }, leaf(card_w - 80.0, 18.0))
                    .unwrap();
                let toggle_row = tree.container(&[toggle, toggle_lbl], row(8.0)).unwrap();

                let acard = tree
                    .card(
                        &[title, sub, cpu_p, cpu_lbl, toggle_row],
                        None,
                        None,
                        Some(6.0),
                        card_style(card_w),
                    )
                    .unwrap();
                agent_cards.push(acard);
            }

            let agents_grid = tree
                .grid(2, 16.0, 12.0, &agent_cards, leaf(content_w, 240.0))
                .unwrap();

            let banner_title = tree
                .label("Autonomous Replicant Orchestration Policy", leaf(content_w - 28.0, 18.0))
                .unwrap();
            let banner_sub = tree
                .label_muted("All autonomous agents operate under Zero-Trust human confirmation protocols. Escalation requires master key approval.", leaf(content_w - 28.0, 14.0))
                .unwrap();
            let banner_card = tree
                .card(
                    &[banner_title, banner_sub],
                    None,
                    None,
                    Some(6.0),
                    card_style(content_w),
                )
                .unwrap();

            tree.container(&[agents_grid, banner_card], column(12.0)).unwrap()
        }
        2 => {
            // --- Tab 2: Tactical Radar (2D CustomPaint Vector Map) ---
            let radar_w = content_w - 28.0;
            let radar_h = 320.0;
            let cx = radar_w * 0.5;
            let cy = radar_h * 0.5;

            let mut painter = Painter::new();

            // 1. Concentric Range Rings
            for r in [40.0, 80.0, 120.0, 150.0] {
                painter.circle([cx, cy], r, None, Some(([0.0, 0.85, 1.0, 0.15], 1.0)));
            }

            // 2. Axis Crosshairs
            painter.line([cx - 150.0, cy], [cx + 150.0, cy], 1.0, [0.0, 0.85, 1.0, 0.2]);
            painter.line([cx, cy - 150.0], [cx, cy + 150.0], 1.0, [0.0, 0.85, 1.0, 0.2]);

            // 3. Rotating Sweep Radar Line
            let sweep_rad = state.radar_sweep_angle.to_radians();
            let sx = cx + sweep_rad.cos() * 150.0;
            let sy = cy + sweep_rad.sin() * 150.0;
            painter.line([cx, cy], [sx, sy], 2.0, [0.06, 0.72, 0.51, 0.8]);

            // 4. Sector Node Blips
            let nodes = [
                ([cx + 60.0, cy - 40.0], "Tokyo Core", [0.0, 0.85, 1.0, 1.0]),
                ([cx - 80.0, cy - 30.0], "Paris Gateway", [0.55, 0.36, 0.96, 1.0]),
                ([cx - 40.0, cy + 90.0], "Frankfurt Relay", [0.06, 0.72, 0.51, 1.0]),
                ([cx + 100.0, cy + 60.0], "Sydney Edge", [0.96, 0.62, 0.04, 1.0]),
            ];

            for (pos, name, col) in nodes {
                painter.circle(pos, 6.0, Some(col), Some(([1.0, 1.0, 1.0, 0.8], 1.5)));
                painter.text([pos[0] + 10.0, pos[1] - 4.0], name.to_string(), 11.0, col);
            }

            // 5. Threat Vector Spline
            painter.bezier(
                [cx - 80.0, cy - 30.0],
                [cx - 20.0, cy - 80.0],
                [cx + 20.0, cy - 10.0],
                [cx + 60.0, cy - 40.0],
                2.5,
                [1.0, 0.25, 0.25, 0.7],
            );

            // 6. HUD Overlay
            painter.text([12.0, 16.0], "TACTICAL SECTOR RADAR // 360° SURVEILLANCE".to_string(), 12.0, [0.0, 0.85, 1.0, 0.8]);
            painter.text([12.0, radar_h - 18.0], format!("SWEEP: {:.0}° | ACTIVE TARGETS: 4 | DEFCON: {}", state.radar_sweep_angle, state.defcon.name()), 11.0, [0.6, 0.7, 0.8, 0.8]);

            let radar_widget = tree
                .custom_paint("tactical_radar_canvas", painter.finish(), leaf(radar_w, radar_h))
                .unwrap();

            let radar_card = tree
                .card(
                    &[radar_widget],
                    None,
                    None,
                    Some(6.0),
                    card_style(content_w),
                )
                .unwrap();

            tree.container(&[radar_card], column(12.0)).unwrap()
        }
        3 => {
            // --- Tab 3: Forensic Signals (GPU Spline Chart & 32-Band FFT Audio Spectrum) ---
            let left_title = tree
                .label("Realtime Ingress Telemetry (Area / Spline Dataviz)", leaf(half_col_w - 28.0, 18.0))
                .unwrap();

            let pts1: Vec<[f32; 2]> = state
                .gpu_telemetry_series
                .iter()
                .enumerate()
                .map(|(i, v)| [i as f32, *v])
                .collect();

            let chart_series = vec![
                ui_widgets::ChartSeries {
                    name: "Ingress Anomaly Rate".to_string(),
                    color: [0.0, 0.85, 1.0, 1.0],
                    points: pts1,
                    filled: true,
                },
            ];

            let chart_w = tree
                .time_series_chart(
                    "forensic_chart",
                    Some("Anomalous Packet Volume (Gbps)"),
                    chart_series,
                    (0.0, 19.0),
                    (0.0, 100.0),
                    true,
                    true,
                    None,
                    leaf(half_col_w - 28.0, 180.0),
                )
                .unwrap();

            let card_left = tree
                .card(
                    &[left_title, chart_w],
                    None,
                    None,
                    Some(8.0),
                    card_style(half_col_w),
                )
                .unwrap();

            let right_title = tree
                .label("Covert Ultrasonic Signal Spectrogram (32 Band FFT)", leaf(half_col_w - 28.0, 18.0))
                .unwrap();
            let right_sub = tree
                .label_muted("Hardware audio buffer DSP analysis for acoustic air-gap exfiltration probes", leaf(half_col_w - 28.0, 14.0))
                .unwrap();

            let visualizer = tree
                .audio_visualizer("covert_audio_fft", &state.audio_spectrum, 1.0, leaf(half_col_w - 28.0, 136.0))
                .unwrap();

            let card_right = tree
                .card(
                    &[right_title, right_sub, visualizer],
                    None,
                    None,
                    Some(8.0),
                    card_style(half_col_w),
                )
                .unwrap();

            tree.container(&[card_left, card_right], row(16.0)).unwrap()
        }
        _ => {
            // --- Tab 4: Cyber Terminal (Live Forensic Log & Interactive Command Shell) ---
            let term_title = tree
                .label("Cybernetic Forensic Console // root@blade-runner", leaf(content_w - 28.0, 18.0))
                .unwrap();

            let mut log_nodes = Vec::new();
            for (line, _color) in state.terminal_logs.iter().rev().take(10).rev() {
                let lbl = tree
                    .label_muted(line.as_str(), leaf(content_w - 48.0, 18.0))
                    .unwrap();
                log_nodes.push(lbl);
            }

            let logs_container = tree
                .container(&log_nodes, column(2.0))
                .unwrap();

            let div_term = tree.divider(false, leaf(content_w - 28.0, 1.0)).unwrap();

            let term_prompt = tree
                .label("root@blade-runner:~#", leaf(140.0, 26.0))
                .unwrap();

            let input_focused = state.focused_input.as_deref() == Some("terminal_input_field");
            let term_input = tree
                .text_input_with_cursor(
                    WidgetId::new("terminal_input_field"),
                    &state.terminal_input,
                    "Type 'help', 'scan', 'quarantine <id>', 'defcon <1-5>'...",
                    input_focused,
                    state.terminal_cursor,
                    None,
                    leaf(content_w - 28.0 - 240.0, 26.0),
                )
                .unwrap();

            let btn_exec = tree
                .button(
                    WidgetId::new("btn_exec_terminal"),
                    "Execute ↵",
                    true,
                    leaf(80.0, 26.0),
                )
                .unwrap();

            let input_row = tree
                .container(&[term_prompt, term_input, btn_exec], row(6.0))
                .unwrap();

            let term_card = tree
                .card(
                    &[term_title, logs_container, div_term, input_row],
                    None,
                    None,
                    Some(8.0),
                    card_style(content_w),
                )
                .unwrap();

            tree.container(&[term_card], column(12.0)).unwrap()
        }
    };

    // 3. Assemble Root Application Workspace (Full bleed, seamless window canvas)
    tree.container(
        &[
            titlebar,
            title_divider,
            top_header,
            top_divider,
            tabbar,
            tab_content,
        ],
        Style {
            size: Size {
                width: length(width),
                height: length(height),
            },
            flex_direction: FlexDirection::Column,
            gap: Size {
                width: length(0.0),
                height: length(8.0),
            },
            padding: Rect {
                left: length(20.0),
                right: length(20.0),
                top: length(10.0),
                bottom: length(16.0),
            },
            ..Default::default()
        },
    )
    .unwrap()
}

// ============================================================================
// Spotlight Command Palette Overlay
// ============================================================================

fn build_command_palette_ui(
    tree: &mut WidgetTree,
    state: &BladeRunnerState,
    width: f32,
    height: f32,
) -> NodeId {
    let dialog_w = 540.0;

    let search_input = tree
        .text_input_with_cursor(
            WidgetId::new("command_palette_search"),
            &state.command_palette_search,
            "Type a command or action...",
            true,
            state.command_palette_search.len(),
            None,
            leaf(dialog_w - 78.0, 28.0),
        )
        .unwrap();

    let esc_badge = tree
        .kbd("Esc", leaf(34.0, 22.0))
        .unwrap();

    let search_row = tree
        .container(&[search_input, esc_badge], row(6.0))
        .unwrap();

    let div = tree.divider(false, leaf(dialog_w - 28.0, 1.0)).unwrap();

    let commands = [
        ("cmd_action_quarantine_all", "Quarantine All Critical Threats", "Isolate all open threat vectors immediately"),
        ("cmd_action_defcon_1", "Switch to DEFCON 1 // Max Alert", "Activate automated emergency countermeasures"),
        ("cmd_action_scan", "Trigger Neural Memory Scan", "Audit all node buffers and TLS keys"),
        ("cmd_action_toggle_deckard", "Toggle Deckard-07 Sentinel", "Activate / Standby primary hunter"),
        ("cmd_action_clear_terminal", "Purge Forensic Terminal Logs", "Reset console memory buffer"),
    ];

    let mut action_nodes = Vec::new();
    for (idx, (action_id, title, desc)) in commands.iter().enumerate() {
        let is_sel = state.command_palette_selected == idx;
        let btn = tree
            .button(
                WidgetId::new(*action_id),
                format!("{} — {}", title, desc),
                true,
                leaf(dialog_w - 28.0, 28.0),
            )
            .unwrap();
        let _ = is_sel;
        action_nodes.push(btn);
    }

    let actions_col = tree.container(&action_nodes, column(4.0)).unwrap();

    let dialog = tree
        .card(
            &[search_row, div, actions_col],
            None,
            None,
            Some(8.0),
            Style {
                size: Size {
                    width: length(dialog_w),
                    height: auto(),
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
                position: ui_layout::Position::Absolute,
                inset: Rect {
                    left: length((width - dialog_w) * 0.5),
                    top: length(120.0),
                    right: auto(),
                    bottom: auto(),
                },
                ..Default::default()
            },
        )
        .unwrap();

    tree.container(&[dialog], leaf(width, height)).unwrap()
}

// ============================================================================
// Notifications Drawer Overlay
// ============================================================================

fn build_notifications_drawer_ui(
    tree: &mut WidgetTree,
    state: &BladeRunnerState,
    width: f32,
    _height: f32,
) -> NodeId {
    let drawer_w = 340.0;
    let drawer_h = 420.0;

    let drawer_title = tree
        .label("Incident Notifications", leaf(drawer_w - 60.0, 22.0))
        .unwrap();
    let close_btn = tree
        .button(
            WidgetId::new("close_drawer_btn"),
            "✕",
            true,
            leaf(24.0, 22.0),
        )
        .unwrap();
    let header_row = tree
        .container(
            &[drawer_title, close_btn],
            Style {
                size: Size {
                    width: length(drawer_w - 28.0),
                    height: length(24.0),
                },
                flex_direction: FlexDirection::Row,
                align_items: Some(AlignItems::Center),
                justify_content: Some(JustifyContent::SpaceBetween),
                ..Default::default()
            },
        )
        .unwrap();

    let div = tree.divider(false, leaf(drawer_w - 28.0, 1.0)).unwrap();

    let mut notif_items = Vec::new();
    for (title, msg, kind, time) in &state.notifications_history {
        let t_lbl = tree.label(format!("{} [{}]", title, time), leaf(drawer_w - 28.0, 16.0)).unwrap();
        let m_lbl = tree.label_muted(msg.as_str(), leaf(drawer_w - 28.0, 14.0)).unwrap();
        let b_kind = match kind {
            ToastKind::Success => ListItemBadge::Success,
            ToastKind::Warning => ListItemBadge::Warning,
            ToastKind::Info => ListItemBadge::None,
            ToastKind::Error => ListItemBadge::Active("ERROR".to_string()),
        };
        let badge = tree.badge(match kind {
            ToastKind::Success => "RESOLVED",
            ToastKind::Warning => "ALERT",
            ToastKind::Info => "TELEMETRY",
            ToastKind::Error => "CRITICAL",
        }, b_kind, leaf(80.0, 20.0)).unwrap();

        let n_card = tree.container(&[t_lbl, m_lbl, badge], column(4.0)).unwrap();
        notif_items.push(n_card);
    }

    let items_col = tree.container(&notif_items, column(8.0)).unwrap();

    let drawer = tree
        .card(
            &[header_row, div, items_col],
            None,
            None,
            Some(8.0),
            Style {
                size: Size {
                    width: length(drawer_w),
                    height: length(drawer_h),
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
                position: ui_layout::Position::Absolute,
                inset: Rect {
                    left: length(width - drawer_w - 24.0),
                    top: length(56.0),
                    right: auto(),
                    bottom: auto(),
                },
                ..Default::default()
            },
        )
        .unwrap();

    tree.container(&[drawer], leaf(width, _height)).unwrap()
}

// ============================================================================
// Winit Application Handler
// ============================================================================

struct App {
    window: Option<Arc<Window>>,
    renderer: Option<GpuRenderer>,
    tree: WidgetTree,
    root: Option<NodeId>,
    overlay_tree: Option<WidgetTree>,
    overlay_root: Option<NodeId>,
    state: BladeRunnerState,
    cursor_pos: (f32, f32),
    pressed: Option<InteractionKey>,
    theme: Arc<RwLock<Theme>>,
    resources: ResourceTable,
}

impl App {
    pub fn new() -> Self {
        Self {
            window: None,
            renderer: None,
            tree: WidgetTree::new(),
            root: None,
            overlay_tree: None,
            overlay_root: None,
            state: BladeRunnerState::new(),
            cursor_pos: (0.0, 0.0),
            pressed: None,
            theme: Arc::new(RwLock::new(Theme::default())),
            resources: ResourceTable::new(),
        }
    }

    fn handle_ui_event(&mut self, event: UiEvent) {
        match event {
            UiEvent::TabSelected { tab_index, .. } => {
                self.state.active_tab = tab_index;
            }
            UiEvent::TableRowSelected { row_index, .. } => {
                self.state.selected_incident = Some(row_index);
            }
            UiEvent::TableHeaderClicked { column_index, .. } => {
                if self.state.table_sort_col == column_index {
                    self.state.table_sort_asc = !self.state.table_sort_asc;
                } else {
                    self.state.table_sort_col = column_index;
                    self.state.table_sort_asc = true;
                }
            }
            UiEvent::PageSelected { page, .. } => {
                self.state.table_page = page;
            }
            UiEvent::ToggleSwitched { widget_id, active, .. } => {
                if widget_id == "toggle_sentinel_deckard" {
                    if let Some(a) = self.state.agents.iter_mut().find(|a| a.id == "sentinel_deckard") {
                        a.active = active;
                    }
                } else if widget_id == "toggle_sentinel_rachael" {
                    if let Some(a) = self.state.agents.iter_mut().find(|a| a.id == "sentinel_rachael") {
                        a.active = active;
                    }
                } else if widget_id == "toggle_sentinel_roy" {
                    if let Some(a) = self.state.agents.iter_mut().find(|a| a.id == "sentinel_roy") {
                        a.active = active;
                    }
                } else if widget_id == "toggle_sentinel_pris" {
                    if let Some(a) = self.state.agents.iter_mut().find(|a| a.id == "sentinel_pris") {
                        a.active = active;
                    }
                }
            }
            UiEvent::ButtonClicked { widget_id } => {
                if widget_id == "btn_win_minimize" {
                    if let Some(w) = &self.window {
                        w.set_minimized(true);
                    }
                } else if widget_id == "btn_win_maximize" {
                    if let Some(w) = &self.window {
                        let is_max = w.is_maximized();
                        w.set_maximized(!is_max);
                    }
                } else if widget_id == "btn_win_close" {
                    self.state.close_requested = true;
                } else if widget_id == "quick_cmd_btn" {
                    self.state.show_command_palette = !self.state.show_command_palette;
                } else if widget_id == "bell_notifications_btn" {
                    self.state.show_notifications_drawer = !self.state.show_notifications_drawer;
                } else if widget_id == "close_drawer_btn" {
                    self.state.show_notifications_drawer = false;
                } else if widget_id == "btn_quarantine_selected" {
                    if let Some(sel) = self.state.selected_incident {
                        if let Some(inc) = self.state.incidents.get_mut(sel) {
                            inc.quarantined = true;
                            self.state.active_toast = Some((
                                "Target Isolated".to_string(),
                                format!("Node '{}' isolated from network.", inc.target_node),
                                ToastKind::Warning,
                            ));
                        }
                    }
                } else if widget_id == "btn_mitigate_all" {
                    for inc in &mut self.state.incidents {
                        inc.quarantined = true;
                    }
                    self.state.active_toast = Some((
                        "All Mitigated".to_string(),
                        "All active threat vectors neutralized.".to_string(),
                        ToastKind::Success,
                    ));
                } else if widget_id == "btn_deep_scan" {
                    self.state.execute_terminal_cmd("scan");
                } else if widget_id == "btn_exec_terminal" {
                    let cmd = self.state.terminal_input.clone();
                    self.state.terminal_input.clear();
                    self.state.terminal_cursor = 0;
                    self.state.execute_terminal_cmd(&cmd);
                } else if let Some(action) = widget_id.strip_prefix("cmd_action_") {
                    self.state.show_command_palette = false;
                    match action {
                        "quarantine_all" => {
                            for inc in &mut self.state.incidents {
                                inc.quarantined = true;
                            }
                            self.state.active_toast = Some((
                                "Emergency Quarantine".to_string(),
                                "All active endpoints quarantined.".to_string(),
                                ToastKind::Warning,
                            ));
                        }
                        "defcon_1" => {
                            self.state.defcon = DefconLevel::Defcon1;
                            self.state.active_toast = Some((
                                "DEFCON 1 Active".to_string(),
                                "Emergency defense posture engaged.".to_string(),
                                ToastKind::Warning,
                            ));
                        }
                        "scan" => self.state.execute_terminal_cmd("scan"),
                        "toggle_deckard" => {
                            if let Some(a) = self.state.agents.iter_mut().find(|a| a.id == "sentinel_deckard") {
                                a.active = !a.active;
                            }
                        }
                        "clear_terminal" => self.state.terminal_logs.clear(),
                        _ => {}
                    }
                }
            }
            UiEvent::FocusChanged { widget_id } => {
                self.state.focused_input = widget_id;
            }
            UiEvent::WindowCloseRequested { .. } => {
                self.state.close_requested = true;
            }
            UiEvent::ToastDismissed { .. } => {
                self.state.active_toast = None;
            }
            _ => {}
        }
    }

    fn handle_press(&mut self) {
        if let (Some(o_tree), Some(o_root)) = (&self.overlay_tree, self.overlay_root) {
            if let Ok(Some(key)) = o_tree.interaction_key_at(o_root, self.cursor_pos) {
                self.pressed = Some(key);
                return;
            }
        }

        let Some(root) = self.root else { return };
        self.pressed = self.tree.interaction_key_at(root, self.cursor_pos).unwrap_or(None);

        if self.cursor_pos.1 <= 44.0 && self.pressed.is_none() {
            if let Some(w) = &self.window {
                let _ = w.drag_window();
            }
        }
    }

    fn handle_release(&mut self) {
        if let (Some(o_tree), Some(o_root)) = (&self.overlay_tree, self.overlay_root) {
            let released_on = o_tree.interaction_key_at(o_root, self.cursor_pos).unwrap_or(None);
            if self.pressed.is_some() && self.pressed == released_on {
                if let Ok(Some(ev)) = o_tree.dispatch_click(o_root, self.cursor_pos) {
                    self.handle_ui_event(ev);
                    self.pressed = None;
                    return;
                }
            }
        }

        if let Some(root) = self.root {
            let released_on = self.tree.interaction_key_at(root, self.cursor_pos).unwrap_or(None);
            if self.pressed.is_some() && self.pressed == released_on {
                if let Ok(Some(ev)) = self.tree.dispatch_click(root, self.cursor_pos) {
                    self.handle_ui_event(ev);
                }
            }
        }
        self.pressed = None;
    }

    fn redraw(&mut self) {
        let Some(window) = &self.window else { return };
        let Some(renderer) = &mut self.renderer else { return };

        let elapsed = self.state.start_time.elapsed().as_secs_f32();
        self.state.radar_sweep_angle = (elapsed * 80.0) % 360.0;

        // Live smooth FFT audio spectrum animation
        for (i, val) in self.state.audio_spectrum.iter_mut().enumerate() {
            let f = i as f32 / 32.0;
            let wave = (elapsed * 3.0 + f * 12.0).sin().abs();
            *val = (f * std::f32::consts::PI * 2.5).sin().abs() * 0.4 + wave * 0.45 + 0.15;
        }

        // Live GPU telemetry waveform
        for (i, val) in self.state.gpu_telemetry_series.iter_mut().enumerate() {
            let t = elapsed * 2.0 + i as f32 * 0.5;
            *val = 32.0 + (t * 0.6).sin() * 18.0 + (t * 1.2).cos() * 8.0;
        }

        let size = window.inner_size();
        let w = size.width as f32;
        let h = size.height as f32;

        let available = Size {
            width: AvailableSpace::Definite(w),
            height: AvailableSpace::Definite(h),
        };

        let current_theme = self.theme.read().unwrap().clone();
        let measure = renderer.text_measure();

        // 1. Build Base Tree
        let mut base_tree = WidgetTree::new();
        let base_root = build_blade_runner_ui(&mut base_tree, &self.state, w, h);
        if let Err(err) = base_tree.compute(base_root, available) {
            eprintln!("[blade_runner] base compute failed: {err}");
            return;
        }

        let base_hovered = base_tree.interaction_key_at(base_root, self.cursor_pos).unwrap_or(None);
        let base_interaction = InteractionState {
            hovered: base_hovered.as_ref(),
            pressed: self.pressed.as_ref(),
            measure: Some(&measure),
        };

        let base_frame = match base_tree.build_frame(base_root, &current_theme, base_interaction) {
            Ok(f) => f,
            Err(err) => {
                eprintln!("[blade_runner] base build_frame failed: {err}");
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

        if self.state.show_command_palette {
            let mut overlay_tree = WidgetTree::new();
            let overlay_root = build_command_palette_ui(&mut overlay_tree, &self.state, w, h);
            if overlay_tree.compute(overlay_root, available).is_ok() {
                let pop_hovered = overlay_tree.interaction_key_at(overlay_root, self.cursor_pos).unwrap_or(None);
                let pop_interaction = InteractionState {
                    hovered: pop_hovered.as_ref(),
                    pressed: self.pressed.as_ref(),
                    measure: Some(&measure),
                };
                if let Ok(pop_frame) = overlay_tree.build_frame(overlay_root, &current_theme, pop_interaction) {
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

                    let pop_layer = RenderLayer {
                        instances: &pop_frame.instances,
                        texts: &pop_text_runs,
                    };

                    let _ = renderer.render_layers(
                        background,
                        &[base_layer, pop_layer],
                        &media,
                        &self.resources,
                    );
                }
            }
            self.overlay_tree = Some(overlay_tree);
            self.overlay_root = Some(overlay_root);
        } else if self.state.show_notifications_drawer {
            let mut overlay_tree = WidgetTree::new();
            let overlay_root = build_notifications_drawer_ui(&mut overlay_tree, &self.state, w, h);
            if overlay_tree.compute(overlay_root, available).is_ok() {
                let pop_hovered = overlay_tree.interaction_key_at(overlay_root, self.cursor_pos).unwrap_or(None);
                let pop_interaction = InteractionState {
                    hovered: pop_hovered.as_ref(),
                    pressed: self.pressed.as_ref(),
                    measure: Some(&measure),
                };
                if let Ok(pop_frame) = overlay_tree.build_frame(overlay_root, &current_theme, pop_interaction) {
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

                    let pop_layer = RenderLayer {
                        instances: &pop_frame.instances,
                        texts: &pop_text_runs,
                    };

                    let _ = renderer.render_layers(
                        background,
                        &[base_layer, pop_layer],
                        &media,
                        &self.resources,
                    );
                }
            }
            self.overlay_tree = Some(overlay_tree);
            self.overlay_root = Some(overlay_root);
        } else {
            let _ = renderer.render_layers(
                background,
                &[base_layer],
                &media,
                &self.resources,
            );
            self.overlay_tree = None;
            self.overlay_root = None;
        }

        self.tree = base_tree;
        self.root = Some(base_root);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = WindowAttributes::default()
            .with_title("Blade Runner // Sentinel Cyber-Ops Workstation")
            .with_decorations(false)
            .with_transparent(true)
            .with_inner_size(winit::dpi::PhysicalSize::new(
                WINDOW_WIDTH as u32,
                WINDOW_HEIGHT as u32,
            ));
        let window = Arc::new(event_loop.create_window(attrs).unwrap());
        let renderer = GpuRenderer::new(window.clone());
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
                        logical_key,
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                match logical_key {
                    Key::Named(NamedKey::Escape) => {
                        if self.state.show_command_palette {
                            self.state.show_command_palette = false;
                        } else if self.state.show_notifications_drawer {
                            self.state.show_notifications_drawer = false;
                        }
                    }
                    Key::Named(NamedKey::Enter) => {
                        if self.state.focused_input.as_deref() == Some("terminal_input_field") && !self.state.terminal_input.is_empty() {
                            let cmd = self.state.terminal_input.clone();
                            self.state.terminal_input.clear();
                            self.state.terminal_cursor = 0;
                            self.state.execute_terminal_cmd(&cmd);
                        }
                    }
                    Key::Named(NamedKey::Backspace) => {
                        if self.state.focused_input.as_deref() == Some("terminal_input_field") && self.state.terminal_cursor > 0 {
                            self.state.terminal_cursor -= 1;
                            self.state.terminal_input.remove(self.state.terminal_cursor);
                        }
                    }
                    Key::Character(s) => {
                        if self.state.focused_input.as_deref() == Some("terminal_input_field") {
                            self.state.terminal_input.insert_str(self.state.terminal_cursor, s.as_str());
                            self.state.terminal_cursor += s.len();
                        }
                    }
                    _ => {}
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
    println!("[blade_runner] Starting Blade Runner Sentinel Cyber-Ops Center...");
    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}

// ============================================================================
// Deterministic Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blade_runner_initial_state() {
        let state = BladeRunnerState::new();
        assert_eq!(state.defcon, DefconLevel::Defcon2);
        assert_eq!(state.incidents.len(), 5);
        assert_eq!(state.agents.len(), 4);
        assert!(state.terminal_logs.len() >= 4);
    }

    #[test]
    fn test_terminal_command_execution() {
        let mut state = BladeRunnerState::new();

        state.execute_terminal_cmd("defcon 1");
        assert_eq!(state.defcon, DefconLevel::Defcon1);

        state.execute_terminal_cmd("quarantine INC-8891");
        assert!(state.incidents.iter().find(|i| i.id == "INC-8891").unwrap().quarantined);

        state.execute_terminal_cmd("mitigate all");
        for inc in &state.incidents {
            assert!(inc.quarantined);
        }
    }

    #[test]
    fn test_layout_generation() {
        let mut tree = WidgetTree::new();
        let state = BladeRunnerState::new();
        let root = build_blade_runner_ui(&mut tree, &state, 1380.0, 880.0);
        let available = Size {
            width: AvailableSpace::Definite(1380.0),
            height: AvailableSpace::Definite(880.0),
        };
        let layout_res = tree.compute(root, available);
        assert!(layout_res.is_ok());
    }
}
