// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0 | Action: Intelligent Floating Overlay Placement with Auto-Flip & Collision Avoidance
use serde::{Deserialize, Serialize};

/// Target side and alignment for floating popovers, tooltips, and context menus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Placement {
    #[default]
    Top,
    TopStart,
    TopEnd,
    Bottom,
    BottomStart,
    BottomEnd,
    Left,
    LeftStart,
    LeftEnd,
    Right,
    RightStart,
    RightEnd,
}

impl Placement {
    /// Returns the opposite placement on collision (e.g. `Top` -> `Bottom`).
    pub fn opposite(&self) -> Self {
        match self {
            Placement::Top => Placement::Bottom,
            Placement::TopStart => Placement::BottomStart,
            Placement::TopEnd => Placement::BottomEnd,
            Placement::Bottom => Placement::Top,
            Placement::BottomStart => Placement::TopStart,
            Placement::BottomEnd => Placement::TopEnd,
            Placement::Left => Placement::Right,
            Placement::LeftStart => Placement::RightStart,
            Placement::LeftEnd => Placement::RightEnd,
            Placement::Right => Placement::Left,
            Placement::RightStart => Placement::LeftStart,
            Placement::RightEnd => Placement::LeftEnd,
        }
    }

    /// Whether this placement is vertical (`Top*` or `Bottom*`).
    pub fn is_vertical(&self) -> bool {
        matches!(
            self,
            Placement::Top
                | Placement::TopStart
                | Placement::TopEnd
                | Placement::Bottom
                | Placement::BottomStart
                | Placement::BottomEnd
        )
    }
}

/// Computed floating geometry result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FloatingRect {
    /// Final bounding rectangle `[x, y, width, height]`.
    pub rect: [f32; 4],
    /// Effective placement used (may differ from requested if flipped).
    pub placement: Placement,
    /// Arrow indicator coordinates `[x, y]`, if requested.
    pub arrow: Option<[f32; 2]>,
}

/// Computes intelligent floating geometry with screen boundary constraints and auto-flip.
pub fn compute_floating_rect(
    anchor: [f32; 4],      // [ax, ay, aw, ah]
    floating_size: [f32; 2], // [fw, fh]
    viewport: [f32; 4],     // [vx, vy, vw, vh]
    offset: f32,            // gap between anchor and floating element
    requested_placement: Placement,
    auto_flip: bool,
) -> FloatingRect {
    let [ax, ay, aw, ah] = anchor;
    let [fw, fh] = floating_size;
    let [vx, vy, vw, vh] = viewport;

    let try_placement = |pl: Placement| -> ([f32; 2], bool) {
        let (mut x, mut y) = match pl {
            Placement::Top => (ax + (aw - fw) * 0.5, ay - fh - offset),
            Placement::TopStart => (ax, ay - fh - offset),
            Placement::TopEnd => (ax + aw - fw, ay - fh - offset),
            Placement::Bottom => (ax + (aw - fw) * 0.5, ay + ah + offset),
            Placement::BottomStart => (ax, ay + ah + offset),
            Placement::BottomEnd => (ax + aw - fw, ay + ah + offset),
            Placement::Left => (ax - fw - offset, ay + (ah - fh) * 0.5),
            Placement::LeftStart => (ax - fw - offset, ay),
            Placement::LeftEnd => (ax - fw - offset, ay + ah - fh),
            Placement::Right => (ax + aw + offset, ay + (ah - fh) * 0.5),
            Placement::RightStart => (ax + aw + offset, ay),
            Placement::RightEnd => (ax + aw + offset, ay + ah - fh),
        };

        let mut overflows = false;
        if pl.is_vertical() {
            if y < vy || (y + fh) > (vy + vh) {
                overflows = true;
            }
            // Clamp cross axis within viewport
            x = x.clamp(vx + 4.0, (vx + vw - fw - 4.0).max(vx + 4.0));
        } else {
            if x < vx || (x + fw) > (vx + vw) {
                overflows = true;
            }
            // Clamp cross axis within viewport
            y = y.clamp(vy + 4.0, (vy + vh - fh - 4.0).max(vy + 4.0));
        }

        ([x, y], overflows)
    };

    let (pos, overflows) = try_placement(requested_placement);
    let (final_pos, final_pl) = if overflows && auto_flip {
        let flipped = requested_placement.opposite();
        let (f_pos, f_overflows) = try_placement(flipped);
        if !f_overflows {
            (f_pos, flipped)
        } else {
            (pos, requested_placement)
        }
    } else {
        (pos, requested_placement)
    };

    // Calculate center arrow coordinate
    let arrow = match final_pl {
        Placement::Top | Placement::TopStart | Placement::TopEnd => {
            Some([ax + aw * 0.5, final_pos[1] + fh])
        }
        Placement::Bottom | Placement::BottomStart | Placement::BottomEnd => {
            Some([ax + aw * 0.5, final_pos[1]])
        }
        Placement::Left | Placement::LeftStart | Placement::LeftEnd => {
            Some([final_pos[0] + fw, ay + ah * 0.5])
        }
        Placement::Right | Placement::RightStart | Placement::RightEnd => {
            Some([final_pos[0], ay + ah * 0.5])
        }
    };

    FloatingRect {
        rect: [final_pos[0], final_pos[1], fw, fh],
        placement: final_pl,
        arrow,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_floating_rect_basic_top() {
        let anchor = [100.0, 200.0, 80.0, 30.0];
        let floating_size = [120.0, 40.0];
        let viewport = [0.0, 0.0, 1000.0, 1000.0];

        let res = compute_floating_rect(
            anchor,
            floating_size,
            viewport,
            8.0,
            Placement::Top,
            true,
        );

        assert_eq!(res.placement, Placement::Top);
        // x centered: 100 + (80 - 120)/2 = 80
        assert_eq!(res.rect[0], 80.0);
        // y: 200 - 40 - 8 = 152
        assert_eq!(res.rect[1], 152.0);
    }

    #[test]
    fn test_floating_rect_auto_flip_when_near_top_edge() {
        let anchor = [100.0, 10.0, 80.0, 30.0]; // close to top y=0
        let floating_size = [120.0, 40.0];
        let viewport = [0.0, 0.0, 1000.0, 1000.0];

        let res = compute_floating_rect(
            anchor,
            floating_size,
            viewport,
            8.0,
            Placement::Top,
            true,
        );

        // Should flip to Bottom because top (10 - 40 - 8 = -38 < 0) overflows viewport
        assert_eq!(res.placement, Placement::Bottom);
        // y: 10 + 30 + 8 = 48
        assert_eq!(res.rect[1], 48.0);
    }

    #[test]
    fn test_floating_rect_clamping_on_cross_axis() {
        let anchor = [5.0, 200.0, 30.0, 30.0]; // close to left edge
        let floating_size = [100.0, 40.0];
        let viewport = [0.0, 0.0, 1000.0, 1000.0];

        let res = compute_floating_rect(
            anchor,
            floating_size,
            viewport,
            8.0,
            Placement::Top,
            true,
        );

        // x centered would be 5 + (30 - 100)/2 = -30, clamped to min 4.0
        assert!(res.rect[0] >= 4.0);
    }
}
