// [WFGY] Zone: TRANSIT | λ: 0.2 | Fallbacks: 0 | Action: Generic LayoutTree decoupled from specific widget representations
//! Taffy flexbox/grid integration for `cyber-agent-ui`.
//!
//! Invariant INV-LAYOUT-1: every coordinate exposed outside this crate
//! (`resolved_bounds`, `hit_test`) is **absolute** (screen space), never
//! relative to parent.
//!
//! `ui-layout` does not depend on any specific widget model: [`LayoutTree`]
//! is generic over the payload associated with each node (see
//! `ui-widgets`, which attaches its own types).

pub mod error;
pub mod tree;

pub use error::LayoutError;
pub use tree::LayoutTree;

pub use taffy::prelude::*;
pub use taffy::style_helpers::percent;
pub use taffy::NodeId;

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestPayload {
        Leaf,
        Container,
    }

    #[test]
    fn resolved_bounds_are_absolute_not_relative_to_parent() {
        let mut tree: LayoutTree<TestPayload> = LayoutTree::new();

        let child_style = Style {
            size: Size { width: length(50.0), height: length(50.0) },
            ..Default::default()
        };
        let child = tree.insert_leaf(child_style, TestPayload::Leaf).unwrap();

        let parent_style = Style {
            size: Size { width: length(200.0), height: length(200.0) },
            padding: Rect {
                left: length(30.0),
                top: length(20.0),
                right: length(0.0),
                bottom: length(0.0),
            },
            ..Default::default()
        };
        let root = tree.insert_container(parent_style, &[child], TestPayload::Container).unwrap();

        tree.compute(root, Size::MAX_CONTENT).unwrap();

        let bounds = tree.resolved_bounds(root).unwrap();
        let root_bounds = bounds[&root];
        let child_bounds = bounds[&child];

        assert_eq!(root_bounds, [0.0, 0.0, 200.0, 200.0]);
        // Child node must be shifted by parent offset (30, 20) + padding,
        // NOT merely equal to its raw relative offset (0, 0) returned by Taffy.
        assert_eq!(child_bounds, [30.0, 20.0, 50.0, 50.0]);
    }

    #[test]
    fn hit_test_prefers_topmost_child_over_container() {
        let mut tree: LayoutTree<TestPayload> = LayoutTree::new();

        let child_style = Style {
            size: Size { width: length(50.0), height: length(50.0) },
            ..Default::default()
        };
        let child = tree.insert_leaf(child_style, TestPayload::Leaf).unwrap();

        let parent_style = Style {
            size: Size { width: length(200.0), height: length(200.0) },
            ..Default::default()
        };
        let root = tree.insert_container(parent_style, &[child], TestPayload::Container).unwrap();
        tree.compute(root, Size::MAX_CONTENT).unwrap();

        assert_eq!(tree.hit_test(root, (10.0, 10.0)).unwrap(), Some(child));
        assert_eq!(tree.hit_test(root, (150.0, 150.0)).unwrap(), Some(root));
        assert_eq!(tree.hit_test(root, (500.0, 500.0)).unwrap(), None);
    }
}
