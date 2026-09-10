// [WFGY] Zone: RISK | λ: 0.3 | Fallbacks: 0 | Action: Declarative widget model in cyber-glass style for AORUI
//! Declarative widget model (button, label, text input, rich list, checkbox,
//! window, tabs, scrollable areas, dropdowns, popovers, toasts, tooltips)
//! rendered in cyber-glass style via `ui-gpu`.
//!
//! This crate does not depend on `wgpu`, `winit`, or `glyphon`: it produces
//! [`ui_core::GpuSdfInstance`] and [`frame::TextSpec`] (renderer-agnostic text specifications),
//! delegating to the renderer integration layer (e.g. `ui-gpu::TextLayer`) to transform
//! them into concrete primitives.
//!
//! No dependency on `agent-runtime`: interaction ([`interaction`]) processes
//! clicks identically whether from a human pointer or programmatic control —
//! the UI remains human-first, optionally agent-controllable.

mod color;
mod effective;
mod frame;
mod id;
mod interaction;
mod kind;
mod media;
pub mod text_measure;
mod theme;
mod tree;

pub use color::{Color, ColorSpace};
pub use effective::{EffectiveBounds, NO_CLIP};
pub use frame::{Frame, InteractionState, TextAlign, TextSpec};
pub use id::WidgetId;
pub use kind::{IconKind, InteractionKey, ListItemBadge, SplitOrientation, ToastKind, WidgetKind};
pub use media::{MediaFit, MediaKind, MediaSpec};
pub use text_measure::{DefaultTextMeasure, TextMeasure};
pub use theme::{FontFamily, FontWeight, Theme, Typography};
pub use tree::WidgetTree;

#[cfg(test)]
mod tests {
    use ui_layout::{length, AlignItems, FlexDirection, Size, Style, TaffyMaxContent};

    use super::*;

    fn leaf_style(w: f32, h: f32) -> Style {
        Style { size: Size { width: length(w), height: length(h) }, ..Default::default() }
    }

    #[test]
    fn button_click_emits_button_clicked() {
        let mut tree = WidgetTree::new();
        let btn = tree.button("submit", "Submit", true, leaf_style(120.0, 40.0)).unwrap();
        let root = tree.container(&[btn], leaf_style(200.0, 200.0)).unwrap();
        tree.compute(root, Size::MAX_CONTENT).unwrap();

        let event = tree.dispatch_click(root, (10.0, 10.0)).unwrap();
        assert_eq!(event, Some(ui_core::UiEvent::ButtonClicked { widget_id: "submit".to_string() }));
    }

    #[test]
    fn disabled_button_does_not_emit_event() {
        let mut tree = WidgetTree::new();
        let btn = tree.button("submit", "Submit", false, leaf_style(120.0, 40.0)).unwrap();
        let root = tree.container(&[btn], leaf_style(200.0, 200.0)).unwrap();
        tree.compute(root, Size::MAX_CONTENT).unwrap();

        assert_eq!(tree.dispatch_click(root, (10.0, 10.0)).unwrap(), None);
    }

    #[test]
    fn checkbox_click_toggles_the_opposite_of_current_state() {
        let mut tree = WidgetTree::new();
        let cb = tree.checkbox("accept_terms", false, leaf_style(24.0, 24.0)).unwrap();
        let root = tree.container(&[cb], leaf_style(200.0, 200.0)).unwrap();
        tree.compute(root, Size::MAX_CONTENT).unwrap();

        let event = tree.dispatch_click(root, (5.0, 5.0)).unwrap();
        assert_eq!(
            event,
            Some(ui_core::UiEvent::CheckboxToggled { widget_id: "accept_terms".to_string(), checked: true })
        );
    }

    #[test]
    fn tabbar_click_reports_owner_id_and_clicked_index() {
        let mut tree = WidgetTree::new();
        let tabs = ["General", "Security", "Network"];
        let bar_style = Style {
            flex_direction: FlexDirection::Row,
            size: Size { width: length(300.0), height: length(32.0) },
            ..Default::default()
        };
        let bar = tree.tabbar("settings_tabs", &tabs, 0, leaf_style(100.0, 32.0), bar_style).unwrap();
        tree.compute(bar, Size::MAX_CONTENT).unwrap();

        // The second tab occupies the [100, 200) range in x.
        let event = tree.dispatch_click(bar, (150.0, 10.0)).unwrap();
        assert_eq!(event, Some(ui_core::UiEvent::TabSelected { widget_id: "settings_tabs".to_string(), tab_index: 1 }));
    }

    #[test]
    fn click_outside_any_widget_yields_none() {
        let mut tree = WidgetTree::new();
        let btn = tree.button("submit", "Submit", true, leaf_style(50.0, 20.0)).unwrap();
        let root = tree.container(&[btn], leaf_style(200.0, 200.0)).unwrap();
        tree.compute(root, Size::MAX_CONTENT).unwrap();

        assert_eq!(tree.dispatch_click(root, (190.0, 190.0)).unwrap(), None);
    }

    #[test]
    fn build_frame_emits_one_instance_and_one_text_per_button() {
        let mut tree = WidgetTree::new();
        let btn = tree.button("submit", "Submit", true, leaf_style(120.0, 40.0)).unwrap();
        let label = tree.label("Simple text", leaf_style(120.0, 20.0)).unwrap();
        let root = tree.container(&[btn, label], leaf_style(200.0, 200.0)).unwrap();
        tree.compute(root, Size::MAX_CONTENT).unwrap();

        let frame = tree.build_frame(root, &Theme::cyber_glass(), InteractionState::default()).unwrap();
        assert_eq!(frame.instances.len(), 1, "only button produces SDF quad, container produces none");
        assert_eq!(frame.texts.len(), 2, "button and label each produce one text spec");
    }

