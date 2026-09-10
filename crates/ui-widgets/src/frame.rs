// [WFGY] Zone: RISK | λ: 0.3 | Fallbacks: 0 | Action: Declarative widget translation to GpuSdfInstance/TextSpec, hover/press visual feedback
use ui_core::GpuSdfInstance;
use ui_layout::{LayoutError, NodeId};

use crate::kind::{InteractionKey, WidgetKind};
use crate::media::MediaSpec;
use crate::theme::{FontWeight, Theme};
use crate::tree::WidgetTree;

use crate::text_measure::TextMeasure;

/// Horizontal text alignment within bounding box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

/// Renderer-agnostic description of a text run to be rendered.
/// Translated to concrete rendering primitives by the renderer integration layer (`ui-gpu::TextLayer`).
#[derive(Debug, Clone, PartialEq)]
pub struct TextSpec {
    pub text: String,
    pub bounds: [f32; 4],
    pub font_size: f32,
    pub color: [f32; 4],
    pub align: TextAlign,
    pub weight: FontWeight,
    /// Screen clipping bounds outside of which text is discarded.
    pub clip: [f32; 4],
}

#[derive(Debug, Clone, Default)]
pub struct Frame {
    pub instances: Vec<GpuSdfInstance>,
    pub texts: Vec<TextSpec>,
    /// Non-SDF media content specs (see [`crate::media`]).
    pub media: Vec<MediaSpec>,
}

/// Current interaction state provided to [`WidgetTree::build_frame`] to drive visual feedback.
#[derive(Clone, Copy, Default)]
pub struct InteractionState<'a> {
    pub hovered: Option<&'a InteractionKey>,
    pub pressed: Option<&'a InteractionKey>,
    pub measure: Option<&'a dyn TextMeasure>,
}

fn get_measure<'a>(interaction: &'a InteractionState<'a>) -> &'a dyn TextMeasure {
    interaction.measure.unwrap_or(&crate::text_measure::DefaultTextMeasure)
}

impl WidgetTree {
    /// Builds the GPU frame (SDF instances + text specs) for the tree rooted at `root`,
    /// evaluated in parent-to-child order.
    pub fn build_frame(&self, root: NodeId, theme: &Theme, interaction: InteractionState) -> Result<Frame, LayoutError> {
        let effective = self.effective_bounds(root)?;
        let mut frame = Frame::default();
        self.visit(root, &effective, theme, interaction, &mut frame)?;
        Ok(frame)
    }

    fn visit(
        &self,
        node: NodeId,
        effective: &std::collections::HashMap<NodeId, crate::effective::EffectiveBounds>,
        theme: &Theme,
        interaction: InteractionState,
        frame: &mut Frame,
    ) -> Result<(), LayoutError> {
        let node_effective = effective[&node];
        if let Some(kind) = self.layout().payload(node) {
            let key = kind.interaction_key();
            let hovered = key.is_some() && key.as_ref() == interaction.hovered;
            let pressed = key.is_some() && key.as_ref() == interaction.pressed;
            let measure = get_measure(&interaction);
            render_kind(kind, node_effective.visual, node_effective.clip, theme, hovered, pressed, measure, frame);
        }
        for child in self.layout().children(node)? {
            self.visit(child, effective, theme, interaction, frame)?;
        }
        Ok(())
    }
}

/// Insets `bounds` by horizontal/vertical margins.
fn inset(bounds: [f32; 4], horizontal: f32, vertical: f32) -> [f32; 4] {
    [
        bounds[0] + horizontal,
        bounds[1] + vertical,
        (bounds[2] - 2.0 * horizontal).max(0.0),
        (bounds[3] - 2.0 * vertical).max(0.0),
    ]
}

/// Tactile button press effect: subtle position offset and shrinking (2px).
fn press_offset(bounds: [f32; 4], pressed: bool) -> [f32; 4] {
    if !pressed {
        return bounds;
    }
    const OFFSET: f32 = 2.0;
    [bounds[0] + OFFSET, bounds[1] + OFFSET, (bounds[2] - OFFSET * 2.0).max(0.0), (bounds[3] - OFFSET * 2.0).max(0.0)]
}

fn custom_glass_instance(
    bounds: [f32; 4],
    clip: [f32; 4],
    bg: [f32; 4],
    glow: [f32; 4],
    radius: f32,
    border_width: f32,
    glow_intensity: f32,
) -> GpuSdfInstance {
    let r = radius.min(bounds[2] * 0.5).min(bounds[3] * 0.5).max(0.0);
    GpuSdfInstance {
        bounds,
        bg_color: bg,
        glow_color: glow,
        radius: r,
        border_width,
        glow_intensity,
        blur_factor: 1.0,
        clip_bounds: clip,
    }
}

fn glass_instance(bounds: [f32; 4], clip: [f32; 4], bg: [f32; 4], glow: [f32; 4], glow_intensity: f32, theme: &Theme) -> GpuSdfInstance {
    // INV-GPU-3: corner radius must never exceed half the smaller side.
    let radius = theme.corner_radius.min(bounds[2] * 0.5).min(bounds[3] * 0.5);
    GpuSdfInstance {
        bounds,
        bg_color: bg,
        glow_color: glow,
        radius,
        border_width: theme.border_width,
        glow_intensity,
        blur_factor: 1.0,
        clip_bounds: clip,
    }
}

/// Semantic role for text styling resolution via `Theme::typography`.
enum TextRole {
    Title,
    Body,
    Small,
    Caption,
}

fn text_spec(
    text: String,
    bounds: [f32; 4],
    clip: [f32; 4],
    theme: &Theme,
    color: [f32; 4],
    align: TextAlign,
    role: TextRole,
) -> TextSpec {
    let (font_size, weight) = match role {
        TextRole::Title => (theme.typography.title_size, theme.typography.heading_weight),
        TextRole::Body => (theme.typography.body_size, theme.typography.body_weight),
        TextRole::Small => (theme.typography.small_size, theme.typography.body_weight),
        TextRole::Caption => (theme.typography.caption_size, theme.typography.body_weight),
    };
    TextSpec { text, bounds, font_size, color, align, weight, clip }
}

/// Minimum scrollbar thumb height in pixels.
const MIN_SCROLLBAR_THUMB: f32 = 16.0;

/// Proportional scrollbar thumb geometry computation.
fn scrollbar_thumb_bounds(track: [f32; 4], content_size: f32, viewport_size: f32, offset: f32) -> [f32; 4] {
    let track_height = track[3];
    let visible_ratio = if content_size > 0.0 { (viewport_size / content_size).clamp(0.0, 1.0) } else { 1.0 };
    let thumb_height = (track_height * visible_ratio).max(MIN_SCROLLBAR_THUMB).min(track_height);

    let max_offset = (content_size - viewport_size).max(0.0);
    let scroll_ratio = if max_offset > 0.0 { (offset / max_offset).clamp(0.0, 1.0) } else { 0.0 };
    let thumb_y = track[1] + scroll_ratio * (track_height - thumb_height);

    [track[0], thumb_y, track[2], thumb_height]
}

/// Combines base intensity with hover/press modifiers.
fn interactive_glow(base_intensity: f32, hovered: bool, pressed: bool, theme: &Theme) -> f32 {
    if pressed {
        (theme.glow_intensity_hover * 0.7).max(base_intensity)
    } else if hovered {
        base_intensity.max(theme.glow_intensity_hover)
    } else {
        base_intensity
    }
}

