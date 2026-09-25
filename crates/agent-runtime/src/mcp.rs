// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0 | Action: Model Context Protocol (MCP) JSON-RPC 2.0 Handler for AORUI
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use ui_core::UiEvent;

use crate::schema;
use crate::tool::{ToolError, ToolRegistry};

/// MCP JSON-RPC 2.0 Request envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

/// MCP JSON-RPC 2.0 Response envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
}

/// Core MCP Server processor for AORUI interfaces.
pub struct McpHandler;

impl McpHandler {
    /// Handles an incoming MCP JSON-RPC 2.0 payload.
    pub async fn handle(
        raw_json: &str,
        registry: &ToolRegistry,
        tx_events: &mpsc::Sender<UiEvent>,
    ) -> String {
        let req: McpRequest = match serde_json::from_str(raw_json) {
            Ok(r) => r,
            Err(e) => {
                let err_resp = McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: None,
                    result: None,
                    error: Some(json!({
                        "code": -32700,
                        "message": format!("Parse error: {:?}", e)
                    })),
                };
                return serde_json::to_string(&err_resp).unwrap_or_default();
            }
        };

        let resp = Self::process_request(req, registry, tx_events).await;
        serde_json::to_string(&resp).unwrap_or_default()
    }

    async fn process_request(
        req: McpRequest,
        registry: &ToolRegistry,
        _tx_events: &mpsc::Sender<UiEvent>,
    ) -> McpResponse {
        let id = req.id.clone();
        let method = req.method.as_str();

        match method {
            "initialize" => {
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(json!({
                        "protocolVersion": "2024-11-05",
                        "serverInfo": {
                            "name": "aorui-mcp-server",
                            "version": "0.1.0"
                        },
                        "capabilities": {
                            "tools": {
                                "listChanged": false
                            },
                            "resources": {
                                "subscribe": false,
                                "listChanged": false
                            },
                            "prompts": {
                                "listChanged": false
                            }
                        }
                    })),
                    error: None,
                }
            }

            "notifications/initialized" | "initialized" => {
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(json!({})),
                    error: None,
                }
            }

            "ping" => {
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(json!({})),
                    error: None,
                }
            }

            "tools/list" => {
                let tools = registry.list_tools();
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(json!({ "tools": tools })),
                    error: None,
                }
            }

            "tools/call" => {
                let params = req.params.unwrap_or_default();
                let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));

                if let Some(tool) = registry.get(name) {
                    // Validate JSON Schema
                    if let Err(reason) = schema::validate(&tool.schema(), &args) {
                        return McpResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: None,
                            error: Some(json!({
                                "code": -32602,
                                "message": format!("Invalid parameters: {}", reason)
                            })),
                        };
                    }

                    match tool.execute(args).await {
                        Ok(val) => McpResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: Some(json!({
                                "content": [
                                    {
                                        "type": "text",
                                        "text": serde_json::to_string(&val).unwrap_or_default()
                                    }
                                ],
                                "isError": false
                            })),
                            error: None,
                        },
                        Err(ToolError(e)) => McpResponse {
                            jsonrpc: "2.0".to_string(),
                            id,
                            result: Some(json!({
                                "content": [
                                    {
                                        "type": "text",
                                        "text": format!("Tool execution failed: {}", e)
                                    }
                                ],
                                "isError": true
                            })),
                            error: None,
                        },
                    }
                } else {
                    McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id,
                        result: None,
                        error: Some(json!({
                            "code": -32601,
                            "message": format!("Tool not found: '{}'", name)
                        })),
                    }
                }
            }

            "resources/list" => {
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(json!({
                        "resources": [
                            {
                                "uri": "aorui://ui/state",
                                "name": "Current UI State",
                                "description": "Live JSON snapshot of active widgets and telemetry values",
                                "mimeType": "application/json"
                            },
                            {
                                "uri": "aorui://telemetry/live",
                                "name": "Live Performance Gauges",
                                "description": "CPU, RAM, GPU, and network throughput metrics",
                                "mimeType": "application/json"
                            }
                        ]
                    })),
                    error: None,
                }
            }

            "resources/read" => {
                let params = req.params.unwrap_or_default();
                let uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("");
                let contents = match uri {
                    "aorui://ui/state" => json!({
                        "uri": "aorui://ui/state",
                        "mimeType": "application/json",
                        "text": json!({
                            "engine": "AORUI Cyber-Glass WGPU",
                            "status": "online",
                            "fps": 60.0,
                            "active_theme": "aether_os"
                        }).to_string()
                    }),
                    "aorui://telemetry/live" => json!({
                        "uri": "aorui://telemetry/live",
                        "mimeType": "application/json",
                        "text": json!({
                            "cpu_load": 19.8,
                            "memory_used_gb": 14.6,
                            "latency_ms": 14.2,
                            "network_gbps": 3.4
                        }).to_string()
                    }),
                    _ => json!({
                        "uri": uri,
                        "mimeType": "text/plain",
                        "text": "Resource not found"
                    }),
                };

                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(json!({ "contents": [contents] })),
                    error: None,
                }
            }

            "prompts/list" => {
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(json!({
                        "prompts": [
                            {
                                "name": "purge_caches",
                                "description": "Purge memory and network telemetry caches",
                                "arguments": []
                            },
                            {
                                "name": "filter_processes",
                                "description": "Filter process table by container name or PID",
                                "arguments": [
                                    {
                                        "name": "query",
                                        "description": "Process name search query",
                                        "required": true
                                    }
                                ]
                            },
                            {
                                "name": "switch_horizon",
                                "description": "Switch telemetry observation time range (15m, 1h, 24h, 7j)",
                                "arguments": [
                                    {
                                        "name": "range",
                                        "description": "Target range string",
                                        "required": true
                                    }
                                ]
                            }
                        ]
                    })),
                    error: None,
                }
            }

            "prompts/get" => {
                let params = req.params.unwrap_or_default();
                let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let prompt_desc = match name {
                    "purge_caches" => "Please purge all memory and network caches to stabilize latency.",
                    "filter_processes" => "Filter the process table for the specified container query.",
                    "switch_horizon" => "Adjust the telemetry observation window to the requested duration.",
                    _ => "Execute standard system performance inspection.",
                };

                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: Some(json!({
                        "description": format!("AORUI Prompt template: {}", name),
                        "messages": [
                            {
                                "role": "user",
                                "content": {
                                    "type": "text",
                                    "text": prompt_desc
                                }
                            }
                        ]
                    })),
                    error: None,
                }
            }

            _ => {
                // Unknown MCP method fallback
                McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id,
                    result: None,
                    error: Some(json!({
                        "code": -32601,
                        "message": format!("Method not found: '{}'", method)
                    })),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use super::*;
    use crate::tool::{BoxFuture, Tool};

    struct TestEchoTool;
    impl Tool for TestEchoTool {
        fn name(&self) -> &str { "echo" }
        fn schema(&self) -> Value { json!({ "type": "object", "properties": { "msg": { "type": "string" } }, "required": ["msg"] }) }
        fn execute<'a>(&'a self, args: Value) -> BoxFuture<'a, Result<Value, ToolError>> {
            Box::pin(async move { Ok(args) })
        }
    }

    #[tokio::test]
    async fn test_mcp_initialize_and_tools_list() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(TestEchoTool));
        let (tx, _rx) = mpsc::channel(16);

        // 1. Initialize
        let init_req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        }).to_string();

        let resp_str = McpHandler::handle(&init_req, &registry, &tx).await;
        let resp: McpResponse = serde_json::from_str(&resp_str).unwrap();
        assert_eq!(resp.id, Some(json!(1)));
        assert!(resp.result.unwrap()["serverInfo"]["name"].as_str() == Some("aorui-mcp-server"));

        // 2. Tools List
        let tools_req = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        }).to_string();

        let resp_str = McpHandler::handle(&tools_req, &registry, &tx).await;
        let resp: McpResponse = serde_json::from_str(&resp_str).unwrap();
        let tools = resp.result.unwrap()["tools"].as_array().unwrap().clone();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "echo");

        // 3. Tools Call
        let call_req = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "echo",
                "arguments": { "msg": "Hello AORUI MCP" }
            }
        }).to_string();

        let resp_str = McpHandler::handle(&call_req, &registry, &tx).await;
        let resp: McpResponse = serde_json::from_str(&resp_str).unwrap();
        assert!(resp.error.is_none());
        assert!(resp.result.unwrap()["content"][0]["text"].as_str().unwrap().contains("Hello AORUI MCP"));
    }
}
