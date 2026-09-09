// [WFGY] Zone: SAFE | λ: 0.1 | Fallbacks: 0 | Action: Layout error types definition
use taffy::TaffyError;

#[derive(Debug, thiserror::Error)]
pub enum LayoutError {
    #[error("taffy layout error: {0}")]
    Taffy(#[from] TaffyError),
    #[error("unknown node {0:?} in layout tree")]
    UnknownNode(taffy::NodeId),
}
