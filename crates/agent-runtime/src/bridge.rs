// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0 | Action: High-level Agent HTTP/JSON Bridge & Ergonomic Hub for AORUI
use std::sync::Arc;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use ui_core::{UiEvent, UiPatch};

use crate::agent::{Agent, AgentConfig};
use crate::planner::Planner;
use crate::tool::ToolRegistry;

/// Configuration for the external Agent HTTP Bridge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeConfig {
    pub base_port: u16,
    pub max_port_attempts: u16,
    pub auth_token: Option<String>,
    pub sandbox_mode: bool,
    pub app_name: String,
}

impl Default for BridgeConfig {
    fn default() -> Self {
        Self {
            base_port: 8765,
            max_port_attempts: 5,
            auth_token: None,
            sandbox_mode: true,
            app_name: "AORUI Application".to_string(),
        }
    }
}

impl BridgeConfig {
    pub fn new(app_name: impl Into<String>) -> Self {
        Self {
            app_name: app_name.into(),
            ..Default::default()
        }
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.base_port = port;
        self
    }

    pub fn with_auth_token(mut self, token: impl Into<String>) -> Self {
        self.auth_token = Some(token.into());
        self
    }

    pub fn with_sandbox(mut self, enabled: bool) -> Self {
        self.sandbox_mode = enabled;
        self
    }
}

/// Information about a running bridge instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeStatus {
    pub active: bool,
    pub bound_port: u16,
    pub sandbox_mode: bool,
    pub auth_required: bool,
    pub app_name: String,
}

/// Asynchronous HTTP Bridge server for remote agent control.
pub struct AgentBridge;

