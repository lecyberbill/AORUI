// [WFGY] Zone: TRANSIT | λ: 0.2 | Fallbacks: 0 | Action: Generic click and scroll dispatching to UiEvent (human or agent)
use ui_core::UiEvent;
use ui_layout::NodeId;

use crate::kind::{InteractionKey, WidgetKind};
use crate::tree::WidgetTree;

impl WidgetTree {
    /// Stable interaction key for the interactive widget under `point`, or `None`.
    /// Used for hover calculations and pressed state tracking (see [`crate::kind::InteractionKey`]).
    pub fn interaction_key_at(&self, root: NodeId, point: (f32, f32)) -> Result<Option<InteractionKey>, ui_layout::LayoutError> {
        let Some(node) = self.hit_test_effective(root, point)? else {
            return Ok(None);
        };
        let Some(kind) = self.layout().payload(node) else {
            return Ok(None);
        };
        Ok(kind.interaction_key())
    }

    /// Dispatches a clicked point (screen space absolute coordinates) to a [`UiEvent`]
    /// via `hit_test_effective` and the [`WidgetKind`] payload.
    ///
    /// The exact same code path handles both human mouse clicks and programmatic agent actions.
    pub fn dispatch_click(&self, root: NodeId, point: (f32, f32)) -> Result<Option<UiEvent>, ui_layout::LayoutError> {
        let Some(node) = self.hit_test_effective(root, point)? else {
            return Ok(None);
        };
        let Some(kind) = self.layout().payload(node) else {
            return Ok(None);
        };
        if !kind.is_interactive() {
            return Ok(None);
        }

        Ok(match kind {
            WidgetKind::Button { id, enabled: true, .. } | WidgetKind::IconButton { id, enabled: true, .. } => {
                Some(UiEvent::ButtonClicked { widget_id: id.to_string() })
            }
            WidgetKind::Button { enabled: false, .. } | WidgetKind::IconButton { enabled: false, .. } => None,
            WidgetKind::Checkbox { id, checked } => {
                Some(UiEvent::CheckboxToggled { widget_id: id.to_string(), checked: !checked })
            }
            WidgetKind::Toggle { id, active } => {
                Some(UiEvent::ToggleSwitched { widget_id: id.to_string(), active: !active })
            }
            WidgetKind::Slider { id, min, max, .. } => {
                let effective = self.effective_bounds(root)?;
                let bounds = effective[&node].visual;
                let track_w = bounds[2];
                let ratio = if track_w > 0.0 { ((point.0 - bounds[0]) / track_w).clamp(0.0, 1.0) } else { 0.0 };
                let value = min + ratio * (max - min);
                Some(UiEvent::SliderChanged { widget_id: id.to_string(), value })
            }
            WidgetKind::WindowCloseButton { owner } => Some(UiEvent::WindowCloseRequested { widget_id: owner.to_string() }),
            WidgetKind::ModalBackdrop { owner } => Some(UiEvent::ModalDismissed { modal_id: owner.to_string() }),
            WidgetKind::TabItem { owner, index, .. } => Some(UiEvent::TabSelected { widget_id: owner.to_string(), tab_index: *index }),
            WidgetKind::ListItem { owner, index, .. } => {
                Some(UiEvent::ListItemSelected { widget_id: owner.to_string(), item_index: *index })
            }
            WidgetKind::TableHeader { owner, column_index, .. } => {
                Some(UiEvent::TableHeaderClicked { table_id: owner.to_string(), column_index: *column_index })
            }
            WidgetKind::TableCell { owner, row_index, .. } => {
                Some(UiEvent::TableRowSelected { table_id: owner.to_string(), row_index: *row_index })
            }
            WidgetKind::AccordionHeader { id, expanded, .. } => {
                Some(UiEvent::AccordionToggled { id: id.to_string(), expanded: !expanded })
            }
            WidgetKind::BreadcrumbItem { owner, index, id, is_last, .. } => {
                if !*is_last {
                    Some(UiEvent::BreadcrumbClicked { bar_id: owner.to_string(), index: *index, item_id: id.clone() })
                } else {
                    None
                }
            }
            WidgetKind::PaginationItem { owner, page, disabled, .. } => {
                if !*disabled {
                    Some(UiEvent::PageSelected { widget_id: owner.to_string(), page: *page })
                } else {
                    None
                }
            }
            WidgetKind::ColorSwatch { id, color, .. } => {
                Some(UiEvent::ColorSelected { widget_id: id.to_string(), color: *color })
            }
            WidgetKind::ColorPicker { id, color, .. } => {
                let effective = self.effective_bounds(root)?;
                let bounds = effective[&node].visual;
                let (cur_h, cur_s, cur_v) = crate::color::Color::from_array(*color).to_hsv();
                let pad = 10.0;

                let top_h = (bounds[3] * 0.42).clamp(70.0, 96.0);
                let swatch_w = 110.0;
                let sv_x = bounds[0] + pad + swatch_w + 8.0;
                let sv_y = bounds[1] + pad;
                let sv_w = (bounds[0] + bounds[2] - pad - sv_x).max(10.0);
                let sv_h = top_h;

                let hue_y = sv_y + sv_h + 8.0;
                let hue_h = 12.0;
                let hue_x = bounds[0] + pad;
                let hue_w = bounds[2] - 2.0 * pad;

                let hex_y = hue_y + hue_h + 8.0;
                let hex_h = 28.0;
                let cards_y = hex_y + hex_h + 6.0;
                let cards_h = (bounds[1] + bounds[3] - pad - cards_y).max(28.0);
                let card_gap = 6.0;
                let card_w = (bounds[2] - 2.0 * pad - 3.0 * card_gap) / 4.0;

                // 1. Click on 2D SV Canvas
                if point.0 >= sv_x && point.0 <= sv_x + sv_w && point.1 >= sv_y && point.1 <= sv_y + sv_h {
                    let s = if sv_w > 0.0 { ((point.0 - sv_x) / sv_w).clamp(0.0, 1.0) } else { 0.0 };
                    let v = if sv_h > 0.0 { (1.0 - (point.1 - sv_y) / sv_h).clamp(0.0, 1.0) } else { 0.0 };
                    let new_col = crate::color::Color::from_hsv(cur_h, s, v, color[3]).to_array();
                    return Ok(Some(UiEvent::ColorChanged {
                        widget_id: id.to_string(),
                        color: new_col,
                    }));
                }

                // 2. Click on Rainbow Hue Slider Bar
                if point.0 >= hue_x && point.0 <= hue_x + hue_w && point.1 >= hue_y - 4.0 && point.1 <= hue_y + hue_h + 4.0 {
                    let ratio = if hue_w > 0.0 { ((point.0 - hue_x) / hue_w).clamp(0.0, 1.0) } else { 0.0 };
                    let h = ratio * 360.0;
                    let new_col = crate::color::Color::from_hsv(h, cur_s, cur_v, color[3]).to_array();
                    return Ok(Some(UiEvent::ColorChanged {
                        widget_id: id.to_string(),
                        color: new_col,
                    }));
                }

                // 3. Click on 4 Multi-space Mini-Cards (RGB, CMYK, HSV, LAB)
                if point.1 >= cards_y && point.1 <= cards_y + cards_h {
                    let modes = ["rgb", "cmyk", "hsv", "lab"];
                    for (idx, mode_id) in modes.iter().enumerate() {
                        let mx = bounds[0] + pad + idx as f32 * (card_w + card_gap);
                        if point.0 >= mx && point.0 <= mx + card_w {
                            return Ok(Some(UiEvent::SelectChanged {
                                widget_id: id.to_string(),
                                selected_id: mode_id.to_string(),
                            }));
                        }
                    }
                }

                Some(UiEvent::ColorSelected { widget_id: id.to_string(), color: *color })
            }
            WidgetKind::SegmentItem { owner, index, .. } => {
                Some(UiEvent::SegmentSelected { widget_id: owner.to_string(), selected_index: *index })
            }
            WidgetKind::RadioButton { id, group_id, .. } => {
                Some(UiEvent::RadioSelected { group_id: group_id.clone(), selected_id: id.to_string() })
            }
            WidgetKind::TextInput { id, .. } | WidgetKind::TextArea { id, .. } => {
                Some(UiEvent::FocusChanged { widget_id: Some(id.to_string()) })
            }
            WidgetKind::PasswordInput { id, revealed, .. } => {
                let effective = self.effective_bounds(root)?;
                let bounds = effective[&node].visual;
                // Eye button is in the rightmost 28px
                if point.0 >= bounds[0] + bounds[2] - 28.0 {
                    Some(UiEvent::PasswordRevealed { widget_id: id.to_string(), revealed: !revealed })
                } else {
                    Some(UiEvent::FocusChanged { widget_id: Some(id.to_string()) })
                }
            }
            WidgetKind::NumberInput { id, value, min, max, step, .. } => {
                let effective = self.effective_bounds(root)?;
                let bounds = effective[&node].visual;
                // Stepper buttons are in the rightmost 28px: top half is +, bottom half is -
                if point.0 >= bounds[0] + bounds[2] - 28.0 {
                    let mid_y = bounds[1] + bounds[3] * 0.5;
                    if point.1 < mid_y {
                        let new_val = (*value + *step).min(*max);
                        Some(UiEvent::NumberChanged { widget_id: id.to_string(), value: new_val })
                    } else {
                        let new_val = (*value - *step).max(*min);
                        Some(UiEvent::NumberChanged { widget_id: id.to_string(), value: new_val })
                    }
                } else {
                    Some(UiEvent::FocusChanged { widget_id: Some(id.to_string()) })
                }
            }
            WidgetKind::MenuBarItem { owner, index, active, .. } => {
                Some(UiEvent::MenuToggled { menu_id: format!("{}:{}", owner, index), open: !active })
            }
            WidgetKind::MenuItem { owner, id, enabled: true, .. } => {
                Some(UiEvent::MenuItemClicked { menu_id: owner.to_string(), item_id: id.to_string() })
            }
            WidgetKind::MenuItem { enabled: false, .. } => None,
            WidgetKind::Dropdown { id, open, .. } => {
                Some(UiEvent::MenuToggled { menu_id: id.to_string(), open: !open })
            }
            WidgetKind::Toast { id, .. } => {
                Some(UiEvent::ToastDismissed { toast_id: id.to_string() })
            }
            WidgetKind::TreeNode { owner, id, is_dir, expanded, .. } => {
                if *is_dir {
                    Some(UiEvent::TreeNodeToggled { tree_id: owner.to_string(), node_id: id.to_string(), expanded: !expanded })
                } else {
                    Some(UiEvent::TreeNodeSelected { tree_id: owner.to_string(), node_id: id.to_string() })
                }
            }
            WidgetKind::PaletteHeader { owner, folded, .. } => {
                let effective = self.effective_bounds(root)?;
                let bounds = effective[&node].visual;
                // Close button is in rightmost 22px
                if point.0 >= bounds[0] + bounds[2] - 24.0 {
                    Some(UiEvent::PaletteClosed { palette_id: owner.to_string() })
                } else if point.0 >= bounds[0] + bounds[2] - 44.0 {
                    // Fold button is in [rightmost - 44 .. rightmost - 24]
                    Some(UiEvent::PaletteFoldToggled { palette_id: owner.to_string(), folded: !folded })
                } else {
                    None
                }
            }
            WidgetKind::PaletteFoldButton { owner, folded } => {
                Some(UiEvent::PaletteFoldToggled { palette_id: owner.to_string(), folded: !folded })
            }
            WidgetKind::PaletteCloseButton { owner } => {
                Some(UiEvent::PaletteClosed { palette_id: owner.to_string() })
            }
            _ => None,
        })
    }

