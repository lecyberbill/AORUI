// [WFGY] Zone: TRANSIT | λ: 0.2 | Fallbacks: 0 | Action: Declarative widget builders (button, label, input, list, checkbox, window, tabs, scroll, menus, overlays)
use std::collections::HashMap;

use ui_layout::{
    auto, length, percent, AlignItems, AvailableSpace, LayoutError, LayoutTree, NodeId, Position,
    Rect, Size, Style,
};

use crate::id::WidgetId;
use crate::kind::WidgetKind;
use crate::media::MediaKind;

const CLOSE_BUTTON_SIZE: f32 = 20.0;

/// Declarative widget tree: wraps [`LayoutTree<WidgetKind>`] and provides
/// ergonomic constructors per widget type. Rebuilt each frame; persistent application
/// state is owned by the application.
pub struct WidgetTree {
    layout: LayoutTree<WidgetKind>,
}

impl WidgetTree {
    pub fn new() -> Self {
        Self {
            layout: LayoutTree::new(),
        }
    }

    pub fn layout(&self) -> &LayoutTree<WidgetKind> {
        &self.layout
    }

    pub fn compute(
        &mut self,
        root: NodeId,
        available_space: Size<AvailableSpace>,
    ) -> Result<(), LayoutError> {
        self.layout.compute(root, available_space)
    }

    pub fn resolved_bounds(&self, root: NodeId) -> Result<HashMap<NodeId, [f32; 4]>, LayoutError> {
        self.layout.resolved_bounds(root)
    }

    pub fn label(&mut self, text: impl Into<String>, style: Style) -> Result<NodeId, LayoutError> {
        self.layout.insert_leaf(
            style,
            WidgetKind::Label {
                text: text.into(),
                muted: false,
            },
        )
    }

    pub fn label_muted(
        &mut self,
        text: impl Into<String>,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.layout.insert_leaf(
            style,
            WidgetKind::Label {
                text: text.into(),
                muted: true,
            },
        )
    }

    pub fn button(
        &mut self,
        id: impl Into<WidgetId>,
        label: impl Into<String>,
        enabled: bool,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.layout.insert_leaf(
            style,
            WidgetKind::Button {
                id: id.into(),
                label: label.into(),
                enabled,
            },
        )
    }

    /// Icon button with hover glow and tactile press feedback.
    pub fn icon_button(
        &mut self,
        id: impl Into<WidgetId>,
        icon: crate::kind::IconKind,
        enabled: bool,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.layout.insert_leaf(
            style,
            WidgetKind::IconButton {
                id: id.into(),
                icon,
                enabled,
            },
        )
    }

    /// Scalable cyber vector glyph icon.
    pub fn icon(
        &mut self,
        kind: crate::kind::IconKind,
        size: f32,
        color: Option<[f32; 4]>,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.layout
            .insert_leaf(style, WidgetKind::Icon { kind, size, color })
    }

    /// Multi-column data table grid with clickable sortable headers and selectable rows.
    pub fn table<H, R, C>(
        &mut self,
        id: impl Into<WidgetId>,
        columns: &[(H, f32, Option<bool>)],
        rows: &[R],
        selected_row: Option<usize>,
        row_height: f32,
        container_style: Style,
    ) -> Result<NodeId, LayoutError>
    where
        H: AsRef<str>,
        R: AsRef<[(C, crate::kind::ListItemBadge)]>,
        C: AsRef<str>,
    {
        let id = id.into();
        let mut all_rows = Vec::with_capacity(rows.len() + 1);

        // 1. Header row
        let mut header_cells = Vec::with_capacity(columns.len());
        for (col_idx, (title, width, sorted_asc)) in columns.iter().enumerate() {
            let h_style = Style {
                size: Size {
                    width: length(*width),
                    height: length(26.0),
                },
                ..Default::default()
            };
            let cell = self.layout.insert_leaf(
                h_style,
                WidgetKind::TableHeader {
                    owner: id.clone(),
                    column_index: col_idx,
                    title: title.as_ref().to_string(),
                    sorted_asc: *sorted_asc,
                },
            )?;
            header_cells.push(cell);
        }
        let header_row_style = Style {
            flex_direction: ui_layout::FlexDirection::Row,
            size: Size {
                width: percent(1.0),
                height: length(26.0),
            },
            gap: Size {
                width: length(4.0),
                height: length(0.0),
            },
            ..Default::default()
        };
        let header_row =
            self.layout
                .insert_container(header_row_style, &header_cells, WidgetKind::Container)?;
        all_rows.push(header_row);

        // 2. Data rows
        for (r_idx, row_data) in rows.iter().enumerate() {
            let cells_data = row_data.as_ref();
            let mut row_cells = Vec::with_capacity(cells_data.len());
            let is_selected = selected_row == Some(r_idx);

            for (c_idx, (text, badge)) in cells_data.iter().enumerate() {
                let col_w = columns.get(c_idx).map(|c| c.1).unwrap_or(100.0);
                let cell_style = Style {
                    size: Size {
                        width: length(col_w),
                        height: length(row_height),
                    },
                    ..Default::default()
                };
                let cell = self.layout.insert_leaf(
                    cell_style,
                    WidgetKind::TableCell {
                        owner: id.clone(),
                        row_index: r_idx,
                        column_index: c_idx,
                        text: text.as_ref().to_string(),
                        badge: badge.clone(),
                        selected: is_selected,
                    },
                )?;
                row_cells.push(cell);
            }

            let row_style = Style {
                flex_direction: ui_layout::FlexDirection::Row,
                size: Size {
                    width: percent(1.0),
                    height: length(row_height),
                },
                gap: Size {
                    width: length(4.0),
                    height: length(0.0),
                },
                ..Default::default()
            };
            let r_node =
                self.layout
                    .insert_container(row_style, &row_cells, WidgetKind::Container)?;
            all_rows.push(r_node);
        }

        let style = Style {
            flex_direction: ui_layout::FlexDirection::Column,
            gap: Size {
                width: length(0.0),
                height: length(2.0),
            },
            ..container_style
        };
        self.layout
            .insert_container(style, &all_rows, WidgetKind::Container)
    }

