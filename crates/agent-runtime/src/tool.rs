// [WFGY] Zone: RISK | λ: 0.2 | Fallbacks: 0 | Action: Asynchronous decoupled Tool trait and registry
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde_json::Value;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ToolError(pub String);

/// An executable tool for the agent runtime, decoupled from the FSM (INV-CORE-1).
///
/// INV-SEC-3: `schema()` describes the expected JSON argument structure — validated
/// by `Agent::execute_tool` prior to `execute`.
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn schema(&self) -> Value;
    fn execute<'a>(&'a self, args: Value) -> BoxFuture<'a, Result<Value, ToolError>>;
}

#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }
}