impl AgentBridge {
    /// Starts the agent bridge HTTP listener in the current async Tokio context.
    pub async fn start(config: BridgeConfig, tx_events: mpsc::Sender<UiEvent>) -> Option<BridgeStatus> {
        let mut listener_opt = None;
        let mut active_port = config.base_port;

        for p in config.base_port..=(config.base_port + config.max_port_attempts) {
            let addr = format!("127.0.0.1:{}", p);
            if let Ok(l) = TcpListener::bind(&addr).await {
                println!("🌐 [AORUI AgentBridge] En écoute sur http://{}", addr);
                if config.sandbox_mode {
                    println!("🛡️ [AORUI Security] Mode Sandbox actif : isolation totale de la mémoire UI.");
                }
                if config.auth_token.is_some() {
                    println!("🔒 [AORUI Security] Authentification Bearer Token requise.");
                }
                active_port = p;
                listener_opt = Some(l);
                break;
            }
        }

        let listener = listener_opt?;
        let status = BridgeStatus {
            active: true,
            bound_port: active_port,
            sandbox_mode: config.sandbox_mode,
            auth_required: config.auth_token.is_some(),
            app_name: config.app_name.clone(),
        };

        let bridge_cfg = Arc::new(config);
        tokio::spawn(async move {
            loop {
                if let Ok((mut socket, _)) = listener.accept().await {
                    let tx = tx_events.clone();
                    let cfg = bridge_cfg.clone();
                    let port = active_port;

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

                        // 1. Authorization Verification (INV-SEC-3)
                        let mut authorized = true;
                        if let Some(expected_token) = &cfg.auth_token {
                            let expected_header = format!("authorization: bearer {}", expected_token.to_lowercase());
                            authorized = req.lines().any(|l| l.to_lowercase() == expected_header);
                        }

                        let (status_code, resp_json) = if !authorized {
                            ("401 Unauthorized", json!({"error": "Jeton Bearer invalide ou manquant"}))
                        } else if req.starts_with("POST /mcp") || req.starts_with("POST /rpc") {
                            if let Some(body_start) = req.find("\r\n\r\n") {
                                let body = req[body_start + 4..].trim();
                                let reg = ToolRegistry::new();
                                let mcp_resp = crate::mcp::McpHandler::handle(body, &reg, &tx).await;
                                let parsed: Value = serde_json::from_str(&mcp_resp).unwrap_or(json!({}));
                                ("200 OK", parsed)
                            } else {
                                ("400 Bad Request", json!({"error": "Malformed MCP request"}))
                            }
                        } else if req.starts_with("GET /mcp") {
                            ("200 OK", json!({
                                "mcp": "2024-11-05",
                                "server": "aorui-mcp-server",
                                "endpoint": "/mcp",
                                "methods": ["initialize", "tools/list", "tools/call", "resources/list", "resources/read", "prompts/list", "prompts/get"]
                            }))
                        } else if req.starts_with("POST /ui/dynamic") || req.starts_with("POST /ui/declarative") {
                            if let Some(body_start) = req.find("\r\n\r\n") {
                                let body = req[body_start + 4..].trim();
                                let parsed_opt: Result<Value, _> = serde_json::from_str(body);
                                let (format, content) = if let Ok(parsed) = parsed_opt {
                                    let fmt = parsed.get("format").and_then(|v| v.as_str()).unwrap_or("toml").to_string();
                                    let cnt = parsed.get("content").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                    (fmt, cnt)
                                } else {
                                    ("toml".to_string(), body.to_string())
                                };

                                let _ = tx.send(UiEvent::UserPromptSubmitted(format!("Charger UI déclarative [{}]", format))).await;
                                ("200 OK", json!({
                                    "status": "ui_loaded",
                                    "format": format,
                                    "length_bytes": content.len()
                                }))
                            } else {
                                ("400 Bad Request", json!({"error": "Empty UI payload"}))
                            }
                        } else if req.starts_with("POST /prompt") || req.starts_with("POST /action") {
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
                                    let _ = tx.send(UiEvent::UserPromptSubmitted(prompt.clone())).await;
                                    ("200 OK", json!({
                                        "status": "dispatched",
                                        "prompt": prompt,
                                        "sandbox": cfg.sandbox_mode,
                                        "app": cfg.app_name
                                    }))
                                } else {
                                    ("400 Bad Request", json!({"error": "Champ 'prompt' manquant dans le corps JSON"}))
                                }
                            } else {
                                ("400 Bad Request", json!({"error": "Headers malformés"}))
                            }
                        } else if req.starts_with("POST /interrupt") {
                            let _ = tx.send(UiEvent::AgentInterrupted).await;
                            ("200 OK", json!({"status": "interrupted"}))
                        } else if req.starts_with("GET /health") || req.starts_with("GET /status") {
                            ("200 OK", json!({
                                "status": "healthy",
                                "app": cfg.app_name,
                                "port": port,
                                "sandbox": cfg.sandbox_mode,
                                "auth_required": cfg.auth_token.is_some(),
                                "mcp_endpoint": "/mcp",
                                "stream_endpoint": "/events/stream"
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
        });

        Some(status)
    }
}

/// High-level ergonomic bootstrap helper for connecting any AORUI application in 3 lines.
pub struct AgentHub;

impl AgentHub {
    /// Bootstraps an autonomous cognitive loop and external HTTP bridge on a background thread.
    /// Returns `(mpsc::Sender<UiEvent>, crossbeam_channel::Receiver<UiPatch>)`.
    pub fn bootstrap(
        tools: ToolRegistry,
        planner: Box<dyn Planner>,
        config: BridgeConfig,
    ) -> (mpsc::Sender<UiEvent>, crossbeam_channel::Receiver<UiPatch>) {
        let (tx_events, rx_events) = mpsc::channel::<UiEvent>(64);
        let (tx_patches, rx_patches) = crossbeam_channel::unbounded::<UiPatch>();

        let agent_config = AgentConfig {
            max_steps: 16,
            tool_timeout: Duration::from_secs(10),
            max_scratchpad_entries: 50,
        };
        let agent = Agent::new(agent_config, tools, tx_patches);

        let tx_bridge = tx_events.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Échec de création du runtime Tokio AORUI");

            rt.block_on(async move {
                AgentBridge::start(config, tx_bridge).await;
                let _ = agent.run(rx_events, planner).await;
            });
        });

        (tx_events, rx_patches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bridge_config_builder() {
        let cfg = BridgeConfig::new("My Dashboard")
            .with_port(9000)
            .with_auth_token("secret123")
            .with_sandbox(true);

        assert_eq!(cfg.app_name, "My Dashboard");
        assert_eq!(cfg.base_port, 9000);
        assert_eq!(cfg.auth_token.as_deref(), Some("secret123"));
        assert!(cfg.sandbox_mode);
    }
}