    /// Collapsible accordion section with animated indicator arrow and expandable body.
    pub fn accordion(
        &mut self,
        id: impl Into<WidgetId>,
        title: impl Into<String>,
        subtitle: Option<impl Into<String>>,
        expanded: bool,
        content: NodeId,
        container_style: Style,
    ) -> Result<NodeId, LayoutError> {
        let id = id.into();
        let subtitle_str: Option<String> = subtitle.map(|s| s.into());
        let header_h = if subtitle_str.is_some() { 46.0 } else { 34.0 };
        let header_style = Style {
            size: Size {
                width: percent(1.0),
                height: length(header_h),
            },
            ..Default::default()
        };
        let header = self.layout.insert_leaf(
            header_style,
            WidgetKind::AccordionHeader {
                id: id.clone(),
                title: title.into(),
                subtitle: subtitle_str,
                expanded,
            },
        )?;

        let mut children = vec![header];
        if expanded {
            children.push(content);
        }

        let style = Style {
            flex_direction: ui_layout::FlexDirection::Column,
            gap: Size {
                width: length(0.0),
                height: length(4.0),
            },
            ..container_style
        };
        self.layout
            .insert_container(style, &children, WidgetKind::Container)
    }

    /// Breadcrumb navigation bar showing hierarchical path segments.
    pub fn breadcrumb<I, L>(
        &mut self,
        id: impl Into<WidgetId>,
        items: &[(I, L)],
        item_style: Style,
        bar_style: Style,
    ) -> Result<NodeId, LayoutError>
    where
        I: Into<String> + Clone,
        L: AsRef<str>,
    {
        let id = id.into();
        let total = items.len();
        let mut child_nodes = Vec::with_capacity(total);

        for (idx, (item_id, label)) in items.iter().enumerate() {
            let is_last = idx + 1 == total;
            let node = self.layout.insert_leaf(
                item_style.clone(),
                WidgetKind::BreadcrumbItem {
                    owner: id.clone(),
                    index: idx,
                    id: item_id.clone().into(),
                    label: label.as_ref().to_string(),
                    is_last,
                },
            )?;
            child_nodes.push(node);
        }

        let style = Style {
            flex_direction: ui_layout::FlexDirection::Row,
            align_items: Some(AlignItems::Center),
            gap: Size {
                width: length(4.0),
                height: length(0.0),
            },
            ..bar_style
        };
        self.layout
            .insert_container(style, &child_nodes, WidgetKind::Container)
    }