    #[test]
    fn corner_radius_never_exceeds_half_the_smaller_side() {
        // INV-GPU-3: corner radius larger than half of smaller side breaks `sd_rounded_box`.
        let mut tree = WidgetTree::new();
        let theme = Theme::cyber_glass();
        assert!(theme.corner_radius > 12.0, "theme must exceed half of side to test clamping");

        let cb = tree.checkbox("accept", true, leaf_style(24.0, 24.0)).unwrap();
        let root = tree.container(&[cb], leaf_style(200.0, 200.0)).unwrap();
        tree.compute(root, Size::MAX_CONTENT).unwrap();

        let frame = tree.build_frame(root, &theme, InteractionState::default()).unwrap();
        assert_eq!(frame.instances.len(), 1);
        assert!(frame.instances[0].radius <= 12.0, "radius must be clamped to half of side (12), got {}", frame.instances[0].radius);
    }

    #[test]
    fn hovering_a_button_boosts_its_glow_intensity() {
        let mut tree = WidgetTree::new();
        let theme = Theme::cyber_glass();
        let btn = tree.button("submit", "Submit", true, leaf_style(120.0, 40.0)).unwrap();
        let root = tree.container(&[btn], leaf_style(200.0, 200.0)).unwrap();
        tree.compute(root, Size::MAX_CONTENT).unwrap();

        let idle = tree.build_frame(root, &theme, InteractionState::default()).unwrap();
        let key = WidgetId::new("submit");
        let hovered_key = crate::InteractionKey { widget_id: key, index: None };
        let hovered = tree
            .build_frame(root, &theme, InteractionState { hovered: Some(&hovered_key), pressed: None, ..Default::default() })
            .unwrap();

        assert!(
            hovered.instances[0].glow_intensity > idle.instances[0].glow_intensity,
            "hover must boost glow intensity above idle state"
        );
    }

    #[test]
    fn window_close_button_dispatches_to_owner_window_id() {
        let mut tree = WidgetTree::new();
        let content = tree.label("Content", leaf_style(100.0, 20.0)).unwrap();
        let window_style = Style { size: Size { width: length(300.0), height: length(200.0) }, ..Default::default() };
        let window = tree.window("prefs_window", "Preferences", &[content], window_style).unwrap();
        tree.compute(window, Size::MAX_CONTENT).unwrap();

        // Close button is anchored at top-right with 20x20 size.
        let event = tree.dispatch_click(window, (300.0 - 8.0 - 10.0, 8.0 + 10.0)).unwrap();
        assert_eq!(event, Some(ui_core::UiEvent::WindowCloseRequested { widget_id: "prefs_window".to_string() }));
    }

    #[test]
    fn media_widget_produces_a_media_spec_not_an_sdf_instance() {
        // Media widget produces an entry in `frame.media`, not an SDF quad.
        let mut tree = WidgetTree::new();
        let img = tree.media("cover_art", MediaKind("image"), "res://cover.png", leaf_style(120.0, 80.0)).unwrap();
        let root = tree.container(&[img], leaf_style(200.0, 200.0)).unwrap();
        tree.compute(root, Size::MAX_CONTENT).unwrap();

        let frame = tree.build_frame(root, &Theme::cyber_glass(), InteractionState::default()).unwrap();
        assert!(frame.instances.is_empty());
        assert!(frame.texts.is_empty());
        assert_eq!(frame.media.len(), 1);
        assert_eq!(frame.media[0].kind, MediaKind("image"));
        assert_eq!(frame.media[0].resource_id, "res://cover.png");
    }

    #[test]
    fn effective_bounds_shift_children_by_scroll_offset_and_clip_to_viewport() {
        // INV-GPU-2: viewport 100x100, scrolled down by 30px (offset.y = 30).
        let mut tree = WidgetTree::new();
        let btn = tree.button("scrolled_btn", "Click", true, leaf_style(50.0, 50.0)).unwrap();
        let scroll = tree.scrollview("scroll_area", [0.0, 30.0], &[btn], leaf_style(100.0, 100.0)).unwrap();
        tree.compute(scroll, Size::MAX_CONTENT).unwrap();

        let effective = tree.effective_bounds(scroll).unwrap();
        let btn_eff = effective[&btn];
        assert_eq!(btn_eff.visual, [0.0, -30.0, 50.0, 50.0]);
        assert_eq!(btn_eff.clip, [0.0, 0.0, 100.0, 100.0]);
        assert_eq!(btn_eff.visible_rect(), [0.0, 0.0, 50.0, 20.0]);
    }

    #[test]
    fn scrolled_button_is_clickable_at_its_new_position_and_not_at_its_old_one() {
        let mut tree = WidgetTree::new();
        let btn = tree.button("scrolled_btn", "Click", true, leaf_style(50.0, 50.0)).unwrap();
        let scroll = tree.scrollview("scroll_area", [0.0, 30.0], &[btn], leaf_style(100.0, 100.0)).unwrap();
        tree.compute(scroll, Size::MAX_CONTENT).unwrap();

        // New visual position: hit should register here
        let hit = tree.dispatch_click(scroll, (25.0, 10.0)).unwrap();
        assert_eq!(hit, Some(ui_core::UiEvent::ButtonClicked { widget_id: "scrolled_btn".to_string() }));

        // Old position before scrolling (y=40): now outside visible viewport
        let miss = tree.dispatch_click(scroll, (25.0, 40.0)).unwrap();
        assert_eq!(miss, None);
    }

    #[test]
    fn scrollview_does_not_shrink_content_to_fit_viewport() {
        let mut tree = WidgetTree::new();
        let tall_child = tree.label("content", leaf_style(100.0, 300.0)).unwrap();
        let viewport_style = Style { align_items: Some(AlignItems::Stretch), ..leaf_style(100.0, 50.0) };
        let scroll = tree.scrollview("scroller", [0.0, 0.0], &[tall_child], viewport_style).unwrap();
        tree.compute(scroll, Size::MAX_CONTENT).unwrap();

        let bounds = tree.resolved_bounds(scroll).unwrap();
        assert_eq!(bounds[&tall_child][3], 300.0, "content height must not be squashed to viewport");
    }

