# 🤖 AORUI Agent Integration & SDK Guide

> **Decoupled Architecture, Memory Sandboxing, Capability Security & Real-Time 60+ FPS Cognitive Control**

---

## 📑 Table of Contents
1. [Vision & Core Architecture Principles](#1-vision--core-architecture-principles)
2. [Quickstart: 3 Lines of Rust to Make Any UI Agent-Ready](#2-quickstart-3-lines-of-rust-to-make-any-ui-agent-ready)
3. [Topological Architecture & Decoupling (INV-CORE-1)](#3-topological-architecture--decoupling-inv-core-1)
4. [Security, Sandboxing & Human-in-the-Loop Gates](#4-security-sandboxing--human-in-the-loop-gates)
5. [HTTP & JSON-RPC Bridge Specification](#5-http--json-rpc-bridge-specification)
6. [Multi-Language Client SDKs (Python, TypeScript, Rust, cURL)](#6-multi-language-client-sdks)
7. [Authoring Custom Deterministic Tools](#7-authoring-custom-deterministic-tools)

---

## 1. Vision & Core Architecture Principles

**AORUI** is engineered from the ground up to be **Agent-Native**:
- **Zero Coupling Between Render Engine and AI Engine:** The hardware GPU renderer (`wgpu` / WGSL) operates uninterrupted at 60+ FPS on the main thread and is never blocked by LLM inference latency, network delays, or slow disk I/O.
- **Immediate Visual Reflection:** External agents drive the interface through atomic `UiPatch` events, instantly activating interactive glowing borders, gauges, charts, tables, and modal dialogues.
- **Deterministic Verification & Safety:** All executable tools adhere to strict JSON schemas, finite step budgets, enforced execution timeouts, and optional human approval barriers.

---

## 2. Quickstart: 3 Lines of Rust to Make Any UI Agent-Ready

Using the `AgentHub` bootstrap module in `agent-runtime`, adding remote agent control to any AORUI application requires only **3 lines of code**:

```rust
use agent_runtime::{AgentHub, BridgeConfig, ToolRegistry};

fn main() {
    // 1. Declare application tools and planner
    let tools = my_application_tools();
    let planner = Box::new(my_application_planner());

    // 2. Bootstrap Agent Cognitive Loop & HTTP Bridge in ONE line
    let (tx_events, rx_patches) = AgentHub::bootstrap(
        tools, 
        planner, 
        BridgeConfig::new("AORUI Workstation")
            .with_port(8765)
            .with_sandbox(true)
            .with_auth_token("secret_token_123") // Optional Bearer Token
    );

    // 3. Connect channels to the Winit event loop & WGPU renderer
    let mut app = MyApp::new(tx_events, rx_patches);
    event_loop.run_app(&mut app).unwrap();
}
```

---

## 3. Topological Architecture & Decoupling (INV-CORE-1)

```mermaid
graph TD
    subgraph "External World & AI Agents"
        AgentLLM["External AI Agent (Python / LangChain / AutoGen / cURL)"]
        UserPrompt["In-App User Prompt (Copilot HUD)"]
    end

    subgraph "Asynchronous Cognitive Runtime (Tokio - Dedicated Background Thread)"
        HttpBridge["AgentBridge (HTTP :8765)"]
        FSM["Cognitive FSM (Idle → Perceiving → Planning → Executing → Reflecting)"]
        ToolRegistry["Validated Tool Registry (JSON Schema + Timeouts)"]
    end

    subgraph "High-Performance Render Engine (Main Thread GPU)"
        RxPatches["Crossbeam Channel (Atomic UiPatch Stream)"]
        LayoutTree["Widget Tree & Taffy Flexbox/Grid Layout"]
        GpuPipeline["3-Pass WGPU Pipeline (Background + Blur + SDF Glass + Text)"]
    end

    AgentLLM -->|"POST /prompt (JSON)"| HttpBridge
    UserPrompt -->|"UiEvent::UserPromptSubmitted"| FSM
    HttpBridge -->|"UiEvent::UserPromptSubmitted"| FSM
    FSM -->|"Execute Tool"| ToolRegistry
    ToolRegistry -->|"Atomic UiPatch"| RxPatches
    RxPatches -->|"about_to_wait()"| LayoutTree
    LayoutTree -->|"RenderLayer"| GpuPipeline
```

---

## 4. Security, Sandboxing & Human-in-the-Loop Gates

### 🛡️ 1. In-Memory Sandbox Isolation (`sandbox_mode: true`)
When sandbox mode is enabled, agent mutations are strictly constrained to application memory state (UI widgets, telemetry gauges, virtual micro-services, and in-memory tables). No destructive host OS commands are ever permitted.

### 🔒 2. Bearer Token Authentication (`with_auth_token`)
For multi-tenant environments or network-exposed instances:
```rust
let config = BridgeConfig::new("Production Console")
    .with_port(8765)
    .with_auth_token("cluster-admin-token-77");
```
Any incoming request missing `Authorization: Bearer cluster-admin-token-77` is immediately rejected with `401 Unauthorized`.

### 👨‍💻 3. Human-in-the-Loop Approval Fences
For high-impact operations (e.g. database schema migrations, service restarts, data purges):
1. The app intercepts the prompt and buffers it into `pending_approval_prompt`.
2. A midnight-navy modal dialog surfaces, asking the human operator for explicit confirmation.
3. The action is executed by the agent runtime only upon clicking **"Confirm & Apply"**.

---

## 5. HTTP & JSON-RPC Bridge Specification

### 1. `GET /health` or `GET /status`
Queries bridge health, current port, and sandbox status.
- **Response (200 OK):**
```json
{
  "status": "healthy",
  "app": "AetherOS Workstation",
  "port": 8765,
  "sandbox": true,
  "auth_required": false
}
```

### 2. `POST /prompt` or `POST /action`
Submits a natural language instruction or structured command to the cognitive loop.
- **Request Body (JSON):**
```json
{
  "prompt": "Purge caches and refresh real-time telemetry streams"
}
```
- **Response (200 OK):**
```json
{
  "status": "dispatched",
  "prompt": "Purge caches and refresh real-time telemetry streams",
  "sandbox": true,
  "app": "AetherOS Workstation"
}
```

### 3. `POST /interrupt`
Immediately cancels the currently executing agent step and transitions the FSM to `AgentState::Idle`.

### 4. `POST /mcp` & `GET /mcp` (Model Context Protocol JSON-RPC 2.0)
Standard MCP 2024-11-05 endpoint for Anthropic Claude Desktop, Cursor, and Antigravity.
- **Tools List (`method: "tools/list"`):**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/list"
}
```
- **Tool Execution (`method: "tools/call"`):**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "adjust_thresholds",
    "arguments": { "cpu_max": 75.0, "latency_max": 12.0 }
  }
}
```
- **Live State Resources (`method: "resources/read"`, `uri: "aorui://ui/state"`):**
Returns real-time UI hierarchy and telemetry without parsing HTML.

### 5. `POST /ui/dynamic` (On-the-Fly Declarative UI Generation)
Allows an external agent to inject dynamically generated UI layouts (TOML or JSON) directly into GPU memory with live hot-reloading.
- **Request Body:**
```json
{
  "format": "toml",
  "content": "[layout]\ncolumns = 3\n..."
}
```

---

## 6. Multi-Language Client SDKs

### 🐍 Python Client (LangChain / AutoGen / CrewAI / OpenAI / Claude)
```python
import requests

class AoruiClient:
    def __init__(self, host: str = "127.0.0.1", port: int = 8765, token: str = None):
        self.base_url = f"http://{host}:{port}"
        self.headers = {"Content-Type": "application/json; charset=utf-8"}
        if token:
            self.headers["Authorization"] = f"Bearer {token}"

    def health(self) -> dict:
        resp = requests.get(f"{self.base_url}/health", headers=self.headers, timeout=3)
        resp.raise_for_status()
        return resp.json()

    def prompt(self, prompt: str) -> dict:
        resp = requests.post(
            f"{self.base_url}/prompt",
            headers=self.headers,
            json={"prompt": prompt},
            timeout=10
        )
        resp.raise_for_status()
        return resp.json()

# Usage Example
if __name__ == "__main__":
    client = AoruiClient()
    print("Health:", client.health())
    result = client.prompt("Filter process table and analyze: postgres")
    print("Dispatched:", result)
```

### ⚡ TypeScript / Node.js
```typescript
async function sendAoruiPrompt(prompt: string, port = 8765, token?: string) {
  const headers: Record<string, string> = { "Content-Type": "application/json" };
  if (token) headers["Authorization"] = `Bearer ${token}`;

  const response = await fetch(`http://127.0.0.1:${port}/prompt`, {
    method: "POST",
    headers,
    body: JSON.stringify({ prompt }),
  });
  return await response.json();
}

// Example usage
sendAoruiPrompt("Adjust telemetry window to 24h").then(console.log);
```

### 💻 cURL / PowerShell
```bash
# Using cURL
curl -X POST http://127.0.0.1:8765/prompt \
  -H "Content-Type: application/json" \
  -d '{"prompt": "Purge caches and refresh telemetry"}'
```

```powershell
# Using PowerShell
.\send_agent_prompt.ps1 -Prompt "Adjust telemetry window to 24h"
```

---

## 7. Authoring Custom Deterministic Tools

Adding custom capabilities to the agent runtime is simple, typed, and verifiable:

```rust
use agent_runtime::{Tool, ToolError, BoxFuture};
use serde_json::{json, Value};
use ui_core::UiPatch;

pub struct SetThemeTool {
    pub tx_patches: crossbeam_channel::Sender<UiPatch>,
}

impl Tool for SetThemeTool {
    fn name(&self) -> &str {
        "set_theme"
    }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "theme_name": { "type": "string", "enum": ["aether_os", "tokyo_night", "cyber_dark"] }
            },
            "required": ["theme_name"]
        })
    }

    fn execute<'a>(&'a self, args: Value) -> BoxFuture<'a, Result<Value, ToolError>> {
        Box::pin(async move {
            let theme = args["theme_name"].as_str().unwrap_or("aether_os");
            
            // Dispatch atomic UiPatch to the GPU render thread
            let _ = self.tx_patches.send(UiPatch::WidgetTextSet {
                widget_id: "active_theme_label".to_string(),
                text: format!("Active Theme: {}", theme),
            });

            Ok(json!({ "status": "applied", "theme": theme }))
        })
    }
}
```

---

## 🏆 Key Guarantees
- **100% Native Rust & WGSL**: Peak performance without heavy web runtimes (no Chromium / Electron overhead).
- **Plug & Play**: Integrates seamlessly with LangChain, AutoGen, CrewAI, OpenAI Assistants, Claude, and Antigravity.
- **Enterprise-Grade Safety**: Strict JSON schemas, memory sandboxing, finite iteration budgets, and optional Bearer token authorization.