    /// Multi-page pagination bar.
    pub fn pagination(
        &mut self,
        id: impl Into<WidgetId>,
        current_page: usize,
        total_pages: usize,
        item_style: Style,
        bar_style: Style,
    ) -> Result<NodeId, LayoutError> {
        let id = id.into();
        let mut child_nodes = Vec::new();

        // 1. Prev button
        let prev_page = current_page.saturating_sub(1);
        let prev_node = self.layout.insert_leaf(
            item_style.clone(),
            WidgetKind::PaginationItem {
                owner: id.clone(),
                page: prev_page,
                label: "‹".to_string(),
                active: false,
                disabled: current_page == 0,
            },
        )?;
        child_nodes.push(prev_node);

        // 2. Page number buttons
        for p in 0..total_pages {
            let p_node = self.layout.insert_leaf(
                item_style.clone(),
                WidgetKind::PaginationItem {
                    owner: id.clone(),
                    page: p,
                    label: (p + 1).to_string(),
                    active: p == current_page,
                    disabled: false,
                },
            )?;
            child_nodes.push(p_node);
        }

        // 3. Next button
        let next_page = (current_page + 1).min(total_pages.saturating_sub(1));
        let next_node = self.layout.insert_leaf(
            item_style.clone(),
            WidgetKind::PaginationItem {
                owner: id.clone(),
                page: next_page,
                label: "›".to_string(),
                active: false,
                disabled: current_page + 1 >= total_pages,
            },
        )?;
        child_nodes.push(next_node);

        let style = Style {
            flex_direction: ui_layout::FlexDirection::Row,
            align_items: Some(AlignItems::Center),
            gap: Size {
                width: length(4.0),
                height: length(0.0),
            },
            ..bar_style
        };
        self.layout
            .insert_container(style, &child_nodes, WidgetKind::Container)
    }

    /// Standalone status badge / tag / capsule pill.
    pub fn badge(
        &mut self,
        label: impl Into<String>,
        badge: crate::kind::ListItemBadge,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.layout.insert_leaf(
            style,
            WidgetKind::Badge {
                label: label.into(),
                badge,
            },
        )
    }

    /// Color preview swatch tile.
    pub fn color_swatch(
        &mut self,
        id: impl Into<WidgetId>,
        color: [f32; 4],
        label: Option<impl Into<String>>,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.layout.insert_leaf(
            style,
            WidgetKind::ColorSwatch {
                id: id.into(),
                color,
                label: label.map(|s| s.into()),
            },
        )
    }

    /// Interactive Color Peeker / Picker card with live swatch and mode conversions (RGB, HEX, CIELAB, CMYK).
    pub fn color_picker(
        &mut self,
        id: impl Into<WidgetId>,
        color: [f32; 4],
        space: crate::color::ColorSpace,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.layout.insert_leaf(
            style,
            WidgetKind::ColorPicker {
                id: id.into(),
                color,
                space,
            },
        )
    }

    pub fn checkbox(
        &mut self,
        id: impl Into<WidgetId>,
        checked: bool,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.layout.insert_leaf(
            style,
            WidgetKind::Checkbox {
                id: id.into(),
                checked,
            },
        )
    }

    pub fn toggle(
        &mut self,
        id: impl Into<WidgetId>,
        active: bool,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.layout.insert_leaf(
            style,
            WidgetKind::Toggle {
                id: id.into(),
                active,
            },
        )
    }

    pub fn slider(
        &mut self,
        id: impl Into<WidgetId>,
        min: f32,
        max: f32,
        value: f32,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.slider_oriented(
            id,
            min,
            max,
            value,
            crate::kind::SliderOrientation::Horizontal,
            style,
        )
    }

    /// Vertical fader slider (e.g. for audio mixers, equalizers, volume faders).
    pub fn slider_vertical(
        &mut self,
        id: impl Into<WidgetId>,
        min: f32,
        max: f32,
        value: f32,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.slider_oriented(
            id,
            min,
            max,
            value,
            crate::kind::SliderOrientation::Vertical,
            style,
        )
    }

    /// Generic oriented slider (Horizontal or Vertical).
    pub fn slider_oriented(
        &mut self,
        id: impl Into<WidgetId>,
        min: f32,
        max: f32,
        value: f32,
        orientation: crate::kind::SliderOrientation,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.layout.insert_leaf(
            style,
            WidgetKind::Slider {
                id: id.into(),
                min,
                max,
                value,
                orientation,
            },
        )
    }

    pub fn progress_bar(&mut self, progress: f32, style: Style) -> Result<NodeId, LayoutError> {
        self.progress_bar_custom(progress, crate::kind::ProgressKind::Horizontal, None, style)
    }

    /// Vertical progress bar filling from bottom to top.
    pub fn progress_bar_vertical(
        &mut self,
        progress: f32,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.progress_bar_custom(progress, crate::kind::ProgressKind::Vertical, None, style)
    }

    /// Circular donut ring progress indicator with optional center label.
    pub fn progress_ring(
        &mut self,
        progress: f32,
        label: Option<impl Into<String>>,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.progress_bar_custom(
            progress,
            crate::kind::ProgressKind::Ring,
            label.map(|s| s.into()),
            style,
        )
    }