    #[test]
    fn scroll_over_a_child_widget_still_scrolls_its_scrollview_ancestor() {
        let mut tree = WidgetTree::new();
        let list_style = Style { flex_direction: FlexDirection::Column, ..Default::default() };
        let list = tree.list("demo_list", &["Item1", "Item2"], None, leaf_style(100.0, 32.0), list_style).unwrap();
        let scroll = tree.scrollview("scroll_area", [0.0, 0.0], &[list], leaf_style(100.0, 40.0)).unwrap();
        tree.compute(scroll, Size::MAX_CONTENT).unwrap();

        let event = tree.dispatch_scroll(scroll, (10.0, 10.0), [0.0, 15.0]).unwrap();
        assert_eq!(event, Some(ui_core::UiEvent::ScrollChanged { widget_id: "scroll_area".to_string(), offset: [0.0, 15.0] }));
    }

    #[test]
    fn toggle_click_emits_toggle_switched_with_opposite_state() {
        let mut tree = WidgetTree::new();
        let toggle = tree.toggle("turbo_mode", false, leaf_style(44.0, 24.0)).unwrap();
        let root = tree.container(&[toggle], leaf_style(100.0, 100.0)).unwrap();
        tree.compute(root, Size::MAX_CONTENT).unwrap();

        let event = tree.dispatch_click(root, (20.0, 12.0)).unwrap();
        assert_eq!(event, Some(ui_core::UiEvent::ToggleSwitched { widget_id: "turbo_mode".to_string(), active: true }));
    }

    #[test]
    fn slider_click_and_drag_calculates_interpolated_value() {
        let mut tree = WidgetTree::new();
        let slider = tree.slider("volume", 0.0, 100.0, 25.0, leaf_style(200.0, 20.0)).unwrap();
        let root = tree.container(&[slider], leaf_style(300.0, 100.0)).unwrap();
        tree.compute(root, Size::MAX_CONTENT).unwrap();

        let event = tree.dispatch_click(root, (100.0, 10.0)).unwrap();
        assert_eq!(event, Some(ui_core::UiEvent::SliderChanged { widget_id: "volume".to_string(), value: 50.0 }));

        let val = tree.slider_value_at(root, (150.0, 10.0)).unwrap();
        assert_eq!(val, Some(("volume".to_string(), 75.0)));

        // Test drag tracking even when cursor Y is far above/below the slider
        let drag_val = tree.slider_drag_value(root, "volume", (180.0, 50.0)).unwrap();
        assert_eq!(drag_val, Some(90.0));
    }

    #[test]
    fn vertical_slider_click_and_drag_calculates_interpolated_value() {
        let mut tree = WidgetTree::new();
        // Vertical fader: 0 at bottom (y=200), 100 at top (y=0)
        let slider = tree.slider_vertical("fader", 0.0, 100.0, 0.0, leaf_style(30.0, 200.0)).unwrap();
        let root = tree.container(&[slider], leaf_style(100.0, 250.0)).unwrap();
        tree.compute(root, Size::MAX_CONTENT).unwrap();

        // Click at middle (y=100) -> 50%
        let event = tree.dispatch_click(root, (15.0, 100.0)).unwrap();
        assert_eq!(event, Some(ui_core::UiEvent::SliderChanged { widget_id: "fader".to_string(), value: 50.0 }));

        // Click near top (y=20) -> 90%
        let val = tree.slider_value_at(root, (15.0, 20.0)).unwrap();
        assert_eq!(val, Some(("fader".to_string(), 90.0)));

        // Drag near bottom (y=180) -> 10%
        let drag_val = tree.slider_drag_value(root, "fader", (100.0, 180.0)).unwrap().unwrap();
        assert!((drag_val - 10.0).abs() < 1e-4);
    }

    #[test]
    fn progress_indicators_generate_instances_and_labels() {
        let mut tree = WidgetTree::new();
        let theme = Theme::cyber_glass();
        let h_bar = tree.progress_bar(0.75, leaf_style(200.0, 8.0)).unwrap();
        let v_bar = tree.progress_bar_vertical(0.50, leaf_style(8.0, 100.0)).unwrap();
        let ring = tree.progress_ring(0.85, Some("85%"), leaf_style(60.0, 60.0)).unwrap();
        let pie = tree.progress_pie(0.40, Some("40%"), leaf_style(60.0, 60.0)).unwrap();
        let root = tree.container(&[h_bar, v_bar, ring, pie], leaf_style(400.0, 200.0)).unwrap();
        tree.compute(root, Size::MAX_CONTENT).unwrap();

        let frame = tree.build_frame(root, &theme, InteractionState::default()).unwrap();
        assert!(!frame.instances.is_empty(), "all progress indicators produce GPU instances");
        assert!(frame.texts.iter().any(|t| t.text == "85%"), "ring produces central label");
        assert!(frame.texts.iter().any(|t| t.text == "40%"), "pie produces central label");
    }

    #[test]
    fn metric_card_produces_title_and_value_text_specs() {
        let mut tree = WidgetTree::new();
        let theme = Theme::cyber_glass();
        let card = tree.metric_card("Throughput", "1.2 Gbps", Some(("+14%", true)), leaf_style(140.0, 60.0)).unwrap();
        let root = tree.container(&[card], leaf_style(200.0, 100.0)).unwrap();
        tree.compute(root, Size::MAX_CONTENT).unwrap();

        let frame = tree.build_frame(root, &theme, InteractionState::default()).unwrap();
        assert_eq!(frame.texts.len(), 3, "title + value + delta");
    }

