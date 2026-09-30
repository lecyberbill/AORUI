// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0 | Action: Universal Cross-Widget Drag and Drop Engine
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::sync::Arc;
use crate::id::WidgetId;

/// Visual representation of the dragged item trailing the cursor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DragGhostSpec {
    pub label: String,
    pub width: f32,
    pub height: f32,
    pub icon: Option<String>,
}

/// Dynamic payload data carried during a drag operation.
#[derive(Clone)]
pub struct DragPayload {
    /// Semantic data type identifier (e.g. "application/json", "aorui/tab", "aorui/row").
    pub mime_type: String,
    /// String / serialized data content.
    pub text_data: String,
    /// Optional arbitrary typed payload.
    pub custom: Option<Arc<dyn Any + Send + Sync>>,
}

impl std::fmt::Debug for DragPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DragPayload")
            .field("mime_type", &self.mime_type)
            .field("text_data", &self.text_data)
            .finish()
    }
}

/// Active status of the drag & drop session.
#[derive(Debug, Clone, Default)]
pub enum DragState {
    #[default]
    Idle,
    Dragging {
        source_id: WidgetId,
        origin_pos: (f32, f32),
        current_pos: (f32, f32),
        payload: DragPayload,
        ghost: DragGhostSpec,
    },
    HoveringTarget {
        source_id: WidgetId,
        target_id: WidgetId,
        current_pos: (f32, f32),
        payload: DragPayload,
        ghost: DragGhostSpec,
        accepted: bool,
    },
    Dropped {
        source_id: WidgetId,
        target_id: WidgetId,
        payload: DragPayload,
    },
}

/// Central Drag and Drop coordinator for desktop UI workflows.
#[derive(Debug, Default)]
pub struct DragDropContext {
    state: DragState,
}

impl DragDropContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// Initiates a new drag operation.
    pub fn start_drag(
        &mut self,
        source_id: impl Into<WidgetId>,
        origin_pos: (f32, f32),
        payload: DragPayload,
        ghost: DragGhostSpec,
    ) {
        self.state = DragState::Dragging {
            source_id: source_id.into(),
            origin_pos,
            current_pos: origin_pos,
            payload,
            ghost,
        };
    }

    /// Updates cursor position during drag.
    pub fn update_cursor(&mut self, cursor_pos: (f32, f32)) {
        match &mut self.state {
            DragState::Dragging { current_pos, .. }
            | DragState::HoveringTarget { current_pos, .. } => {
                *current_pos = cursor_pos;
            }
            _ => {}
        }
    }

    /// Sets whether the active drag is hovering over a valid drop target.
    pub fn set_target_hover(&mut self, target_id: Option<WidgetId>, accepted: bool) {
        let (source, pos, payload, ghost) = match &self.state {
            DragState::Dragging {
                source_id,
                current_pos,
                payload,
                ghost,
                ..
            }
            | DragState::HoveringTarget {
                source_id,
                current_pos,
                payload,
                ghost,
                ..
            } => (source_id.clone(), *current_pos, payload.clone(), ghost.clone()),
            _ => return,
        };

        if let Some(target) = target_id {
            self.state = DragState::HoveringTarget {
                source_id: source,
                target_id: target,
                current_pos: pos,
                payload,
                ghost,
                accepted,
            };
        } else {
            self.state = DragState::Dragging {
                source_id: source,
                origin_pos: pos,
                current_pos: pos,
                payload,
                ghost,
            };
        }
    }

    /// Completes the drop operation if hovering an accepted target.
    pub fn drop(&mut self) -> Option<(WidgetId, WidgetId, DragPayload)> {
        match std::mem::take(&mut self.state) {
            DragState::HoveringTarget {
                source_id,
                target_id,
                payload,
                accepted,
                ..
            } => {
                if accepted {
                    Some((source_id, target_id, payload))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Cancels active drag operation.
    pub fn cancel(&mut self) {
        self.state = DragState::Idle;
    }

    /// Returns `true` if a drag is currently in progress.
    pub fn is_dragging(&self) -> bool {
        !matches!(self.state, DragState::Idle | DragState::Dropped { .. })
    }

    /// Returns current ghost preview geometry `[x, y, w, h]` if dragging.
    pub fn ghost_rect(&self) -> Option<[f32; 4]> {
        match &self.state {
            DragState::Dragging { current_pos, ghost, .. }
            | DragState::HoveringTarget { current_pos, ghost, .. } => {
                Some([
                    current_pos.0 - ghost.width * 0.5,
                    current_pos.1 - ghost.height * 0.5,
                    ghost.width,
                    ghost.height,
                ])
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drag_drop_workflow() {
        let mut dnd = DragDropContext::new();
        assert!(!dnd.is_dragging());

        let payload = DragPayload {
            mime_type: "text/plain".to_string(),
            text_data: "Nexus Node Data".to_string(),
            custom: None,
        };
        let ghost = DragGhostSpec {
            label: "Dragging Node".to_string(),
            width: 100.0,
            height: 30.0,
            icon: None,
        };

        dnd.start_drag(WidgetId::new("node_1"), (50.0, 50.0), payload, ghost);
        assert!(dnd.is_dragging());
        assert_eq!(dnd.ghost_rect(), Some([0.0, 35.0, 100.0, 30.0]));

        dnd.update_cursor((150.0, 200.0));
        assert_eq!(dnd.ghost_rect(), Some([100.0, 185.0, 100.0, 30.0]));

        // Hover over target
        dnd.set_target_hover(Some(WidgetId::new("drop_zone_target")), true);
        assert!(dnd.is_dragging());

        // Perform drop
        let result = dnd.drop();
        assert!(result.is_some());
        let (source, target, p) = result.unwrap();
        assert_eq!(source.as_str(), "node_1");
        assert_eq!(target.as_str(), "drop_zone_target");
        assert_eq!(p.text_data, "Nexus Node Data");
        assert!(!dnd.is_dragging());
    }
}