    /// Computes the new split ratio of a Splitter under pointer drag.
    pub fn split_ratio_at(
        &self,
        root: NodeId,
        splitter_owner: &str,
        point: (f32, f32),
    ) -> Result<Option<f32>, ui_layout::LayoutError> {
        let effective = self.effective_bounds(root)?;
        for (node, _) in &effective {
            if let Some(WidgetKind::Splitter { owner, orientation }) = self.layout().payload(*node) {
                if owner.as_str() == splitter_owner {
                    if let Ok(Some(parent)) = self.layout().parent(*node) {
                        let parent_bounds = effective[&parent].visual;
                        let ratio = match orientation {
                            crate::kind::SplitOrientation::Horizontal => {
                                if parent_bounds[2] > 0.0 {
                                    ((point.0 - parent_bounds[0]) / parent_bounds[2]).clamp(0.1, 0.9)
                                } else {
                                    0.5
                                }
                            }
                            crate::kind::SplitOrientation::Vertical => {
                                if parent_bounds[3] > 0.0 {
                                    ((point.1 - parent_bounds[1]) / parent_bounds[3]).clamp(0.1, 0.9)
                                } else {
                                    0.5
                                }
                            }
                        };
                        return Ok(Some(ratio));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Computes the value of a Slider under `point` (click or drag).
    pub fn slider_value_at(
        &self,
        root: NodeId,
        point: (f32, f32),
    ) -> Result<Option<(String, f32)>, ui_layout::LayoutError> {
        let Some(node) = self.hit_test_effective(root, point)? else {
            return Ok(None);
        };
        let Some(WidgetKind::Slider { id, min, max, .. }) = self.layout().payload(node) else {
            return Ok(None);
        };
        let effective = self.effective_bounds(root)?;
        let bounds = effective[&node].visual;
        let track_w = bounds[2];
        let ratio = if track_w > 0.0 { ((point.0 - bounds[0]) / track_w).clamp(0.0, 1.0) } else { 0.0 };
        let value = min + ratio * (max - min);
        Ok(Some((id.to_string(), value)))
    }

    /// Computes the picked color of a ColorPicker under `point` (2D SV canvas or 1D Hue slider click/drag).
    pub fn color_picker_hue_at(
        &self,
        root: NodeId,
        point: (f32, f32),
    ) -> Result<Option<(String, [f32; 4])>, ui_layout::LayoutError> {
        let Some(node) = self.hit_test_effective(root, point)? else {
            return Ok(None);
        };
        let Some(WidgetKind::ColorPicker { id, color, .. }) = self.layout().payload(node) else {
            return Ok(None);
        };
        let effective = self.effective_bounds(root)?;
        let bounds = effective[&node].visual;
        let (cur_h, cur_s, cur_v) = crate::color::Color::from_array(*color).to_hsv();
        let pad = 10.0;

        let top_h = (bounds[3] * 0.42).clamp(70.0, 96.0);
        let swatch_w = 110.0;
        let sv_x = bounds[0] + pad + swatch_w + 8.0;
        let sv_y = bounds[1] + pad;
        let sv_w = (bounds[0] + bounds[2] - pad - sv_x).max(10.0);
        let sv_h = top_h;

        let hue_y = sv_y + sv_h + 8.0;
        let hue_h = 12.0;
        let hue_x = bounds[0] + pad;
        let hue_w = bounds[2] - 2.0 * pad;

        // 1. Drag / click on 2D Saturation / Value Canvas
        if point.0 >= sv_x - 4.0 && point.0 <= sv_x + sv_w + 4.0 && point.1 >= sv_y - 4.0 && point.1 <= sv_y + sv_h + 4.0 {
            let s = if sv_w > 0.0 { ((point.0 - sv_x) / sv_w).clamp(0.0, 1.0) } else { 0.0 };
            let v = if sv_h > 0.0 { (1.0 - (point.1 - sv_y) / sv_h).clamp(0.0, 1.0) } else { 0.0 };
            let new_col = crate::color::Color::from_hsv(cur_h, s, v, color[3]).to_array();
            return Ok(Some((id.to_string(), new_col)));
        }

        // 2. Drag / click on Rainbow Hue Slider Bar
        if point.0 >= hue_x - 4.0 && point.0 <= hue_x + hue_w + 4.0 && point.1 >= hue_y - 6.0 && point.1 <= hue_y + hue_h + 6.0 {
            let ratio = if hue_w > 0.0 { ((point.0 - hue_x) / hue_w).clamp(0.0, 1.0) } else { 0.0 };
            let h = ratio * 360.0;
            let new_col = crate::color::Color::from_hsv(h, cur_s, cur_v, color[3]).to_array();
            return Ok(Some((id.to_string(), new_col)));
        }

        Ok(None)
    }

    /// Translates scrolling at `point` into [`UiEvent::ScrollChanged`]
    /// for the nearest ancestor `ScrollView`.
    pub fn dispatch_scroll(
        &self,
        root: NodeId,
        point: (f32, f32),
        delta: [f32; 2],
    ) -> Result<Option<UiEvent>, ui_layout::LayoutError> {
        let Some(node) = self.scrollview_at(root, point)? else {
            return Ok(None);
        };
        let Some(WidgetKind::ScrollView { id, offset }) = self.layout().payload(node) else {
            return Ok(None);
        };
        Ok(Some(UiEvent::ScrollChanged {
            widget_id: id.to_string(),
            offset: [offset[0] + delta[0], offset[1] + delta[1]],
        }))
    }

    /// Checks whether `point` lies inside the draggable title bar of a window
    /// (excluding interactive child controls and buttons).
    pub fn is_window_title_bar(&self, root: NodeId, point: (f32, f32)) -> Result<bool, ui_layout::LayoutError> {
        let effective = self.effective_bounds(root)?;
        let Some(hit_node) = self.hit_test_effective(root, point)? else {
            return Ok(false);
        };
        if let Some(kind) = self.layout().payload(hit_node) {
            if kind.is_interactive() {
                return Ok(false);
            }
        }
        let bounds = effective[&root].visual;
        let in_top_bar = point.0 >= bounds[0]
            && point.0 <= bounds[0] + bounds[2]
            && point.1 >= bounds[1]
            && point.1 <= bounds[1] + 48.0;
        Ok(in_top_bar)
    }

    /// Checks whether `point` lies inside the draggable header of a modal dialog
    /// (excluding close buttons and controls).
    pub fn is_modal_title_bar(&self, modal_root: NodeId, point: (f32, f32)) -> Result<bool, ui_layout::LayoutError> {
        let effective = self.effective_bounds(modal_root)?;
        for (node, bounds) in &effective {
            if let Some(WidgetKind::Modal { .. }) = self.layout().payload(*node) {
                if let Some(hit) = self.hit_test_effective(modal_root, point)? {
                    if let Some(kind) = self.layout().payload(hit) {
                        if kind.is_interactive() {
                            return Ok(false);
                        }
                    }
                }
                let in_header = point.0 >= bounds.visual[0]
                    && point.0 <= bounds.visual[0] + bounds.visual[2]
                    && point.1 >= bounds.visual[1]
                    && point.1 <= bounds.visual[1] + 44.0;
                return Ok(in_header);
            }
        }
        Ok(false)
    }

    /// Returns the next or previous focusable widget ID in depth-first layout order.
    pub fn next_focusable(&self, root: NodeId, current_focused: Option<&str>, backward: bool) -> Result<Option<String>, ui_layout::LayoutError> {
        let mut focusable = Vec::new();
        self.collect_focusable(root, &mut focusable)?;

        if focusable.is_empty() {
            return Ok(None);
        }

        let Some(current) = current_focused else {
            return Ok(Some(if backward { focusable.last().unwrap().clone() } else { focusable.first().unwrap().clone() }));
        };

        let pos = focusable.iter().position(|id| id.as_str() == current);
        match pos {
            Some(idx) => {
                if backward {
                    if idx == 0 {
                        Ok(Some(focusable.last().unwrap().clone()))
                    } else {
                        Ok(Some(focusable[idx - 1].clone()))
                    }
                } else {
                    if idx + 1 >= focusable.len() {
                        Ok(Some(focusable.first().unwrap().clone()))
                    } else {
                        Ok(Some(focusable[idx + 1].clone()))
                    }
                }
            }
            None => Ok(Some(focusable.first().unwrap().clone())),
        }
    }

    /// Checks if `point` is over a Palette header (excluding fold/close buttons)
    /// and returns the palette ID and drag anchor offset `(cursor.x - palette.x, cursor.y - palette.y)`.
    pub fn palette_drag_anchor_at(
        &self,
        root: NodeId,
        point: (f32, f32),
    ) -> Result<Option<(String, (f32, f32))>, ui_layout::LayoutError> {
        let effective = self.effective_bounds(root)?;
        for (node, bounds) in &effective {
            if let Some(WidgetKind::PaletteHeader { owner, .. }) = self.layout().payload(*node) {
                let hb = bounds.visual;
                // Exclude the rightmost 46px (fold and close buttons)
                if point.0 >= hb[0] && point.0 <= hb[0] + hb[2] - 46.0 && point.1 >= hb[1] && point.1 <= hb[1] + hb[3] {
                    if let Ok(Some(parent)) = self.layout().parent(*node) {
                        let pb = effective[&parent].visual;
                        return Ok(Some((owner.to_string(), (point.0 - pb[0], point.1 - pb[1]))));
                    }
                    return Ok(Some((owner.to_string(), (point.0 - hb[0], point.1 - hb[1]))));
                }
            }
        }
        Ok(None)
    }

    /// Checks if `point` is over a Palette's ResizeGrip (bottom-right corner)
    /// and returns the palette ID and initial palette dimensions `(width, height)`.
    pub fn palette_resize_at(
        &self,
        root: NodeId,
        point: (f32, f32),
    ) -> Result<Option<(String, (f32, f32))>, ui_layout::LayoutError> {
        let effective = self.effective_bounds(root)?;
        for (node, bounds) in &effective {
            if let Some(WidgetKind::ResizeGrip { owner }) = self.layout().payload(*node) {
                let gb = bounds.visual;
                if point.0 >= gb[0] - 4.0 && point.0 <= gb[0] + gb[2] + 4.0 && point.1 >= gb[1] - 4.0 && point.1 <= gb[1] + gb[3] + 4.0 {
                    if let Ok(Some(parent)) = self.layout().parent(*node) {
                        let pb = effective[&parent].visual;
                        return Ok(Some((owner.to_string(), (pb[2], pb[3]))));
                    }
                }
            }
        }
        Ok(None)
    }

    /// Converts mouse click coordinates into a character cursor offset for focused text inputs.
    pub fn text_cursor_at(
        &self,
        root: NodeId,
        point: (f32, f32),
        body_font_size: f32,
    ) -> Result<Option<(String, usize)>, ui_layout::LayoutError> {
        let effective = self.effective_bounds(root)?;
        for (node, bounds) in &effective {
            let b = bounds.visual;
            if point.0 < b[0] || point.0 > b[0] + b[2] || point.1 < b[1] || point.1 > b[1] + b[3] {
                continue;
            }
            if let Some(kind) = self.layout().payload(*node) {
                match kind {
                    WidgetKind::TextInput { id, value, .. } => {
                        let text_start_x = b[0] + 12.0;
                        let rel_x = (point.0 - text_start_x).max(0.0);
                        let idx = crate::frame::find_cursor_index_in_line(value, rel_x, body_font_size);
                        return Ok(Some((id.to_string(), idx)));
                    }
                    WidgetKind::PasswordInput { id, value, revealed, .. } => {
                        let text_start_x = b[0] + 12.0;
                        let rel_x = (point.0 - text_start_x).max(0.0);
                        let idx = if *revealed {
                            crate::frame::find_cursor_index_in_line(value, rel_x, body_font_size)
                        } else {
                            let char_w = body_font_size * 0.55;
                            ((rel_x / char_w).round() as usize).min(value.len())
                        };
                        return Ok(Some((id.to_string(), idx)));
                    }
                    WidgetKind::TextArea { id, value, line_numbers, .. } => {
                        let gutter_w = if *line_numbers { 32.0 } else { 0.0 };
                        let text_offset_x = b[0] + gutter_w + 10.0;
                        let line_h = 20.0;
                        let start_y = b[1] + 8.0;

                        let rel_y = (point.1 - start_y).max(0.0);
                        let target_line_idx = (rel_y / line_h) as usize;

                        let lines: Vec<&str> = value.split('\n').collect();
                        let actual_line_idx = target_line_idx.min(lines.len().saturating_sub(1));

                        let line_str = lines.get(actual_line_idx).unwrap_or(&"");
                        let rel_x = (point.0 - text_offset_x).max(0.0);
                        let col_idx = crate::frame::find_cursor_index_in_line(line_str, rel_x, body_font_size);

                        // Convert (actual_line_idx, col_idx) to global character offset
                        let mut byte_offset = 0;
                        for (i, l) in lines.iter().enumerate() {
                            if i == actual_line_idx {
                                byte_offset += col_idx.min(l.len());
                                break;
                            }
                            byte_offset += l.len() + 1; // +1 for '\n'
                        }
                        return Ok(Some((id.to_string(), byte_offset.min(value.len()))));
                    }
                    _ => {}
                }
            }
        }
        Ok(None)
    }

    fn collect_focusable(&self, node: NodeId, list: &mut Vec<String>) -> Result<(), ui_layout::LayoutError> {
        if let Some(kind) = self.layout().payload(node) {
            match kind {
                WidgetKind::TextInput { id, .. }
                | WidgetKind::TextArea { id, .. }
                | WidgetKind::PasswordInput { id, .. }
                | WidgetKind::NumberInput { id, .. }
                | WidgetKind::Button { id, enabled: true, .. }
                | WidgetKind::IconButton { id, enabled: true, .. }
                | WidgetKind::AccordionHeader { id, .. }
                | WidgetKind::Checkbox { id, .. }
                | WidgetKind::Toggle { id, .. }
                | WidgetKind::Slider { id, .. } => {
                    list.push(id.to_string());
                }
                _ => {}
            }
        }
        for child in self.layout().children(node)? {
            self.collect_focusable(child, list)?;
        }
        Ok(())
    }
}
