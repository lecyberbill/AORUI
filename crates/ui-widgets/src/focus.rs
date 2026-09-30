// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0 | Action: Global Keyboard Shortcuts, KeyChord Mapping and Tab Focus Ring Navigation Engine
use crate::id::WidgetId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a keyboard modifier combination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool,
}

impl Modifiers {
    pub const NONE: Self = Self {
        ctrl: false,
        shift: false,
        alt: false,
        meta: false,
    };
    pub const CTRL: Self = Self {
        ctrl: true,
        shift: false,
        alt: false,
        meta: false,
    };
    pub const SHIFT: Self = Self {
        ctrl: false,
        shift: true,
        alt: false,
        meta: false,
    };
    pub const ALT: Self = Self {
        ctrl: false,
        shift: false,
        alt: true,
        meta: false,
    };
    pub const META: Self = Self {
        ctrl: false,
        shift: false,
        alt: false,
        meta: true,
    };
    pub const CTRL_SHIFT: Self = Self {
        ctrl: true,
        shift: true,
        alt: false,
        meta: false,
    };

    pub fn new(ctrl: bool, shift: bool, alt: bool, meta: bool) -> Self {
        Self {
            ctrl,
            shift,
            alt,
            meta,
        }
    }

    pub fn union(self, other: Self) -> Self {
        Self {
            ctrl: self.ctrl || other.ctrl,
            shift: self.shift || other.shift,
            alt: self.alt || other.alt,
            meta: self.meta || other.meta,
        }
    }

    pub fn contains(&self, other: Self) -> bool {
        (!other.ctrl || self.ctrl)
            && (!other.shift || self.shift)
            && (!other.alt || self.alt)
            && (!other.meta || self.meta)
    }
}

/// Normalized representation of a physical or named key.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyCode {
    Char(char),
    Enter,
    Escape,
    Tab,
    Backspace,
    Delete,
    Space,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,
    F(u8),
}

/// A combined keychord (key + modifiers) that can trigger a command.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyChord {
    pub key: KeyCode,
    pub modifiers: Modifiers,
}

impl KeyChord {
    /// Creates a key chord with arbitrary modifiers.
    pub fn new(modifiers: Modifiers, key: KeyCode) -> Self {
        Self { key, modifiers }
    }

    /// Creates a key chord with no modifiers.
    pub fn key(key: KeyCode) -> Self {
        Self {
            key,
            modifiers: Modifiers::NONE,
        }
    }

    /// Creates a key chord with Ctrl modifier (or Cmd on macOS).
    pub fn ctrl(key: KeyCode) -> Self {
        Self {
            key,
            modifiers: Modifiers::CTRL,
        }
    }

    /// Creates a key chord with Ctrl + Shift modifiers.
    pub fn ctrl_shift(key: KeyCode) -> Self {
        Self {
            key,
            modifiers: Modifiers::CTRL_SHIFT,
        }
    }

    /// Creates a key chord with Alt modifier.
    pub fn alt(key: KeyCode) -> Self {
        Self {
            key,
            modifiers: Modifiers::ALT,
        }
    }
}

/// Central keybinding registry for mapping key combinations to action identifiers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeyMap {
    bindings: HashMap<KeyChord, String>,
}

impl KeyMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a keychord mapping to an action ID.
    pub fn bind(&mut self, chord: KeyChord, action_id: impl Into<String>) {
        self.bindings.insert(chord, action_id.into());
    }

    /// Looks up an action ID triggered by the given keychord.
    pub fn resolve(&self, chord: &KeyChord) -> Option<&str> {
        self.bindings.get(chord).map(|s| s.as_str())
    }

    /// Clears all bindings.
    pub fn clear(&mut self) {
        self.bindings.clear();
    }
}

/// Manages interactive tab-ordering and keyboard focus state across the widget tree.
#[derive(Debug, Clone, Default)]
pub struct FocusManager {
    focused: Option<WidgetId>,
    tab_order: Vec<WidgetId>,
}

impl FocusManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the ID of the currently focused widget, if any.
    pub fn focused(&self) -> Option<&WidgetId> {
        self.focused.as_ref()
    }

    /// Sets explicit focus to a widget ID.
    pub fn set_focus(&mut self, id: Option<WidgetId>) {
        self.focused = id;
    }

    /// Registers the sequence of focusable widgets for the current frame.
    pub fn set_tab_order(&mut self, order: Vec<WidgetId>) {
        self.tab_order = order;
        // If the currently focused widget is no longer in tab order, keep or clear
        if let Some(focused) = &self.focused {
            if !self.tab_order.contains(focused) {
                // Focus remains valid unless explicitly cleared
            }
        }
    }

    /// Advances focus to the next element in the tab order (Tab key).
    pub fn focus_next(&mut self) -> Option<&WidgetId> {
        if self.tab_order.is_empty() {
            self.focused = None;
            return None;
        }

        let next_idx = match &self.focused {
            Some(curr) => {
                if let Some(pos) = self.tab_order.iter().position(|id| id == curr) {
                    (pos + 1) % self.tab_order.len()
                } else {
                    0
                }
            }
            None => 0,
        };

        self.focused = self.tab_order.get(next_idx).cloned();
        self.focused.as_ref()
    }

    /// Moves focus to the previous element in the tab order (Shift+Tab).
    pub fn focus_prev(&mut self) -> Option<&WidgetId> {
        if self.tab_order.is_empty() {
            self.focused = None;
            return None;
        }

        let prev_idx = match &self.focused {
            Some(curr) => {
                if let Some(pos) = self.tab_order.iter().position(|id| id == curr) {
                    if pos == 0 {
                        self.tab_order.len() - 1
                    } else {
                        pos - 1
                    }
                } else {
                    self.tab_order.len() - 1
                }
            }
            None => self.tab_order.len() - 1,
        };

        self.focused = self.tab_order.get(prev_idx).cloned();
        self.focused.as_ref()
    }

    /// Clears keyboard focus.
    pub fn clear(&mut self) {
        self.focused = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keymap_resolution() {
        let mut map = KeyMap::new();
        map.bind(KeyChord::ctrl(KeyCode::Char('k')), "open_spotlight");
        map.bind(KeyChord::ctrl(KeyCode::Char('s')), "save_file");
        map.bind(KeyChord::key(KeyCode::Escape), "dismiss_modal");

        assert_eq!(
            map.resolve(&KeyChord::ctrl(KeyCode::Char('k'))),
            Some("open_spotlight")
        );
        assert_eq!(
            map.resolve(&KeyChord::ctrl(KeyCode::Char('s'))),
            Some("save_file")
        );
        assert_eq!(
            map.resolve(&KeyChord::key(KeyCode::Escape)),
            Some("dismiss_modal")
        );
        assert_eq!(map.resolve(&KeyChord::key(KeyCode::Char('k'))), None);
    }

    #[test]
    fn test_focus_cycle() {
        let mut fm = FocusManager::new();
        let b1 = WidgetId::new("btn_1");
        let b2 = WidgetId::new("btn_2");
        let b3 = WidgetId::new("btn_3");

        fm.set_tab_order(vec![b1.clone(), b2.clone(), b3.clone()]);

        assert_eq!(fm.focus_next(), Some(&b1));
        assert_eq!(fm.focus_next(), Some(&b2));
        assert_eq!(fm.focus_next(), Some(&b3));
        assert_eq!(
            fm.focus_next(),
            Some(&b1),
            "Should wrap around to first widget"
        );

        assert_eq!(
            fm.focus_prev(),
            Some(&b3),
            "Should wrap backward to last widget"
        );
        assert_eq!(fm.focus_prev(), Some(&b2));
    }
}