    /// Circular pie / camembert progress indicator with optional center label.
    pub fn progress_pie(
        &mut self,
        progress: f32,
        label: Option<impl Into<String>>,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.progress_bar_custom(
            progress,
            crate::kind::ProgressKind::Pie,
            label.map(|s| s.into()),
            style,
        )
    }

    /// Generic custom progress indicator (Horizontal, Vertical, Ring, Pie).
    pub fn progress_bar_custom(
        &mut self,
        progress: f32,
        kind: crate::kind::ProgressKind,
        label: Option<String>,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.layout.insert_leaf(
            style,
            WidgetKind::ProgressBar {
                progress,
                kind,
                label,
            },
        )
    }

    pub fn metric_card(
        &mut self,
        title: impl Into<String>,
        value: impl Into<String>,
        delta: Option<(impl Into<String>, bool)>,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        let delta = delta.map(|(text, positive)| (text.into(), positive));
        self.layout.insert_leaf(
            style,
            WidgetKind::MetricCard {
                title: title.into(),
                value: value.into(),
                delta,
            },
        )
    }

    /// Vertical scrollbar for the `ScrollView` identified by `id`.
    /// `content_size`/`viewport_size` define proportional thumb size, `offset` defines track position.
    pub fn scrollbar(
        &mut self,
        id: impl Into<WidgetId>,
        content_size: f32,
        viewport_size: f32,
        offset: f32,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.layout.insert_leaf(
            style,
            WidgetKind::Scrollbar {
                id: id.into(),
                content_size,
                viewport_size,
                offset,
            },
        )
    }

    pub fn text_input(
        &mut self,
        id: impl Into<WidgetId>,
        value: impl Into<String>,
        placeholder: impl Into<String>,
        focused: bool,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        let val_str = value.into();
        let cursor = val_str.len();
        self.text_input_with_cursor(id, val_str, placeholder, focused, cursor, None, style)
    }

    pub fn text_input_with_cursor(
        &mut self,
        id: impl Into<WidgetId>,
        value: impl Into<String>,
        placeholder: impl Into<String>,
        focused: bool,
        cursor: usize,
        selection: Option<(usize, usize)>,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        let val = value.into();
        let safe_cursor = cursor.min(val.len());
        self.layout.insert_leaf(
            style,
            WidgetKind::TextInput {
                id: id.into(),
                value: val,
                placeholder: placeholder.into(),
                focused,
                cursor: safe_cursor,
                selection,
            },
        )
    }

    /// Multi-line text area editor with optional line numbers gutter.
    pub fn text_area(
        &mut self,
        id: impl Into<WidgetId>,
        value: impl Into<String>,
        placeholder: impl Into<String>,
        focused: bool,
        line_numbers: bool,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        let val_str = value.into();
        let cursor = val_str.len();
        self.text_area_with_cursor(
            id,
            val_str,
            placeholder,
            focused,
            line_numbers,
            cursor,
            None,
            style,
        )
    }

    pub fn text_area_with_cursor(
        &mut self,
        id: impl Into<WidgetId>,
        value: impl Into<String>,
        placeholder: impl Into<String>,
        focused: bool,
        line_numbers: bool,
        cursor: usize,
        selection: Option<(usize, usize)>,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        let val = value.into();
        let safe_cursor = cursor.min(val.len());
        self.layout.insert_leaf(
            style,
            WidgetKind::TextArea {
                id: id.into(),
                value: val,
                placeholder: placeholder.into(),
                focused,
                line_numbers,
                cursor: safe_cursor,
                selection,
            },
        )
    }

    /// Password / masked input field with toggleable eye reveal glyph.
    pub fn password_input(
        &mut self,
        id: impl Into<WidgetId>,
        value: impl Into<String>,
        placeholder: impl Into<String>,
        focused: bool,
        revealed: bool,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        let val_str = value.into();
        let cursor = val_str.len();
        self.password_input_with_cursor(id, val_str, placeholder, focused, revealed, cursor, style)
    }

    pub fn password_input_with_cursor(
        &mut self,
        id: impl Into<WidgetId>,
        value: impl Into<String>,
        placeholder: impl Into<String>,
        focused: bool,
        revealed: bool,
        cursor: usize,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        let val = value.into();
        let safe_cursor = cursor.min(val.len());
        self.layout.insert_leaf(
            style,
            WidgetKind::PasswordInput {
                id: id.into(),
                value: val,
                placeholder: placeholder.into(),
                focused,
                revealed,
                cursor: safe_cursor,
            },
        )
    }