    #[test]
    fn text_input_click_emits_focus_changed() {
        let mut tree = WidgetTree::new();
        let input = tree.text_input("search_box", "value", "placeholder", false, leaf_style(150.0, 32.0)).unwrap();
        let root = tree.container(&[input], leaf_style(200.0, 100.0)).unwrap();
        tree.compute(root, Size::MAX_CONTENT).unwrap();

        let event = tree.dispatch_click(root, (50.0, 16.0)).unwrap();
        assert_eq!(event, Some(ui_core::UiEvent::FocusChanged { widget_id: Some("search_box".to_string()) }));
    }

    #[test]
    fn radio_click_emits_radio_selected() {
        let mut tree = WidgetTree::new();
        let r1 = tree.radio("opt_a", "mode_group", "Mode A", true, leaf_style(120.0, 24.0)).unwrap();
        let r2 = tree.radio("opt_b", "mode_group", "Mode B", false, leaf_style(120.0, 24.0)).unwrap();
        let root = tree.container(&[r1, r2], leaf_style(200.0, 100.0)).unwrap();
        tree.compute(root, Size::MAX_CONTENT).unwrap();

        let event = tree.dispatch_click(root, (10.0, 12.0)).unwrap();
        assert_eq!(event, Some(ui_core::UiEvent::RadioSelected { group_id: "mode_group".to_string(), selected_id: "opt_a".to_string() }));
    }

    #[test]
    fn segmented_control_click_emits_segment_selected() {
        let mut tree = WidgetTree::new();
        let options = ["1H", "24H", "7D"];
        let seg = tree.segmented_control("time_span", &options, 0, leaf_style(50.0, 28.0), leaf_style(150.0, 28.0)).unwrap();
        tree.compute(seg, Size::MAX_CONTENT).unwrap();

        let event = tree.dispatch_click(seg, (75.0, 14.0)).unwrap();
        assert_eq!(event, Some(ui_core::UiEvent::SegmentSelected { widget_id: "time_span".to_string(), selected_index: 1 }));
    }

    #[test]
    fn modal_backdrop_click_emits_modal_dismissed() {
        let mut tree = WidgetTree::new();
        let inner = tree.label("Modal Message", leaf_style(100.0, 20.0)).unwrap();
        let modal = tree.modal("confirm_dialog", "Confirmation", &[inner], leaf_style(200.0, 100.0), leaf_style(400.0, 400.0)).unwrap();
        tree.compute(modal, Size::MAX_CONTENT).unwrap();

        let event = tree.dispatch_click(modal, (350.0, 350.0)).unwrap();
        assert_eq!(event, Some(ui_core::UiEvent::ModalDismissed { modal_id: "confirm_dialog".to_string() }));
    }

    #[test]
    fn divider_produces_sdf_instance_without_text() {
        let mut tree = WidgetTree::new();
        let div = tree.divider(false, leaf_style(200.0, 1.0)).unwrap();
        let root = tree.container(&[div], leaf_style(200.0, 50.0)).unwrap();
        tree.compute(root, Size::MAX_CONTENT).unwrap();

        let frame = tree.build_frame(root, &Theme::cyber_glass(), InteractionState::default()).unwrap();
        assert_eq!(frame.instances.len(), 1);
        assert_eq!(frame.texts.len(), 0);
    }

    #[test]
    fn menubar_click_emits_menu_toggled() {
        let mut tree = WidgetTree::new();
        let menus = ["File", "Edit", "Help"];
        let bar = tree.menubar("main_menu", &menus, None, leaf_style(60.0, 24.0), leaf_style(200.0, 24.0)).unwrap();
        tree.compute(bar, Size::MAX_CONTENT).unwrap();

        let event = tree.dispatch_click(bar, (30.0, 12.0)).unwrap();
        assert_eq!(event, Some(ui_core::UiEvent::MenuToggled { menu_id: "main_menu:0".to_string(), open: true }));
    }

    #[test]
    fn menu_popover_click_emits_menu_item_clicked() {
        let mut tree = WidgetTree::new();
        let items = [
            (WidgetId::new("item_new"), "New", Some("Ctrl+N"), true),
            (WidgetId::new("item_open"), "Open", Some("Ctrl+O"), true),
            (WidgetId::new("item_disabled"), "Disabled", None, false),
        ];
        let popover = tree.menu_popover("file_menu", &items, leaf_style(160.0, 28.0), leaf_style(160.0, 84.0)).unwrap();
        tree.compute(popover, Size::MAX_CONTENT).unwrap();

        let event1 = tree.dispatch_click(popover, (50.0, 14.0)).unwrap();
        assert_eq!(event1, Some(ui_core::UiEvent::MenuItemClicked { menu_id: "file_menu".to_string(), item_id: "item_new".to_string() }));

        let event2 = tree.dispatch_click(popover, (50.0, 70.0)).unwrap();
        assert_eq!(event2, None);
    }

    #[test]
    fn dropdown_click_emits_menu_toggled() {
        let mut tree = WidgetTree::new();
        let dd = tree.dropdown("env_select", "Environment", "Production", false, leaf_style(180.0, 32.0)).unwrap();
        tree.compute(dd, Size::MAX_CONTENT).unwrap();

        let event = tree.dispatch_click(dd, (90.0, 16.0)).unwrap();
        assert_eq!(event, Some(ui_core::UiEvent::MenuToggled { menu_id: "env_select".to_string(), open: true }));
    }

    #[test]
    fn grid_layout_distributes_children_into_columns() {
        let mut tree = WidgetTree::new();
        let b1 = tree.button("b1", "1", true, leaf_style(100.0, 30.0)).unwrap();
        let b2 = tree.button("b2", "2", true, leaf_style(100.0, 30.0)).unwrap();
        let b3 = tree.button("b3", "3", true, leaf_style(100.0, 30.0)).unwrap();
        let b4 = tree.button("b4", "4", true, leaf_style(100.0, 30.0)).unwrap();

        let grid = tree.grid(2, 10.0, 10.0, &[b1, b2, b3, b4], Style {
            size: Size { width: length(210.0), height: length(70.0) },
            ..Default::default()
        }).unwrap();
        tree.compute(grid, Size::MAX_CONTENT).unwrap();

        let bounds = tree.resolved_bounds(grid).unwrap();
        assert_eq!(bounds[&b1], [0.0, 0.0, 100.0, 30.0]);
        assert_eq!(bounds[&b2], [110.0, 0.0, 100.0, 30.0]);
        assert_eq!(bounds[&b3], [0.0, 40.0, 100.0, 30.0]);
        assert_eq!(bounds[&b4], [110.0, 40.0, 100.0, 30.0]);
    }

