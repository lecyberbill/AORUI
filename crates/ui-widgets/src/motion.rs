// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0 | Action: High-performance Spring Physics & Motion Easing Engine for UI micro-interactions
use serde::{Deserialize, Serialize};

/// Easing functions for smooth parametric animations ($t \in [0.0, 1.0]$).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Easing {
    #[default]
    Linear,
    EaseInQuad,
    EaseOutQuad,
    EaseInOutQuad,
    EaseInCubic,
    EaseOutCubic,
    EaseInOutCubic,
    EaseOutQuart,
    EaseOutExpo,
    EaseOutElastic,
    EaseOutBounce,
}

impl Easing {
    /// Evaluates easing curve at progress $t \in [0.0, 1.0]$.
    pub fn evaluate(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            Easing::EaseInQuad => t * t,
            Easing::EaseOutQuad => t * (2.0 - t),
            Easing::EaseInOutQuad => {
                if t < 0.5 {
                    2.0 * t * t
                } else {
                    -1.0 + (4.0 - 2.0 * t) * t
                }
            }
            Easing::EaseInCubic => t * t * t,
            Easing::EaseOutCubic => {
                let p = t - 1.0;
                p * p * p + 1.0
            }
            Easing::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    let p = 2.0 * t - 2.0;
                    0.5 * p * p * p + 1.0
                }
            }
            Easing::EaseOutQuart => {
                let p = t - 1.0;
                1.0 - p * p * p * p
            }
            Easing::EaseOutExpo => {
                if t == 1.0 {
                    1.0
                } else {
                    1.0 - 2.0f32.powf(-10.0 * t)
                }
            }
            Easing::EaseOutElastic => {
                if t == 0.0 {
                    0.0
                } else if t == 1.0 {
                    1.0
                } else {
                    let p = 0.3;
                    let s = p / 4.0;
                    2.0f32.powf(-10.0 * t) * ((t - s) * (2.0 * std::f32::consts::PI) / p).sin()
                        + 1.0
                }
            }
            Easing::EaseOutBounce => {
                let n1 = 7.5625;
                let d1 = 2.75;
                if t < 1.0 / d1 {
                    n1 * t * t
                } else if t < 2.0 / d1 {
                    let t = t - 1.5 / d1;
                    n1 * t * t + 0.75
                } else if t < 2.5 / d1 {
                    let t = t - 2.25 / d1;
                    n1 * t * t + 0.9375
                } else {
                    let t = t - 2.625 / d1;
                    n1 * t * t + 0.984375
                }
            }
        }
    }
}

/// Harmonic oscillator spring configuration ($m \cdot x'' + c \cdot x' + k \cdot (x - x_0) = 0$).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Spring {
    /// Spring stiffness constant ($k$). Higher = faster, snappier oscillations.
    pub stiffness: f32,
    /// Damping coefficient ($c$). Higher = less oscillation/overshoot.
    pub damping: f32,
    /// Mass of the animated object ($m$). Default = 1.0.
    pub mass: f32,
}

impl Default for Spring {
    fn default() -> Self {
        Self::smooth()
    }
}

impl Spring {
    /// New custom spring parameters.
    pub fn new(stiffness: f32, damping: f32, mass: f32) -> Self {
        Self {
            stiffness: stiffness.max(0.1),
            damping: damping.max(0.0),
            mass: mass.max(0.01),
        }
    }

    /// Smooth, natural critically-damped spring (Apple/macOS style).
    pub fn smooth() -> Self {
        Self {
            stiffness: 170.0,
            damping: 26.0,
            mass: 1.0,
        }
    }

    /// Snappy, fast responsive spring for UI buttons and toggles.
    pub fn snappy() -> Self {
        Self {
            stiffness: 300.0,
            damping: 32.0,
            mass: 1.0,
        }
    }

    /// Bouncy spring with perceptible overshoot for celebratory/tactile micro-interactions.
    pub fn bouncy() -> Self {
        Self {
            stiffness: 180.0,
            damping: 14.0,
            mass: 1.0,
        }
    }

    /// Gentle, slow and graceful spring for modal sheets and background shifts.
    pub fn gentle() -> Self {
        Self {
            stiffness: 120.0,
            damping: 18.0,
            mass: 1.0,
        }
    }