    /// Numeric spinner input field with stepper increment/decrement buttons and min/max clamping.
    pub fn number_input(
        &mut self,
        id: impl Into<WidgetId>,
        value: f64,
        min: f64,
        max: f64,
        step: f64,
        precision: usize,
        focused: bool,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.number_input_state(id, value, min, max, step, precision, focused, true, style)
    }

    /// Numeric spinner input field with customizable enabled state.
    pub fn number_input_state(
        &mut self,
        id: impl Into<WidgetId>,
        value: f64,
        min: f64,
        max: f64,
        step: f64,
        precision: usize,
        focused: bool,
        enabled: bool,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.layout.insert_leaf(
            style,
            WidgetKind::NumberInput {
                id: id.into(),
                value: value.clamp(min, max),
                min,
                max,
                step,
                precision,
                focused,
                enabled,
            },
        )
    }

    /// Context menu popover positioned at cursor coordinate (x, y).
    pub fn context_menu<S1, S2>(
        &mut self,
        id: impl Into<WidgetId>,
        items: &[(impl Into<WidgetId> + Clone, S1, Option<S2>, bool)],
        item_style: Style,
        popover_style: Style,
    ) -> Result<NodeId, LayoutError>
    where
        S1: AsRef<str>,
        S2: AsRef<str>,
    {
        self.menu_popover(id, items, item_style, popover_style)
    }

    pub fn radio(
        &mut self,
        id: impl Into<WidgetId>,
        group_id: impl Into<String>,
        label: impl Into<String>,
        selected: bool,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.layout.insert_leaf(
            style,
            WidgetKind::RadioButton {
                id: id.into(),
                group_id: group_id.into(),
                label: label.into(),
                selected,
            },
        )
    }

    pub fn divider(&mut self, vertical: bool, style: Style) -> Result<NodeId, LayoutError> {
        self.layout
            .insert_leaf(style, WidgetKind::Divider { vertical })
    }

    pub fn segmented_control(
        &mut self,
        id: impl Into<WidgetId>,
        options: &[impl AsRef<str>],
        selected_index: usize,
        option_style: Style,
        bar_style: Style,
    ) -> Result<NodeId, LayoutError> {
        let id = id.into();
        let mut items = Vec::with_capacity(options.len());
        for (index, label) in options.iter().enumerate() {
            let kind = WidgetKind::SegmentItem {
                owner: id.clone(),
                index,
                label: label.as_ref().to_string(),
                selected: index == selected_index,
            };
            items.push(self.layout.insert_leaf(option_style.clone(), kind)?);
        }
        self.layout
            .insert_container(bar_style, &items, WidgetKind::Container)
    }

    pub fn modal(
        &mut self,
        id: impl Into<WidgetId>,
        title: impl Into<String>,
        children: &[NodeId],
        dialog_style: Style,
        backdrop_style: Style,
    ) -> Result<NodeId, LayoutError> {
        let id = id.into();

        let close_button_style = Style {
            position: Position::Absolute,
            inset: Rect {
                top: length(8.0),
                right: length(8.0),
                bottom: auto(),
                left: auto(),
            },
            size: Size {
                width: length(CLOSE_BUTTON_SIZE),
                height: length(CLOSE_BUTTON_SIZE),
            },
            ..Default::default()
        };
        let close_button = self.layout.insert_leaf(
            close_button_style,
            WidgetKind::ModalBackdrop { owner: id.clone() },
        )?;

        let mut all_children = Vec::with_capacity(children.len() + 1);
        all_children.extend_from_slice(children);
        all_children.push(close_button);

        let dialog = self.layout.insert_container(
            dialog_style,
            &all_children,
            WidgetKind::Modal {
                id: id.clone(),
                title: title.into(),
            },
        )?;

        self.layout.insert_container(
            backdrop_style,
            &[dialog],
            WidgetKind::ModalBackdrop { owner: id },
        )
    }

    /// Standard modal overlay dialog: automatically creates a 100% viewport backdrop,
    /// centered layout, and close button.
    pub fn modal_dialog(
        &mut self,
        id: impl Into<WidgetId>,
        title: impl Into<String>,
        children: &[NodeId],
        dialog_style: Style,
    ) -> Result<NodeId, LayoutError> {
        let backdrop_style = Style {
            position: Position::Absolute,
            inset: Rect {
                top: length(0.0),
                right: length(0.0),
                bottom: length(0.0),
                left: length(0.0),
            },
            size: Size {
                width: percent(1.0),
                height: percent(1.0),
            },
            ..Default::default()
        };
        self.modal(id, title, children, dialog_style, backdrop_style)
    }