    #[test]
    fn tree_view_dispatches_toggle_and_selection() {
        let mut tree = WidgetTree::new();
        let nodes = [
            ("src_dir", "src", 0, true, true, false),
            ("main_rs", "main.rs", 1, false, false, true),
        ];
        let root = tree.tree_view("proj_tree", &nodes, leaf_style(200.0, 24.0), leaf_style(200.0, 48.0)).unwrap();
        tree.compute(root, Size::MAX_CONTENT).unwrap();

        // Clicking directory toggles expanded state
        let ev1 = tree.dispatch_click(root, (10.0, 12.0)).unwrap();
        assert_eq!(ev1, Some(ui_core::UiEvent::TreeNodeToggled { tree_id: "proj_tree".to_string(), node_id: "src_dir".to_string(), expanded: false }));

        // Clicking leaf file selects node
        let ev2 = tree.dispatch_click(root, (10.0, 36.0)).unwrap();
        assert_eq!(ev2, Some(ui_core::UiEvent::TreeNodeSelected { tree_id: "proj_tree".to_string(), node_id: "main_rs".to_string() }));
    }

    #[test]
    fn split_view_calculates_drag_ratio() {
        let mut tree = WidgetTree::new();
        let p1 = tree.button("p1", "Panel 1", true, leaf_style(100.0, 100.0)).unwrap();
        let p2 = tree.button("p2", "Panel 2", true, leaf_style(100.0, 100.0)).unwrap();
        let split = tree.split_view("main_split", crate::kind::SplitOrientation::Horizontal, 0.5, p1, p2, leaf_style(200.0, 100.0)).unwrap();
        tree.compute(split, Size::MAX_CONTENT).unwrap();

        // Dragging at x=60px within 200px container -> ratio = 60 / 200 = 0.30
        let ratio = tree.split_ratio_at(split, "main_split", (60.0, 50.0)).unwrap();
        assert_eq!(ratio, Some(0.30));
    }

    #[test]
    fn text_area_focus_and_multiline_frame() {
        let mut tree = WidgetTree::new();
        let ta = tree.text_area("code_editor", "fn main() {\n    println!(\"hello\");\n}", "placeholder", true, true, leaf_style(300.0, 80.0)).unwrap();
        tree.compute(ta, Size::MAX_CONTENT).unwrap();

        let ev = tree.dispatch_click(ta, (50.0, 30.0)).unwrap();
        assert_eq!(ev, Some(ui_core::UiEvent::FocusChanged { widget_id: Some("code_editor".to_string()) }));

        let theme = crate::Theme::default();
        let frame = tree.build_frame(ta, &theme, Default::default()).unwrap();
        // Gutter + separator + line 1 + line 2 texts + cursor
        assert!(frame.texts.iter().any(|t| t.text == "fn main() {"));
        assert!(frame.texts.iter().any(|t| t.text == "    println!(\"hello\");"));
        assert!(frame.texts.iter().any(|t| t.text == "1"));
        assert!(frame.texts.iter().any(|t| t.text == "2"));
    }

    #[test]
    fn password_input_reveal_toggle() {
        let mut tree = WidgetTree::new();
        let pwd = tree.password_input("user_pwd", "secret123", "password", false, false, leaf_style(200.0, 32.0)).unwrap();
        tree.compute(pwd, Size::MAX_CONTENT).unwrap();

        // Clicking eye toggle (rightmost 28px: x = 185px on 200px width)
        let ev = tree.dispatch_click(pwd, (185.0, 16.0)).unwrap();
        assert_eq!(ev, Some(ui_core::UiEvent::PasswordRevealed { widget_id: "user_pwd".to_string(), revealed: true }));

        // Frame rendering with revealed=false renders bullet masking
        let theme = crate::Theme::default();
        let frame = tree.build_frame(pwd, &theme, Default::default()).unwrap();
        assert!(frame.texts.iter().any(|t| t.text.contains("•••••••••")));
    }

    #[test]
    fn number_input_stepper_and_clamping() {
        let mut tree = WidgetTree::new();
        let num = tree.number_input("spin_count", 10.0, 0.0, 100.0, 5.0, 1, false, leaf_style(160.0, 32.0)).unwrap();
        tree.compute(num, Size::MAX_CONTENT).unwrap();

        // Clicking upper right stepper (+5) -> x = 145px, y = 6px (top half of 32px height)
        let ev_inc = tree.dispatch_click(num, (145.0, 6.0)).unwrap();
        assert_eq!(ev_inc, Some(ui_core::UiEvent::NumberChanged { widget_id: "spin_count".to_string(), value: 15.0 }));

        // Clicking lower right stepper (-5) -> x = 145px, y = 24px (bottom half of 32px height)
        let ev_dec = tree.dispatch_click(num, (145.0, 24.0)).unwrap();
        assert_eq!(ev_dec, Some(ui_core::UiEvent::NumberChanged { widget_id: "spin_count".to_string(), value: 5.0 }));
    }