fn render_kind(
    kind: &WidgetKind,
    bounds: [f32; 4],
    clip: [f32; 4],
    theme: &Theme,
    hovered: bool,
    pressed: bool,
    measure: &dyn TextMeasure,
    frame: &mut Frame,
) {
    let bounds = press_offset(bounds, pressed);

    match kind {
        WidgetKind::Container | WidgetKind::ScrollView { .. } => {
            // Structural nodes with no independent visual quad
        }

        WidgetKind::Label { text, muted } => {
            let (color, role) = if *muted {
                (theme.text_muted, TextRole::Caption)
            } else {
                (theme.text_color, TextRole::Body)
            };
            frame.texts.push(text_spec(text.clone(), inset(bounds, 4.0, 0.0), clip, theme, color, TextAlign::Left, role));
        }

        WidgetKind::Button { label, enabled, .. } => {
            let glow = if *enabled { theme.accent } else { [0.4, 0.4, 0.4, 1.0] };
            let base = if *enabled { theme.glow_intensity } else { 0.0 };
            let intensity = if *enabled { interactive_glow(base, hovered, pressed, theme) } else { 0.0 };
            let bg = [theme.glass_bg[0] + 0.05, theme.glass_bg[1] + 0.07, theme.glass_bg[2] + 0.10, 0.85];
            frame.instances.push(glass_instance(bounds, clip, bg, glow, intensity, theme));
            frame.texts.push(text_spec(label.clone(), bounds, clip, theme, theme.text_color, TextAlign::Center, TextRole::Body));
        }

        WidgetKind::IconButton { icon, enabled, .. } => {
            let glow = if *enabled { theme.accent } else { [0.4, 0.4, 0.4, 1.0] };
            let base = if *enabled { theme.glow_intensity * 0.5 } else { 0.0 };
            let intensity = if *enabled { interactive_glow(base, hovered, pressed, theme) } else { 0.0 };
            let bg = if hovered {
                [theme.accent[0] * 0.20, theme.accent[1] * 0.20, theme.accent[2] * 0.20, 0.90]
            } else {
                [theme.glass_bg[0] * 0.70, theme.glass_bg[1] * 0.70, theme.glass_bg[2] * 0.70, 0.60]
            };
            frame.instances.push(custom_glass_instance(bounds, clip, bg, glow, 6.0, theme.border_width, intensity));
            let glyph = icon.glyph();
            let icon_color = if *enabled { if hovered { [1.0, 1.0, 1.0, 1.0] } else { theme.text_color } } else { theme.text_muted };
            frame.texts.push(text_spec(glyph.to_string(), bounds, clip, theme, icon_color, TextAlign::Center, TextRole::Body));
        }

        WidgetKind::Icon { kind, size, color } => {
            let glyph = kind.glyph();
            let icon_color = color.unwrap_or(theme.accent);
            frame.texts.push(TextSpec {
                text: glyph.to_string(),
                bounds,
                font_size: *size,
                color: icon_color,
                align: TextAlign::Center,
                weight: FontWeight::Normal,
                clip,
            });
        }

        WidgetKind::TableHeader { title, sorted_asc, .. } => {
            let bg = if hovered {
                [theme.accent[0] * 0.18, theme.accent[1] * 0.18, theme.accent[2] * 0.18, 0.85]
            } else {
                [theme.glass_bg[0] * 0.85, theme.glass_bg[1] * 0.85, theme.glass_bg[2] * 0.85, 0.75]
            };
            frame.instances.push(custom_glass_instance(bounds, clip, bg, theme.accent_secondary, 0.0, 0.5, if hovered { 0.15 } else { 0.0 }));

            let sort_indicator = match sorted_asc {
                Some(true) => " ▴",
                Some(false) => " ▾",
                None => "",
            };
            let header_text = format!("{}{}", title, sort_indicator);
            let text_bounds = [bounds[0] + 8.0, bounds[1], bounds[2] - 16.0, bounds[3]];
            let text_color = if sorted_asc.is_some() { theme.accent } else if hovered { [1.0, 1.0, 1.0, 1.0] } else { theme.text_color };
            frame.texts.push(text_spec(header_text, text_bounds, clip, theme, text_color, TextAlign::Left, TextRole::Small));
        }

        WidgetKind::TableCell { text, badge, selected, row_index, .. } => {
            let is_even = row_index % 2 == 0;
            let bg = if *selected {
                [theme.accent[0] * 0.22, theme.accent[1] * 0.22, theme.accent[2] * 0.22, 0.88]
            } else if hovered {
                [theme.accent[0] * 0.10, theme.accent[1] * 0.10, theme.accent[2] * 0.10, 0.50]
            } else if is_even {
                [theme.glass_bg[0] * 0.40, theme.glass_bg[1] * 0.40, theme.glass_bg[2] * 0.40, 0.35]
            } else {
                [0.0, 0.0, 0.0, 0.0]
            };
            if bg[3] > 0.0 {
                frame.instances.push(custom_glass_instance(bounds, clip, bg, theme.accent, 0.0, if *selected { 0.5 } else { 0.0 }, if *selected { 0.2 } else { 0.0 }));
            }

            let badge_reserve = match badge {
                crate::kind::ListItemBadge::Success | crate::kind::ListItemBadge::Warning => 22.0,
                crate::kind::ListItemBadge::Active(_) | crate::kind::ListItemBadge::Custom { .. } => 64.0,
                crate::kind::ListItemBadge::None => 16.0,
            };

            if !text.is_empty() {
                let text_color = if *selected { [1.0, 1.0, 1.0, 1.0] } else if hovered { theme.text_color } else { [0.88, 0.92, 0.98, 1.0] };
                let text_w = (bounds[2] - badge_reserve).max(10.0);
                let text_bounds = [bounds[0] + 8.0, bounds[1], text_w, bounds[3]];
                frame.texts.push(text_spec(text.clone(), text_bounds, clip, theme, text_color, TextAlign::Left, TextRole::Small));
            }

            match badge {
                crate::kind::ListItemBadge::Success => {
                    let dot_size = 8.0;
                    let dot_bounds = [bounds[0] + bounds[2] - dot_size - 10.0, bounds[1] + (bounds[3] - dot_size) * 0.5, dot_size, dot_size];
                    frame.instances.push(glass_instance(dot_bounds, clip, theme.success, theme.success, 0.2, theme));
                }
                crate::kind::ListItemBadge::Warning => {
                    let dot_size = 8.0;
                    let dot_bounds = [bounds[0] + bounds[2] - dot_size - 10.0, bounds[1] + (bounds[3] - dot_size) * 0.5, dot_size, dot_size];
                    frame.instances.push(glass_instance(dot_bounds, clip, theme.warning, theme.warning, 0.2, theme));
                }
                crate::kind::ListItemBadge::Active(lbl) => {
                    let badge_w = 54.0;
                    let badge_h = 18.0;
                    let badge_bounds = [bounds[0] + bounds[2] - badge_w - 6.0, bounds[1] + (bounds[3] - badge_h) * 0.5, badge_w, badge_h];
                    frame.instances.push(glass_instance(badge_bounds, clip, [theme.accent[0]*0.15, theme.accent[1]*0.15, theme.accent[2]*0.15, 0.8], theme.accent, 0.15, theme));
                    frame.texts.push(text_spec(lbl.clone(), badge_bounds, clip, theme, theme.accent, TextAlign::Center, TextRole::Caption));
                }
                _ => {}
            }
        }

        WidgetKind::AccordionHeader { title, subtitle, expanded, .. } => {
            let intensity = interactive_glow(if *expanded { theme.glow_intensity * 0.6 } else { 0.0 }, hovered, pressed, theme);
            let border_color = if *expanded { theme.accent } else { theme.accent_secondary };
            let bg = if hovered {
                [theme.glass_bg[0] + 0.05, theme.glass_bg[1] + 0.07, theme.glass_bg[2] + 0.11, 0.88]
            } else {
                [theme.glass_bg[0] + 0.02, theme.glass_bg[1] + 0.03, theme.glass_bg[2] + 0.05, 0.75]
            };
            frame.instances.push(custom_glass_instance(bounds, clip, bg, border_color, 8.0, theme.border_width, intensity));

            let arrow = if *expanded { "▼" } else { "▶" };
            let arrow_bounds = [bounds[0] + 12.0, bounds[1], 16.0, bounds[3]];
            frame.texts.push(text_spec(arrow.to_string(), arrow_bounds, clip, theme, theme.accent, TextAlign::Center, TextRole::Caption));

            if let Some(sub) = subtitle {
                let title_h = 18.0;
                let sub_h = 14.0;
                let total_text_h = title_h + sub_h + 2.0;
                let start_y = bounds[1] + (bounds[3] - total_text_h) * 0.5;

                let title_bounds = [bounds[0] + 34.0, start_y, bounds[2] - 46.0, title_h];
                frame.texts.push(text_spec(title.clone(), title_bounds, clip, theme, [1.0, 1.0, 1.0, 1.0], TextAlign::Left, TextRole::Body));

                let sub_bounds = [bounds[0] + 34.0, start_y + title_h + 2.0, bounds[2] - 46.0, sub_h];
                frame.texts.push(text_spec(sub.clone(), sub_bounds, clip, theme, theme.text_muted, TextAlign::Left, TextRole::Caption));
            } else {
                let title_bounds = [bounds[0] + 34.0, bounds[1], bounds[2] - 46.0, bounds[3]];
                frame.texts.push(text_spec(title.clone(), title_bounds, clip, theme, [1.0, 1.0, 1.0, 1.0], TextAlign::Left, TextRole::Body));
            }
        }

        WidgetKind::Checkbox { checked, .. } => {
            let bg = if *checked {
                [theme.accent[0] * 0.35, theme.accent[1] * 0.35, theme.accent[2] * 0.35, 0.95]
            } else {
                theme.glass_bg
            };
            let base = if *checked { theme.glow_intensity_hover * 0.8 } else { 0.0 };
            let intensity = interactive_glow(base, hovered, pressed, theme);
            frame.instances.push(glass_instance(bounds, clip, bg, theme.accent, intensity, theme));
        }

        WidgetKind::Toggle { active, .. } => {
            let bg = if *active {
                [theme.accent[0] * 0.25, theme.accent[1] * 0.25, theme.accent[2] * 0.25, 0.85]
            } else {
                [theme.glass_bg[0] * 0.6, theme.glass_bg[1] * 0.6, theme.glass_bg[2] * 0.6, 0.5]
            };
            let glow = if *active { theme.accent } else { [0.3, 0.3, 0.3, 1.0] };
            let base = if *active { theme.glow_intensity } else { 0.0 };
            let intensity = interactive_glow(base, hovered, pressed, theme);
            frame.instances.push(glass_instance(bounds, clip, bg, glow, intensity, theme));

            let thumb_size = (bounds[3] - 6.0).max(4.0);
            let thumb_x = if *active {
                bounds[0] + bounds[2] - thumb_size - 3.0
            } else {
                bounds[0] + 3.0
            };
            let thumb_y = bounds[1] + 3.0;
            let thumb_bounds = [thumb_x, thumb_y, thumb_size, thumb_size];
            let thumb_color = if *active { theme.accent } else { [0.65, 0.72, 0.82, 1.0] };
            let thumb_glow = if *active { 0.30 } else { 0.0 };
            frame.instances.push(glass_instance(thumb_bounds, clip, thumb_color, thumb_color, thumb_glow, theme));
        }

        WidgetKind::Slider { min, max, value, .. } => {
            let track_h = 6.0;
            let track_y = bounds[1] + (bounds[3] - track_h) * 0.5;
            let track_bounds = [bounds[0], track_y, bounds[2], track_h];
            frame.instances.push(glass_instance(track_bounds, clip, theme.glass_bg, theme.accent_secondary, 0.0, theme));

            let range = (max - min).max(1.0e-5);
            let ratio = ((value - min) / range).clamp(0.0, 1.0);

            if ratio > 0.0 {
                let fill_w = bounds[2] * ratio;
                let fill_bounds = [bounds[0], track_y, fill_w, track_h];
                frame.instances.push(glass_instance(fill_bounds, clip, theme.accent, theme.accent, 0.35, theme));
            }

            let thumb_w = 14.0;
            let thumb_h = (bounds[3] - 4.0).max(14.0);
            let thumb_x = bounds[0] + ratio * bounds[2] - thumb_w * 0.5;
            let thumb_y = bounds[1] + (bounds[3] - thumb_h) * 0.5;
            let thumb_bounds = [thumb_x, thumb_y, thumb_w, thumb_h];
            let intensity = interactive_glow(theme.glow_intensity * 0.8, hovered, pressed, theme);
            frame.instances.push(glass_instance(thumb_bounds, clip, [0.95, 0.98, 1.0, 1.0], theme.accent, intensity, theme));
        }

        WidgetKind::ProgressBar { progress } => {
            let track_h = (bounds[3] - 2.0).max(4.0);
            let track_y = bounds[1] + (bounds[3] - track_h) * 0.5;
            let track_bounds = [bounds[0], track_y, bounds[2], track_h];
            frame.instances.push(glass_instance(track_bounds, clip, [theme.glass_bg[0] * 0.7, theme.glass_bg[1] * 0.7, theme.glass_bg[2] * 0.7, 0.6], theme.accent_secondary, 0.0, theme));

            let ratio = progress.clamp(0.0, 1.0);
            if ratio > 0.0 {
                let fill_w = (bounds[2] * ratio).max(4.0);
                let fill_bounds = [bounds[0], track_y, fill_w, track_h];
                frame.instances.push(glass_instance(fill_bounds, clip, theme.accent, theme.accent, 0.4, theme));
            }
        }

        WidgetKind::MetricCard { title, value, delta } => {
            frame.instances.push(glass_instance(bounds, clip, theme.glass_bg, theme.accent_secondary, 0.05, theme));

            let title_bounds = [bounds[0] + 12.0, bounds[1] + 6.0, bounds[2] - 70.0, 16.0];
            frame.texts.push(text_spec(title.clone(), title_bounds, clip, theme, theme.text_muted, TextAlign::Left, TextRole::Caption));

            let val_h = (bounds[3] - 26.0).max(18.0);
            let val_bounds = [bounds[0] + 12.0, bounds[1] + 20.0, bounds[2] - 24.0, val_h];
            frame.texts.push(TextSpec {
                text: value.clone(),
                bounds: val_bounds,
                font_size: 16.5,
                color: theme.text_color,
                align: TextAlign::Left,
                weight: FontWeight::Bold,
                clip,
            });

            if let Some((delta_text, positive)) = delta {
                let badge_w = 46.0;
                let badge_h = 18.0;
                let badge_bounds = [bounds[0] + bounds[2] - badge_w - 10.0, bounds[1] + 8.0, badge_w, badge_h];
                let color = if *positive { theme.success } else { theme.danger };
                let bg = [color[0] * 0.15, color[1] * 0.15, color[2] * 0.15, 0.75];
                frame.instances.push(glass_instance(badge_bounds, clip, bg, color, 0.15, theme));
                frame.texts.push(text_spec(delta_text.clone(), badge_bounds, clip, theme, color, TextAlign::Center, TextRole::Caption));
            }
        }

        WidgetKind::TextInput { value, placeholder, focused, cursor, selection, .. } => {
            let base = if *focused { theme.glow_intensity_hover * 1.1 } else { theme.glow_intensity * 0.4 };
            let intensity = interactive_glow(base, hovered, pressed, theme);
            let border_accent = if *focused { theme.accent } else { theme.accent_secondary };
            frame.instances.push(glass_instance(bounds, clip, theme.glass_bg, border_accent, intensity, theme));
            let (text, color) = if value.is_empty() {
                (placeholder.clone(), theme.text_muted)
            } else {
                (value.clone(), theme.text_color)
            };
            frame.texts.push(text_spec(text, inset(bounds, 12.0, 0.0), clip, theme, color, TextAlign::Left, TextRole::Body));

            if *focused {
                let cursor_h = (bounds[3] - 14.0).max(12.0);
                let cursor_y = bounds[1] + (bounds[3] - cursor_h) * 0.5;

                // Selection highlight
                if let Some((s_start, s_end)) = selection {
                    if s_start != s_end {
                        let min_s = (*s_start).min(*s_end).min(value.len());
                        let max_s = (*s_start).max(*s_end).min(value.len());
                        let x1 = bounds[0] + 12.0 + measure.caret_x(value, &theme.typography.family, theme.typography.body_size, min_s);
                        let x2 = bounds[0] + 12.0 + measure.caret_x(value, &theme.typography.family, theme.typography.body_size, max_s);
                        let sel_bounds = [x1, cursor_y, (x2 - x1).max(2.0), cursor_h];
                        let sel_bg = [1.0 / 255.0, 35.0 / 255.0, 45.0 / 255.0, 0.90];
                        let sel_border = [theme.accent[0] * 0.7, theme.accent[1] * 0.7, theme.accent[2] * 0.7, 0.8];
                        frame.instances.push(custom_glass_instance(sel_bounds, clip, sel_bg, sel_border, 2.0, 1.0, 0.15));
                    }
                }

                // Caret line at exact cursor index
                let safe_cursor = (*cursor).min(value.len());
                let text_w = measure.caret_x(value, &theme.typography.family, theme.typography.body_size, safe_cursor);
                let cursor_x = (bounds[0] + 12.0 + text_w).min(bounds[0] + bounds[2] - 14.0);
                let cursor_bounds = [cursor_x, cursor_y, 2.0, cursor_h];
                frame.instances.push(glass_instance(cursor_bounds, clip, theme.accent, theme.accent, 0.6, theme));
            }
        }

        WidgetKind::TextArea { value, placeholder, focused, line_numbers, cursor, selection, .. } => {
            let base = if *focused { theme.glow_intensity_hover * 1.1 } else { theme.glow_intensity * 0.4 };
            let intensity = interactive_glow(base, hovered, pressed, theme);
            let border_accent = if *focused { theme.accent } else { theme.accent_secondary };
            frame.instances.push(glass_instance(bounds, clip, theme.glass_bg, border_accent, intensity, theme));

            let gutter_w = if *line_numbers { 32.0 } else { 0.0 };
            if *line_numbers {
                let gutter_bounds = [bounds[0], bounds[1], gutter_w, bounds[3]];
                let gutter_bg = [theme.glass_bg[0] * 0.5, theme.glass_bg[1] * 0.5, theme.glass_bg[2] * 0.5, 0.65];
                frame.instances.push(glass_instance(gutter_bounds, clip, gutter_bg, [0.0, 0.0, 0.0, 0.0], 0.0, theme));

                let div_bounds = [bounds[0] + gutter_w, bounds[1], 1.0, bounds[3]];
                frame.instances.push(glass_instance(div_bounds, clip, theme.accent_secondary, theme.accent_secondary, 0.0, theme));
            }

            let text_offset_x = bounds[0] + gutter_w + 10.0;
            let text_w = (bounds[2] - gutter_w - 18.0).max(10.0);
            let line_h = 20.0;
            let start_y = bounds[1] + 8.0;

            if value.is_empty() {
                let line_bounds = [text_offset_x, start_y, text_w, line_h];
                frame.texts.push(text_spec(placeholder.clone(), line_bounds, clip, theme, theme.text_muted, TextAlign::Left, TextRole::Body));
                if *line_numbers {
                    let num_bounds = [bounds[0] + 2.0, start_y, gutter_w - 6.0, line_h];
                    frame.texts.push(text_spec("1".to_string(), num_bounds, clip, theme, theme.text_muted, TextAlign::Right, TextRole::Caption));
                }
                if *focused {
                    let cursor_bounds = [text_offset_x, start_y + 2.0, 2.0, line_h - 4.0];
                    frame.instances.push(glass_instance(cursor_bounds, clip, theme.accent, theme.accent, 0.6, theme));
                }
            } else {
                let lines: Vec<&str> = value.split('\n').collect();
                for (idx, line_str) in lines.iter().enumerate() {
                    let cur_y = start_y + idx as f32 * line_h;
                    if cur_y + line_h > bounds[1] + bounds[3] {
                        break;
                    }
                    if *line_numbers {
                        let num_bounds = [bounds[0] + 2.0, cur_y, gutter_w - 6.0, line_h];
                        frame.texts.push(text_spec((idx + 1).to_string(), num_bounds, clip, theme, theme.text_muted, TextAlign::Right, TextRole::Caption));
                    }
                    let line_bounds = [text_offset_x, cur_y, text_w, line_h];
                    frame.texts.push(text_spec(line_str.to_string(), line_bounds, clip, theme, theme.text_color, TextAlign::Left, TextRole::Body));
                }

                if *focused {
                    // Optional selection highlight
                    if let Some((s_start, s_end)) = selection {
                        if s_start != s_end {
                            let min_s = (*s_start).min(*s_end).min(value.len());
                            let max_s = (*s_start).max(*s_end).min(value.len());
                            let mut line_offset = 0;
                            for (idx, line_str) in lines.iter().enumerate() {
                                let line_len = line_str.len();
                                let line_end = line_offset + line_len;
                                if max_s > line_offset && min_s < line_end {
                                    let l_start = min_s.saturating_sub(line_offset).min(line_len);
                                    let l_end = (max_s - line_offset).min(line_len);
                                    let x1 = text_offset_x + measure.caret_x(line_str, &theme.typography.family, theme.typography.body_size, l_start);
                                    let x2 = text_offset_x + measure.caret_x(line_str, &theme.typography.family, theme.typography.body_size, l_end);
                                    let cur_y = start_y + idx as f32 * line_h;
                                    if cur_y + line_h <= bounds[1] + bounds[3] {
                                        let sel_bounds = [x1, cur_y + 1.0, (x2 - x1).max(2.0), line_h - 2.0];
                                        let sel_bg = [1.0 / 255.0, 35.0 / 255.0, 45.0 / 255.0, 0.90];
                                        let sel_border = [theme.accent[0] * 0.7, theme.accent[1] * 0.7, theme.accent[2] * 0.7, 0.8];
                                        frame.instances.push(custom_glass_instance(sel_bounds, clip, sel_bg, sel_border, 2.0, 1.0, 0.15));
                                    }
                                }
                                line_offset += line_len + 1;
                            }
                        }
                    }

                    // Resolve cursor line and column index
                    let mut acc = 0;
                    let mut cur_line_idx = 0;
                    let mut cur_col_idx = 0;
                    let safe_cursor = (*cursor).min(value.len());

                    for (i, l) in lines.iter().enumerate() {
                        let line_len = l.len();
                        if acc + line_len >= safe_cursor || i == lines.len() - 1 {
                            cur_line_idx = i;
                            cur_col_idx = safe_cursor.saturating_sub(acc).min(line_len);
                            break;
                        }
                        acc += line_len + 1; // +1 for '\n'
                    }

                    let cur_line_str = lines.get(cur_line_idx).unwrap_or(&"");
                    let cursor_x = (text_offset_x + measure.caret_x(cur_line_str, &theme.typography.family, theme.typography.body_size, cur_col_idx)).min(bounds[0] + bounds[2] - 12.0);
                    let cur_y = start_y + cur_line_idx as f32 * line_h;

                    if cur_y + line_h <= bounds[1] + bounds[3] {
                        let cursor_bounds = [cursor_x, cur_y + 2.0, 2.0, line_h - 4.0];
                        frame.instances.push(glass_instance(cursor_bounds, clip, theme.accent, theme.accent, 0.6, theme));
                    }
                }
            }
        }

        WidgetKind::PasswordInput { value, placeholder, focused, revealed, cursor, .. } => {
            let base = if *focused { theme.glow_intensity_hover * 1.1 } else { theme.glow_intensity * 0.4 };
            let intensity = interactive_glow(base, hovered, pressed, theme);
            let border_accent = if *focused { theme.accent } else { theme.accent_secondary };
            frame.instances.push(glass_instance(bounds, clip, theme.glass_bg, border_accent, intensity, theme));

            let display_text = if value.is_empty() {
                placeholder.clone()
            } else if *revealed {
                value.clone()
            } else {
                "•".repeat(value.chars().count())
            };
            let text_color = if value.is_empty() { theme.text_muted } else { theme.text_color };
            let text_box = [bounds[0] + 12.0, bounds[1], bounds[2] - 44.0, bounds[3]];
            frame.texts.push(text_spec(display_text, text_box, clip, theme, text_color, TextAlign::Left, TextRole::Body));

            // Eye reveal toggle glyph on the right
            let eye_box = [bounds[0] + bounds[2] - 28.0, bounds[1], 24.0, bounds[3]];
            let eye_glyph = if *revealed { "👁" } else { "Ø" };
            let eye_color = if *revealed { theme.accent } else { theme.text_muted };
            frame.texts.push(text_spec(eye_glyph.to_string(), eye_box, clip, theme, eye_color, TextAlign::Center, TextRole::Body));

            if *focused {
                let safe_cursor = (*cursor).min(value.len());
                let text_w = if *revealed {
                    measure.caret_x(value, &theme.typography.family, theme.typography.body_size, safe_cursor)
                } else {
                    let char_count = value[..safe_cursor].chars().count();
                    char_count as f32 * (theme.typography.body_size * 0.55)
                };
                let cursor_x = (bounds[0] + 12.0 + text_w).min(bounds[0] + bounds[2] - 34.0);
                let cursor_h = (bounds[3] - 14.0).max(12.0);
                let cursor_y = bounds[1] + (bounds[3] - cursor_h) * 0.5;
                let cursor_bounds = [cursor_x, cursor_y, 2.0, cursor_h];
                frame.instances.push(glass_instance(cursor_bounds, clip, theme.accent, theme.accent, 0.6, theme));
            }
        }

        WidgetKind::NumberInput { value, precision, focused, .. } => {
            let base = if *focused { theme.glow_intensity_hover * 1.1 } else { theme.glow_intensity * 0.4 };
            let intensity = interactive_glow(base, hovered, pressed, theme);
            let border_accent = if *focused { theme.accent } else { theme.accent_secondary };
            frame.instances.push(glass_instance(bounds, clip, theme.glass_bg, border_accent, intensity, theme));

            let num_text = format!("{:.precision$}", value, precision = *precision);
            let text_box = [bounds[0] + 12.0, bounds[1], bounds[2] - 38.0, bounds[3]];
            frame.texts.push(text_spec(num_text, text_box, clip, theme, theme.text_color, TextAlign::Left, TextRole::Body));

            // Stepper buttons on the right side
            let stepper_w = 24.0;
            let stepper_x = bounds[0] + bounds[2] - stepper_w - 2.0;
            let half_h = (bounds[3] - 4.0) * 0.5;

            let up_box = [stepper_x, bounds[1] + 2.0, stepper_w, half_h];
            let down_box = [stepper_x, bounds[1] + 2.0 + half_h, stepper_w, half_h];

            let btn_bg = [theme.glass_bg[0] * 0.8, theme.glass_bg[1] * 0.8, theme.glass_bg[2] * 0.8, 0.5];
            frame.instances.push(glass_instance(up_box, clip, btn_bg, [0.0, 0.0, 0.0, 0.0], 0.0, theme));
            frame.instances.push(glass_instance(down_box, clip, btn_bg, [0.0, 0.0, 0.0, 0.0], 0.0, theme));

            frame.texts.push(text_spec("▲".to_string(), up_box, clip, theme, theme.text_muted, TextAlign::Center, TextRole::Caption));
            frame.texts.push(text_spec("▼".to_string(), down_box, clip, theme, theme.text_muted, TextAlign::Center, TextRole::Caption));
        }

        WidgetKind::RadioButton { label, selected, .. } => {
            let circle_size = 18.0;
            let circle_bounds = [bounds[0] + 4.0, bounds[1] + (bounds[3] - circle_size) * 0.5, circle_size, circle_size];
            let glow = if *selected { theme.accent } else { [0.3, 0.3, 0.3, 1.0] };
            let intensity = if *selected { interactive_glow(theme.glow_intensity_hover * 0.8, hovered, pressed, theme) } else { interactive_glow(0.0, hovered, pressed, theme) };
            let bg = if *selected {
                [theme.accent[0] * 0.25, theme.accent[1] * 0.25, theme.accent[2] * 0.25, 0.9]
            } else {
                theme.glass_bg
            };
            frame.instances.push(glass_instance(circle_bounds, clip, bg, glow, intensity, theme));

            if *selected {
                let dot_size = 8.0;
                let dot_bounds = [circle_bounds[0] + (circle_size - dot_size) * 0.5, circle_bounds[1] + (circle_size - dot_size) * 0.5, dot_size, dot_size];
                frame.instances.push(glass_instance(dot_bounds, clip, theme.accent, theme.accent, 0.4, theme));
            }

            let text_bounds = [bounds[0] + circle_size + 12.0, bounds[1], bounds[2] - circle_size - 12.0, bounds[3]];
            let text_color = if *selected { theme.text_color } else { theme.text_muted };
            frame.texts.push(text_spec(label.clone(), text_bounds, clip, theme, text_color, TextAlign::Left, TextRole::Body));
        }

        WidgetKind::SegmentItem { label, selected, .. } => {
            let glow = if *selected { theme.accent } else { [0.25, 0.25, 0.25, 1.0] };
            let base = if *selected { theme.glow_intensity * 0.8 } else { 0.0 };
            let intensity = interactive_glow(base, hovered, pressed, theme);
            let bg = if *selected {
                [theme.accent[0] * 0.28, theme.accent[1] * 0.28, theme.accent[2] * 0.28, 0.95]
            } else {
                [theme.glass_bg[0] * 0.55, theme.glass_bg[1] * 0.55, theme.glass_bg[2] * 0.55, 0.45]
            };
            frame.instances.push(glass_instance(bounds, clip, bg, glow, intensity, theme));
            let text_color = if *selected { [1.0, 1.0, 1.0, 1.0] } else { theme.text_muted };
            frame.texts.push(text_spec(label.clone(), bounds, clip, theme, text_color, TextAlign::Center, TextRole::Small));
        }

        WidgetKind::Divider { .. } => {
            let color = [theme.accent_secondary[0] * 0.35, theme.accent_secondary[1] * 0.35, theme.accent_secondary[2] * 0.35, 0.5];
            frame.instances.push(glass_instance(bounds, clip, color, color, 0.0, theme));
        }

        WidgetKind::Modal { title, .. } => {
            let bg = [0.08, 0.11, 0.18, 0.98];
            frame.instances.push(glass_instance(bounds, clip, bg, theme.accent, theme.glow_intensity * 1.5, theme));
            let title_bar = [bounds[0] + 20.0, bounds[1] + 16.0, bounds[2] - 48.0, 24.0];
            frame.texts.push(text_spec(title.clone(), title_bar, clip, theme, theme.text_color, TextAlign::Left, TextRole::Title));
        }

        WidgetKind::ModalBackdrop { .. } => {
            let backdrop_bg = [0.01, 0.02, 0.04, 0.42];
            frame.instances.push(glass_instance(bounds, clip, backdrop_bg, backdrop_bg, 0.0, theme));
        }

        WidgetKind::Window { title, .. } => {
            frame.instances.push(glass_instance(bounds, clip, theme.glass_bg, theme.accent_secondary, theme.glow_intensity, theme));
            let title_bar = [bounds[0] + 16.0, bounds[1] + 8.0, bounds[2] - 48.0, 28.0];
            frame.texts.push(text_spec(title.clone(), title_bar, clip, theme, theme.text_color, TextAlign::Left, TextRole::Title));
        }

        WidgetKind::Palette { .. } => {
            let bg = [theme.glass_bg[0] + 0.02, theme.glass_bg[1] + 0.03, theme.glass_bg[2] + 0.06, 0.95];
            let glow = if hovered || pressed { theme.accent } else { theme.accent_secondary };
            let intensity = if hovered || pressed { 0.25 } else { 0.08 };
            frame.instances.push(custom_glass_instance(bounds, clip, bg, glow, 8.0, 1.2, intensity));
        }

        WidgetKind::PaletteHeader { title, folded, .. } => {
            let h_bg = [theme.glass_bg[0] * 0.75, theme.glass_bg[1] * 0.75, theme.glass_bg[2] * 0.75, 0.85];
            frame.instances.push(custom_glass_instance(bounds, clip, h_bg, [0.0, 0.0, 0.0, 0.0], 6.0, 0.0, 0.0));

            // Textured grip indicator [::] on left
            let grip_box = [bounds[0] + 6.0, bounds[1], 16.0, bounds[3]];
            frame.texts.push(text_spec("⋮⋮".to_string(), grip_box, clip, theme, theme.accent, TextAlign::Center, TextRole::Small));

            // Title text
            let title_box = [bounds[0] + 24.0, bounds[1], bounds[2] - 68.0, bounds[3]];
            frame.texts.push(TextSpec {
                text: title.clone(),
                bounds: title_box,
                font_size: 11.5,
                color: [0.95, 0.98, 1.0, 1.0],
                align: TextAlign::Left,
                weight: FontWeight::Bold,
                clip,
            });

            // Fold / Unfold button glyph [^] / [v]
            let fold_box = [bounds[0] + bounds[2] - 42.0, bounds[1], 18.0, bounds[3]];
            let fold_glyph = if *folded { "▼" } else { "▲" };
            frame.texts.push(text_spec(fold_glyph.to_string(), fold_box, clip, theme, theme.text_muted, TextAlign::Center, TextRole::Caption));

            // Close button glyph [×]
            let close_box = [bounds[0] + bounds[2] - 22.0, bounds[1], 18.0, bounds[3]];
            frame.texts.push(text_spec("✕".to_string(), close_box, clip, theme, theme.danger, TextAlign::Center, TextRole::Caption));
        }

        WidgetKind::PaletteFoldButton { folded, .. } => {
            let glyph = if *folded { "▼" } else { "▲" };
            frame.texts.push(text_spec(glyph.to_string(), bounds, clip, theme, theme.text_muted, TextAlign::Center, TextRole::Caption));
        }

        WidgetKind::PaletteCloseButton { .. } => {
            frame.texts.push(text_spec("✕".to_string(), bounds, clip, theme, theme.danger, TextAlign::Center, TextRole::Caption));
        }

        WidgetKind::ResizeGrip { .. } => {
            frame.texts.push(text_spec("⇲".to_string(), bounds, clip, theme, theme.accent, TextAlign::Center, TextRole::Caption));
        }

        WidgetKind::WindowCloseButton { .. } => {
            let intensity = interactive_glow(theme.glow_intensity, hovered, pressed, theme);
            frame.instances.push(glass_instance(bounds, clip, theme.glass_bg, theme.accent_secondary, intensity, theme));
            let inner = inset(bounds, 5.0, 5.0);
            frame.instances.push(glass_instance(inner, clip, [theme.accent[0] * 0.2, theme.accent[1] * 0.2, theme.accent[2] * 0.2, 0.8], theme.accent, intensity * 0.8, theme));
        }

        WidgetKind::Scrollbar { content_size, viewport_size, offset, .. } => {
            frame.instances.push(glass_instance(bounds, clip, theme.glass_bg, theme.accent, 0.0, theme));
            let thumb_bounds = scrollbar_thumb_bounds(bounds, *content_size, *viewport_size, *offset);
            let intensity = interactive_glow(theme.glow_intensity * 0.7, hovered, pressed, theme);
            frame.instances.push(glass_instance(thumb_bounds, clip, theme.accent, theme.accent, intensity, theme));
        }

        WidgetKind::TabItem { label, active, .. } => {
            let glow = if *active { theme.accent } else { theme.accent_secondary };
            let base = if *active { theme.glow_intensity } else { 0.0 };
            let intensity = interactive_glow(base, hovered, pressed, theme);
            let bg = if *active {
                [theme.glass_bg[0] + 0.06, theme.glass_bg[1] + 0.09, theme.glass_bg[2] + 0.13, 0.88]
            } else {
                [theme.glass_bg[0] * 0.65, theme.glass_bg[1] * 0.65, theme.glass_bg[2] * 0.65, 0.55]
            };
            frame.instances.push(glass_instance(bounds, clip, bg, glow, intensity, theme));

            if *active {
                let bar_h = 2.0;
                let bar_bounds = [bounds[0] + 8.0, bounds[1] + bounds[3] - bar_h - 1.0, bounds[2] - 16.0, bar_h];
                frame.instances.push(glass_instance(bar_bounds, clip, theme.accent, theme.accent, 0.45, theme));
            }

            let text_color = if *active { theme.accent } else { theme.text_muted };
            frame.texts.push(text_spec(label.clone(), bounds, clip, theme, text_color, TextAlign::Center, TextRole::Small));
        }

        WidgetKind::MenuBar => {
            let bar_bg = [theme.glass_bg[0] * 0.45, theme.glass_bg[1] * 0.45, theme.glass_bg[2] * 0.45, 0.82];
            let bar_border = [theme.accent_secondary[0] * 0.35, theme.accent_secondary[1] * 0.35, theme.accent_secondary[2] * 0.35, 0.45];
            frame.instances.push(custom_glass_instance(bounds, clip, bar_bg, bar_border, 6.0, 1.0, 0.0));
        }

        WidgetKind::MenuBarItem { label, active, .. } => {
            if *active || hovered {
                let bg = if *active {
                    [theme.accent[0] * 0.28, theme.accent[1] * 0.28, theme.accent[2] * 0.28, 0.92]
                } else {
                    [theme.accent[0] * 0.16, theme.accent[1] * 0.16, theme.accent[2] * 0.16, 0.75]
                };
                let glow_intensity = if *active { 0.20 } else { 0.0 };
                let border_width = if *active { 1.0 } else { 0.0 };
                frame.instances.push(custom_glass_instance(bounds, clip, bg, theme.accent, 4.0, border_width, glow_intensity));
            }

            let text_color = if *active {
                [1.0, 1.0, 1.0, 1.0]
            } else if hovered {
                theme.text_color
            } else {
                [0.82, 0.88, 0.95, 1.0]
            };
            frame.texts.push(text_spec(label.clone(), bounds, clip, theme, text_color, TextAlign::Center, TextRole::Small));
        }

        WidgetKind::MenuPopover => {
            // Distinct popover palette container card with rounded corners and glowing border
            let popover_bg = [0.05, 0.07, 0.13, 0.98];
            let popover_border = [theme.accent[0] * 0.65, theme.accent[1] * 0.65, theme.accent[2] * 0.65, 0.85];
            frame.instances.push(custom_glass_instance(bounds, clip, popover_bg, popover_border, 8.0, 1.2, 0.30));
        }

        WidgetKind::MenuItem { label, shortcut, enabled, .. } => {
            if hovered && *enabled {
                let item_bg = [theme.accent[0] * 0.32, theme.accent[1] * 0.32, theme.accent[2] * 0.32, 0.95];
                frame.instances.push(custom_glass_instance(bounds, clip, item_bg, [0.0, 0.0, 0.0, 0.0], 4.0, 0.0, 0.0));
            }

            let text_color = if *enabled {
                if hovered { [1.0, 1.0, 1.0, 1.0] } else { [0.88, 0.93, 0.98, 1.0] }
            } else {
                theme.text_muted
            };
            let label_bounds = [bounds[0] + 12.0, bounds[1], bounds[2] - 68.0, bounds[3]];
            frame.texts.push(text_spec(label.clone(), label_bounds, clip, theme, text_color, TextAlign::Left, TextRole::Small));

            if let Some(sc) = shortcut {
                let sc_color = if hovered && *enabled { theme.accent } else { theme.text_muted };
                let sc_bounds = [bounds[0] + bounds[2] - 60.0, bounds[1], 48.0, bounds[3]];
                frame.texts.push(text_spec(sc.clone(), sc_bounds, clip, theme, sc_color, TextAlign::Right, TextRole::Caption));
            }
        }

        WidgetKind::Dropdown { label, selected_text, open, .. } => {
            let intensity = interactive_glow(theme.glow_intensity * 0.6, hovered || *open, pressed, theme);
            let border_color = if *open { theme.accent } else { theme.accent_secondary };
            let bg = [theme.glass_bg[0] + 0.04, theme.glass_bg[1] + 0.06, theme.glass_bg[2] + 0.09, 0.88];
            frame.instances.push(custom_glass_instance(bounds, clip, bg, border_color, 6.0, theme.border_width, intensity));

            let label_text = if selected_text.is_empty() { label.clone() } else { format!("{}: {}", label, selected_text) };
            let text_bounds = [bounds[0] + 12.0, bounds[1], bounds[2] - 32.0, bounds[3]];
            frame.texts.push(text_spec(label_text, text_bounds, clip, theme, theme.text_color, TextAlign::Left, TextRole::Small));

            let arrow = if *open { "▲" } else { "▼" };
            let arrow_bounds = [bounds[0] + bounds[2] - 22.0, bounds[1], 16.0, bounds[3]];
            frame.texts.push(text_spec(arrow.to_string(), arrow_bounds, clip, theme, theme.accent, TextAlign::Center, TextRole::Caption));
        }

        WidgetKind::Toast { title, message, kind, .. } => {
            let (accent_color, glow_mult) = match kind {
                crate::kind::ToastKind::Info => ([0.0, 0.85, 1.0, 1.0], 1.0),
                crate::kind::ToastKind::Success => ([0.15, 0.92, 0.45, 1.0], 1.3),
                crate::kind::ToastKind::Warning => ([1.0, 0.78, 0.12, 1.0], 1.3),
                crate::kind::ToastKind::Error => ([1.0, 0.28, 0.32, 1.0], 1.5),
            };
            // High-contrast, crystal clear solid cyber-glass toast card
            let bg = [0.04, 0.07, 0.13, 0.98];
            frame.instances.push(custom_glass_instance(bounds, clip, bg, accent_color, 8.0, 1.5, theme.glow_intensity * glow_mult));

            // Left vertical indicator bar
            let bar_w = 4.0;
            let bar_bounds = [bounds[0] + 10.0, bounds[1] + 10.0, bar_w, (bounds[3] - 20.0).max(4.0)];
            frame.instances.push(custom_glass_instance(bar_bounds, clip, accent_color, accent_color, 2.0, 0.0, 0.4));

            // Glowing indicator dot
            let dot_size = 8.0;
            let dot_bounds = [bounds[0] + 20.0, bounds[1] + 14.0, dot_size, dot_size];
            frame.instances.push(custom_glass_instance(dot_bounds, clip, accent_color, accent_color, 4.0, 0.0, 0.6));

            // High-contrast crisp title
            let title_bounds = [bounds[0] + 34.0, bounds[1] + 8.0, bounds[2] - 44.0, 20.0];
            frame.texts.push(text_spec(title.clone(), title_bounds, clip, theme, [1.0, 1.0, 1.0, 1.0], TextAlign::Left, TextRole::Body));

            // High-contrast legible message
            let msg_bounds = [bounds[0] + 34.0, bounds[1] + 28.0, bounds[2] - 44.0, (bounds[3] - 32.0).max(16.0)];
            frame.texts.push(text_spec(message.clone(), msg_bounds, clip, theme, [0.86, 0.93, 1.0, 1.0], TextAlign::Left, TextRole::Small));
        }

        WidgetKind::Tooltip { text } => {
            let bg = [0.04, 0.06, 0.10, 0.96];
            frame.instances.push(glass_instance(bounds, clip, bg, theme.accent, 0.25, theme));
            frame.texts.push(text_spec(text.clone(), bounds, clip, theme, theme.text_color, TextAlign::Center, TextRole::Caption));
        }

        WidgetKind::Splitter { orientation, .. } => {
            let base_color = if hovered || pressed {
                [theme.accent[0] * 0.45, theme.accent[1] * 0.45, theme.accent[2] * 0.45, 0.90]
            } else {
                [theme.accent_secondary[0] * 0.35, theme.accent_secondary[1] * 0.35, theme.accent_secondary[2] * 0.35, 0.45]
            };
            let intensity = if hovered || pressed { 0.35 } else { 0.0 };
            frame.instances.push(custom_glass_instance(bounds, clip, base_color, theme.accent, 2.0, 0.0, intensity));

            // Centered tactile grip handle
            match orientation {
                crate::kind::SplitOrientation::Horizontal => {
                    let grip_w = 2.0;
                    let grip_h = 24.0;
                    let grip_bounds = [bounds[0] + (bounds[2] - grip_w) * 0.5, bounds[1] + (bounds[3] - grip_h) * 0.5, grip_w, grip_h];
                    let grip_color = if hovered || pressed { [1.0, 1.0, 1.0, 1.0] } else { theme.accent };
                    frame.instances.push(custom_glass_instance(grip_bounds, clip, grip_color, theme.accent, 1.0, 0.0, if hovered || pressed { 0.4 } else { 0.0 }));
                }
                crate::kind::SplitOrientation::Vertical => {
                    let grip_w = 24.0;
                    let grip_h = 2.0;
                    let grip_bounds = [bounds[0] + (bounds[2] - grip_w) * 0.5, bounds[1] + (bounds[3] - grip_h) * 0.5, grip_w, grip_h];
                    let grip_color = if hovered || pressed { [1.0, 1.0, 1.0, 1.0] } else { theme.accent };
                    frame.instances.push(custom_glass_instance(grip_bounds, clip, grip_color, theme.accent, 1.0, 0.0, if hovered || pressed { 0.4 } else { 0.0 }));
                }
            }
        }

        WidgetKind::TreeNode { label, depth, is_dir, expanded, selected, .. } => {
            let base = if *selected { theme.glow_intensity } else { 0.0 };
            let intensity = interactive_glow(base, hovered, pressed, theme);
            if *selected || hovered {
                let bg = if *selected {
                    [theme.accent[0] * 0.24, theme.accent[1] * 0.24, theme.accent[2] * 0.24, 0.90]
                } else {
                    [theme.accent[0] * 0.12, theme.accent[1] * 0.12, theme.accent[2] * 0.12, 0.60]
                };
                frame.instances.push(custom_glass_instance(bounds, clip, bg, theme.accent, 4.0, if *selected { 1.0 } else { 0.0 }, intensity));
            }

            let indent_offset = (*depth as f32) * 16.0;

            if *is_dir {
                let arrow_str = if *expanded { "▼" } else { "▶" };
                let arrow_bounds = [bounds[0] + 6.0 + indent_offset, bounds[1], 14.0, bounds[3]];
                frame.texts.push(text_spec(arrow_str.to_string(), arrow_bounds, clip, theme, theme.accent, TextAlign::Center, TextRole::Caption));
            } else {
                let dot_bounds = [bounds[0] + 11.0 + indent_offset, bounds[1] + (bounds[3] - 4.0) * 0.5, 4.0, 4.0];
                frame.instances.push(custom_glass_instance(dot_bounds, clip, theme.accent_secondary, theme.accent_secondary, 2.0, 0.0, 0.0));
            }

            let text_color = if *selected {
                [1.0, 1.0, 1.0, 1.0]
            } else if hovered {
                theme.text_color
            } else {
                [0.85, 0.90, 0.96, 1.0]
            };

            let label_x = bounds[0] + 24.0 + indent_offset;
            let label_w = (bounds[2] - 28.0 - indent_offset).max(10.0);
            let label_bounds = [label_x, bounds[1], label_w, bounds[3]];
            frame.texts.push(text_spec(label.clone(), label_bounds, clip, theme, text_color, TextAlign::Left, TextRole::Small));
        }

        WidgetKind::ListItem { text, selected, badge, .. } => {
            let base = if *selected { theme.glow_intensity } else { 0.0 };
            let intensity = interactive_glow(base, hovered, pressed, theme);
            let bg = if *selected {
                [theme.glass_bg[0] + 0.04, theme.glass_bg[1] + 0.07, theme.glass_bg[2] + 0.11, 0.85]
            } else {
                [theme.glass_bg[0] * 0.60, theme.glass_bg[1] * 0.60, theme.glass_bg[2] * 0.60, 0.45]
            };
            frame.instances.push(glass_instance(bounds, clip, bg, theme.accent, intensity, theme));

            let text_color = if *selected { theme.text_color } else { theme.text_muted };
            let left_text_bounds = [bounds[0] + 12.0, bounds[1], bounds[2] - 90.0, bounds[3]];
            frame.texts.push(text_spec(text.clone(), left_text_bounds, clip, theme, text_color, TextAlign::Left, TextRole::Small));

            match badge {
                crate::kind::ListItemBadge::Success => {
                    let badge_size = 14.0;
                    let badge_bounds = [bounds[0] + bounds[2] - badge_size - 14.0, bounds[1] + (bounds[3] - badge_size) * 0.5, badge_size, badge_size];
                    frame.instances.push(glass_instance(badge_bounds, clip, theme.success, theme.success, 0.12, theme));
                }
                crate::kind::ListItemBadge::Warning => {
                    let badge_size = 14.0;
                    let badge_bounds = [bounds[0] + bounds[2] - badge_size - 14.0, bounds[1] + (bounds[3] - badge_size) * 0.5, badge_size, badge_size];
                    frame.instances.push(glass_instance(badge_bounds, clip, theme.warning, theme.warning, 0.12, theme));
                }
                crate::kind::ListItemBadge::Active(label) => {
                    let badge_w = 68.0;
                    let badge_h = 20.0;
                    let badge_bounds = [bounds[0] + bounds[2] - badge_w - 12.0, bounds[1] + (bounds[3] - badge_h) * 0.5, badge_w, badge_h];
                    let badge_bg = [theme.accent[0] * 0.12, theme.accent[1] * 0.12, theme.accent[2] * 0.12, 0.7];
                    frame.instances.push(glass_instance(badge_bounds, clip, badge_bg, theme.accent, 0.18, theme));
                    frame.texts.push(text_spec(format!("~ {label}"), badge_bounds, clip, theme, theme.accent, TextAlign::Center, TextRole::Caption));
                }
                crate::kind::ListItemBadge::Custom { text: badge_text, color } => {
                    let badge_w = 60.0;
                    let badge_h = 20.0;
                    let badge_bounds = [bounds[0] + bounds[2] - badge_w - 12.0, bounds[1] + (bounds[3] - badge_h) * 0.5, badge_w, badge_h];
                    let badge_bg = [color[0] * 0.12, color[1] * 0.12, color[2] * 0.12, 0.7];
                    frame.instances.push(glass_instance(badge_bounds, clip, badge_bg, *color, 0.15, theme));
                    frame.texts.push(text_spec(badge_text.clone(), badge_bounds, clip, theme, *color, TextAlign::Center, TextRole::Caption));
                }
                crate::kind::ListItemBadge::None => {}
            }
        }

        WidgetKind::BreadcrumbItem { label, is_last, .. } => {
            if hovered && !*is_last {
                let bg = [theme.accent[0] * 0.15, theme.accent[1] * 0.15, theme.accent[2] * 0.15, 0.60];
                frame.instances.push(custom_glass_instance(bounds, clip, bg, [0.0, 0.0, 0.0, 0.0], 4.0, 0.0, 0.0));
            }

            let text_color = if *is_last {
                [1.0, 1.0, 1.0, 1.0]
            } else if hovered {
                theme.accent
            } else {
                theme.text_muted
            };

            let text_w = if *is_last { bounds[2] } else { bounds[2] - 14.0 };
            let text_bounds = [bounds[0], bounds[1], text_w, bounds[3]];
            let role = if *is_last { TextRole::Body } else { TextRole::Small };
            frame.texts.push(text_spec(label.clone(), text_bounds, clip, theme, text_color, TextAlign::Left, role));

            if !*is_last {
                let sep_bounds = [bounds[0] + bounds[2] - 12.0, bounds[1], 10.0, bounds[3]];
                frame.texts.push(text_spec("›".to_string(), sep_bounds, clip, theme, theme.text_muted, TextAlign::Center, TextRole::Caption));
            }
        }

        WidgetKind::PaginationItem { label, active, disabled, .. } => {
            let intensity = if *active {
                0.35
            } else if hovered && !*disabled {
                0.20
            } else {
                0.0
            };

            let bg = if *active {
                [theme.accent[0] * 0.35, theme.accent[1] * 0.35, theme.accent[2] * 0.35, 0.95]
            } else if *disabled {
                [theme.glass_bg[0] * 0.40, theme.glass_bg[1] * 0.40, theme.glass_bg[2] * 0.40, 0.30]
            } else if hovered {
                [theme.accent[0] * 0.20, theme.accent[1] * 0.20, theme.accent[2] * 0.20, 0.70]
            } else {
                [theme.glass_bg[0] * 0.60, theme.glass_bg[1] * 0.60, theme.glass_bg[2] * 0.60, 0.50]
            };

            let border_color = if *active {
                theme.accent
            } else if hovered && !*disabled {
                theme.accent_secondary
            } else {
                [0.0, 0.0, 0.0, 0.0]
            };

            frame.instances.push(custom_glass_instance(bounds, clip, bg, border_color, 4.0, if *active || hovered { 1.0 } else { 0.0 }, intensity));

            let text_color = if *active {
                [1.0, 1.0, 1.0, 1.0]
            } else if *disabled {
                [theme.text_muted[0], theme.text_muted[1], theme.text_muted[2], 0.35]
            } else if hovered {
                theme.accent
            } else {
                theme.text_color
            };

            frame.texts.push(text_spec(label.clone(), bounds, clip, theme, text_color, TextAlign::Center, TextRole::Small));
        }

        WidgetKind::Badge { label, badge } => {
            let (accent_color, bg_tint) = match badge {
                crate::kind::ListItemBadge::Success => (theme.success, [theme.success[0] * 0.15, theme.success[1] * 0.15, theme.success[2] * 0.15, 0.85]),
                crate::kind::ListItemBadge::Warning => (theme.warning, [theme.warning[0] * 0.15, theme.warning[1] * 0.15, theme.warning[2] * 0.15, 0.85]),
                crate::kind::ListItemBadge::Active(_) => (theme.accent, [theme.accent[0] * 0.15, theme.accent[1] * 0.15, theme.accent[2] * 0.15, 0.85]),
                crate::kind::ListItemBadge::Custom { color, .. } => (*color, [color[0] * 0.15, color[1] * 0.15, color[2] * 0.15, 0.85]),
                crate::kind::ListItemBadge::None => (theme.accent_secondary, [theme.glass_bg[0] * 0.6, theme.glass_bg[1] * 0.6, theme.glass_bg[2] * 0.6, 0.6]),
            };

            frame.instances.push(custom_glass_instance(bounds, clip, bg_tint, accent_color, 4.0, 1.0, 0.15));
            frame.texts.push(text_spec(label.clone(), bounds, clip, theme, accent_color, TextAlign::Center, TextRole::Caption));
        }

        WidgetKind::ColorSwatch { color, label, .. } => {
            let intensity = if hovered || pressed { 0.35 } else { 0.0 };
            let border_color = if hovered || pressed { theme.accent } else { theme.accent_secondary };

            let (box_bounds, text_bounds) = if let Some(_) = label {
                let box_h = (bounds[3] - 18.0).max(12.0);
                ([bounds[0], bounds[1], bounds[2], box_h], Some([bounds[0], bounds[1] + box_h + 2.0, bounds[2], 14.0]))
            } else {
                (bounds, None)
            };

            // Outer glass frame
            frame.instances.push(custom_glass_instance(box_bounds, clip, *color, border_color, 6.0, 1.0, intensity));

            if let (Some(lbl), Some(tb)) = (label, text_bounds) {
                frame.texts.push(text_spec(lbl.clone(), tb, clip, theme, theme.text_muted, TextAlign::Center, TextRole::Caption));
            }
        }

        WidgetKind::ColorPicker { color, space, .. } => {
            let col = crate::color::Color::from_array(*color);
            let (cur_h, cur_s, cur_v) = col.to_hsv();
            let intensity = if hovered || pressed { 0.25 } else { 0.0 };
            let pad = 10.0;

            // 1. Outer container glass card
            let bg = [theme.glass_bg[0] + 0.02, theme.glass_bg[1] + 0.03, theme.glass_bg[2] + 0.06, 0.94];
            frame.instances.push(custom_glass_instance(bounds, clip, bg, theme.accent_secondary, 12.0, 1.2, intensity));

            // 2. Top Area: [Preview Swatch] (Left) + [2D SV Canvas] (Right)
            let top_h = (bounds[3] * 0.42).clamp(70.0, 96.0);
            let swatch_w = 110.0;
            let swatch_bounds = [bounds[0] + pad, bounds[1] + pad, swatch_w, top_h];
            frame.instances.push(custom_glass_instance(swatch_bounds, clip, *color, [1.0, 1.0, 1.0, 0.4], 6.0, 1.0, 0.2));

            // Right 2D Saturation / Value Gradient Canvas
            let sv_x = swatch_bounds[0] + swatch_w + 8.0;
            let sv_y = bounds[1] + pad;
            let sv_w = (bounds[0] + bounds[2] - pad - sv_x).max(10.0);
            let sv_h = top_h;
            let sv_bounds = [sv_x, sv_y, sv_w, sv_h];

            // Render 16x8 bilinear 2D color cells for smooth SV field
            let cols = 16;
            let rows = 8;
            let cell_w = sv_w / cols as f32;
            let cell_h = sv_h / rows as f32;
            for r in 0..rows {
                let v = 1.0 - (r as f32 / rows as f32);
                for c in 0..cols {
                    let s = c as f32 / cols as f32;
                    let cell_col = crate::color::Color::from_hsv(cur_h, s, v, 1.0).to_array();
                    let cx = sv_x + c as f32 * cell_w;
                    let cy = sv_y + r as f32 * cell_h;
                    let cb = [cx, cy, cell_w + 0.5, cell_h + 0.5];
                    frame.instances.push(custom_glass_instance(cb, clip, cell_col, [0.0, 0.0, 0.0, 0.0], 0.0, 0.0, 0.0));
                }
            }
            // Canvas border overlay
            frame.instances.push(custom_glass_instance(sv_bounds, clip, [0.0, 0.0, 0.0, 0.0], [0.35, 0.45, 0.55, 0.7], 6.0, 1.0, 0.0));

            // Reticle circle on 2D SV Canvas
            let reticle_r = 7.0;
            let rx = (sv_x + cur_s * sv_w - reticle_r).clamp(sv_x, sv_x + sv_w - reticle_r * 2.0);
            let ry = (sv_y + (1.0 - cur_v) * sv_h - reticle_r).clamp(sv_y, sv_y + sv_h - reticle_r * 2.0);
            let reticle_bounds = [rx, ry, reticle_r * 2.0, reticle_r * 2.0];
            frame.instances.push(custom_glass_instance(reticle_bounds, clip, *color, [1.0, 1.0, 1.0, 1.0], reticle_r, 2.0, 0.4));

            // 3. Middle Area: [Rainbow Hue Slider Bar]
            let hue_y = sv_y + sv_h + 8.0;
            let hue_h = 12.0;
            let hue_x = bounds[0] + pad;
            let hue_w = bounds[2] - 2.0 * pad;
            let hue_bounds = [hue_x, hue_y, hue_w, hue_h];

            let seg_count = 24;
            for i in 0..seg_count {
                let h = (i as f32 / seg_count as f32) * 360.0;
                let seg_color = crate::color::Color::from_hsv(h, 1.0, 1.0, 1.0).to_array();
                let sx = hue_x + (i as f32 * hue_w / seg_count as f32);
                let sw = (hue_w / seg_count as f32) + 0.5;
                let sb = [sx, hue_y, sw, hue_h];
                frame.instances.push(custom_glass_instance(sb, clip, seg_color, [0.0, 0.0, 0.0, 0.0], 0.0, 0.0, 0.0));
            }
            frame.instances.push(custom_glass_instance(hue_bounds, clip, [0.0, 0.0, 0.0, 0.0], [0.35, 0.45, 0.55, 0.6], 6.0, 1.0, 0.0));

            // Hue Thumb Indicator
            let hue_ratio = (cur_h / 360.0).clamp(0.0, 1.0);
            let thumb_r = 8.0;
            let thumb_x = (hue_x + hue_ratio * hue_w - thumb_r).clamp(hue_x, hue_x + hue_w - thumb_r * 2.0);
            let thumb_y = hue_y - 2.0;
            let thumb_bounds = [thumb_x, thumb_y, thumb_r * 2.0, thumb_r * 2.0];
            let pure_hue = crate::color::Color::from_hsv(cur_h, 1.0, 1.0, 1.0).to_array();
            frame.instances.push(custom_glass_instance(thumb_bounds, clip, pure_hue, [1.0, 1.0, 1.0, 1.0], thumb_r, 2.0, 0.5));

            // 4. Middle-Bottom Area: [HEX Box with Copy Indicator]
            let hex_y = hue_y + hue_h + 8.0;
            let hex_h = 28.0;
            let hex_x = bounds[0] + pad;
            let hex_w = bounds[2] - 2.0 * pad;
            let hex_bounds = [hex_x, hex_y, hex_w, hex_h];
            let hex_bg = [theme.glass_bg[0] * 0.7, theme.glass_bg[1] * 0.7, theme.glass_bg[2] * 0.7, 0.6];
            frame.instances.push(custom_glass_instance(hex_bounds, clip, hex_bg, theme.accent_secondary, 6.0, 1.0, 0.05));

            let hex_text = format!("HEX   {}", col.to_hex_with_alpha());
            let hex_text_bounds = [hex_x + 12.0, hex_y, hex_w - 40.0, hex_h];
            frame.texts.push(TextSpec {
                text: hex_text,
                bounds: hex_text_bounds,
                font_size: 13.5,
                color: [0.95, 0.98, 1.0, 1.0],
                align: TextAlign::Left,
                weight: FontWeight::Bold,
                clip,
            });
            let copy_bounds = [hex_x + hex_w - 28.0, hex_y, 22.0, hex_h];
            frame.texts.push(text_spec("⎘".to_string(), copy_bounds, clip, theme, theme.accent, TextAlign::Center, TextRole::Body));

            // 5. Bottom Area: [4 Multi-space Mini-Cards] (RGB, CMYK, HSV, LAB)
            let cards_y = hex_y + hex_h + 6.0;
            let cards_h = (bounds[1] + bounds[3] - pad - cards_y).max(28.0);
            let card_gap = 6.0;
            let card_w = (bounds[2] - 2.0 * pad - 3.0 * card_gap) / 4.0;

            let (r, g, b) = col.to_rgb_u8();
            let (c, m, y, k) = col.to_cmyk();
            let (l, a_val, b_val) = col.to_lab();

            let metrics = [
                ("RGB", format!("{}, {}, {}", r, g, b), *space == crate::color::ColorSpace::Rgb),
                ("CMYK", format!("{:.0}%, {:.0}%, {:.0}%, {:.0}%", c, m, y, k), *space == crate::color::ColorSpace::Cmyk),
                ("HSV", format!("{:.0}°, {:.0}%, {:.0}%", cur_h, cur_s * 100.0, cur_v * 100.0), false),
                ("LAB", format!("{:.0}, {:+.0}, {:+.0}", l, a_val, b_val), *space == crate::color::ColorSpace::Lab),
            ];

            for (idx, (m_title, m_val, is_sel)) in metrics.iter().enumerate() {
                let mx = bounds[0] + pad + idx as f32 * (card_w + card_gap);
                let mb = [mx, cards_y, card_w, cards_h];
                let m_bg = if *is_sel {
                    [theme.accent[0] * 0.25, theme.accent[1] * 0.25, theme.accent[2] * 0.25, 0.9]
                } else {
                    [theme.glass_bg[0] * 0.5, theme.glass_bg[1] * 0.5, theme.glass_bg[2] * 0.5, 0.4]
                };
                let m_border = if *is_sel { theme.accent } else { [theme.accent_secondary[0] * 0.25, theme.accent_secondary[1] * 0.25, theme.accent_secondary[2] * 0.25, 0.35] };
                frame.instances.push(custom_glass_instance(mb, clip, m_bg, m_border, 5.0, 1.0, if *is_sel { 0.15 } else { 0.0 }));

                let t_box = [mx + 4.0, cards_y + 2.0, card_w - 8.0, 12.0];
                frame.texts.push(text_spec(m_title.to_string(), t_box, clip, theme, if *is_sel { theme.accent } else { theme.text_muted }, TextAlign::Center, TextRole::Caption));

                let v_box = [mx + 2.0, cards_y + 14.0, card_w - 4.0, 14.0];
                frame.texts.push(TextSpec {
                    text: m_val.clone(),
                    bounds: v_box,
                    font_size: 10.5,
                    color: [0.92, 0.95, 1.0, 1.0],
                    align: TextAlign::Center,
                    weight: FontWeight::Normal,
                    clip,
                });
            }
        }

        WidgetKind::Media { kind, resource_id, .. } => {
            frame.media.push(MediaSpec { kind: *kind, bounds, resource_id: resource_id.clone() });
        }
    }
}