    pub fn container(&mut self, children: &[NodeId], style: Style) -> Result<NodeId, LayoutError> {
        self.layout
            .insert_container(style, children, WidgetKind::Container)
    }

    /// High-level CSS Grid container helper with N equal-fraction columns and customizable row/col gaps.
    pub fn grid(
        &mut self,
        columns: usize,
        column_gap: f32,
        row_gap: f32,
        children: &[NodeId],
        base_style: Style,
    ) -> Result<NodeId, LayoutError> {
        let grid_style = Style {
            display: ui_layout::Display::Grid,
            grid_template_columns: vec![ui_layout::fr(1.0); columns.max(1)],
            gap: Size {
                width: length(column_gap),
                height: length(row_gap),
            },
            ..base_style
        };
        self.layout
            .insert_container(grid_style, children, WidgetKind::Container)
    }

    /// Resizable split view container with a draggable splitter bar.
    pub fn split_view(
        &mut self,
        id: impl Into<WidgetId>,
        orientation: crate::kind::SplitOrientation,
        _ratio: f32,
        first: NodeId,
        second: NodeId,
        container_style: Style,
    ) -> Result<NodeId, LayoutError> {
        let id = id.into();
        let splitter_size = 6.0;

        let splitter_style = match orientation {
            crate::kind::SplitOrientation::Horizontal => Style {
                size: Size {
                    width: length(splitter_size),
                    height: percent(1.0),
                },
                flex_shrink: 0.0,
                ..Default::default()
            },
            crate::kind::SplitOrientation::Vertical => Style {
                size: Size {
                    width: percent(1.0),
                    height: length(splitter_size),
                },
                flex_shrink: 0.0,
                ..Default::default()
            },
        };

        let splitter = self.layout.insert_leaf(
            splitter_style,
            WidgetKind::Splitter {
                owner: id.clone(),
                orientation,
            },
        )?;

        let flex_dir = match orientation {
            crate::kind::SplitOrientation::Horizontal => ui_layout::FlexDirection::Row,
            crate::kind::SplitOrientation::Vertical => ui_layout::FlexDirection::Column,
        };

        let style = Style {
            flex_direction: flex_dir,
            align_items: Some(AlignItems::Stretch),
            ..container_style
        };

        self.layout
            .insert_container(style, &[first, splitter, second], WidgetKind::Container)
    }

    /// Hierarchical tree view container.
    pub fn tree_view<I, S>(
        &mut self,
        owner: impl Into<WidgetId>,
        nodes: &[(I, S, usize, bool, bool, bool)],
        item_style: Style,
        container_style: Style,
    ) -> Result<NodeId, LayoutError>
    where
        I: Into<WidgetId> + Clone,
        S: AsRef<str>,
    {
        let owner = owner.into();
        let mut child_nodes = Vec::with_capacity(nodes.len());
        for (node_id, label, depth, is_dir, expanded, selected) in nodes {
            let kind = WidgetKind::TreeNode {
                owner: owner.clone(),
                id: node_id.clone().into(),
                label: label.as_ref().to_string(),
                depth: *depth,
                is_dir: *is_dir,
                expanded: *expanded,
                selected: *selected,
            };
            child_nodes.push(self.layout.insert_leaf(item_style.clone(), kind)?);
        }
        let style = Style {
            flex_direction: ui_layout::FlexDirection::Column,
            ..container_style
        };
        self.layout
            .insert_container(style, &child_nodes, WidgetKind::Container)
    }

    /// Window container: cyber-glass frame with title bar and top-right close button.
    pub fn window(
        &mut self,
        id: impl Into<WidgetId>,
        title: impl Into<String>,
        children: &[NodeId],
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        let id = id.into();

        let close_button_style = Style {
            position: Position::Absolute,
            inset: Rect {
                top: length(8.0),
                right: length(8.0),
                bottom: auto(),
                left: auto(),
            },
            size: Size {
                width: length(CLOSE_BUTTON_SIZE),
                height: length(CLOSE_BUTTON_SIZE),
            },
            ..Default::default()
        };
        let close_button = self.layout.insert_leaf(
            close_button_style,
            WidgetKind::WindowCloseButton { owner: id.clone() },
        )?;

        let mut all_children = Vec::with_capacity(children.len() + 1);
        all_children.extend_from_slice(children);
        all_children.push(close_button);

        self.layout.insert_container(
            style,
            &all_children,
            WidgetKind::Window {
                id,
                title: title.into(),
            },
        )
    }

