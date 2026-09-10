// [WFGY] Zone: SAFE | λ: 0.1 | Fallbacks: 0 | Action: 2D Drawing commands & Painter builder for CustomPaint surface

/// Declarative 2D drawing command for the [`CustomPaint`](crate::kind::WidgetKind::CustomPaint) surface.
#[derive(Debug, Clone, PartialEq)]
pub enum PaintCommand {
    /// Draw a line segment between two points.
    Line {
        from: [f32; 2],
        to: [f32; 2],
        stroke_width: f32,
        color: [f32; 4],
    },
    /// Draw a rectangle with optional rounded corners, fill, and border stroke.
    Rect {
        bounds: [f32; 4], // [x, y, w, h] in canvas local coordinates
        corner_radius: f32,
        fill: Option<[f32; 4]>,
        stroke: Option<([f32; 4], f32)>, // (color, stroke_width)
    },
    /// Draw a circle centered at `center` with `radius`.
    Circle {
        center: [f32; 2],
        radius: f32,
        fill: Option<[f32; 4]>,
        stroke: Option<([f32; 4], f32)>,
    },
    /// Draw a cubic Bézier curve evaluated into smooth GPU line segments.
    Bezier {
        start: [f32; 2],
        ctrl1: [f32; 2],
        ctrl2: [f32; 2],
        end: [f32; 2],
        stroke_width: f32,
        color: [f32; 4],
    },
    /// Draw a series of connected line segments (e.g. waveform, polygon, freehand stroke).
    Polyline {
        points: Vec<[f32; 2]>,
        stroke_width: f32,
        color: [f32; 4],
        closed: bool,
    },
    /// Draw a text label placed at `position` relative to the canvas origin.
    Text {
        position: [f32; 2],
        text: String,
        font_size: f32,
        color: [f32; 4],
    },
}

/// Fluent builder for developers to construct 2D vector drawings easily.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Painter {
    pub commands: Vec<PaintCommand>,
}

impl Painter {
    /// Create a new empty painter.
    #[inline]
    pub fn new() -> Self {
        Self { commands: Vec::new() }
    }

    /// Add a line segment.
    pub fn line(&mut self, from: [f32; 2], to: [f32; 2], stroke_width: f32, color: [f32; 4]) -> &mut Self {
        self.commands.push(PaintCommand::Line { from, to, stroke_width, color });
        self
    }

    /// Add a rectangle.
    pub fn rect(
        &mut self,
        bounds: [f32; 4],
        corner_radius: f32,
        fill: Option<[f32; 4]>,
        stroke: Option<([f32; 4], f32)>,
    ) -> &mut Self {
        self.commands.push(PaintCommand::Rect { bounds, corner_radius, fill, stroke });
        self
    }

    /// Add a circle.
    pub fn circle(
        &mut self,
        center: [f32; 2],
        radius: f32,
        fill: Option<[f32; 4]>,
        stroke: Option<([f32; 4], f32)>,
    ) -> &mut Self {
        self.commands.push(PaintCommand::Circle { center, radius, fill, stroke });
        self
    }

    /// Add a cubic Bézier curve.
    pub fn bezier(
        &mut self,
        start: [f32; 2],
        ctrl1: [f32; 2],
        ctrl2: [f32; 2],
        end: [f32; 2],
        stroke_width: f32,
        color: [f32; 4],
    ) -> &mut Self {
        self.commands.push(PaintCommand::Bezier { start, ctrl1, ctrl2, end, stroke_width, color });
        self
    }

    /// Add a connected polyline.
    pub fn polyline(
        &mut self,
        points: Vec<[f32; 2]>,
        stroke_width: f32,
        color: [f32; 4],
        closed: bool,
    ) -> &mut Self {
        self.commands.push(PaintCommand::Polyline { points, stroke_width, color, closed });
        self
    }

    /// Add a text label.
    pub fn text(
        &mut self,
        position: [f32; 2],
        text: impl Into<String>,
        font_size: f32,
        color: [f32; 4],
    ) -> &mut Self {
        self.commands.push(PaintCommand::Text {
            position,
            text: text.into(),
            font_size,
            color,
        });
        self
    }

    /// Consume builder and return the vector of paint commands.
    #[inline]
    pub fn finish(self) -> Vec<PaintCommand> {
        self.commands
    }
}

/// Helper function to evaluate a cubic Bézier point at parameter `t` in `[0.0, 1.0]`.
#[inline]
pub fn eval_cubic_bezier(p0: [f32; 2], p1: [f32; 2], p2: [f32; 2], p3: [f32; 2], t: f32) -> [f32; 2] {
    let t2 = t * t;
    let t3 = t2 * t;
    let u = 1.0 - t;
    let u2 = u * u;
    let u3 = u2 * u;

    [
        u3 * p0[0] + 3.0 * u2 * t * p1[0] + 3.0 * u * t2 * p2[0] + t3 * p3[0],
        u3 * p0[1] + 3.0 * u2 * t * p1[1] + 3.0 * u * t2 * p2[1] + t3 * p3[1],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_painter_builder() {
        let mut p = Painter::new();
        p.line([0.0, 0.0], [10.0, 10.0], 2.0, [1.0, 1.0, 1.0, 1.0])
            .circle([50.0, 50.0], 25.0, Some([0.0, 1.0, 0.0, 1.0]), None)
            .text([10.0, 20.0], "Hello Canvas", 14.0, [1.0, 0.0, 0.0, 1.0]);

        let cmds = p.finish();
        assert_eq!(cmds.len(), 3);
    }

    #[test]
    fn test_cubic_bezier_endpoints() {
        let p0 = [0.0, 0.0];
        let p1 = [10.0, 50.0];
        let p2 = [90.0, 50.0];
        let p3 = [100.0, 100.0];

        let start = eval_cubic_bezier(p0, p1, p2, p3, 0.0);
        let end = eval_cubic_bezier(p0, p1, p2, p3, 1.0);

        assert!((start[0] - 0.0).abs() < 1e-5);
        assert!((start[1] - 0.0).abs() < 1e-5);
        assert!((end[0] - 100.0).abs() < 1e-5);
        assert!((end[1] - 100.0).abs() < 1e-5);
    }
}
