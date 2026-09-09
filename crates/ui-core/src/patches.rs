// [WFGY] Zone: TRANSIT | λ: 0.2 | Fallbacks: 0 | Action: Outbound UI state patch definitions
use serde::{Deserialize, Serialize};

/// Declarative UI state updates dispatched to the Render Thread via `crossbeam_channel::Receiver<UiPatch>`.
///
/// The Render Thread applies these patches in a non-blocking manner (`try_recv`
/// inside the frame loop); no patch produced on the application logic side (human or
/// agent) should ever block waiting for GPU resources.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UiPatch {
    // --- Generic widget state mutations ---
    WidgetTextSet { widget_id: String, text: String },
    WidgetCheckedSet { widget_id: String, checked: bool },
    WidgetVisibleSet { widget_id: String, visible: bool },
    TabActivated { widget_id: String, tab_index: usize },

    // --- Agent runtime state mutations (preserved for agent-runtime) ---
    StatusChanged { state: String, glow_hue: [f32; 4] },
    StepLogged { step_id: String, tool: String, status: StepStatus },
    MetricUpdated { key: String, value: f32 },
    ScratchpadAppended(String),
    ModalRequested { title: String, content: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    Running,
    Success,
    Failed(String),
}