    /// Horizontal application menu bar.
    pub fn menubar(
        &mut self,
        id: impl Into<WidgetId>,
        items: &[impl AsRef<str>],
        active_menu: Option<usize>,
        item_style: Style,
        bar_style: Style,
    ) -> Result<NodeId, LayoutError> {
        let id = id.into();
        let mut menu_nodes = Vec::with_capacity(items.len());
        for (index, label) in items.iter().enumerate() {
            let kind = WidgetKind::MenuBarItem {
                owner: id.clone(),
                index,
                label: label.as_ref().to_string(),
                active: active_menu == Some(index),
            };
            menu_nodes.push(self.layout.insert_leaf(item_style.clone(), kind)?);
        }
        self.layout
            .insert_container(bar_style, &menu_nodes, WidgetKind::MenuBar)
    }

    /// Floating dropdown contextual menu (popover).
    pub fn menu_popover<I, S, O>(
        &mut self,
        owner: impl Into<WidgetId>,
        items: &[(I, S, Option<O>, bool)],
        item_style: Style,
        popover_style: Style,
    ) -> Result<NodeId, LayoutError>
    where
        I: Into<WidgetId> + Clone,
        S: AsRef<str>,
        O: AsRef<str>,
    {
        let owner = owner.into();
        let mut item_nodes = Vec::with_capacity(items.len());
        for (item_id, label, shortcut, enabled) in items {
            let kind = WidgetKind::MenuItem {
                owner: owner.clone(),
                id: item_id.clone().into(),
                label: label.as_ref().to_string(),
                shortcut: shortcut.as_ref().map(|s| s.as_ref().to_string()),
                enabled: *enabled,
            };
            item_nodes.push(self.layout.insert_leaf(item_style.clone(), kind)?);
        }
        self.layout
            .insert_container(popover_style, &item_nodes, WidgetKind::MenuPopover)
    }

    /// Dropdown / Select input box.
    pub fn dropdown(
        &mut self,
        id: impl Into<WidgetId>,
        label: impl Into<String>,
        selected_text: impl Into<String>,
        open: bool,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.layout.insert_leaf(
            style,
            WidgetKind::Dropdown {
                id: id.into(),
                label: label.into(),
                selected_text: selected_text.into(),
                open,
            },
        )
    }

    /// Floating Toast status notification.
    pub fn toast(
        &mut self,
        id: impl Into<WidgetId>,
        title: impl Into<String>,
        message: impl Into<String>,
        kind: crate::kind::ToastKind,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.layout.insert_leaf(
            style,
            WidgetKind::Toast {
                id: id.into(),
                title: title.into(),
                message: message.into(),
                kind,
            },
        )
    }

    /// Floating tooltip.
    pub fn tooltip(
        &mut self,
        text: impl Into<String>,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.layout
            .insert_leaf(style, WidgetKind::Tooltip { text: text.into() })
    }

    /// Tab selection bar container.
    pub fn tabbar(
        &mut self,
        id: impl Into<WidgetId>,
        tabs: &[impl AsRef<str>],
        active_index: usize,
        tab_style: Style,
        bar_style: Style,
    ) -> Result<NodeId, LayoutError> {
        let id = id.into();
        let mut items = Vec::with_capacity(tabs.len());
        for (index, label) in tabs.iter().enumerate() {
            let kind = WidgetKind::TabItem {
                owner: id.clone(),
                index,
                label: label.as_ref().to_string(),
                active: index == active_index,
            };
            items.push(self.layout.insert_leaf(tab_style.clone(), kind)?);
        }
        self.layout
            .insert_container(bar_style, &items, WidgetKind::Container)
    }

    /// Scrollable rich list with status badges.
    pub fn rich_list(
        &mut self,
        id: impl Into<WidgetId>,
        items: &[(impl AsRef<str>, crate::kind::ListItemBadge)],
        selected: Option<usize>,
        item_style: Style,
        list_style: Style,
    ) -> Result<NodeId, LayoutError> {
        let id = id.into();
        let mut rows = Vec::with_capacity(items.len());
        for (index, (text, badge)) in items.iter().enumerate() {
            let kind = WidgetKind::ListItem {
                owner: id.clone(),
                index,
                text: text.as_ref().to_string(),
                selected: selected == Some(index),
                badge: badge.clone(),
            };
            rows.push(self.layout.insert_leaf(item_style.clone(), kind)?);
        }
        self.layout
            .insert_container(list_style, &rows, WidgetKind::Container)
    }

    pub fn list(
        &mut self,
        id: impl Into<WidgetId>,
        items: &[impl AsRef<str>],
        selected: Option<usize>,
        item_style: Style,
        list_style: Style,
    ) -> Result<NodeId, LayoutError> {
        let items_with_badges: Vec<_> = items
            .iter()
            .map(|s| (s, crate::kind::ListItemBadge::None))
            .collect();
        self.rich_list(id, &items_with_badges, selected, item_style, list_style)
    }

