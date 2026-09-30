// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0 | Action: Modular Studio Docking, Split Tiles & Flexible Panel Layout Engine
use serde::{Deserialize, Serialize};

/// Orientation of a split separator between two docked panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitOrientation {
    Horizontal, // Left | Right
    Vertical,   // Top / Bottom
}

/// Docking direction relative to an existing panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DockDirection {
    Left,
    Right,
    Above,
    Below,
    Center, // Add as a tab into the same container
}

/// A tab within a docked panel leaf.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DockTab {
    pub id: String,
    pub title: String,
    pub icon: Option<String>,
    pub closable: bool,
}

impl DockTab {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            icon: None,
            closable: true,
        }
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn not_closable(mut self) -> Self {
        self.closable = false;
        self
    }
}

/// A node in the binary space partitioning (BSP) dock hierarchy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DockNode {
    /// A single panel holding one or more tabs.
    Leaf {
        id: String,
        tabs: Vec<DockTab>,
        active_tab: usize,
    },
    /// A split dividing two child sub-trees.
    Split {
        id: String,
        orientation: SplitOrientation,
        ratio: f32, // [0.05, 0.95]
        first: Box<DockNode>,
        second: Box<DockNode>,
    },
}

impl DockNode {
    /// Creates a new leaf node containing initial tabs.
    pub fn leaf(id: impl Into<String>, tabs: Vec<DockTab>) -> Self {
        Self::Leaf {
            id: id.into(),
            tabs,
            active_tab: 0,
        }
    }

    /// Recursively finds a leaf node by its ID.
    pub fn find_leaf(&self, leaf_id: &str) -> Option<&DockNode> {
        match self {
            DockNode::Leaf { id, .. } if id == leaf_id => Some(self),
            DockNode::Split { first, second, .. } => first
                .find_leaf(leaf_id)
                .or_else(|| second.find_leaf(leaf_id)),
            _ => None,
        }
    }

    /// Recursively finds a mutable leaf node by its ID.
    pub fn find_leaf_mut(&mut self, leaf_id: &str) -> Option<&mut DockNode> {
        match self {
            DockNode::Leaf { id, .. } if id == leaf_id => Some(self),
            DockNode::Split { first, second, .. } => {
                if first.find_leaf(leaf_id).is_some() {
                    first.find_leaf_mut(leaf_id)
                } else {
                    second.find_leaf_mut(leaf_id)
                }
            }
            _ => None,
        }
    }
}

/// Top-level workspace docking container managing multi-panel layouts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DockArea {
    pub root: DockNode,
}

impl DockArea {
    /// Creates a new dock area with a single root leaf panel.
    pub fn new(initial_leaf_id: impl Into<String>, tabs: Vec<DockTab>) -> Self {
        Self {
            root: DockNode::leaf(initial_leaf_id, tabs),
        }
    }

    /// Docks a new tab relative to an existing target leaf.
    pub fn dock_tab(
        &mut self,
        target_leaf_id: &str,
        tab: DockTab,
        direction: DockDirection,
        new_leaf_id: impl Into<String>,
    ) -> bool {
        let new_leaf_id = new_leaf_id.into();
        self.dock_node_internal(
            &mut self.root.clone(),
            target_leaf_id,
            tab,
            direction,
            new_leaf_id,
        )
    }

    fn dock_node_internal(
        &mut self,
        _current: &mut DockNode,
        target_leaf_id: &str,
        tab: DockTab,
        direction: DockDirection,
        new_leaf_id: String,
    ) -> bool {
        if direction == DockDirection::Center {
            if let Some(DockNode::Leaf {
                tabs, active_tab, ..
            }) = self.root.find_leaf_mut(target_leaf_id)
            {
                tabs.push(tab);
                *active_tab = tabs.len() - 1;
                return true;
            }
            return false;
        }

        // Split target leaf
        let mut replace_target = |node: &mut DockNode| -> bool {
            if let DockNode::Leaf {
                id,
                tabs,
                active_tab,
            } = node
            {
                if id == target_leaf_id {
                    let existing_leaf = DockNode::Leaf {
                        id: id.clone(),
                        tabs: tabs.clone(),
                        active_tab: *active_tab,
                    };
                    let new_leaf = DockNode::Leaf {
                        id: new_leaf_id.clone(),
                        tabs: vec![tab.clone()],
                        active_tab: 0,
                    };

                    let (orientation, first, second) = match direction {
                        DockDirection::Left => {
                            (SplitOrientation::Horizontal, new_leaf, existing_leaf)
                        }
                        DockDirection::Right => {
                            (SplitOrientation::Horizontal, existing_leaf, new_leaf)
                        }
                        DockDirection::Above => {
                            (SplitOrientation::Vertical, new_leaf, existing_leaf)
                        }
                        DockDirection::Below => {
                            (SplitOrientation::Vertical, existing_leaf, new_leaf)
                        }
                        DockDirection::Center => unreachable!(),
                    };

                    *node = DockNode::Split {
                        id: format!("split_{}", new_leaf_id),
                        orientation,
                        ratio: 0.5,
                        first: Box::new(first),
                        second: Box::new(second),
                    };
                    return true;
                }
            }
            false
        };

        fn traverse(
            node: &mut DockNode,
            target_id: &str,
            replacer: &mut dyn FnMut(&mut DockNode) -> bool,
        ) -> bool {
            match node {
                DockNode::Leaf { id, .. } if id == target_id => replacer(node),
                DockNode::Split { first, second, .. } => {
                    traverse(first, target_id, replacer) || traverse(second, target_id, replacer)
                }
                _ => false,
            }
        }

        traverse(&mut self.root, target_leaf_id, &mut replace_target)
    }

    /// Selects an active tab by index in a leaf.
    pub fn select_tab(&mut self, leaf_id: &str, tab_index: usize) -> bool {
        if let Some(DockNode::Leaf {
            tabs, active_tab, ..
        }) = self.root.find_leaf_mut(leaf_id)
        {
            if tab_index < tabs.len() {
                *active_tab = tab_index;
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dock_area_creation_and_tab_addition() {
        let mut dock = DockArea::new(
            "main_panel",
            vec![DockTab::new("tab_editor", "Code Editor")],
        );

        let ok = dock.dock_tab(
            "main_panel",
            DockTab::new("tab_terminal", "Terminal"),
            DockDirection::Center,
            "panel_term",
        );
        assert!(ok);

        if let Some(DockNode::Leaf {
            tabs, active_tab, ..
        }) = dock.root.find_leaf("main_panel")
        {
            assert_eq!(tabs.len(), 2);
            assert_eq!(*active_tab, 1);
        } else {
            panic!("Expected leaf main_panel");
        }
    }

    #[test]
    fn test_dock_area_split_docking() {
        let mut dock = DockArea::new("editor_panel", vec![DockTab::new("tab_code", "Main.rs")]);
        let ok = dock.dock_tab(
            "editor_panel",
            DockTab::new("tab_radar", "Radar"),
            DockDirection::Right,
            "radar_panel",
        );
        assert!(ok);

        match &dock.root {
            DockNode::Split {
                orientation,
                ratio,
                first,
                second,
                ..
            } => {
                assert_eq!(*orientation, SplitOrientation::Horizontal);
                assert_eq!(*ratio, 0.5);
                assert!(first.find_leaf("editor_panel").is_some());
                assert!(second.find_leaf("radar_panel").is_some());
            }
            _ => panic!("Expected root split node"),
        }
    }
}