    #[test]
    fn keyboard_tab_focus_cycle() {
        let mut tree = WidgetTree::new();
        let i1 = tree.text_input("input_1", "", "Input 1", false, leaf_style(100.0, 30.0)).unwrap();
        let i2 = tree.password_input("input_pwd", "", "Password", false, false, leaf_style(100.0, 30.0)).unwrap();
        let b1 = tree.button("btn_submit", "Submit", true, leaf_style(100.0, 30.0)).unwrap();
        let form = tree.container(&[i1, i2, b1], leaf_style(300.0, 100.0)).unwrap();
        tree.compute(form, Size::MAX_CONTENT).unwrap();

        // Initial focus (None) forward -> first focusable ("input_1")
        assert_eq!(tree.next_focusable(form, None, false).unwrap(), Some("input_1".to_string()));

        // Tab forward from "input_1" -> "input_pwd"
        assert_eq!(tree.next_focusable(form, Some("input_1"), false).unwrap(), Some("input_pwd".to_string()));

        // Tab forward from "input_pwd" -> "btn_submit"
        assert_eq!(tree.next_focusable(form, Some("input_pwd"), false).unwrap(), Some("btn_submit".to_string()));

        // Tab forward from "btn_submit" -> wraps around to "input_1"
        assert_eq!(tree.next_focusable(form, Some("btn_submit"), false).unwrap(), Some("input_1".to_string()));

        // Shift+Tab backward from "input_1" -> wraps to "btn_submit"
        assert_eq!(tree.next_focusable(form, Some("input_1"), true).unwrap(), Some("btn_submit".to_string()));
    }

    #[test]
    fn icon_and_icon_button_render_and_click() {
        let mut tree = WidgetTree::new();
        let icon_node = tree.icon(crate::kind::IconKind::Shield, 16.0, None, leaf_style(24.0, 24.0)).unwrap();
        let btn_node = tree.icon_button("gear_btn", crate::kind::IconKind::Settings, true, leaf_style(32.0, 32.0)).unwrap();
        let cont = tree.container(&[icon_node, btn_node], leaf_style(100.0, 40.0)).unwrap();
        tree.compute(cont, Size::MAX_CONTENT).unwrap();

        let theme = crate::Theme::default();
        let frame = tree.build_frame(cont, &theme, Default::default()).unwrap();
        assert!(frame.texts.iter().any(|t| t.text == "⛨"));
        assert!(frame.texts.iter().any(|t| t.text == "⚙"));

        // Click icon button
        let ev = tree.dispatch_click(cont, (35.0, 16.0)).unwrap();
        assert_eq!(ev, Some(ui_core::UiEvent::ButtonClicked { widget_id: "gear_btn".to_string() }));
    }

    #[test]
    fn table_header_click_and_row_selection() {
        let mut tree = WidgetTree::new();
        let cols = [
            ("Service", 120.0, Some(true)),
            ("Status", 80.0, None),
            ("Latency", 60.0, None),
        ];
        let row1 = [
            ("Core Node", crate::kind::ListItemBadge::None),
            ("Online", crate::kind::ListItemBadge::Success),
            ("4.2ms", crate::kind::ListItemBadge::None),
        ];
        let row2 = [
            ("Edge Gateway", crate::kind::ListItemBadge::None),
            ("Warning", crate::kind::ListItemBadge::Warning),
            ("18.5ms", crate::kind::ListItemBadge::None),
        ];
        let rows = [row1, row2];

        let table = tree.table("mesh_table", &cols, &rows, Some(0), 24.0, leaf_style(260.0, 80.0)).unwrap();
        tree.compute(table, Size::MAX_CONTENT).unwrap();

        // Click first column header (x=50px, y=10px) -> TableHeaderClicked
        let ev_hdr = tree.dispatch_click(table, (50.0, 10.0)).unwrap();
        assert_eq!(ev_hdr, Some(ui_core::UiEvent::TableHeaderClicked { table_id: "mesh_table".to_string(), column_index: 0 }));

        // Click row 2 (y=60px) -> TableRowSelected
        let ev_row = tree.dispatch_click(table, (50.0, 60.0)).unwrap();
        assert_eq!(ev_row, Some(ui_core::UiEvent::TableRowSelected { table_id: "mesh_table".to_string(), row_index: 1 }));
    }

    #[test]
    fn accordion_toggle_and_children_collapse() {
        let mut tree = WidgetTree::new();
        let body = tree.button("acc_btn", "Inner Action", true, leaf_style(100.0, 30.0)).unwrap();
        let acc_collapsed = tree.accordion("acc_sec", "Diagnostics", Some("Real-time telemetry"), false, body, leaf_style(200.0, 40.0)).unwrap();
        tree.compute(acc_collapsed, Size::MAX_CONTENT).unwrap();

        // When collapsed, click header toggles to true
        let ev = tree.dispatch_click(acc_collapsed, (50.0, 15.0)).unwrap();
        assert_eq!(ev, Some(ui_core::UiEvent::AccordionToggled { id: "acc_sec".to_string(), expanded: true }));

        // Tree only contains header when collapsed
        assert_eq!(tree.layout().children(acc_collapsed).unwrap().len(), 1);
    }

    #[test]
    fn breadcrumb_and_pagination_events() {
        let mut tree = WidgetTree::new();
        let crumbs = [("home", "Home"), ("sec", "Security"), ("rule", "Rule #402")];
        let bc = tree.breadcrumb("main_bc", &crumbs, leaf_style(60.0, 20.0), leaf_style(200.0, 20.0)).unwrap();
        tree.compute(bc, Size::MAX_CONTENT).unwrap();

        // Click crumb 0 ("home")
        let ev = tree.dispatch_click(bc, (20.0, 10.0)).unwrap();
        assert_eq!(ev, Some(ui_core::UiEvent::BreadcrumbClicked { bar_id: "main_bc".to_string(), index: 0, item_id: "home".to_string() }));

        // Pagination: page 2 of 5
        let pag = tree.pagination("data_pag", 2, 5, leaf_style(30.0, 24.0), leaf_style(240.0, 24.0)).unwrap();
        tree.compute(pag, Size::MAX_CONTENT).unwrap();

        // Click next (last element at ~220px)
        let ev_next = tree.dispatch_click(pag, (220.0, 12.0)).unwrap();
        assert_eq!(ev_next, Some(ui_core::UiEvent::PageSelected { widget_id: "data_pag".to_string(), page: 3 }));
    }