/// Precise proportional width of a single character calibrated to Sans-Serif (Segoe UI / Roboto / Inter).
pub fn estimate_char_width(ch: char, font_size: f32) -> f32 {
    let ratio = match ch {
        ' ' => 0.26,
        'i' | 'j' | 'l' | '!' | '|' | ':' | ';' | '\'' | '`' | ',' | '.' => 0.23,
        'f' | 't' | 'I' | 'r' | '(' | ')' | '[' | ']' | '{' | '}' | '-' => 0.31,
        '/' | '\\' | '*' | '?' | '"' | '^' => 0.38,
        's' | 'z' | 'c' | 'k' | 'J' => 0.44,
        'e' | 'x' | 'v' | 'y' => 0.47,
        'a' | 'b' | 'd' | 'g' | 'h' | 'n' | 'o' | 'p' | 'q' | 'u' | '0'..='9' | 'é' | 'è' | 'ê' | 'ë' | 'à' | 'â' | 'î' | 'ï' | 'ô' | 'ù' | 'û' | 'ç' => 0.50,
        'w' => 0.69,
        'm' => 0.77,
        'M' | 'W' | '@' | '%' | '&' | '#' | '_' | '~' | '+' | '=' | '<' | '>' => 0.75,
        'A'..='Z' => 0.59,
        _ => 0.50,
    };
    ratio * font_size
}

