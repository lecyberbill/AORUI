// [WFGY] Zone: RISK | λ: 0.3 | Fallbacks: 0 | Action: Tokio cognitive loop (FSM + budget + timeout + patch emission)
use std::time::Duration;

use serde_json::Value;
use tokio::sync::mpsc;
use ui_core::{StepStatus, UiEvent, UiPatch};

use crate::error::AgentError;
use crate::fsm::AgentState;
use crate::planner::{PlanDecision, Planner};
use crate::schema;
use crate::tool::{ToolError, ToolRegistry};

/// INV-BUDGET-1: `max_steps` bounds iterations between Planning -> ExecutingTool -> Reflecting.
/// INV-TIMEOUT-1: `tool_timeout` is applied to every tool execution via `tokio::time::timeout`.
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub max_steps: u32,
    pub tool_timeout: Duration,
    pub max_scratchpad_entries: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_steps: 12,
            tool_timeout: Duration::from_secs(30),
            max_scratchpad_entries: 500,
        }
    }
}

pub struct Agent {
    config: AgentConfig,
    state: AgentState,
    step_count: u32,
    scratchpad: Vec<String>,
    tools: ToolRegistry,
    tx_patches: crossbeam_channel::Sender<UiPatch>,
    pending_tool: Option<(String, Value)>,
}

impl Agent {
    pub fn new(
        config: AgentConfig,
        tools: ToolRegistry,
        tx_patches: crossbeam_channel::Sender<UiPatch>,
    ) -> Self {
        Self {
            config,
            state: AgentState::Idle,
            step_count: 0,
            scratchpad: Vec::new(),
            tools,
            tx_patches,
            pending_tool: None,
        }
    }

    pub fn state(&self) -> AgentState {
        self.state
    }

    fn transition(&mut self, next: AgentState) -> Result<(), AgentError> {
        if !self.state.can_transition_to(next) {
            return Err(AgentError::InvalidTransition {
                from: self.state,
                to: next,
            });
        }
        self.state = next;
        let _ = self.tx_patches.send(UiPatch::StatusChanged {
            state: next.to_string(),
            glow_hue: glow_color_for(next),
        });
        Ok(())
    }

    /// Appends an entry to memory scratchpad, bounded by `max_scratchpad_entries`.
    fn push_scratchpad(&mut self, entry: String) {
        if self.scratchpad.len() >= self.config.max_scratchpad_entries {
            self.scratchpad.remove(0);
        }
        self.scratchpad.push(entry.clone());
        let _ = self.tx_patches.send(UiPatch::ScratchpadAppended(entry));
    }

    fn is_interrupted(rx_actions: &mut mpsc::Receiver<UiEvent>) -> bool {
        matches!(rx_actions.try_recv(), Ok(UiEvent::AgentInterrupted))
    }

    /// Validates arguments against schema (INV-SEC-3) and executes under timeout (INV-TIMEOUT-1).
    async fn execute_tool(&self, name: &str, args: Value) -> Result<Value, AgentError> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| AgentError::UnknownTool(name.to_string()))?;

        schema::validate(&tool.schema(), &args).map_err(|reason| AgentError::SchemaValidation {
            tool: name.to_string(),
            reason: reason.to_string(),
        })?;

        match tokio::time::timeout(self.config.tool_timeout, tool.execute(args)).await {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(ToolError(reason))) => Err(AgentError::ToolExecution {
                tool: name.to_string(),
                reason,
            }),
            Err(_elapsed) => Err(AgentError::ToolTimeout {
                tool: name.to_string(),
                timeout_ms: self.config.tool_timeout.as_millis(),
            }),
        }
    }

    /// Main cognitive execution loop.
    pub async fn run(
        mut self,
        mut rx_actions: mpsc::Receiver<UiEvent>,
        mut planner: Box<dyn Planner>,
    ) -> Result<(), AgentError> {
        loop {
            match self.state {
                AgentState::Idle => match rx_actions.recv().await {
                    Some(UiEvent::UserPromptSubmitted(prompt)) => {
                        self.step_count = 0;
                        self.push_scratchpad(format!("prompt: {prompt}"));
                        self.transition(AgentState::Perceiving)?;
                    }
                    Some(_) => continue,
                    None => return Ok(()),
                },

                AgentState::Perceiving => {
                    self.transition(AgentState::Planning)?;
                }

                AgentState::Planning => {
                    if Self::is_interrupted(&mut rx_actions) {
                        self.push_scratchpad("interrupted by user".to_string());
                        self.transition(AgentState::Failed)?;
                        return Err(AgentError::Interrupted);
                    }
                    if self.step_count >= self.config.max_steps {
                        let err = AgentError::StepBudgetExceeded {
                            steps: self.step_count,
                            max_steps: self.config.max_steps,
                        };
                        self.push_scratchpad(err.to_string());
                        self.transition(AgentState::Failed)?;
                        return Err(err);
                    }
                    self.step_count += 1;

                    match planner.plan(&self.scratchpad).await {
                        PlanDecision::CallTool { tool, args } => {
                            self.pending_tool = Some((tool, args));
                            self.transition(AgentState::ExecutingTool)?;
                        }
                        PlanDecision::Complete => {
                            self.transition(AgentState::Completed)?;
                        }
                        PlanDecision::Fail(reason) => {
                            self.push_scratchpad(format!("planning failed: {reason}"));
                            self.transition(AgentState::Failed)?;
                            return Err(AgentError::PlanningFailed(reason));
                        }
                    }
                }

                AgentState::ExecutingTool => {
                    let (tool, args) = self
                        .pending_tool
                        .take()
                        .expect("INV-FSM-1 violated: ExecutingTool without pending tool");
                    let step_id = format!("step-{}", self.step_count);

                    let _ = self.tx_patches.send(UiPatch::StepLogged {
                        step_id: step_id.clone(),
                        tool: tool.clone(),
                        status: StepStatus::Running,
                    });

                    match self.execute_tool(&tool, args).await {
                        Ok(value) => {
                            let _ = self.tx_patches.send(UiPatch::StepLogged {
                                step_id,
                                tool: tool.clone(),
                                status: StepStatus::Success,
                            });
                            self.push_scratchpad(format!("result[{tool}]: {value}"));
                            self.transition(AgentState::Reflecting)?;
                        }
                        Err(err) => {
                            let _ = self.tx_patches.send(UiPatch::StepLogged {
                                step_id,
                                tool: tool.clone(),
                                status: StepStatus::Failed(err.to_string()),
                            });
                            self.push_scratchpad(err.to_string());
                            self.transition(AgentState::Failed)?;
                            return Err(err);
                        }
                    }
                }

                AgentState::Reflecting => {
                    self.transition(AgentState::Planning)?;
                }

                AgentState::Completed | AgentState::Failed => {
                    self.transition(AgentState::Idle)?;
                }
            }
        }
    }
}

fn glow_color_for(state: AgentState) -> [f32; 4] {
    match state {
        AgentState::Idle => [0.2, 0.9, 0.9, 1.0],
        AgentState::Perceiving => [0.2, 0.6, 1.0, 1.0],
        AgentState::Planning => [0.6, 0.3, 1.0, 1.0],
        AgentState::ExecutingTool => [1.0, 0.8, 0.1, 1.0],
        AgentState::Reflecting => [0.1, 1.0, 0.5, 1.0],
        AgentState::Completed => [0.1, 1.0, 0.3, 1.0],
        AgentState::Failed => [1.0, 0.15, 0.2, 1.0],
    }
}
