// [WFGY] Zone: RISK | λ: 0.3 | Fallbacks: 0 | Action: Tokio cognitive loop implementation (FSM, Tool, budget, timeout)
//! Agent cognitive runtime, decoupled from the Render Thread (INV-CORE-1):
//! this crate depends neither on `wgpu` nor on `winit`.
//!
//! INV-SEC-3: all tool executions pass through JSON schema validation
//! ([`schema::validate`]) and mandatory timeout limits
//! (`tokio::time::timeout`), see [`agent::Agent::execute_tool`].

mod agent;
mod error;
mod fsm;
mod planner;
mod schema;
mod tool;

pub use agent::{Agent, AgentConfig};
pub use error::AgentError;
pub use fsm::AgentState;
pub use planner::{PlanDecision, Planner};
pub use tool::{BoxFuture, Tool, ToolError, ToolRegistry};

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::{json, Value};
    use tokio::sync::mpsc;
    use ui_core::{UiEvent, UiPatch};

    use super::*;

    struct EchoTool;
    impl Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }
        fn schema(&self) -> Value {
            json!({ "type": "object", "required": ["text"], "properties": { "text": { "type": "string" } } })
        }
        fn execute<'a>(&'a self, args: Value) -> BoxFuture<'a, Result<Value, ToolError>> {
            Box::pin(async move { Ok(args) })
        }
    }

    struct SleepyTool;
    impl Tool for SleepyTool {
        fn name(&self) -> &str {
            "sleepy"
        }
        fn schema(&self) -> Value {
            json!({ "type": "object" })
        }
        fn execute<'a>(&'a self, _args: Value) -> BoxFuture<'a, Result<Value, ToolError>> {
            Box::pin(async move {
                tokio::time::sleep(Duration::from_secs(60)).await;
                Ok(Value::Null)
            })
        }
    }

    struct ScriptedPlanner {
        decisions: Vec<PlanDecision>,
    }

    impl ScriptedPlanner {
        fn new(decisions: Vec<PlanDecision>) -> Self {
            Self { decisions }
        }
    }

    impl Planner for ScriptedPlanner {
        fn plan<'a>(&'a mut self, _scratchpad: &'a [String]) -> BoxFuture<'a, PlanDecision> {
            Box::pin(async move {
                if self.decisions.is_empty() {
                    PlanDecision::Complete
                } else {
                    self.decisions.remove(0)
                }
            })
        }
    }

    fn make_test_setup(
        max_steps: u32,
        tool_timeout: Duration,
    ) -> (
        Agent,
        mpsc::Sender<UiEvent>,
        mpsc::Receiver<UiEvent>,
        crossbeam_channel::Receiver<UiPatch>,
    ) {
        let (tx_events, rx_events) = mpsc::channel(32);
        let (tx_patches, rx_patches) = crossbeam_channel::unbounded();
        let mut tools = ToolRegistry::new();
        tools.register(std::sync::Arc::new(EchoTool));
        tools.register(std::sync::Arc::new(SleepyTool));

        let config = AgentConfig { max_steps, tool_timeout, max_scratchpad_entries: 50 };
        let agent = Agent::new(config, tools, tx_patches);
        (agent, tx_events, rx_events, rx_patches)
    }

    #[tokio::test]
    async fn nominal_run_completes_after_scripted_tool_calls() {
        let (agent, tx_events, rx_events, rx_patches) =
            make_test_setup(5, Duration::from_secs(5));

        let planner = Box::new(ScriptedPlanner::new(vec![
            PlanDecision::CallTool {
                tool: "echo".into(),
                args: json!({ "text": "hello" }),
            },
            PlanDecision::Complete,
        ]));

        let handle = tokio::spawn(async move { agent.run(rx_events, planner).await });

        tx_events.send(UiEvent::UserPromptSubmitted("say hello".into())).await.unwrap();
        drop(tx_events);

        let res = handle.await.unwrap();
        assert!(res.is_ok(), "nominal run should succeed");

        let mut received = Vec::new();
        while let Ok(patch) = rx_patches.try_recv() {
            received.push(patch);
        }

        assert!(
            received.iter().any(|p| matches!(p, UiPatch::StatusChanged { state, .. } if state == "Idle")),
            "agent should transition through Idle"
        );
        assert!(
            received.iter().any(|p| matches!(p, UiPatch::StepLogged { tool, .. } if tool == "echo")),
            "agent should log the tool execution"
        );
    }

    #[tokio::test]
    async fn step_budget_is_enforced() {
        let (agent, tx_events, rx_events, rx_patches) =
            make_test_setup(2, Duration::from_secs(5));

        let infinite_tool_calls: Vec<_> = (0..10)
            .map(|_| PlanDecision::CallTool {
                tool: "echo".into(),
                args: json!({ "text": "loop" }),
            })
            .collect();
        let planner = Box::new(ScriptedPlanner::new(infinite_tool_calls));

        let handle = tokio::spawn(async move { agent.run(rx_events, planner).await });
        tx_events.send(UiEvent::UserPromptSubmitted("spin".into())).await.unwrap();
        drop(tx_events);

        let res = handle.await.unwrap();
        assert!(matches!(res, Err(AgentError::StepBudgetExceeded { steps: 2, max_steps: 2 })));

        let mut received = Vec::new();
        while let Ok(patch) = rx_patches.try_recv() {
            received.push(patch);
        }
        assert!(received.iter().any(|p| matches!(p, UiPatch::StatusChanged { state, .. } if state == "Failed")));
    }

    #[tokio::test]
    async fn tool_timeout_is_enforced_and_reported() {
        let (agent, tx_events, rx_events, _rx_patches) =
            make_test_setup(5, Duration::from_millis(50));

        let planner = Box::new(ScriptedPlanner::new(vec![PlanDecision::CallTool {
            tool: "sleepy".into(),
            args: json!({}),
        }]));

        let handle = tokio::spawn(async move { agent.run(rx_events, planner).await });
        tx_events.send(UiEvent::UserPromptSubmitted("sleep".into())).await.unwrap();
        drop(tx_events);

        let res = handle.await.unwrap();
        assert!(
            matches!(res, Err(AgentError::ToolTimeout { ref tool, .. }) if tool == "sleepy"),
            "timeout should trigger error"
        );
    }

    #[tokio::test]
    async fn unknown_tool_is_rejected_before_any_schema_check() {
        let (agent, tx_events, rx_events, _rx_patches) =
            make_test_setup(5, Duration::from_secs(5));

        let planner = Box::new(ScriptedPlanner::new(vec![PlanDecision::CallTool {
            tool: "does_not_exist".into(),
            args: json!({}),
        }]));

        let handle = tokio::spawn(async move { agent.run(rx_events, planner).await });
        tx_events.send(UiEvent::UserPromptSubmitted("run missing".into())).await.unwrap();
        drop(tx_events);

        let res = handle.await.unwrap();
        assert!(matches!(res, Err(AgentError::UnknownTool(ref tool)) if tool == "does_not_exist"));
    }
}