/// Accurate proportional text width estimation for caret positioning.
pub fn estimate_text_width(text: &str, font_size: f32) -> f32 {
    text.chars().map(|ch| estimate_char_width(ch, font_size)).sum()
}

/// Finds the character boundary in `line` whose accumulated text width is closest to `target_x`.
pub fn find_cursor_index_in_line(line: &str, target_x: f32, font_size: f32) -> usize {
    if line.is_empty() || target_x <= 0.0 {
        return 0;
    }
    let mut current_x = 0.0;
    let mut last_idx = 0;

    for (idx, ch) in line.char_indices() {
        let ch_w = estimate_char_width(ch, font_size);
        if target_x < current_x + ch_w * 0.5 {
            return idx;
        }
        current_x += ch_w;
        last_idx = idx + ch.len_utf8();
    }
    last_idx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estimate_text_width_is_proportional_to_character_classes() {
        let narrow = estimate_text_width("iiii", 15.0);
        let wide = estimate_text_width("wwww", 15.0);
        assert!(narrow < wide);
        assert_eq!(narrow, 0.23 * 15.0 * 4.0);
        assert_eq!(wide, 0.69 * 15.0 * 4.0);
    }

    const TRACK: [f32; 4] = [0.0, 0.0, 10.0, 100.0];

    #[test]
    fn thumb_size_is_proportional_to_visible_content_ratio() {
        // 100/400 = 25% of visible content -> thumb is 25% of track height.
        let thumb = scrollbar_thumb_bounds(TRACK, 400.0, 100.0, 0.0);
        assert_eq!(thumb, [0.0, 0.0, 10.0, 25.0]);
    }

    #[test]
    fn thumb_moves_to_the_bottom_of_the_track_at_max_scroll() {
        // max_offset = 400-100 = 300 ; offset=300 -> bottom of track
        let thumb = scrollbar_thumb_bounds(TRACK, 400.0, 100.0, 300.0);
        assert_eq!(thumb, [0.0, 75.0, 10.0, 25.0]);
    }

    #[test]
    fn thumb_height_never_shrinks_below_the_minimum() {
        let thumb = scrollbar_thumb_bounds(TRACK, 100_000.0, 100.0, 0.0);
        assert_eq!(thumb[3], MIN_SCROLLBAR_THUMB);
    }

    #[test]
    fn thumb_fills_the_whole_track_when_content_fits_entirely() {
        let thumb = scrollbar_thumb_bounds(TRACK, 80.0, 100.0, 0.0);
        assert_eq!(thumb, TRACK);
    }
}