    #[test]
    fn badge_and_color_swatch_render() {
        let mut tree = WidgetTree::new();
        let b = tree.badge("SECURE", crate::kind::ListItemBadge::Success, leaf_style(60.0, 20.0)).unwrap();
        let cs = tree.color_swatch("theme_accent", [0.0, 0.85, 1.0, 1.0], Some("#00D9FF"), leaf_style(40.0, 50.0)).unwrap();
        let cont = tree.container(&[b, cs], leaf_style(120.0, 60.0)).unwrap();
        tree.compute(cont, Size::MAX_CONTENT).unwrap();

        let theme = crate::Theme::cyber_glass();
        let frame = tree.build_frame(cont, &theme, Default::default()).unwrap();
        assert!(frame.texts.iter().any(|t| t.text == "SECURE"));
        assert!(frame.texts.iter().any(|t| t.text == "#00D9FF"));

        let ev = tree.dispatch_click(cont, (80.0, 20.0)).unwrap();
        assert_eq!(ev, Some(ui_core::UiEvent::ColorSelected { widget_id: "theme_accent".to_string(), color: [0.0, 0.85, 1.0, 1.0] }));
    }

    #[test]
    fn color_picker_renders_and_dispatches_mode_switches() {
        let mut tree = WidgetTree::new();
        let color = [0.0, 0.85, 1.0, 1.0]; // Bright Cyan
        let cp = tree.color_picker("main_peeker", color, ColorSpace::Lab, leaf_style(508.0, 208.0)).unwrap();
        let root = tree.container(&[cp], leaf_style(508.0, 208.0)).unwrap();
        tree.compute(root, Size::MAX_CONTENT).unwrap();

        let theme = crate::Theme::cyber_glass();
        let frame = tree.build_frame(root, &theme, Default::default()).unwrap();
        assert!(frame.texts.iter().any(|t| t.text.starts_with("HEX")));
        assert!(frame.texts.iter().any(|t| t.text == "RGB"));
        assert!(frame.texts.iter().any(|t| t.text == "CMYK"));
        assert!(frame.texts.iter().any(|t| t.text == "HSV"));
        assert!(frame.texts.iter().any(|t| t.text == "LAB"));

        // 1. Clicking on 2D Saturation / Value Canvas (x=250.0, y=50.0)
        let ev_sv = tree.dispatch_click(root, (250.0, 50.0)).unwrap();
        assert!(matches!(ev_sv, Some(ui_core::UiEvent::ColorChanged { .. })));

        // 2. Clicking on Rainbow Hue Slider Bar (x=200.0, y=105.0)
        let ev_hue = tree.dispatch_click(root, (200.0, 105.0)).unwrap();
        assert!(matches!(ev_hue, Some(ui_core::UiEvent::ColorChanged { .. })));

        // 3. Continuous drag sampling on 2D Canvas and Hue Slider
        let drag_sv = tree.color_picker_hue_at(root, (250.0, 50.0)).unwrap();
        assert!(drag_sv.is_some());
        assert_eq!(drag_sv.unwrap().0, "main_peeker");

        let drag_hue = tree.color_picker_hue_at(root, (200.0, 105.0)).unwrap();
        assert!(drag_hue.is_some());
        assert_eq!(drag_hue.unwrap().0, "main_peeker");

        // 4. Clicking CMYK mini-card in bottom row (column 1: x in ~140..250, y=170)
        let ev_card = tree.dispatch_click(root, (180.0, 175.0)).unwrap();
        assert_eq!(ev_card, Some(ui_core::UiEvent::SelectChanged {
            widget_id: "main_peeker".to_string(),
            selected_id: "cmyk".to_string(),
        }));

        // 5. Clicking swatch (x=30.0, y=30.0)
        let ev_swatch = tree.dispatch_click(root, (30.0, 30.0)).unwrap();
        assert_eq!(ev_swatch, Some(ui_core::UiEvent::ColorSelected {
            widget_id: "main_peeker".to_string(),
            color: [0.0, 0.85, 1.0, 1.0],
        }));
    }

    #[test]
    fn palette_folding_closing_and_dragging() {
        let mut tree = WidgetTree::new();
        let btn = tree.button("brush_btn", "Brush", true, leaf_style(80.0, 26.0)).unwrap();
        let pal = tree.palette(
            "test_palette",
            "Tools",
            false,
            Some(btn),
            leaf_style(160.0, 120.0),
        ).unwrap();
        tree.compute(pal, Size::MAX_CONTENT).unwrap();

        let theme = crate::Theme::cyber_glass();
        let frame = tree.build_frame(pal, &theme, Default::default()).unwrap();
        assert!(frame.texts.iter().any(|t| t.text == "Tools"));
        assert!(frame.texts.iter().any(|t| t.text == "▲"));
        assert!(frame.texts.iter().any(|t| t.text == "✕"));
        assert!(frame.texts.iter().any(|t| t.text == "⇲"));

        // 1. Header dragging detection
        let drag_anchor = tree.palette_drag_anchor_at(pal, (60.0, 14.0)).unwrap();
        assert!(drag_anchor.is_some());
        let (drag_id, (ax, ay)) = drag_anchor.unwrap();
        assert_eq!(drag_id, "test_palette");
        assert_eq!(ax, 60.0);
        assert_eq!(ay, 14.0);

        // 2. Resize grip detection at bottom-right (x=152.0, y=112.0)
        let resize_info = tree.palette_resize_at(pal, (152.0, 112.0)).unwrap();
        assert!(resize_info.is_some());
        let (res_id, (cw, ch)) = resize_info.unwrap();
        assert_eq!(res_id, "test_palette");
        assert_eq!(cw, 160.0);
        assert_eq!(ch, 120.0);

        // 3. Fold button click (fold button is around x=118..138, y=14)
        let ev_fold = tree.dispatch_click(pal, (126.0, 14.0)).unwrap();
        assert_eq!(ev_fold, Some(ui_core::UiEvent::PaletteFoldToggled {
            palette_id: "test_palette".to_string(),
            folded: true,
        }));

        // 4. Close button click (close button is around x=138..158, y=14)
        let ev_close = tree.dispatch_click(pal, (148.0, 14.0)).unwrap();
        assert_eq!(ev_close, Some(ui_core::UiEvent::PaletteClosed {
            palette_id: "test_palette".to_string(),
        }));
    }