    /// Advances the spring simulation by `dt` seconds using semi-implicit Euler integration.
    /// Returns `(new_position, new_velocity)`.
    pub fn step(&self, dt: f32, current: f32, target: f32, velocity: f32) -> (f32, f32) {
        let dt = dt.clamp(0.0, 0.05); // Cap delta time to 50ms for numerical stability
        let displacement = current - target;
        let spring_force = -self.stiffness * displacement;
        let damping_force = -self.damping * velocity;
        let acceleration = (spring_force + damping_force) / self.mass;

        let new_velocity = velocity + acceleration * dt;
        let new_position = current + new_velocity * dt;

        (new_position, new_velocity)
    }

    /// Checks whether the spring simulation has settled within tolerance.
    pub fn is_settled(&self, current: f32, target: f32, velocity: f32, tolerance: f32) -> bool {
        (current - target).abs() < tolerance && velocity.abs() < tolerance
    }
}

/// Stateful animated scalar driven by spring physics or easing curves.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimatedValue {
    pub current: f32,
    pub target: f32,
    pub velocity: f32,
    pub spring: Spring,
    pub tolerance: f32,
}

impl AnimatedValue {
    /// Creates a new animated value initialized at `initial`.
    pub fn new(initial: f32) -> Self {
        Self {
            current: initial,
            target: initial,
            velocity: 0.0,
            spring: Spring::smooth(),
            tolerance: 0.001,
        }
    }

    /// Custom spring configuration.
    pub fn with_spring(mut self, spring: Spring) -> Self {
        self.spring = spring;
        self
    }

    /// Sets the destination target value.
    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Instantly snaps the current and target value to `value`, resetting velocity.
    pub fn snap_to(&mut self, value: f32) {
        self.current = value;
        self.target = value;
        self.velocity = 0.0;
    }

    /// Advances simulation by `dt` seconds. Returns `current`.
    pub fn update(&mut self, dt: f32) -> f32 {
        if self.is_settled() {
            self.current = self.target;
            self.velocity = 0.0;
            return self.current;
        }

        let (pos, vel) = self
            .spring
            .step(dt, self.current, self.target, self.velocity);
        self.current = pos;
        self.velocity = vel;

        if self.is_settled() {
            self.current = self.target;
            self.velocity = 0.0;
        }

        self.current
    }

    /// Returns the current animated scalar value.
    pub fn value(&self) -> f32 {
        self.current
    }

    /// Returns true if the value has reached its target and velocity is negligible.
    pub fn is_settled(&self) -> bool {
        self.spring
            .is_settled(self.current, self.target, self.velocity, self.tolerance)
    }
}

/// Linear interpolation helper for floats.
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

/// Linear interpolation helper for 4-channel colors `[r, g, b, a]`.
pub fn lerp_color(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_easing_boundaries() {
        let easings = [
            Easing::Linear,
            Easing::EaseInQuad,
            Easing::EaseOutQuad,
            Easing::EaseInOutQuad,
            Easing::EaseInCubic,
            Easing::EaseOutCubic,
            Easing::EaseInOutCubic,
            Easing::EaseOutQuart,
            Easing::EaseOutExpo,
            Easing::EaseOutElastic,
            Easing::EaseOutBounce,
        ];

        for e in easings {
            assert!((e.evaluate(0.0) - 0.0).abs() < 0.001);
            assert!((e.evaluate(1.0) - 1.0).abs() < 0.001);
            assert!(e.evaluate(0.5) >= 0.0 && e.evaluate(0.5) <= 1.5);
        }
    }

    #[test]
    fn test_spring_convergence() {
        let mut anim = AnimatedValue::new(0.0).with_spring(Spring::snappy());
        anim.set_target(100.0);

        let dt = 1.0 / 60.0;
        for _ in 0..120 {
            anim.update(dt);
        }

        assert!(
            anim.is_settled(),
            "Spring should converge to target within 2 seconds"
        );
        assert_eq!(anim.current, 100.0);
    }

    #[test]
    fn test_color_lerp() {
        let c1 = [0.0, 0.0, 0.0, 1.0];
        let c2 = [1.0, 0.5, 0.25, 1.0];
        let mid = lerp_color(c1, c2, 0.5);

        assert_eq!(mid, [0.5, 0.25, 0.125, 1.0]);
    }
}
