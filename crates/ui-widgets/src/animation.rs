// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0 | Action: Automatic Widget State Transitions & Micro-Animations Engine
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::id::WidgetId;
use crate::motion::{AnimatedValue, Spring};

/// Per-widget micro-animation tracker for smooth visual state transitions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetAnimationState {
    /// Smooth hover intensity [0.0 = idle, 1.0 = fully hovered].
    pub hover: AnimatedValue,
    /// Smooth press/down intensity [0.0 = released, 1.0 = fully pressed].
    pub press: AnimatedValue,
    /// Smooth keyboard/tab focus ring intensity [0.0 = unfocused, 1.0 = focused].
    pub focus: AnimatedValue,
    /// Smooth physical elevation [0.0 = flat, 1.0+ = lifted].
    pub elevation: AnimatedValue,
}

impl Default for WidgetAnimationState {
    fn default() -> Self {
        Self {
            hover: AnimatedValue::new(0.0).with_spring(Spring::smooth()),
            press: AnimatedValue::new(0.0).with_spring(Spring::snappy()),
            focus: AnimatedValue::new(0.0).with_spring(Spring::snappy()),
            elevation: AnimatedValue::new(0.0).with_spring(Spring::gentle()),
        }
    }
}

impl WidgetAnimationState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Advances all spring physics simulations by delta time `dt` seconds.
    pub fn update(&mut self, dt: f32) {
        self.hover.update(dt);
        self.press.update(dt);
        self.focus.update(dt);
        self.elevation.update(dt);
    }

    /// Returns `true` if all state transitions have converged to their targets.
    pub fn is_idle(&self) -> bool {
        self.hover.is_settled()
            && self.press.is_settled()
            && self.focus.is_settled()
            && self.elevation.is_settled()
    }
}

/// Central transition engine managing smooth visual states for all interactive widgets.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransitionEngine {
    states: HashMap<WidgetId, WidgetAnimationState>,
}

impl TransitionEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Advances all active widget transitions by `dt` seconds.
    pub fn update(&mut self, dt: f32) {
        for state in self.states.values_mut() {
            state.update(dt);
        }
    }

    /// Sets the hover state target for a widget.
    pub fn set_hovered(&mut self, id: &WidgetId, hovered: bool) {
        let entry = self.states.entry(id.clone()).or_default();
        entry.hover.set_target(if hovered { 1.0 } else { 0.0 });
    }

    /// Sets the pressed state target for a widget.
    pub fn set_pressed(&mut self, id: &WidgetId, pressed: bool) {
        let entry = self.states.entry(id.clone()).or_default();
        entry.press.set_target(if pressed { 1.0 } else { 0.0 });
    }

    /// Sets the keyboard focus ring target for a widget.
    pub fn set_focused(&mut self, id: &WidgetId, focused: bool) {
        let entry = self.states.entry(id.clone()).or_default();
        entry.focus.set_target(if focused { 1.0 } else { 0.0 });
    }

    /// Sets the elevation level target for a widget.
    pub fn set_elevation(&mut self, id: &WidgetId, elevation: f32) {
        let entry = self.states.entry(id.clone()).or_default();
        entry.elevation.set_target(elevation.max(0.0));
    }

    /// Gets current interpolated hover progress [0.0..1.0].
    pub fn hover_progress(&self, id: &WidgetId) -> f32 {
        self.states.get(id).map(|s| s.hover.value()).unwrap_or(0.0)
    }

    /// Gets current interpolated press progress [0.0..1.0].
    pub fn press_progress(&self, id: &WidgetId) -> f32 {
        self.states.get(id).map(|s| s.press.value()).unwrap_or(0.0)
    }

    /// Gets current interpolated focus progress [0.0..1.0].
    pub fn focus_progress(&self, id: &WidgetId) -> f32 {
        self.states.get(id).map(|s| s.focus.value()).unwrap_or(0.0)
    }

    /// Gets current interpolated elevation value.
    pub fn elevation_value(&self, id: &WidgetId) -> f32 {
        self.states.get(id).map(|s| s.elevation.value()).unwrap_or(0.0)
    }

    /// Cleans up settled idle states to prevent unbound map growth.
    pub fn cleanup_idle(&mut self) {
        self.states.retain(|_, state| {
            !state.is_idle()
                || state.hover.value() > 0.001
                || state.press.value() > 0.001
                || state.focus.value() > 0.001
                || state.elevation.value() > 0.001
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transition_engine_lifecycle() {
        let mut engine = TransitionEngine::new();
        let btn_id = WidgetId::new("submit_button");

        assert_eq!(engine.hover_progress(&btn_id), 0.0);

        engine.set_hovered(&btn_id, true);
        assert_eq!(engine.hover_progress(&btn_id), 0.0);

        // Step 16ms
        engine.update(1.0 / 60.0);
        let p1 = engine.hover_progress(&btn_id);
        assert!(p1 > 0.0, "hover progress should increase towards 1.0");

        // Step 1 second
        for _ in 0..60 {
            engine.update(1.0 / 60.0);
        }
        let p_final = engine.hover_progress(&btn_id);
        assert!((p_final - 1.0).abs() < 0.02, "hover should converge to 1.0");

        // Release hover
        engine.set_hovered(&btn_id, false);
        for _ in 0..60 {
            engine.update(1.0 / 60.0);
        }
        let p_released = engine.hover_progress(&btn_id);
        assert!(p_released < 0.02, "hover should converge back to 0.0");
    }

    #[test]
    fn test_focus_and_elevation_transitions() {
        let mut engine = TransitionEngine::new();
        let card_id = WidgetId::new("metric_card_01");

        engine.set_focused(&card_id, true);
        engine.set_elevation(&card_id, 4.0);

        for _ in 0..60 {
            engine.update(1.0 / 60.0);
        }

        assert!((engine.focus_progress(&card_id) - 1.0).abs() < 0.02);
        assert!((engine.elevation_value(&card_id) - 4.0).abs() < 0.05);
    }
}
