// [WFGY] Zone: SAFE | λ: 0.1 | Fallbacks: 0 | Action: Agent runtime error types
use crate::fsm::AgentState;

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("invalid transition: {from} -> {to}")]
    InvalidTransition { from: AgentState, to: AgentState },

    #[error("execution budget exceeded: {steps} steps (max: {max_steps})")]
    StepBudgetExceeded { steps: u32, max_steps: u32 },

    #[error("unknown tool: {0}")]
    UnknownTool(String),

    #[error("schema validation failed for tool {tool}: {reason}")]
    SchemaValidation { tool: String, reason: String },

    #[error("tool timeout for {tool} after {timeout_ms}ms")]
    ToolTimeout { tool: String, timeout_ms: u128 },

    #[error("tool execution failed for {tool}: {reason}")]
    ToolExecution { tool: String, reason: String },

    #[error("planning failed: {0}")]
    PlanningFailed(String),

    #[error("agent execution interrupted by user")]
    Interrupted,

    #[error("UiEvent action channel closed, terminating agent")]
    ActionChannelClosed,
}