    /// Scrollable viewport container tracking current 2D scroll offset.
    pub fn scrollview(
        &mut self,
        id: impl Into<WidgetId>,
        offset: [f32; 2],
        children: &[NodeId],
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        let style = Style {
            align_items: Some(AlignItems::FlexStart),
            ..style
        };
        self.layout.insert_container(
            style,
            children,
            WidgetKind::ScrollView {
                id: id.into(),
                offset,
            },
        )
    }

    /// Floating Tool Palette (Photoshop / Blender style sub-window).
    /// Draggable header bar, fold / collapse button, close button, and optional resize grip.
    pub fn palette(
        &mut self,
        id: impl Into<WidgetId>,
        title: impl Into<String>,
        folded: bool,
        content: Option<NodeId>,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        let wid = id.into();
        let title_str = title.into();

        let header_node = self.layout.insert_leaf(
            Style {
                size: Size {
                    width: ui_layout::percent(1.0),
                    height: ui_layout::length(28.0),
                },
                ..Default::default()
            },
            WidgetKind::PaletteHeader {
                owner: wid.clone(),
                title: title_str.clone(),
                folded,
            },
        )?;

        let mut children = vec![header_node];
        if !folded {
            if let Some(c) = content {
                children.push(c);
            }
            let grip_node = self.layout.insert_leaf(
                Style {
                    position: ui_layout::Position::Absolute,
                    inset: ui_layout::Rect {
                        right: ui_layout::length(2.0),
                        bottom: ui_layout::length(2.0),
                        left: ui_layout::auto(),
                        top: ui_layout::auto(),
                    },
                    size: Size {
                        width: ui_layout::length(14.0),
                        height: ui_layout::length(14.0),
                    },
                    ..Default::default()
                },
                WidgetKind::ResizeGrip { owner: wid.clone() },
            )?;
            children.push(grip_node);
        }

        let style = Style {
            flex_direction: ui_layout::FlexDirection::Column,
            ..style
        };

        self.layout.insert_container(
            style,
            &children,
            WidgetKind::Palette {
                id: wid,
                title: title_str,
                folded,
            },
        )
    }

    /// Generic media widget.
    pub fn media(
        &mut self,
        id: impl Into<WidgetId>,
        kind: MediaKind,
        resource_id: impl Into<String>,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.layout.insert_leaf(
            style,
            WidgetKind::Media {
                id: id.into(),
                kind,
                resource_id: resource_id.into(),
                fit: crate::media::MediaFit::Cover,
                radius: None,
            },
        )
    }

    /// Image widget with custom aspect fitting mode and corner radius.
    pub fn image(
        &mut self,
        id: impl Into<WidgetId>,
        resource_id: impl Into<String>,
        fit: crate::media::MediaFit,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.layout.insert_leaf(
            style,
            WidgetKind::Media {
                id: id.into(),
                kind: MediaKind("image"),
                resource_id: resource_id.into(),
                fit,
                radius: None,
            },
        )
    }

    /// Video player widget with transport controls and status.
    pub fn video_player(
        &mut self,
        id: impl Into<WidgetId>,
        resource_id: impl Into<String>,
        playing: bool,
        progress: f32,
        duration_sec: f32,
        volume: f32,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.layout.insert_leaf(
            style,
            WidgetKind::VideoPlayer {
                id: id.into(),
                resource_id: resource_id.into(),
                playing,
                progress,
                duration_sec,
                volume,
            },
        )
    }

    /// Real-time audio spectrum analyzer / visualizer.
    pub fn audio_visualizer(
        &mut self,
        id: impl Into<WidgetId>,
        values: &[f32],
        peak: f32,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.layout.insert_leaf(
            style,
            WidgetKind::AudioVisualizer {
                id: id.into(),
                values: values.to_vec(),
                peak,
            },
        )
    }

    /// 2D Custom drawing and graphics surface (`CustomPaint`).
    pub fn custom_paint(
        &mut self,
        id: impl Into<WidgetId>,
        commands: impl IntoIterator<Item = crate::paint::PaintCommand>,
        style: Style,
    ) -> Result<NodeId, LayoutError> {
        self.layout.insert_leaf(
            style,
            WidgetKind::CustomPaint {
                id: id.into(),
                commands: commands.into_iter().collect(),
            },
        )
    }
}

impl Default for WidgetTree {
    fn default() -> Self {
        Self::new()
    }
}
