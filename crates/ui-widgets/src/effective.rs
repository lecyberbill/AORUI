// [WFGY] Zone: RISK | λ: 0.2 | Fallbacks: 0 | Action: Effective bounds calculation (cumulative scroll offsets + clipping) for rendering and hit-testing
use std::collections::HashMap;

use ui_layout::{LayoutError, NodeId};

use crate::kind::WidgetKind;
use crate::tree::WidgetTree;

/// Unbounded region: covers a region significantly larger than any realistic display,
/// serving as the default clip rectangle for widgets without a `ScrollView` ancestor.
pub const NO_CLIP: [f32; 4] = [-1.0e7, -1.0e7, 2.0e7, 2.0e7];

/// Effective bounds of a layout node: its true visual screen position after applying
/// cumulative scroll offsets from ancestor `ScrollView`s (`visual`), and the visible
/// viewport boundary region formed by intersecting all ancestor scroll clipping rects (`clip`).
///
/// INV-GPU-2: Shared between [`crate::frame`] (rendering) and [`crate::interaction`] (hit-testing)
/// ensuring that widgets scrolled out of view are neither rendered nor clickable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectiveBounds {
    pub visual: [f32; 4],
    pub clip: [f32; 4],
}

impl EffectiveBounds {
    /// Intersection of `visual` and `clip`: the true visible screen rectangle of this node.
    pub fn visible_rect(&self) -> [f32; 4] {
        intersect(self.visual, self.clip)
    }
}

pub(crate) fn intersect(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    let x0 = a[0].max(b[0]);
    let y0 = a[1].max(b[1]);
    let x1 = (a[0] + a[2]).min(b[0] + b[2]);
    let y1 = (a[1] + a[3]).min(b[1] + b[3]);
    [x0, y0, (x1 - x0).max(0.0), (y1 - y0).max(0.0)]
}

impl WidgetTree {
    /// Computes [`EffectiveBounds`] for all nodes descending from `root` (including root).
    /// Nodes without a `ScrollView` ancestor carry `clip = NO_CLIP`.
    pub fn effective_bounds(&self, root: NodeId) -> Result<HashMap<NodeId, EffectiveBounds>, LayoutError> {
        let raw = self.resolved_bounds(root)?;
        let mut out = HashMap::new();
        self.walk_effective(root, &raw, [0.0, 0.0], NO_CLIP, &mut out)?;
        Ok(out)
    }

    fn walk_effective(
        &self,
        node: NodeId,
        raw: &HashMap<NodeId, [f32; 4]>,
        offset: [f32; 2],
        clip: [f32; 4],
        out: &mut HashMap<NodeId, EffectiveBounds>,
    ) -> Result<(), LayoutError> {
        let raw_bounds = raw[&node];
        let visual = [raw_bounds[0] - offset[0], raw_bounds[1] - offset[1], raw_bounds[2], raw_bounds[3]];
        out.insert(node, EffectiveBounds { visual, clip });

        // Children of a ScrollView inherit accumulated scroll offsets and a clip
        // rectangle intersected with its visual viewport.
        let (child_offset, child_clip) = match self.layout().payload(node) {
            Some(WidgetKind::ScrollView { offset: scroll_offset, .. }) => {
                ([offset[0] + scroll_offset[0], offset[1] + scroll_offset[1]], intersect(clip, visual))
            }
            _ => (offset, clip),
        };

        for child in self.layout().children(node)? {
            self.walk_effective(child, raw, child_offset, child_clip, out)?;
        }
        Ok(())
    }

    /// Scroll-aware hit testing: a pointer coordinate falling outside the visible
    /// rectangle of a widget's ancestor `ScrollView` will not hit the widget.
    pub fn hit_test_effective(&self, root: NodeId, point: (f32, f32)) -> Result<Option<NodeId>, LayoutError> {
        let effective = self.effective_bounds(root)?;
        self.hit_test_effective_recursive(root, point, &effective)
    }

    fn hit_test_effective_recursive(
        &self,
        node: NodeId,
        point: (f32, f32),
        effective: &HashMap<NodeId, EffectiveBounds>,
    ) -> Result<Option<NodeId>, LayoutError> {
        let visible = effective[&node].visible_rect();
        let inside = point.0 >= visible[0]
            && point.0 <= visible[0] + visible[2]
            && point.1 >= visible[1]
            && point.1 <= visible[1] + visible[3];
        if !inside {
            return Ok(None);
        }

        for child in self.layout().children(node)?.into_iter().rev() {
            if let Some(hit) = self.hit_test_effective_recursive(child, point, effective)? {
                return Ok(Some(hit));
            }
        }
        Ok(Some(node))
    }

    /// Finds the nearest ancestor `ScrollView` containing `point` (either the `ScrollView` itself
    /// or one containing the deepest hovered child widget).
    pub fn scrollview_at(&self, root: NodeId, point: (f32, f32)) -> Result<Option<NodeId>, LayoutError> {
        let effective = self.effective_bounds(root)?;
        let mut found = None;
        self.scrollview_at_recursive(root, point, &effective, &mut found)?;
        Ok(found)
    }

    fn scrollview_at_recursive(
        &self,
        node: NodeId,
        point: (f32, f32),
        effective: &HashMap<NodeId, EffectiveBounds>,
        found: &mut Option<NodeId>,
    ) -> Result<(), LayoutError> {
        let visible = effective[&node].visible_rect();
        let inside = point.0 >= visible[0]
            && point.0 <= visible[0] + visible[2]
            && point.1 >= visible[1]
            && point.1 <= visible[1] + visible[3];
        if !inside {
            return Ok(());
        }

        if matches!(self.layout().payload(node), Some(WidgetKind::ScrollView { .. })) {
            *found = Some(node);
        }

        for child in self.layout().children(node)? {
            self.scrollview_at_recursive(child, point, effective, found)?;
        }
        Ok(())
    }
}
