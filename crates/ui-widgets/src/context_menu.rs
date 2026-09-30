// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0 | Action: Hierarchical Context Menu & Submenu Navigation Engine
use crate::floating::{compute_floating_rect, Placement};
use crate::id::WidgetId;
use serde::{Deserialize, Serialize};

/// Single item within a context menu tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MenuItemSpec {
    Action {
        id: WidgetId,
        label: String,
        shortcut: Option<String>,
        icon: Option<String>,
        disabled: bool,
    },
    Submenu {
        label: String,
        icon: Option<String>,
        items: Vec<MenuItemSpec>,
    },
    Separator,
}

/// Runtime state and coordinate tracker for hierarchical context menus.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextMenuState {
    pub is_open: bool,
    pub anchor_pos: (f32, f32),
    pub items: Vec<MenuItemSpec>,
    /// Selected index path into submenus (e.g. `[1, 0]` = 2nd item's 1st submenu child).
    pub hover_path: Vec<usize>,
}

impl ContextMenuState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Opens context menu at the specified cursor position.
    pub fn open_at(&mut self, pos: (f32, f32), items: Vec<MenuItemSpec>) {
        self.is_open = true;
        self.anchor_pos = pos;
        self.items = items;
        self.hover_path.clear();
    }

    /// Closes active context menu.
    pub fn close(&mut self) {
        self.is_open = false;
        self.hover_path.clear();
        self.items.clear();
    }

    /// Computes the bounding geometry for the root context menu box within viewport.
    pub fn root_rect(&self, item_height: f32, menu_width: f32, viewport: [f32; 4]) -> [f32; 4] {
        let total_h = self
            .items
            .iter()
            .map(|it| match it {
                MenuItemSpec::Separator => 6.0,
                _ => item_height,
            })
            .sum::<f32>()
            + 12.0;

        let anchor = [self.anchor_pos.0, self.anchor_pos.1, 1.0, 1.0];
        let fl = compute_floating_rect(
            anchor,
            [menu_width, total_h],
            viewport,
            2.0,
            Placement::BottomStart,
            true,
        );
        fl.rect
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_menu_open_and_root_rect() {
        let mut cm = ContextMenuState::new();
        assert!(!cm.is_open);

        let items = vec![
            MenuItemSpec::Action {
                id: WidgetId::new("copy"),
                label: "Copy".to_string(),
                shortcut: Some("Ctrl+C".to_string()),
                icon: None,
                disabled: false,
            },
            MenuItemSpec::Separator,
            MenuItemSpec::Submenu {
                label: "Format".to_string(),
                icon: None,
                items: vec![MenuItemSpec::Action {
                    id: WidgetId::new("bold"),
                    label: "Bold".to_string(),
                    shortcut: None,
                    icon: None,
                    disabled: false,
                }],
            },
        ];

        cm.open_at((200.0, 300.0), items);
        assert!(cm.is_open);

        let rect = cm.root_rect(24.0, 160.0, [0.0, 0.0, 1920.0, 1080.0]);
        assert_eq!(rect[0], 200.0);
        assert_eq!(rect[2], 160.0);
        // Height: 24 (copy) + 6 (separator) + 24 (format) + 12 padding = 66
        assert_eq!(rect[3], 66.0);

        cm.close();
        assert!(!cm.is_open);
    }
}
