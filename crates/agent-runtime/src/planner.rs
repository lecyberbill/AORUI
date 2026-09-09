// [WFGY] Zone: TRANSIT | λ: 0.2 | Fallbacks: 0 | Action: Planner interface decoupled from concrete LLM backends
use serde_json::Value;

use crate::tool::BoxFuture;

/// Decision produced by the `Planning` stage of the FSM.
#[derive(Debug, Clone, PartialEq)]
pub enum PlanDecision {
    CallTool { tool: String, args: Value },
    Complete,
    Fail(String),
}

/// Asynchronous planning strategy trait (`BoxFuture`).
/// The agent runtime does not depend on a specific LLM SDK — implementations
/// provide custom planners to satisfy this interface (INV-CORE-1).
pub trait Planner: Send {
    fn plan<'a>(&'a mut self, scratchpad: &'a [String]) -> BoxFuture<'a, PlanDecision>;
}