    #[test]
    fn text_cursor_click_and_caret_positioning() {
        let mut tree = WidgetTree::new();
        let input = tree.text_input_with_cursor("search", "abcdef", "placeholder", true, 3, None, leaf_style(200.0, 32.0)).unwrap();
        let root = tree.container(&[input], leaf_style(200.0, 32.0)).unwrap();
        tree.compute(root, Size::MAX_CONTENT).unwrap();

        let theme = crate::Theme::cyber_glass();
        let frame = tree.build_frame(root, &theme, Default::default()).unwrap();
        // Should produce background instance + caret instance
        assert!(frame.instances.len() >= 2);

        // Click at text start -> char index 0
        let cur_start = tree.text_cursor_at(root, (12.0, 16.0), &theme.typography.family, 15.0, None).unwrap();
        assert_eq!(cur_start, Some(("search".to_string(), 0)));

        // Click far to the right -> char index 6 (end of string)
        let cur_end = tree.text_cursor_at(root, (180.0, 16.0), &theme.typography.family, 15.0, None).unwrap();
        assert_eq!(cur_end, Some(("search".to_string(), 6)));
    }

    #[test]
    fn textarea_multiline_cursor_and_selection() {
        let mut tree = WidgetTree::new();
        let text = "Line 1: Hello\nLine 2: World\nLine 3: Rust";
        let editor = tree.text_area_with_cursor("editor", text, "type...", true, true, 16, Some((0, 6)), leaf_style(300.0, 120.0)).unwrap();
        let root = tree.container(&[editor], leaf_style(300.0, 120.0)).unwrap();
        tree.compute(root, Size::MAX_CONTENT).unwrap();

        let theme = crate::Theme::cyber_glass();
        let frame = tree.build_frame(root, &theme, Default::default()).unwrap();
        // Has line numbers 1, 2, 3
        assert!(frame.texts.iter().any(|t| t.text == "1"));
        assert!(frame.texts.iter().any(|t| t.text == "2"));
        assert!(frame.texts.iter().any(|t| t.text == "3"));
        assert!(frame.texts.iter().any(|t| t.text == "Line 1: Hello"));

        // Click on Line 2 (start_y=8, line_h=20 -> y=32 is Line 2)
        // Line 1 is 13 chars + 1 ('\n') = 14 bytes offset.
        let cur_line2 = tree.text_cursor_at(root, (50.0, 32.0), &theme.typography.family, 15.0, None).unwrap();
        assert!(cur_line2.is_some());
        let (id, offset) = cur_line2.unwrap();
        assert_eq!(id, "editor");
        assert!(offset >= 14); // Points into line 2
    }

    #[test]
    fn image_widget_produces_configured_media_spec() {
        let mut tree = WidgetTree::new();
        let img = tree.image("artwork", "res://cyber.png", MediaFit::Contain, leaf_style(200.0, 150.0)).unwrap();
        let root = tree.container(&[img], leaf_style(300.0, 200.0)).unwrap();
        tree.compute(root, Size::MAX_CONTENT).unwrap();

        let theme = Theme::cyber_glass();
        let frame = tree.build_frame(root, &theme, Default::default()).unwrap();
        assert_eq!(frame.media.len(), 1);
        assert_eq!(frame.media[0].kind, MediaKind("image"));
        assert_eq!(frame.media[0].resource_id, "res://cyber.png");
        assert_eq!(frame.media[0].fit, MediaFit::Contain);
    }

    #[test]
    fn video_player_renders_media_spec_and_transport_controls() {
        let mut tree = WidgetTree::new();
        let player = tree.video_player("stream_player", "res://stream_feed", true, 0.45, 120.0, 1.0, leaf_style(400.0, 240.0)).unwrap();
        let root = tree.container(&[player], leaf_style(400.0, 240.0)).unwrap();
        tree.compute(root, Size::MAX_CONTENT).unwrap();

        let theme = Theme::cyber_glass();
        let frame = tree.build_frame(root, &theme, Default::default()).unwrap();
        assert_eq!(frame.media.len(), 1);
        assert_eq!(frame.media[0].kind, MediaKind("video"));
        assert_eq!(frame.media[0].resource_id, "res://stream_feed");

        // Transport controls: pause glyph (playing=true) + timecode string
        assert!(frame.texts.iter().any(|t| t.text == "⏸"));
        assert!(frame.texts.iter().any(|t| t.text.contains("00:54 / 02:00")));
    }

    #[test]
    fn audio_visualizer_generates_proportional_frequency_bars() {
        let mut tree = WidgetTree::new();
        let spectrum = [0.2, 0.8, 0.5, 0.95, 0.3, 0.6];
        let viz = tree.audio_visualizer("spectrum_bars", &spectrum, 1.0, leaf_style(200.0, 60.0)).unwrap();
        let root = tree.container(&[viz], leaf_style(200.0, 60.0)).unwrap();
        tree.compute(root, Size::MAX_CONTENT).unwrap();

        let theme = Theme::cyber_glass();
        let frame = tree.build_frame(root, &theme, Default::default()).unwrap();
        // Background instance + 6 frequency bar instances
        assert_eq!(frame.instances.len(), 7);
    }
}
