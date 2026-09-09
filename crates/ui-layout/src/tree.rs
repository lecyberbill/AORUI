// [WFGY] Zone: TRANSIT | λ: 0.2 | Fallbacks: 0 | Action: Generic layout tree decoupled from specific widget representations
use std::collections::HashMap;

use taffy::geometry::Point;
use taffy::prelude::*;
use taffy::{Layout, NodeId, TaffyTree};

use crate::error::LayoutError;

/// Wraps the Taffy tree and associates an arbitrary payload `V` with each node
/// (supplied by caller: widget id, widget type, or `()` if only geometry matters).
///
/// `ui-layout` knows no concrete widget model — `ui-widgets` chooses `V`
/// (e.g. `WidgetKind`). This decoupling prevents the layout engine from depending
/// on foreign component vocabularies.
///
/// INV-LAYOUT-1: [`LayoutTree::resolved_bounds`] and [`LayoutTree::hit_test`]
/// expose only **absolute** coordinates (screen space), never the relative-to-parent
/// offsets returned natively by Taffy.
pub struct LayoutTree<V> {
    taffy: TaffyTree<()>,
    payloads: HashMap<NodeId, V>,
}

impl<V> LayoutTree<V> {
    pub fn new() -> Self {
        Self { taffy: TaffyTree::new(), payloads: HashMap::new() }
    }

    /// Inserts a leaf node (no children) carrying payload `payload`.
    pub fn insert_leaf(&mut self, style: Style, payload: V) -> Result<NodeId, LayoutError> {
        let node = self.taffy.new_leaf(style)?;
        self.payloads.insert(node, payload);
        Ok(node)
    }

    /// Inserts a container (Flexbox/Grid) with existing children.
    pub fn insert_container(&mut self, style: Style, children: &[NodeId], payload: V) -> Result<NodeId, LayoutError> {
        let node = self.taffy.new_with_children(style, children)?;
        self.payloads.insert(node, payload);
        Ok(node)
    }

    /// Resolves Flexbox/Grid for the tree rooted at `root`, within available space `available_space`
    /// (typically window size).
    pub fn compute(&mut self, root: NodeId, available_space: Size<AvailableSpace>) -> Result<(), LayoutError> {
        self.taffy.compute_layout(root, available_space)?;
        Ok(())
    }

    pub fn payload(&self, node: NodeId) -> Option<&V> {
        self.payloads.get(&node)
    }

    /// Direct children of `node`, in insertion order (useful for draw-order traversal, parent before children).
    pub fn children(&self, node: NodeId) -> Result<Vec<NodeId>, LayoutError> {
        Ok(self.taffy.children(node)?)
    }

    /// Parent node of `node`, or `None` if `node` is a root.
    pub fn parent(&self, node: NodeId) -> Result<Option<NodeId>, LayoutError> {
        Ok(self.taffy.parent(node))
    }

    /// Absolute bounds `[x, y, width, height]` of each descendant of `root` (root included),
    /// computed by accumulating parent -> child relative offsets from Taffy.
    pub fn resolved_bounds(&self, root: NodeId) -> Result<HashMap<NodeId, [f32; 4]>, LayoutError> {
        let mut bounds = HashMap::new();
        self.accumulate_bounds(root, Point { x: 0.0, y: 0.0 }, &mut bounds)?;
        Ok(bounds)
    }

    fn accumulate_bounds(
        &self,
        node: NodeId,
        parent_origin: Point<f32>,
        out: &mut HashMap<NodeId, [f32; 4]>,
    ) -> Result<(), LayoutError> {
        let layout: &Layout = self.taffy.layout(node)?;
        let origin =
            Point { x: parent_origin.x + layout.location.x, y: parent_origin.y + layout.location.y };
        out.insert(node, [origin.x, origin.y, layout.size.width, layout.size.height]);

        for child in self.taffy.children(node)? {
            self.accumulate_bounds(child, origin, out)?;
        }
        Ok(())
    }

    /// Returns topmost node (deepest child, drawn last = topmost in standard paint order)
    /// containing `point` (screen space absolute coordinates), or `None`.
    pub fn hit_test(&self, root: NodeId, point: (f32, f32)) -> Result<Option<NodeId>, LayoutError> {
        let bounds = self.resolved_bounds(root)?;
        self.hit_test_recursive(root, point, &bounds)
    }

    fn hit_test_recursive(
        &self,
        node: NodeId,
        point: (f32, f32),
        bounds: &HashMap<NodeId, [f32; 4]>,
    ) -> Result<Option<NodeId>, LayoutError> {
        let Some(&[x, y, w, h]) = bounds.get(&node) else {
            return Err(LayoutError::UnknownNode(node));
        };
        if point.0 < x || point.0 > x + w || point.1 < y || point.1 > y + h {
            return Ok(None);
        }

        for child in self.taffy.children(node)?.into_iter().rev() {
            if let Some(hit) = self.hit_test_recursive(child, point, bounds)? {
                return Ok(Some(hit));
            }
        }
        Ok(Some(node))
    }
}

impl<V> Default for LayoutTree<V> {
    fn default() -> Self {
        Self::new()
    }
}
