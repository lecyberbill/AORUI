// [WFGY] Zone: TRANSIT | λ: 0.2 | Fallbacks: 0 | Action: Strictly typed agent FSM with explicit transition table
/// Cognitive loop execution states of the autonomous agent runtime.
///
/// INV-FSM-1: Every state transition must pass through [`AgentState::can_transition_to`]
/// (invoked by `Agent::transition`) — preventing invalid state mutations across the crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AgentState {
    Idle,
    Perceiving,
    Planning,
    ExecutingTool,
    Reflecting,
    Completed,
    Failed,
}

impl AgentState {
    /// Permitted state transitions table. `Failed` is reachable from any active state
    /// (failures can occur during any phase), but can only transition back to `Idle`.
    pub fn can_transition_to(self, next: AgentState) -> bool {
        use AgentState::*;
        match (self, next) {
            (Idle, Perceiving) => true,
            (Perceiving, Planning) => true,
            (Planning, ExecutingTool) => true,
            (Planning, Completed) => true,
            (ExecutingTool, Reflecting) => true,
            (Reflecting, Planning) => true,
            (Completed, Idle) => true,
            (Failed, Idle) => true,
            (_, Failed) => !matches!(self, Failed),
            _ => false,
        }
    }
}

impl std::fmt::Display for AgentState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

#[cfg(test)]
mod tests {
    use super::AgentState::*;

    #[test]
    fn nominal_cycle_is_allowed() {
        assert!(Idle.can_transition_to(Perceiving));
        assert!(Perceiving.can_transition_to(Planning));
        assert!(Planning.can_transition_to(ExecutingTool));
        assert!(ExecutingTool.can_transition_to(Reflecting));
        assert!(Reflecting.can_transition_to(Planning));
        assert!(Planning.can_transition_to(Completed));
        assert!(Completed.can_transition_to(Idle));
    }

    #[test]
    fn any_active_state_can_fail_except_failed_itself() {
        assert!(Perceiving.can_transition_to(Failed));
        assert!(Planning.can_transition_to(Failed));
        assert!(ExecutingTool.can_transition_to(Failed));
        assert!(Reflecting.can_transition_to(Failed));
        assert!(!Failed.can_transition_to(Failed));
    }

    #[test]
    fn skipping_a_state_is_rejected() {
        assert!(!Idle.can_transition_to(ExecutingTool));
        assert!(!Perceiving.can_transition_to(Completed));
    }
}
