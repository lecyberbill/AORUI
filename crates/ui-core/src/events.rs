// [WFGY] Zone: TRANSIT | λ: 0.2 | Fallbacks: 0 | Action: Inbound interaction event definitions (human and/or agent)
use serde::{Deserialize, Serialize};

/// Inbound interaction events routed to application logic via `mpsc::Sender<UiEvent>`.
///
/// This channel is generic: human mouse/keyboard interactions and programmatic
/// control from `agent-runtime` produce the exact same event type. None carries
/// GPU state — only intentions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UiEvent {
    // --- Generic widget interactions (human or agent) ---
    PointerMoved { x: f32, y: f32 },
    PointerDown { x: f32, y: f32 },
    PointerUp { x: f32, y: f32 },
    ButtonClicked { widget_id: String },
    CheckboxToggled { widget_id: String, checked: bool },
    ToggleSwitched { widget_id: String, active: bool },
    SliderChanged { widget_id: String, value: f32 },
    TextChanged { widget_id: String, value: String },
    TextCursorMoved { widget_id: String, cursor: usize },
    TextSubmitted { widget_id: String, text: String },
    FocusChanged { widget_id: Option<String> },
    RadioSelected { group_id: String, selected_id: String },
    SegmentSelected { widget_id: String, selected_index: usize },
    ModalDismissed { modal_id: String },
    TabSelected { widget_id: String, tab_index: usize },
    ListItemSelected { widget_id: String, item_index: usize },
    ScrollChanged { widget_id: String, offset: [f32; 2] },
    WindowCloseRequested { widget_id: String },
    MenuItemClicked { menu_id: String, item_id: String },
    MenuToggled { menu_id: String, open: bool },
    SelectChanged { widget_id: String, selected_id: String },
    ToastDismissed { toast_id: String },
    SplitRatioChanged { widget_id: String, ratio: f32 },
    TreeNodeToggled { tree_id: String, node_id: String, expanded: bool },
    TreeNodeSelected { tree_id: String, node_id: String },
    NumberChanged { widget_id: String, value: f64 },
    PasswordRevealed { widget_id: String, revealed: bool },
    ContextMenuRequested { x: f32, y: f32, target_id: Option<String> },
    TableHeaderClicked { table_id: String, column_index: usize },
    TableRowSelected { table_id: String, row_index: usize },
    AccordionToggled { id: String, expanded: bool },
    BreadcrumbClicked { bar_id: String, index: usize, item_id: String },
    PageSelected { widget_id: String, page: usize },
    ColorSelected { widget_id: String, color: [f32; 4] },
    ColorChanged { widget_id: String, color: [f32; 4] },
    PaletteFoldToggled { palette_id: String, folded: bool },
    PaletteClosed { palette_id: String },
    PaletteMoved { palette_id: String, x: f32, y: f32 },
    PaletteResized { palette_id: String, width: f32, height: f32 },
    MediaPlayToggled { widget_id: String, playing: bool },
    MediaSeeked { widget_id: String, progress: f32 },
    CustomPaintPointerDown { widget_id: String, local_pos: [f32; 2], normalized_pos: [f32; 2] },
    CustomPaintPointerMove { widget_id: String, local_pos: [f32; 2], delta: [f32; 2] },

    // --- Agent runtime commands (preserved for agent-runtime) ---
    UserPromptSubmitted(String),
    StepApproved(String),
    AgentInterrupted,
    ParameterAdjusted { key: String, value: f32 },
}
