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
    /// Non-SDF media content specs (see [`MediaSpec`]).
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
    interaction
        .measure
        .unwrap_or(&crate::text_measure::DefaultTextMeasure)
}

impl WidgetTree {
    /// Builds the GPU frame (SDF instances + text specs) for the tree rooted at `root`,
    /// evaluated in parent-to-child order.
    pub fn build_frame(
        &self,
        root: NodeId,
        theme: &Theme,
        interaction: InteractionState,
    ) -> Result<Frame, LayoutError> {
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
            render_kind(
                kind,
                node_effective.visual,
                node_effective.clip,
                theme,
                hovered,
                pressed,
                measure,
                frame,
            );
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
    [
        bounds[0] + OFFSET,
        bounds[1] + OFFSET,
        (bounds[2] - OFFSET * 2.0).max(0.0),
        (bounds[3] - OFFSET * 2.0).max(0.0),
    ]
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

fn glass_instance(
    bounds: [f32; 4],
    clip: [f32; 4],
    bg: [f32; 4],
    glow: [f32; 4],
    glow_intensity: f32,
    theme: &Theme,
) -> GpuSdfInstance {
    // INV-GPU-3: corner radius must never exceed half the smaller side.
    let radius = theme
        .corner_radius
        .min(bounds[2] * 0.5)
        .min(bounds[3] * 0.5);
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
    TextSpec {
        text,
        bounds,
        font_size,
        color,
        align,
        weight,
        clip,
    }
}

/// Minimum scrollbar thumb height in pixels.
const MIN_SCROLLBAR_THUMB: f32 = 16.0;

/// Proportional scrollbar thumb geometry computation.
fn scrollbar_thumb_bounds(
    track: [f32; 4],
    content_size: f32,
    viewport_size: f32,
    offset: f32,
) -> [f32; 4] {
    let track_height = track[3];
    let visible_ratio = if content_size > 0.0 {
        (viewport_size / content_size).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let thumb_height = (track_height * visible_ratio)
        .max(MIN_SCROLLBAR_THUMB)
        .min(track_height);

    let max_offset = (content_size - viewport_size).max(0.0);
    let scroll_ratio = if max_offset > 0.0 {
        (offset / max_offset).clamp(0.0, 1.0)
    } else {
        0.0
    };
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

        WidgetKind::Card { bg, border, radius } => {
            let bg_col = bg.unwrap_or([
                (theme.glass_bg[0] * 1.5).min(1.0),
                (theme.glass_bg[1] * 1.4).min(1.0),
                (theme.glass_bg[2] * 1.4).min(1.0),
                theme.glass_bg[3].min(0.95),
            ]);
            let border_col = border.unwrap_or([
                (theme.glass_bg[0] * 3.0).min(1.0),
                (theme.glass_bg[1] * 2.7).min(1.0),
                (theme.glass_bg[2] * 2.3).min(1.0),
                0.60,
            ]);
            let r = radius.unwrap_or(theme.corner_radius.min(8.0));

            // Soft tactile drop shadow under the card (matching screen3.png)
            let shadow_color = [0.0, 0.0, 0.0, 0.50];
            let shadow_glow = [0.0, 0.0, 0.0, 0.65];
            let shadow_bounds = [bounds[0], bounds[1] + 4.0, bounds[2], bounds[3]];
            frame.instances.push(custom_glass_instance(
                shadow_bounds,
                clip,
                shadow_color,
                shadow_glow,
                r + 1.0,
                0.0,
                0.40,
            ));

            frame.instances.push(custom_glass_instance(
                bounds,
                clip,
                bg_col,
                border_col,
                r,
                theme.border_width,
                0.01,
            ));
        }

        WidgetKind::Panel { bg, border } => {
            let bg_col = bg.unwrap_or(theme.glass_bg);
            let border_col = border.unwrap_or([
                theme.glass_bg[0] * 2.5,
                theme.glass_bg[1] * 2.2,
                theme.glass_bg[2] * 1.9,
                0.50,
            ]);
            let r = theme.corner_radius.min(8.0);

            // Soft drop shadow under the panel
            let shadow_color = [0.0, 0.0, 0.0, 0.40];
            let shadow_glow = [0.0, 0.0, 0.0, 0.55];
            let shadow_bounds = [bounds[0], bounds[1] + 3.0, bounds[2], bounds[3]];
            frame.instances.push(custom_glass_instance(
                shadow_bounds,
                clip,
                shadow_color,
                shadow_glow,
                r + 1.0,
                0.0,
                0.35,
            ));

            frame.instances.push(custom_glass_instance(
                bounds,
                clip,
                bg_col,
                border_col,
                r,
                theme.border_width,
                0.005,
            ));
        }

        WidgetKind::Label { text, muted } => {
            let (color, role) = if *muted {
                (theme.text_muted, TextRole::Caption)
            } else {
                (theme.text_color, TextRole::Body)
            };
            frame.texts.push(text_spec(
                text.clone(),
                inset(bounds, 4.0, 0.0),
                clip,
                theme,
                color,
                TextAlign::Left,
                role,
            ));
        }

        WidgetKind::Button { label, enabled, variant, .. } => {
            let (bg, border_color, glow_mult, text_color, radius) = match variant {
                crate::kind::ButtonVariant::Primary => {
                    if hovered && *enabled {
                        ([theme.accent[0] * 0.95, theme.accent[1] * 0.95, theme.accent[2] * 0.95, 1.0], [1.0, 1.0, 1.0, 1.0], 0.35, [0.06, 0.08, 0.13, 1.0], 6.0)
                    } else if *enabled {
                        (theme.accent, [theme.accent[0] * 1.1, theme.accent[1] * 1.1, theme.accent[2] * 1.1, 1.0], 0.20, [0.06, 0.08, 0.13, 1.0], 6.0)
                    } else {
                        ([theme.accent[0] * 0.25, theme.accent[1] * 0.25, theme.accent[2] * 0.25, 0.4], [0.2, 0.2, 0.25, 0.3], 0.0, theme.text_muted, 6.0)
                    }
                }
                crate::kind::ButtonVariant::Secondary => {
                    if hovered && *enabled {
                        ([theme.accent_secondary[0] * 0.25, theme.accent_secondary[1] * 0.25, theme.accent_secondary[2] * 0.25, 0.95], theme.accent_secondary, 0.30, [1.0, 1.0, 1.0, 1.0], 6.0)
                    } else if *enabled {
                        ([theme.accent_secondary[0] * 0.16, theme.accent_secondary[1] * 0.16, theme.accent_secondary[2] * 0.16, 0.85], [theme.accent_secondary[0] * 0.6, theme.accent_secondary[1] * 0.6, theme.accent_secondary[2] * 0.6, 0.7], 0.10, theme.accent_secondary, 6.0)
                    } else {
                        ([0.06, 0.09, 0.14, 0.40], [0.12, 0.16, 0.22, 0.3], 0.0, theme.text_muted, 6.0)
                    }
                }
                crate::kind::ButtonVariant::Danger => {
                    if hovered && *enabled {
                        ([theme.danger[0] * 0.35, theme.danger[1] * 0.35, theme.danger[2] * 0.35, 0.95], theme.danger, 0.35, [1.0, 1.0, 1.0, 1.0], 6.0)
                    } else if *enabled {
                        ([theme.danger[0] * 0.20, theme.danger[1] * 0.20, theme.danger[2] * 0.20, 0.85], [theme.danger[0] * 0.7, theme.danger[1] * 0.7, theme.danger[2] * 0.7, 0.7], 0.12, theme.danger, 6.0)
                    } else {
                        ([0.06, 0.09, 0.14, 0.40], [0.12, 0.16, 0.22, 0.3], 0.0, theme.text_muted, 6.0)
                    }
                }
                crate::kind::ButtonVariant::Ghost => {
                    if hovered && *enabled {
                        ([0.14, 0.18, 0.28, 0.70], [0.25, 0.32, 0.45, 0.60], 0.10, [1.0, 1.0, 1.0, 1.0], 6.0)
                    } else if *enabled {
                        ([0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0], 0.0, theme.text_muted, 6.0)
                    } else {
                        ([0.0, 0.0, 0.0, 0.0], [0.0, 0.0, 0.0, 0.0], 0.0, [0.35, 0.40, 0.50, 0.5], 6.0)
                    }
                }
                crate::kind::ButtonVariant::Default => {
                    if hovered && *enabled {
                        // High-contrast midnight navy solid fill with luminous cyan/periwinkle outline
                        ([0.14, 0.18, 0.28, 0.96], [0.48, 0.82, 1.0, 0.85], 0.20, [1.0, 1.0, 1.0, 1.0], 6.0)
                    } else if *enabled {
                        // Solid distinct midnight navy container with crisp subtle structural border
                        ([0.08, 0.11, 0.18, 0.92], [0.18, 0.24, 0.36, 0.65], 0.04, theme.text_color, 6.0)
                    } else {
                        ([0.05, 0.07, 0.11, 0.40], [0.10, 0.13, 0.20, 0.3], 0.0, [0.35, 0.45, 0.60, 0.6], 6.0)
                    }
                }
            };

            let intensity = if *enabled {
                if pressed {
                    (theme.glow_intensity_hover * 0.7).max(glow_mult)
                } else if hovered {
                    glow_mult.max(theme.glow_intensity_hover)
                } else {
                    glow_mult
                }
            } else {
                0.0
            };

            frame.instances.push(custom_glass_instance(
                bounds,
                clip,
                bg,
                border_color,
                radius,
                1.0,
                intensity,
            ));
            frame.texts.push(text_spec(
                label.clone(),
                bounds,
                clip,
                theme,
                text_color,
                TextAlign::Center,
                TextRole::Body,
            ));
        }

        WidgetKind::IconButton { icon, enabled, .. } => {
            let (bg, border, glow_mult, icon_color) = if hovered && *enabled {
                ([0.14, 0.18, 0.28, 0.96], [0.48, 0.82, 1.0, 0.85], 0.22, [1.0, 1.0, 1.0, 1.0])
            } else if *enabled {
                ([0.08, 0.11, 0.18, 0.92], [0.18, 0.24, 0.36, 0.65], 0.04, theme.text_color)
            } else {
                ([0.05, 0.07, 0.11, 0.40], [0.10, 0.13, 0.20, 0.3], 0.0, theme.text_muted)
            };
            let intensity = if *enabled {
                if pressed { 0.40 } else if hovered { glow_mult } else { 0.05 }
            } else {
                0.0
            };
            frame.instances.push(custom_glass_instance(
                bounds,
                clip,
                bg,
                border,
                6.0,
                1.0,
                intensity,
            ));
            let glyph = icon.glyph();
            frame.texts.push(text_spec(
                glyph.to_string(),
                bounds,
                clip,
                theme,
                icon_color,
                TextAlign::Center,
                TextRole::Body,
            ));
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

        WidgetKind::TableHeader {
            title, sorted_asc, ..
        } => {
            let bg = if hovered {
                [
                    theme.accent[0] * 0.18,
                    theme.accent[1] * 0.18,
                    theme.accent[2] * 0.18,
                    0.85,
                ]
            } else {
                [
                    theme.glass_bg[0] * 0.85,
                    theme.glass_bg[1] * 0.85,
                    theme.glass_bg[2] * 0.85,
                    0.75,
                ]
            };
            frame.instances.push(custom_glass_instance(
                bounds,
                clip,
                bg,
                theme.accent_secondary,
                0.0,
                0.5,
                if hovered { 0.15 } else { 0.0 },
            ));

            let sort_indicator = match sorted_asc {
                Some(true) => " ▴",
                Some(false) => " ▾",
                None => "",
            };
            let header_text = format!("{}{}", title, sort_indicator);
            let text_bounds = [bounds[0] + 8.0, bounds[1], bounds[2] - 16.0, bounds[3]];
            let text_color = if sorted_asc.is_some() {
                theme.accent
            } else if hovered {
                [1.0, 1.0, 1.0, 1.0]
            } else {
                theme.text_color
            };
            frame.texts.push(text_spec(
                header_text,
                text_bounds,
                clip,
                theme,
                text_color,
                TextAlign::Left,
                TextRole::Small,
            ));
        }

        WidgetKind::TableCell {
            text,
            badge,
            selected,
            row_index,
            ..
        } => {
            let is_even = row_index % 2 == 0;
            let bg = if *selected {
                [
                    theme.accent[0] * 0.22,
                    theme.accent[1] * 0.22,
                    theme.accent[2] * 0.22,
                    0.88,
                ]
            } else if hovered {
                [
                    theme.accent[0] * 0.10,
                    theme.accent[1] * 0.10,
                    theme.accent[2] * 0.10,
                    0.50,
                ]
            } else if is_even {
                [
                    theme.glass_bg[0] * 0.40,
                    theme.glass_bg[1] * 0.40,
                    theme.glass_bg[2] * 0.40,
                    0.35,
                ]
            } else {
                [0.0, 0.0, 0.0, 0.0]
            };
            if bg[3] > 0.0 {
                frame.instances.push(custom_glass_instance(
                    bounds,
                    clip,
                    bg,
                    theme.accent,
                    0.0,
                    if *selected { 0.5 } else { 0.0 },
                    if *selected { 0.2 } else { 0.0 },
                ));
            }

            let badge_reserve = match badge {
                crate::kind::ListItemBadge::Success | crate::kind::ListItemBadge::Warning => 22.0,
                crate::kind::ListItemBadge::Active(_)
                | crate::kind::ListItemBadge::Custom { .. } => 64.0,
                crate::kind::ListItemBadge::None => 16.0,
            };

            if !text.is_empty() {
                let text_color = if *selected {
                    [1.0, 1.0, 1.0, 1.0]
                } else if hovered {
                    theme.text_color
                } else {
                    [0.88, 0.92, 0.98, 1.0]
                };
                let text_w = (bounds[2] - badge_reserve).max(10.0);
                let text_bounds = [bounds[0] + 8.0, bounds[1], text_w, bounds[3]];
                frame.texts.push(text_spec(
                    text.clone(),
                    text_bounds,
                    clip,
                    theme,
                    text_color,
                    TextAlign::Left,
                    TextRole::Small,
                ));
            }

            match badge {
                crate::kind::ListItemBadge::Success => {
                    let dot_size = 8.0;
                    let dot_bounds = [
                        bounds[0] + bounds[2] - dot_size - 10.0,
                        bounds[1] + (bounds[3] - dot_size) * 0.5,
                        dot_size,
                        dot_size,
                    ];
                    frame.instances.push(glass_instance(
                        dot_bounds,
                        clip,
                        theme.success,
                        theme.success,
                        0.2,
                        theme,
                    ));
                }
                crate::kind::ListItemBadge::Warning => {
                    let dot_size = 8.0;
                    let dot_bounds = [
                        bounds[0] + bounds[2] - dot_size - 10.0,
                        bounds[1] + (bounds[3] - dot_size) * 0.5,
                        dot_size,
                        dot_size,
                    ];
                    frame.instances.push(glass_instance(
                        dot_bounds,
                        clip,
                        theme.warning,
                        theme.warning,
                        0.2,
                        theme,
                    ));
                }
                crate::kind::ListItemBadge::Active(lbl) => {
                    let badge_w = 54.0;
                    let badge_h = 18.0;
                    let badge_bounds = [
                        bounds[0] + bounds[2] - badge_w - 6.0,
                        bounds[1] + (bounds[3] - badge_h) * 0.5,
                        badge_w,
                        badge_h,
                    ];
                    frame.instances.push(glass_instance(
                        badge_bounds,
                        clip,
                        [
                            theme.accent[0] * 0.15,
                            theme.accent[1] * 0.15,
                            theme.accent[2] * 0.15,
                            0.8,
                        ],
                        theme.accent,
                        0.15,
                        theme,
                    ));
                    frame.texts.push(text_spec(
                        lbl.clone(),
                        badge_bounds,
                        clip,
                        theme,
                        theme.accent,
                        TextAlign::Center,
                        TextRole::Caption,
                    ));
                }
                _ => {}
            }
        }

        WidgetKind::AccordionHeader {
            title,
            subtitle,
            expanded,
            ..
        } => {
            let intensity = interactive_glow(
                if *expanded {
                    theme.glow_intensity * 0.6
                } else {
                    0.0
                },
                hovered,
                pressed,
                theme,
            );
            let border_color = if *expanded {
                theme.accent
            } else {
                theme.accent_secondary
            };
            let bg = if hovered {
                [
                    theme.glass_bg[0] + 0.05,
                    theme.glass_bg[1] + 0.07,
                    theme.glass_bg[2] + 0.11,
                    0.88,
                ]
            } else {
                [
                    theme.glass_bg[0] + 0.02,
                    theme.glass_bg[1] + 0.03,
                    theme.glass_bg[2] + 0.05,
                    0.75,
                ]
            };
            frame.instances.push(custom_glass_instance(
                bounds,
                clip,
                bg,
                border_color,
                8.0,
                theme.border_width,
                intensity,
            ));

            let arrow = if *expanded { "▼" } else { "▶" };
            let arrow_bounds = [bounds[0] + 12.0, bounds[1], 16.0, bounds[3]];
            frame.texts.push(text_spec(
                arrow.to_string(),
                arrow_bounds,
                clip,
                theme,
                theme.accent,
                TextAlign::Center,
                TextRole::Caption,
            ));

            if let Some(sub) = subtitle {
                let title_h = 18.0;
                let sub_h = 14.0;
                let total_text_h = title_h + sub_h + 2.0;
                let start_y = bounds[1] + (bounds[3] - total_text_h) * 0.5;

                let title_bounds = [bounds[0] + 34.0, start_y, bounds[2] - 46.0, title_h];
                frame.texts.push(text_spec(
                    title.clone(),
                    title_bounds,
                    clip,
                    theme,
                    [1.0, 1.0, 1.0, 1.0],
                    TextAlign::Left,
                    TextRole::Body,
                ));

                let sub_bounds = [
                    bounds[0] + 34.0,
                    start_y + title_h + 2.0,
                    bounds[2] - 46.0,
                    sub_h,
                ];
                frame.texts.push(text_spec(
                    sub.clone(),
                    sub_bounds,
                    clip,
                    theme,
                    theme.text_muted,
                    TextAlign::Left,
                    TextRole::Caption,
                ));
            } else {
                let title_bounds = [bounds[0] + 34.0, bounds[1], bounds[2] - 46.0, bounds[3]];
                frame.texts.push(text_spec(
                    title.clone(),
                    title_bounds,
                    clip,
                    theme,
                    [1.0, 1.0, 1.0, 1.0],
                    TextAlign::Left,
                    TextRole::Body,
                ));
            }
        }

        WidgetKind::Checkbox { checked, .. } => {
            let bg = if *checked {
                [
                    theme.accent[0] * 0.35,
                    theme.accent[1] * 0.35,
                    theme.accent[2] * 0.35,
                    0.95,
                ]
            } else {
                theme.glass_bg
            };
            let base = if *checked {
                theme.glow_intensity_hover * 0.8
            } else {
                0.0
            };
            let intensity = interactive_glow(base, hovered, pressed, theme);
            frame.instances.push(glass_instance(
                bounds,
                clip,
                bg,
                theme.accent,
                intensity,
                theme,
            ));
        }

        WidgetKind::Toggle { active, .. } => {
            let bg = if *active {
                [
                    theme.accent[0] * 0.25,
                    theme.accent[1] * 0.25,
                    theme.accent[2] * 0.25,
                    0.85,
                ]
            } else {
                [
                    theme.glass_bg[0] * 0.6,
                    theme.glass_bg[1] * 0.6,
                    theme.glass_bg[2] * 0.6,
                    0.5,
                ]
            };
            let glow = if *active {
                theme.accent
            } else {
                [0.3, 0.3, 0.3, 1.0]
            };
            let base = if *active { theme.glow_intensity } else { 0.0 };
            let intensity = interactive_glow(base, hovered, pressed, theme);
            frame
                .instances
                .push(glass_instance(bounds, clip, bg, glow, intensity, theme));

            let thumb_size = (bounds[3] - 6.0).max(4.0);
            let thumb_x = if *active {
                bounds[0] + bounds[2] - thumb_size - 3.0
            } else {
                bounds[0] + 3.0
            };
            let thumb_y = bounds[1] + 3.0;
            let thumb_bounds = [thumb_x, thumb_y, thumb_size, thumb_size];
            let thumb_color = if *active {
                theme.accent
            } else {
                [0.65, 0.72, 0.82, 1.0]
            };
            let thumb_glow = if *active { 0.30 } else { 0.0 };
            frame.instances.push(glass_instance(
                thumb_bounds,
                clip,
                thumb_color,
                thumb_color,
                thumb_glow,
                theme,
            ));
        }

        WidgetKind::Slider {
            min,
            max,
            value,
            orientation,
            ..
        } => {
            let range = (max - min).max(1.0e-5);
            let ratio = ((value - min) / range).clamp(0.0, 1.0);

            match orientation {
                crate::kind::SliderOrientation::Horizontal => {
                    let track_h = 6.0;
                    let track_y = bounds[1] + (bounds[3] - track_h) * 0.5;
                    let track_bounds = [bounds[0], track_y, bounds[2], track_h];
                    frame.instances.push(glass_instance(
                        track_bounds,
                        clip,
                        theme.glass_bg,
                        theme.accent_secondary,
                        0.0,
                        theme,
                    ));

                    if ratio > 0.0 {
                        let fill_w = bounds[2] * ratio;
                        let fill_bounds = [bounds[0], track_y, fill_w, track_h];
                        frame.instances.push(glass_instance(
                            fill_bounds,
                            clip,
                            theme.accent,
                            theme.accent,
                            0.06,
                            theme,
                        ));
                    }

                    let thumb_w = 14.0;
                    let thumb_h = (bounds[3] - 4.0).max(14.0);
                    let thumb_x = bounds[0] + ratio * bounds[2] - thumb_w * 0.5;
                    let thumb_y = bounds[1] + (bounds[3] - thumb_h) * 0.5;
                    let thumb_bounds = [thumb_x, thumb_y, thumb_w, thumb_h];
                    let thumb_glow = if pressed { 0.20 } else if hovered { 0.12 } else { 0.04 };
                    frame.instances.push(glass_instance(
                        thumb_bounds,
                        clip,
                        [0.95, 0.98, 1.0, 1.0],
                        theme.accent,
                        thumb_glow,
                        theme,
                    ));
                }
                crate::kind::SliderOrientation::Vertical => {
                    let track_w = 6.0;
                    let track_x = bounds[0] + (bounds[2] - track_w) * 0.5;
                    let track_bounds = [track_x, bounds[1], track_w, bounds[3]];
                    frame.instances.push(glass_instance(
                        track_bounds,
                        clip,
                        theme.glass_bg,
                        theme.accent_secondary,
                        0.0,
                        theme,
                    ));

                    if ratio > 0.0 {
                        let fill_h = bounds[3] * ratio;
                        let fill_y = bounds[1] + bounds[3] - fill_h;
                        let fill_bounds = [track_x, fill_y, track_w, fill_h];
                        frame.instances.push(glass_instance(
                            fill_bounds,
                            clip,
                            theme.accent,
                            theme.accent,
                            0.06,
                            theme,
                        ));
                    }

                    let thumb_w = (bounds[2] - 4.0).max(20.0);
                    let thumb_h = 10.0;
                    let thumb_x = bounds[0] + (bounds[2] - thumb_w) * 0.5;
                    let thumb_y = bounds[1] + (1.0 - ratio) * bounds[3] - thumb_h * 0.5;
                    let thumb_bounds = [thumb_x, thumb_y, thumb_w, thumb_h];
                    let thumb_glow = if pressed { 0.20 } else if hovered { 0.12 } else { 0.04 };
                    frame.instances.push(glass_instance(
                        thumb_bounds,
                        clip,
                        [0.95, 0.98, 1.0, 1.0],
                        theme.accent,
                        thumb_glow,
                        theme,
                    ));
                }
            }
        }

        WidgetKind::ProgressBar {
            progress,
            kind,
            label,
        } => {
            let ratio = progress.clamp(0.0, 1.0);
            match kind {
                crate::kind::ProgressKind::Horizontal => {
                    let track_h = (bounds[3] - 2.0).max(4.0);
                    let track_y = bounds[1] + (bounds[3] - track_h) * 0.5;
                    let track_bounds = [bounds[0], track_y, bounds[2], track_h];
                    frame.instances.push(glass_instance(
                        track_bounds,
                        clip,
                        [
                            theme.glass_bg[0] * 0.7,
                            theme.glass_bg[1] * 0.7,
                            theme.glass_bg[2] * 0.7,
                            0.6,
                        ],
                        theme.accent_secondary,
                        0.0,
                        theme,
                    ));

                    if ratio > 0.0 {
                        let fill_w = (bounds[2] * ratio).max(4.0);
                        let fill_bounds = [bounds[0], track_y, fill_w, track_h];
                        frame.instances.push(glass_instance(
                            fill_bounds,
                            clip,
                            theme.accent,
                            theme.accent,
                            0.08,
                            theme,
                        ));
                    }
                }
                crate::kind::ProgressKind::Vertical => {
                    let track_w = (bounds[2] - 2.0).max(4.0);
                    let track_x = bounds[0] + (bounds[2] - track_w) * 0.5;
                    let track_bounds = [track_x, bounds[1], track_w, bounds[3]];
                    frame.instances.push(glass_instance(
                        track_bounds,
                        clip,
                        [
                            theme.glass_bg[0] * 0.7,
                            theme.glass_bg[1] * 0.7,
                            theme.glass_bg[2] * 0.7,
                            0.6,
                        ],
                        theme.accent_secondary,
                        0.0,
                        theme,
                    ));

                    if ratio > 0.0 {
                        let fill_h = (bounds[3] * ratio).max(4.0);
                        let fill_y = bounds[1] + bounds[3] - fill_h;
                        let fill_bounds = [track_x, fill_y, track_w, fill_h];
                        frame.instances.push(glass_instance(
                            fill_bounds,
                            clip,
                            theme.accent,
                            theme.accent,
                            0.08,
                            theme,
                        ));
                    }
                }
                crate::kind::ProgressKind::Ring => {
                    // Circular Donut Gauge / Ring
                    let size = bounds[2].min(bounds[3]);
                    let x = bounds[0] + (bounds[2] - size) * 0.5;
                    let y = bounds[1] + (bounds[3] - size) * 0.5;
                    let radius = size * 0.5;
                    let ring_bounds = [x, y, size, size];

                    // Outer circle track
                    let ring_glow = if ratio > 0.0 {
                        theme.glow_intensity * 0.6 * ratio
                    } else {
                        0.0
                    };
                    let border_color = if ratio > 0.0 {
                        theme.accent
                    } else {
                        theme.accent_secondary
                    };
                    let ring_bg = [
                        theme.glass_bg[0] * 0.5,
                        theme.glass_bg[1] * 0.5,
                        theme.glass_bg[2] * 0.5,
                        0.85,
                    ];
                    frame.instances.push(custom_glass_instance(
                        ring_bounds,
                        clip,
                        ring_bg,
                        border_color,
                        radius,
                        3.5,
                        ring_glow,
                    ));

                    // Inner core cutout
                    let hole_size = (size - 18.0).max(10.0);
                    let hole_x = bounds[0] + (bounds[2] - hole_size) * 0.5;
                    let hole_y = bounds[1] + (bounds[3] - hole_size) * 0.5;
                    let hole_radius = hole_size * 0.5;
                    let hole_bounds = [hole_x, hole_y, hole_size, hole_size];
                    frame.instances.push(custom_glass_instance(
                        hole_bounds,
                        clip,
                        theme.glass_bg,
                        [
                            theme.accent_secondary[0],
                            theme.accent_secondary[1],
                            theme.accent_secondary[2],
                            0.3,
                        ],
                        hole_radius,
                        1.0,
                        0.0,
                    ));

                    // Center percentage or label text
                    let display_text = label
                        .clone()
                        .unwrap_or_else(|| format!("{:.0}%", ratio * 100.0));
                    let text_h = 16.0;
                    let text_y = bounds[1] + (bounds[3] - text_h) * 0.5;
                    let text_bounds = [hole_x, text_y, hole_size, text_h];
                    frame.texts.push(text_spec(
                        display_text,
                        text_bounds,
                        clip,
                        theme,
                        theme.text_color,
                        TextAlign::Center,
                        TextRole::Caption,
                    ));
                }
                crate::kind::ProgressKind::Pie => {
                    // Filled circular disc progress
                    let size = bounds[2].min(bounds[3]);
                    let x = bounds[0] + (bounds[2] - size) * 0.5;
                    let y = bounds[1] + (bounds[3] - size) * 0.5;
                    let radius = size * 0.5;
                    let pie_bounds = [x, y, size, size];

                    let pie_glow = (ratio * theme.glow_intensity * 0.8).max(0.1);
                    let pie_bg = [
                        theme.accent[0] * (0.2 + ratio * 0.5),
                        theme.accent[1] * (0.2 + ratio * 0.5),
                        theme.accent[2] * (0.2 + ratio * 0.5),
                        0.85 + ratio * 0.1,
                    ];
                    frame.instances.push(custom_glass_instance(
                        pie_bounds,
                        clip,
                        pie_bg,
                        theme.accent,
                        radius,
                        2.0,
                        pie_glow,
                    ));

                    let display_text = label
                        .clone()
                        .unwrap_or_else(|| format!("{:.0}%", ratio * 100.0));
                    let text_h = 16.0;
                    let text_y = bounds[1] + (bounds[3] - text_h) * 0.5;
                    let text_bounds = [x, text_y, size, text_h];
                    frame.texts.push(text_spec(
                        display_text,
                        text_bounds,
                        clip,
                        theme,
                        [1.0, 1.0, 1.0, 1.0],
                        TextAlign::Center,
                        TextRole::Caption,
                    ));
                }
            }
        }

        WidgetKind::MetricCard {
            title,
            value,
            delta,
        } => {
            // Soft drop shadow under the metric card
            let shadow_color = [0.0, 0.0, 0.0, 0.40];
            let shadow_glow = [0.0, 0.0, 0.0, 0.55];
            let shadow_bounds = [bounds[0], bounds[1] + 3.0, bounds[2], bounds[3]];
            frame.instances.push(custom_glass_instance(
                shadow_bounds,
                clip,
                shadow_color,
                shadow_glow,
                theme.corner_radius.min(6.0) + 1.0,
                0.0,
                0.35,
            ));

            frame.instances.push(glass_instance(
                bounds,
                clip,
                theme.glass_bg,
                theme.accent_secondary,
                0.05,
                theme,
            ));

            let title_bounds = [bounds[0] + 12.0, bounds[1] + 6.0, bounds[2] - 70.0, 16.0];
            frame.texts.push(text_spec(
                title.clone(),
                title_bounds,
                clip,
                theme,
                theme.text_muted,
                TextAlign::Left,
                TextRole::Caption,
            ));

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
                let badge_bounds = [
                    bounds[0] + bounds[2] - badge_w - 10.0,
                    bounds[1] + 8.0,
                    badge_w,
                    badge_h,
                ];
                let color = if *positive {
                    theme.success
                } else {
                    theme.danger
                };
                let bg = [color[0] * 0.15, color[1] * 0.15, color[2] * 0.15, 0.75];
                frame
                    .instances
                    .push(glass_instance(badge_bounds, clip, bg, color, 0.15, theme));
                frame.texts.push(text_spec(
                    delta_text.clone(),
                    badge_bounds,
                    clip,
                    theme,
                    color,
                    TextAlign::Center,
                    TextRole::Caption,
                ));
            }
        }

        WidgetKind::TextInput {
            value,
            placeholder,
            focused,
            cursor,
            selection,
            ..
        } => {
            let base = if *focused {
                theme.glow_intensity_hover * 1.1
            } else {
                theme.glow_intensity * 0.4
            };
            let intensity = interactive_glow(base, hovered, pressed, theme);
            let border_accent = if *focused {
                theme.accent
            } else {
                theme.accent_secondary
            };
            frame.instances.push(glass_instance(
                bounds,
                clip,
                theme.glass_bg,
                border_accent,
                intensity,
                theme,
            ));
            let (text, color) = if value.is_empty() {
                (placeholder.clone(), theme.text_muted)
            } else {
                (value.clone(), theme.text_color)
            };
            frame.texts.push(text_spec(
                text,
                inset(bounds, 12.0, 0.0),
                clip,
                theme,
                color,
                TextAlign::Left,
                TextRole::Body,
            ));

            if *focused {
                let cursor_h = (bounds[3] - 14.0).max(12.0);
                let cursor_y = bounds[1] + (bounds[3] - cursor_h) * 0.5;

                // Selection highlight
                if let Some((s_start, s_end)) = selection {
                    if s_start != s_end {
                        let min_s = (*s_start).min(*s_end).min(value.len());
                        let max_s = (*s_start).max(*s_end).min(value.len());
                        let x1 = bounds[0]
                            + 12.0
                            + measure.caret_x(
                                value,
                                &theme.typography.family,
                                theme.typography.body_size,
                                min_s,
                            );
                        let x2 = bounds[0]
                            + 12.0
                            + measure.caret_x(
                                value,
                                &theme.typography.family,
                                theme.typography.body_size,
                                max_s,
                            );
                        let sel_bounds = [x1, cursor_y, (x2 - x1).max(2.0), cursor_h];
                        let sel_bg = [1.0 / 255.0, 35.0 / 255.0, 45.0 / 255.0, 0.90];
                        let sel_border = [
                            theme.accent[0] * 0.7,
                            theme.accent[1] * 0.7,
                            theme.accent[2] * 0.7,
                            0.8,
                        ];
                        frame.instances.push(custom_glass_instance(
                            sel_bounds, clip, sel_bg, sel_border, 2.0, 1.0, 0.15,
                        ));
                    }
                }

                // Caret line at exact cursor index
                let safe_cursor = (*cursor).min(value.len());
                let text_w = measure.caret_x(
                    value,
                    &theme.typography.family,
                    theme.typography.body_size,
                    safe_cursor,
                );
                let cursor_x = (bounds[0] + 12.0 + text_w).min(bounds[0] + bounds[2] - 14.0);
                let cursor_bounds = [cursor_x, cursor_y, 1.5, cursor_h];
                frame.instances.push(custom_glass_instance(
                    cursor_bounds,
                    clip,
                    theme.accent,
                    [0.0, 0.0, 0.0, 0.0],
                    0.0,
                    0.0,
                    0.0,
                ));
            }
        }

        WidgetKind::TextArea {
            value,
            placeholder,
            focused,
            line_numbers,
            cursor,
            selection,
            ..
        } => {
            let base = if *focused {
                theme.glow_intensity_hover * 1.1
            } else {
                theme.glow_intensity * 0.4
            };
            let intensity = interactive_glow(base, hovered, pressed, theme);
            let border_accent = if *focused {
                theme.accent
            } else {
                theme.accent_secondary
            };
            frame.instances.push(glass_instance(
                bounds,
                clip,
                theme.glass_bg,
                border_accent,
                intensity,
                theme,
            ));

            let gutter_w = if *line_numbers { 32.0 } else { 0.0 };
            if *line_numbers {
                let gutter_bounds = [bounds[0], bounds[1], gutter_w, bounds[3]];
                let gutter_bg = [
                    theme.glass_bg[0] * 0.5,
                    theme.glass_bg[1] * 0.5,
                    theme.glass_bg[2] * 0.5,
                    0.65,
                ];
                frame.instances.push(glass_instance(
                    gutter_bounds,
                    clip,
                    gutter_bg,
                    [0.0, 0.0, 0.0, 0.0],
                    0.0,
                    theme,
                ));

                let div_bounds = [bounds[0] + gutter_w, bounds[1], 1.0, bounds[3]];
                frame.instances.push(glass_instance(
                    div_bounds,
                    clip,
                    theme.accent_secondary,
                    theme.accent_secondary,
                    0.0,
                    theme,
                ));
            }

            let text_offset_x = bounds[0] + gutter_w + 10.0;
            let text_w = (bounds[2] - gutter_w - 18.0).max(10.0);
            let line_h = 20.0;
            let start_y = bounds[1] + 8.0;

            if value.is_empty() {
                let line_bounds = [text_offset_x, start_y, text_w, line_h];
                frame.texts.push(text_spec(
                    placeholder.clone(),
                    line_bounds,
                    clip,
                    theme,
                    theme.text_muted,
                    TextAlign::Left,
                    TextRole::Body,
                ));
                if *line_numbers {
                    let num_bounds = [bounds[0] + 2.0, start_y, gutter_w - 6.0, line_h];
                    frame.texts.push(text_spec(
                        "1".to_string(),
                        num_bounds,
                        clip,
                        theme,
                        theme.text_muted,
                        TextAlign::Right,
                        TextRole::Caption,
                    ));
                }
                if *focused {
                    let cursor_bounds = [text_offset_x, start_y + 2.0, 1.5, line_h - 4.0];
                    frame.instances.push(custom_glass_instance(
                        cursor_bounds,
                        clip,
                        theme.accent,
                        [0.0, 0.0, 0.0, 0.0],
                        0.0,
                        0.0,
                        0.0,
                    ));
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
                        frame.texts.push(text_spec(
                            (idx + 1).to_string(),
                            num_bounds,
                            clip,
                            theme,
                            theme.text_muted,
                            TextAlign::Right,
                            TextRole::Caption,
                        ));
                    }
                    let line_bounds = [text_offset_x, cur_y, text_w, line_h];
                    frame.texts.push(text_spec(
                        line_str.to_string(),
                        line_bounds,
                        clip,
                        theme,
                        theme.text_color,
                        TextAlign::Left,
                        TextRole::Body,
                    ));
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
                                    let x1 = text_offset_x
                                        + measure.caret_x(
                                            line_str,
                                            &theme.typography.family,
                                            theme.typography.body_size,
                                            l_start,
                                        );
                                    let x2 = text_offset_x
                                        + measure.caret_x(
                                            line_str,
                                            &theme.typography.family,
                                            theme.typography.body_size,
                                            l_end,
                                        );
                                    let cur_y = start_y + idx as f32 * line_h;
                                    if cur_y + line_h <= bounds[1] + bounds[3] {
                                        let sel_bounds =
                                            [x1, cur_y + 1.0, (x2 - x1).max(2.0), line_h - 2.0];
                                        let sel_bg =
                                            [1.0 / 255.0, 35.0 / 255.0, 45.0 / 255.0, 0.90];
                                        let sel_border = [
                                            theme.accent[0] * 0.7,
                                            theme.accent[1] * 0.7,
                                            theme.accent[2] * 0.7,
                                            0.8,
                                        ];
                                        frame.instances.push(custom_glass_instance(
                                            sel_bounds, clip, sel_bg, sel_border, 2.0, 1.0, 0.15,
                                        ));
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
                    let cursor_x = (text_offset_x
                        + measure.caret_x(
                            cur_line_str,
                            &theme.typography.family,
                            theme.typography.body_size,
                            cur_col_idx,
                        ))
                    .min(bounds[0] + bounds[2] - 12.0);
                    let cur_y = start_y + cur_line_idx as f32 * line_h;

                    if cur_y + line_h <= bounds[1] + bounds[3] {
                        let cursor_bounds = [cursor_x, cur_y + 2.0, 1.5, line_h - 4.0];
                        frame.instances.push(custom_glass_instance(
                            cursor_bounds,
                            clip,
                            theme.accent,
                            [0.0, 0.0, 0.0, 0.0],
                            0.0,
                            0.0,
                            0.0,
                        ));
                    }
                }
            }
        }

        WidgetKind::PasswordInput {
            value,
            placeholder,
            focused,
            revealed,
            cursor,
            ..
        } => {
            let base = if *focused {
                theme.glow_intensity_hover * 1.1
            } else {
                theme.glow_intensity * 0.4
            };
            let intensity = interactive_glow(base, hovered, pressed, theme);
            let border_accent = if *focused {
                theme.accent
            } else {
                theme.accent_secondary
            };
            frame.instances.push(glass_instance(
                bounds,
                clip,
                theme.glass_bg,
                border_accent,
                intensity,
                theme,
            ));

            let display_text = if value.is_empty() {
                placeholder.clone()
            } else if *revealed {
                value.clone()
            } else {
                "•".repeat(value.chars().count())
            };
            let text_color = if value.is_empty() {
                theme.text_muted
            } else {
                theme.text_color
            };
            let text_box = [bounds[0] + 12.0, bounds[1], bounds[2] - 44.0, bounds[3]];
            frame.texts.push(text_spec(
                display_text,
                text_box,
                clip,
                theme,
                text_color,
                TextAlign::Left,
                TextRole::Body,
            ));

            // Eye reveal toggle glyph on the right
            let eye_box = [bounds[0] + bounds[2] - 28.0, bounds[1], 24.0, bounds[3]];
            let eye_glyph = if *revealed { "👁" } else { "Ø" };
            let eye_color = if *revealed {
                theme.accent
            } else {
                theme.text_muted
            };
            frame.texts.push(text_spec(
                eye_glyph.to_string(),
                eye_box,
                clip,
                theme,
                eye_color,
                TextAlign::Center,
                TextRole::Body,
            ));

            if *focused {
                let safe_cursor = (*cursor).min(value.len());
                let text_w = if *revealed {
                    measure.caret_x(
                        value,
                        &theme.typography.family,
                        theme.typography.body_size,
                        safe_cursor,
                    )
                } else {
                    let char_count = value[..safe_cursor].chars().count();
                    char_count as f32 * (theme.typography.body_size * 0.55)
                };
                let cursor_x = (bounds[0] + 12.0 + text_w).min(bounds[0] + bounds[2] - 34.0);
                let cursor_h = (bounds[3] - 14.0).max(12.0);
                let cursor_y = bounds[1] + (bounds[3] - cursor_h) * 0.5;
                let cursor_bounds = [cursor_x, cursor_y, 1.5, cursor_h];
                frame.instances.push(custom_glass_instance(
                    cursor_bounds,
                    clip,
                    theme.accent,
                    [0.0, 0.0, 0.0, 0.0],
                    0.0,
                    0.0,
                    0.0,
                ));
            }
        }

        WidgetKind::NumberInput {
            value,
            precision,
            focused,
            enabled,
            ..
        } => {
            let base = if *focused {
                theme.glow_intensity_hover * 1.1
            } else if *enabled {
                theme.glow_intensity * 0.4
            } else {
                0.0
            };
            let intensity = interactive_glow(base, hovered && *enabled, pressed && *enabled, theme);
            let border_accent = if *focused {
                theme.accent
            } else if *enabled {
                theme.accent_secondary
            } else {
                [0.2, 0.25, 0.35, 0.4]
            };
            let bg = if *enabled {
                theme.glass_bg
            } else {
                [
                    theme.glass_bg[0] * 0.6,
                    theme.glass_bg[1] * 0.6,
                    theme.glass_bg[2] * 0.6,
                    0.4,
                ]
            };
            frame.instances.push(glass_instance(
                bounds,
                clip,
                bg,
                border_accent,
                intensity,
                theme,
            ));

            let num_text = format!("{:.precision$}", value, precision = *precision);
            let text_color = if *enabled {
                theme.text_color
            } else {
                theme.text_muted
            };
            let text_box = [bounds[0] + 12.0, bounds[1], bounds[2] - 38.0, bounds[3]];
            frame.texts.push(text_spec(
                num_text.clone(),
                text_box,
                clip,
                theme,
                text_color,
                TextAlign::Left,
                TextRole::Body,
            ));

            if *focused && *enabled {
                let text_w = estimate_text_width(&num_text, theme.typography.body_size);
                let cursor_x = (bounds[0] + 12.0 + text_w).min(bounds[0] + bounds[2] - 34.0);
                let cursor_h = (bounds[3] - 14.0).max(12.0);
                let cursor_y = bounds[1] + (bounds[3] - cursor_h) * 0.5;
                let cursor_bounds = [cursor_x, cursor_y, 1.5, cursor_h];
                frame.instances.push(custom_glass_instance(
                    cursor_bounds,
                    clip,
                    theme.accent,
                    [0.0, 0.0, 0.0, 0.0],
                    0.0,
                    0.0,
                    0.0,
                ));
            }

            // Stepper buttons on the right side
            let stepper_w = 24.0;
            let stepper_x = bounds[0] + bounds[2] - stepper_w - 2.0;
            let half_h = (bounds[3] - 4.0) * 0.5;

            let up_box = [stepper_x, bounds[1] + 2.0, stepper_w, half_h];
            let down_box = [stepper_x, bounds[1] + 2.0 + half_h, stepper_w, half_h];

            let btn_bg = if *enabled {
                [
                    theme.glass_bg[0] * 0.8,
                    theme.glass_bg[1] * 0.8,
                    theme.glass_bg[2] * 0.8,
                    0.5,
                ]
            } else {
                [0.1, 0.1, 0.1, 0.2]
            };
            frame.instances.push(glass_instance(
                up_box,
                clip,
                btn_bg,
                [0.0, 0.0, 0.0, 0.0],
                0.0,
                theme,
            ));
            frame.instances.push(glass_instance(
                down_box,
                clip,
                btn_bg,
                [0.0, 0.0, 0.0, 0.0],
                0.0,
                theme,
            ));

            let arrow_color = if *enabled {
                theme.text_muted
            } else {
                [0.35, 0.35, 0.4, 0.4]
            };
            frame.texts.push(text_spec(
                "▲".to_string(),
                up_box,
                clip,
                theme,
                arrow_color,
                TextAlign::Center,
                TextRole::Caption,
            ));
            frame.texts.push(text_spec(
                "▼".to_string(),
                down_box,
                clip,
                theme,
                arrow_color,
                TextAlign::Center,
                TextRole::Caption,
            ));
        }

        WidgetKind::RadioButton {
            label, selected, ..
        } => {
            let circle_size = 18.0;
            let circle_bounds = [
                bounds[0] + 4.0,
                bounds[1] + (bounds[3] - circle_size) * 0.5,
                circle_size,
                circle_size,
            ];
            let glow = if *selected {
                theme.accent
            } else {
                [0.3, 0.3, 0.3, 1.0]
            };
            let intensity = if *selected {
                interactive_glow(theme.glow_intensity_hover * 0.8, hovered, pressed, theme)
            } else {
                interactive_glow(0.0, hovered, pressed, theme)
            };
            let bg = if *selected {
                [
                    theme.accent[0] * 0.25,
                    theme.accent[1] * 0.25,
                    theme.accent[2] * 0.25,
                    0.9,
                ]
            } else {
                theme.glass_bg
            };
            frame.instances.push(glass_instance(
                circle_bounds,
                clip,
                bg,
                glow,
                intensity,
                theme,
            ));

            if *selected {
                let dot_size = 8.0;
                let dot_bounds = [
                    circle_bounds[0] + (circle_size - dot_size) * 0.5,
                    circle_bounds[1] + (circle_size - dot_size) * 0.5,
                    dot_size,
                    dot_size,
                ];
                frame.instances.push(glass_instance(
                    dot_bounds,
                    clip,
                    theme.accent,
                    theme.accent,
                    0.4,
                    theme,
                ));
            }

            let text_bounds = [
                bounds[0] + circle_size + 12.0,
                bounds[1],
                bounds[2] - circle_size - 12.0,
                bounds[3],
            ];
            let text_color = if *selected {
                theme.text_color
            } else {
                theme.text_muted
            };
            frame.texts.push(text_spec(
                label.clone(),
                text_bounds,
                clip,
                theme,
                text_color,
                TextAlign::Left,
                TextRole::Body,
            ));
        }

        WidgetKind::SegmentItem {
            label, selected, ..
        } => {
            let radius = (bounds[3] * 0.5).min(bounds[2] * 0.5);
            let (bg, border_color, glow_mult, text_color): ([f32; 4], [f32; 4], f32, [f32; 4]) = if *selected {
                (
                    [
                        theme.accent[0] * 0.22,
                        theme.accent[1] * 0.22,
                        theme.accent[2] * 0.22,
                        0.95,
                    ],
                    theme.accent,
                    0.25f32,
                    theme.accent,
                )
            } else if hovered {
                (
                    [0.14, 0.18, 0.28, 0.85],
                    [0.30, 0.40, 0.55, 0.60],
                    0.10f32,
                    theme.text_color,
                )
            } else {
                (
                    [0.08, 0.11, 0.18, 0.65],
                    [0.16, 0.22, 0.32, 0.45],
                    0.0f32,
                    theme.text_muted,
                )
            };

            let intensity = if pressed {
                0.35
            } else if hovered {
                glow_mult.max(theme.glow_intensity_hover)
            } else {
                glow_mult
            };

            frame.instances.push(custom_glass_instance(
                bounds,
                clip,
                bg,
                border_color,
                radius,
                1.0,
                intensity,
            ));

            frame.texts.push(text_spec(
                label.clone(),
                bounds,
                clip,
                theme,
                text_color,
                TextAlign::Center,
                TextRole::Small,
            ));
        }

        WidgetKind::Divider { .. } => {
            let color = [
                theme.accent_secondary[0] * 0.35,
                theme.accent_secondary[1] * 0.35,
                theme.accent_secondary[2] * 0.35,
                0.5,
            ];
            frame
                .instances
                .push(glass_instance(bounds, clip, color, color, 0.0, theme));
        }

        WidgetKind::Modal { title, .. } => {
            // Soft atmospheric drop shadow under the modal
            let shadow_color = [0.0, 0.0, 0.0, 0.65];
            let shadow_glow = [0.0, 0.0, 0.0, 0.80];
            let shadow_bounds = [bounds[0], bounds[1] + 6.0, bounds[2], bounds[3]];
            frame.instances.push(custom_glass_instance(
                shadow_bounds,
                clip,
                shadow_color,
                shadow_glow,
                theme.corner_radius + 2.0,
                0.0,
                0.70,
            ));

            // Deep midnight navy glass body derived strictly from theme tokens in linear space
            let bg = [
                theme.glass_bg[0],
                theme.glass_bg[1],
                theme.glass_bg[2],
                0.98,
            ];
            let border_color = [
                (theme.accent_secondary[0] * 0.35).min(1.0),
                (theme.accent_secondary[1] * 0.35).min(1.0),
                (theme.accent_secondary[2] * 0.35).min(1.0),
                0.85,
            ];
            frame.instances.push(custom_glass_instance(
                bounds,
                clip,
                bg,
                border_color,
                theme.corner_radius,
                theme.border_width,
                theme.glow_intensity * 0.6,
            ));
            let title_bar = [bounds[0] + 20.0, bounds[1] + 16.0, bounds[2] - 48.0, 24.0];
            frame.texts.push(text_spec(
                title.clone(),
                title_bar,
                clip,
                theme,
                theme.text_color,
                TextAlign::Left,
                TextRole::Title,
            ));
        }

        WidgetKind::ModalBackdrop { .. } => {
            let backdrop_bg = [
                theme.glass_bg[0] * 0.30,
                theme.glass_bg[1] * 0.30,
                theme.glass_bg[2] * 0.30,
                0.75,
            ];
            frame.instances.push(custom_glass_instance(
                bounds,
                clip,
                backdrop_bg,
                [0.0, 0.0, 0.0, 0.0],
                0.0,
                0.0,
                0.0,
            ));
        }

        WidgetKind::Window { title, .. } => {
            // Soft drop shadow under the main window card
            let shadow_color = [0.0, 0.0, 0.0, 0.55];
            let shadow_glow = [0.0, 0.0, 0.0, 0.70];
            let shadow_bounds = [bounds[0], bounds[1] + 5.0, bounds[2], bounds[3]];
            frame.instances.push(custom_glass_instance(
                shadow_bounds,
                clip,
                shadow_color,
                shadow_glow,
                theme.corner_radius + 2.0,
                0.0,
                0.60,
            ));

            // Main window glass body
            frame.instances.push(glass_instance(
                bounds,
                clip,
                theme.glass_bg,
                theme.accent_secondary,
                theme.glow_intensity,
                theme,
            ));
            let title_bar = [bounds[0] + 16.0, bounds[1] + 8.0, bounds[2] - 48.0, 28.0];
            frame.texts.push(text_spec(
                title.clone(),
                title_bar,
                clip,
                theme,
                theme.text_color,
                TextAlign::Left,
                TextRole::Title,
            ));
        }

        WidgetKind::Palette { .. } => {
            let bg = [
                theme.glass_bg[0] + 0.02,
                theme.glass_bg[1] + 0.03,
                theme.glass_bg[2] + 0.06,
                0.95,
            ];
            let glow = if hovered || pressed {
                theme.accent
            } else {
                theme.accent_secondary
            };
            let intensity = if hovered || pressed { 0.25 } else { 0.08 };
            frame.instances.push(custom_glass_instance(
                bounds, clip, bg, glow, 8.0, 1.2, intensity,
            ));
        }

        WidgetKind::PaletteHeader { title, folded, .. } => {
            let h_bg = [
                theme.glass_bg[0] * 0.75,
                theme.glass_bg[1] * 0.75,
                theme.glass_bg[2] * 0.75,
                0.85,
            ];
            frame.instances.push(custom_glass_instance(
                bounds,
                clip,
                h_bg,
                [0.0, 0.0, 0.0, 0.0],
                6.0,
                0.0,
                0.0,
            ));

            // Textured grip indicator [::] on left
            let grip_box = [bounds[0] + 6.0, bounds[1], 16.0, bounds[3]];
            frame.texts.push(text_spec(
                "⋮⋮".to_string(),
                grip_box,
                clip,
                theme,
                theme.accent,
                TextAlign::Center,
                TextRole::Small,
            ));

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
            frame.texts.push(text_spec(
                fold_glyph.to_string(),
                fold_box,
                clip,
                theme,
                theme.text_muted,
                TextAlign::Center,
                TextRole::Caption,
            ));

            // Close button glyph [×]
            let close_box = [bounds[0] + bounds[2] - 22.0, bounds[1], 18.0, bounds[3]];
            frame.texts.push(text_spec(
                "✕".to_string(),
                close_box,
                clip,
                theme,
                theme.danger,
                TextAlign::Center,
                TextRole::Caption,
            ));
        }

        WidgetKind::PaletteFoldButton { folded, .. } => {
            let glyph = if *folded { "▼" } else { "▲" };
            frame.texts.push(text_spec(
                glyph.to_string(),
                bounds,
                clip,
                theme,
                theme.text_muted,
                TextAlign::Center,
                TextRole::Caption,
            ));
        }

        WidgetKind::PaletteCloseButton { .. } => {
            frame.texts.push(text_spec(
                "✕".to_string(),
                bounds,
                clip,
                theme,
                theme.danger,
                TextAlign::Center,
                TextRole::Caption,
            ));
        }

        WidgetKind::ResizeGrip { .. } => {
            frame.texts.push(text_spec(
                "⇲".to_string(),
                bounds,
                clip,
                theme,
                theme.accent,
                TextAlign::Center,
                TextRole::Caption,
            ));
        }

        WidgetKind::WindowCloseButton { .. } => {
            let intensity = interactive_glow(theme.glow_intensity, hovered, pressed, theme);
            frame.instances.push(glass_instance(
                bounds,
                clip,
                theme.glass_bg,
                theme.accent_secondary,
                intensity,
                theme,
            ));
            let inner = inset(bounds, 5.0, 5.0);
            frame.instances.push(glass_instance(
                inner,
                clip,
                [
                    theme.accent[0] * 0.2,
                    theme.accent[1] * 0.2,
                    theme.accent[2] * 0.2,
                    0.8,
                ],
                theme.accent,
                intensity * 0.8,
                theme,
            ));
        }

        WidgetKind::Scrollbar {
            content_size,
            viewport_size,
            offset,
            ..
        } => {
            frame.instances.push(glass_instance(
                bounds,
                clip,
                theme.glass_bg,
                theme.accent,
                0.0,
                theme,
            ));
            let thumb_bounds =
                scrollbar_thumb_bounds(bounds, *content_size, *viewport_size, *offset);
            let intensity = interactive_glow(theme.glow_intensity * 0.7, hovered, pressed, theme);
            frame.instances.push(glass_instance(
                thumb_bounds,
                clip,
                theme.accent,
                theme.accent,
                intensity,
                theme,
            ));
        }

        WidgetKind::TabItem { label, active, .. } => {
            let glow = if *active {
                theme.accent
            } else {
                theme.accent_secondary
            };
            let base = if *active { theme.glow_intensity } else { 0.0 };
            let intensity = interactive_glow(base, hovered, pressed, theme);
            let bg = if *active {
                [
                    theme.glass_bg[0] + 0.06,
                    theme.glass_bg[1] + 0.09,
                    theme.glass_bg[2] + 0.13,
                    0.88,
                ]
            } else {
                [
                    theme.glass_bg[0] * 0.65,
                    theme.glass_bg[1] * 0.65,
                    theme.glass_bg[2] * 0.65,
                    0.55,
                ]
            };
            frame
                .instances
                .push(glass_instance(bounds, clip, bg, glow, intensity, theme));

            if *active {
                let bar_h = 2.0;
                let bar_bounds = [
                    bounds[0] + 8.0,
                    bounds[1] + bounds[3] - bar_h - 1.0,
                    bounds[2] - 16.0,
                    bar_h,
                ];
                frame.instances.push(glass_instance(
                    bar_bounds,
                    clip,
                    theme.accent,
                    theme.accent,
                    0.45,
                    theme,
                ));
            }

            let text_color = if *active {
                theme.accent
            } else {
                theme.text_muted
            };
            frame.texts.push(text_spec(
                label.clone(),
                bounds,
                clip,
                theme,
                text_color,
                TextAlign::Center,
                TextRole::Small,
            ));
        }

        WidgetKind::MenuBar => {
            let bar_bg = [
                theme.glass_bg[0] * 0.45,
                theme.glass_bg[1] * 0.45,
                theme.glass_bg[2] * 0.45,
                0.82,
            ];
            let bar_border = [
                theme.accent_secondary[0] * 0.35,
                theme.accent_secondary[1] * 0.35,
                theme.accent_secondary[2] * 0.35,
                0.45,
            ];
            frame.instances.push(custom_glass_instance(
                bounds, clip, bar_bg, bar_border, 6.0, 1.0, 0.0,
            ));
        }

        WidgetKind::MenuBarItem { label, active, .. } => {
            if *active || hovered {
                let bg = if *active {
                    [
                        theme.accent[0] * 0.28,
                        theme.accent[1] * 0.28,
                        theme.accent[2] * 0.28,
                        0.92,
                    ]
                } else {
                    [
                        theme.accent[0] * 0.16,
                        theme.accent[1] * 0.16,
                        theme.accent[2] * 0.16,
                        0.75,
                    ]
                };
                let glow_intensity = if *active { 0.20 } else { 0.0 };
                let border_width = if *active { 1.0 } else { 0.0 };
                frame.instances.push(custom_glass_instance(
                    bounds,
                    clip,
                    bg,
                    theme.accent,
                    4.0,
                    border_width,
                    glow_intensity,
                ));
            }

            let text_color = if *active {
                [1.0, 1.0, 1.0, 1.0]
            } else if hovered {
                theme.text_color
            } else {
                [0.82, 0.88, 0.95, 1.0]
            };
            frame.texts.push(text_spec(
                label.clone(),
                bounds,
                clip,
                theme,
                text_color,
                TextAlign::Center,
                TextRole::Small,
            ));
        }

        WidgetKind::MenuPopover => {
            // Distinct popover palette container card with rounded corners and glowing border
            let popover_bg = [0.05, 0.07, 0.13, 0.98];
            let popover_border = [
                theme.accent[0] * 0.65,
                theme.accent[1] * 0.65,
                theme.accent[2] * 0.65,
                0.85,
            ];
            frame.instances.push(custom_glass_instance(
                bounds,
                clip,
                popover_bg,
                popover_border,
                8.0,
                1.2,
                0.30,
            ));
        }

        WidgetKind::MenuItem {
            label,
            shortcut,
            enabled,
            ..
        } => {
            if hovered && *enabled {
                let item_bg = [
                    theme.accent[0] * 0.32,
                    theme.accent[1] * 0.32,
                    theme.accent[2] * 0.32,
                    0.95,
                ];
                frame.instances.push(custom_glass_instance(
                    bounds,
                    clip,
                    item_bg,
                    [0.0, 0.0, 0.0, 0.0],
                    4.0,
                    0.0,
                    0.0,
                ));
            }

            let text_color = if *enabled {
                if hovered {
                    [1.0, 1.0, 1.0, 1.0]
                } else {
                    [0.88, 0.93, 0.98, 1.0]
                }
            } else {
                theme.text_muted
            };
            let label_bounds = [bounds[0] + 12.0, bounds[1], bounds[2] - 68.0, bounds[3]];
            frame.texts.push(text_spec(
                label.clone(),
                label_bounds,
                clip,
                theme,
                text_color,
                TextAlign::Left,
                TextRole::Small,
            ));

            if let Some(sc) = shortcut {
                let sc_color = if hovered && *enabled {
                    theme.accent
                } else {
                    theme.text_muted
                };
                let sc_bounds = [bounds[0] + bounds[2] - 60.0, bounds[1], 48.0, bounds[3]];
                frame.texts.push(text_spec(
                    sc.clone(),
                    sc_bounds,
                    clip,
                    theme,
                    sc_color,
                    TextAlign::Right,
                    TextRole::Caption,
                ));
            }
        }

        WidgetKind::Dropdown {
            label,
            selected_text,
            open,
            ..
        } => {
            let intensity =
                interactive_glow(theme.glow_intensity * 0.6, hovered || *open, pressed, theme);
            let border_color = if *open {
                theme.accent
            } else {
                theme.accent_secondary
            };
            let bg = [
                theme.glass_bg[0] + 0.04,
                theme.glass_bg[1] + 0.06,
                theme.glass_bg[2] + 0.09,
                0.88,
            ];
            frame.instances.push(custom_glass_instance(
                bounds,
                clip,
                bg,
                border_color,
                6.0,
                theme.border_width,
                intensity,
            ));

            let label_text = if selected_text.is_empty() {
                label.clone()
            } else {
                format!("{}: {}", label, selected_text)
            };
            let text_bounds = [bounds[0] + 12.0, bounds[1], bounds[2] - 32.0, bounds[3]];
            frame.texts.push(text_spec(
                label_text,
                text_bounds,
                clip,
                theme,
                theme.text_color,
                TextAlign::Left,
                TextRole::Small,
            ));

            let arrow = if *open { "▲" } else { "▼" };
            let arrow_bounds = [bounds[0] + bounds[2] - 22.0, bounds[1], 16.0, bounds[3]];
            frame.texts.push(text_spec(
                arrow.to_string(),
                arrow_bounds,
                clip,
                theme,
                theme.accent,
                TextAlign::Center,
                TextRole::Caption,
            ));
        }

        WidgetKind::Toast {
            title,
            message,
            kind,
            ..
        } => {
            let (accent_color, glow_mult) = match kind {
                crate::kind::ToastKind::Info => ([0.0, 0.85, 1.0, 1.0], 1.0),
                crate::kind::ToastKind::Success => ([0.15, 0.92, 0.45, 1.0], 1.3),
                crate::kind::ToastKind::Warning => ([1.0, 0.78, 0.12, 1.0], 1.3),
                crate::kind::ToastKind::Error => ([1.0, 0.28, 0.32, 1.0], 1.5),
            };

            // Strict inner clip: ensures text and indicators never overflow outside card borders
            let toast_clip = crate::effective::intersect(clip, [
                bounds[0] + 2.0,
                bounds[1] + 2.0,
                (bounds[2] - 4.0).max(10.0),
                (bounds[3] - 4.0).max(10.0),
            ]);

            // High-contrast solid cyber-glass toast card
            let bg = [0.04, 0.07, 0.13, 0.98];
            frame.instances.push(custom_glass_instance(
                bounds,
                clip,
                bg,
                accent_color,
                8.0,
                1.5,
                theme.glow_intensity * glow_mult,
            ));

            // Left vertical indicator bar
            let bar_w = 4.0;
            let bar_bounds = [
                bounds[0] + 10.0,
                bounds[1] + 10.0,
                bar_w,
                (bounds[3] - 20.0).max(4.0),
            ];
            frame.instances.push(custom_glass_instance(
                bar_bounds,
                toast_clip,
                accent_color,
                accent_color,
                2.0,
                0.0,
                0.4,
            ));

            // Glowing indicator dot
            let dot_size = 8.0;
            let dot_bounds = [bounds[0] + 20.0, bounds[1] + 13.0, dot_size, dot_size];
            frame.instances.push(custom_glass_instance(
                dot_bounds,
                toast_clip,
                accent_color,
                accent_color,
                4.0,
                0.0,
                0.6,
            ));

            // High-contrast crisp title
            let title_bounds = [
                bounds[0] + 34.0,
                bounds[1] + 8.0,
                (bounds[2] - 44.0).max(20.0),
                18.0,
            ];
            frame.texts.push(TextSpec {
                text: title.clone(),
                bounds: title_bounds,
                font_size: 12.0,
                color: [1.0, 1.0, 1.0, 1.0],
                align: TextAlign::Left,
                weight: FontWeight::Bold,
                clip: toast_clip,
            });

            // High-contrast legible message with strict clipping and multi-line safety
            let msg_bounds = [
                bounds[0] + 34.0,
                bounds[1] + 26.0,
                (bounds[2] - 44.0).max(20.0),
                (bounds[3] - 30.0).max(14.0),
            ];
            frame.texts.push(TextSpec {
                text: message.clone(),
                bounds: msg_bounds,
                font_size: 10.5,
                color: [0.86, 0.93, 1.0, 0.95],
                align: TextAlign::Left,
                weight: FontWeight::Normal,
                clip: toast_clip,
            });
        }

        WidgetKind::Tooltip { text, shortcut, placement: _ } => {
            let bg = [0.03, 0.06, 0.11, 0.98];
            let border_col = theme.accent;
            let tooltip_clip = crate::effective::intersect(clip, [
                bounds[0] + 1.0,
                bounds[1] + 1.0,
                (bounds[2] - 2.0).max(4.0),
                (bounds[3] - 2.0).max(4.0),
            ]);

            // High-contrast cyber-glass tooltip bubble
            frame.instances.push(custom_glass_instance(
                bounds,
                clip,
                bg,
                border_col,
                6.0,
                1.0,
                theme.glow_intensity * 0.75,
            ));

            if let Some(ref sc) = shortcut {
                let badge_w = (sc.len() as f32 * 6.5 + 14.0).clamp(36.0, 90.0);
                let text_w = (bounds[2] - badge_w - 18.0).max(10.0);

                // Main Tooltip Text
                frame.texts.push(TextSpec {
                    text: text.clone(),
                    bounds: [bounds[0] + 8.0, bounds[1], text_w, bounds[3]],
                    font_size: 11.0,
                    color: [0.95, 0.97, 1.0, 1.0],
                    align: TextAlign::Left,
                    weight: FontWeight::Normal,
                    clip: tooltip_clip,
                });

                // Shortcut Key Badge
                let badge_h = (bounds[3] - 8.0).clamp(14.0, 20.0);
                let badge_x = bounds[0] + bounds[2] - badge_w - 6.0;
                let badge_y = bounds[1] + (bounds[3] - badge_h) * 0.5;
                let badge_bounds = [badge_x, badge_y, badge_w, badge_h];

                frame.instances.push(custom_glass_instance(
                    badge_bounds,
                    tooltip_clip,
                    [0.10, 0.18, 0.28, 0.90],
                    [theme.accent[0] * 0.7, theme.accent[1] * 0.7, theme.accent[2] * 0.7, 0.60],
                    4.0,
                    1.0,
                    0.1,
                ));

                frame.texts.push(TextSpec {
                    text: sc.clone(),
                    bounds: badge_bounds,
                    font_size: 10.0,
                    color: theme.accent,
                    align: TextAlign::Center,
                    weight: FontWeight::Bold,
                    clip: tooltip_clip,
                });
            } else {
                frame.texts.push(TextSpec {
                    text: text.clone(),
                    bounds: [bounds[0] + 8.0, bounds[1], (bounds[2] - 16.0).max(10.0), bounds[3]],
                    font_size: 11.0,
                    color: [0.95, 0.97, 1.0, 1.0],
                    align: TextAlign::Center,
                    weight: FontWeight::Normal,
                    clip: tooltip_clip,
                });
            }
        }

        WidgetKind::Splitter { orientation, .. } => {
            let base_color = if hovered || pressed {
                [
                    theme.accent[0] * 0.45,
                    theme.accent[1] * 0.45,
                    theme.accent[2] * 0.45,
                    0.90,
                ]
            } else {
                [
                    theme.accent_secondary[0] * 0.35,
                    theme.accent_secondary[1] * 0.35,
                    theme.accent_secondary[2] * 0.35,
                    0.45,
                ]
            };
            let intensity = if hovered || pressed { 0.35 } else { 0.0 };
            frame.instances.push(custom_glass_instance(
                bounds,
                clip,
                base_color,
                theme.accent,
                2.0,
                0.0,
                intensity,
            ));

            // Centered tactile grip handle
            match orientation {
                crate::kind::SplitOrientation::Horizontal => {
                    let grip_w = 2.0;
                    let grip_h = 24.0;
                    let grip_bounds = [
                        bounds[0] + (bounds[2] - grip_w) * 0.5,
                        bounds[1] + (bounds[3] - grip_h) * 0.5,
                        grip_w,
                        grip_h,
                    ];
                    let grip_color = if hovered || pressed {
                        [1.0, 1.0, 1.0, 1.0]
                    } else {
                        theme.accent
                    };
                    frame.instances.push(custom_glass_instance(
                        grip_bounds,
                        clip,
                        grip_color,
                        theme.accent,
                        1.0,
                        0.0,
                        if hovered || pressed { 0.4 } else { 0.0 },
                    ));
                }
                crate::kind::SplitOrientation::Vertical => {
                    let grip_w = 24.0;
                    let grip_h = 2.0;
                    let grip_bounds = [
                        bounds[0] + (bounds[2] - grip_w) * 0.5,
                        bounds[1] + (bounds[3] - grip_h) * 0.5,
                        grip_w,
                        grip_h,
                    ];
                    let grip_color = if hovered || pressed {
                        [1.0, 1.0, 1.0, 1.0]
                    } else {
                        theme.accent
                    };
                    frame.instances.push(custom_glass_instance(
                        grip_bounds,
                        clip,
                        grip_color,
                        theme.accent,
                        1.0,
                        0.0,
                        if hovered || pressed { 0.4 } else { 0.0 },
                    ));
                }
            }
        }

        WidgetKind::TreeNode {
            label,
            depth,
            is_dir,
            expanded,
            selected,
            ..
        } => {
            let base = if *selected { theme.glow_intensity } else { 0.0 };
            let intensity = interactive_glow(base, hovered, pressed, theme);
            if *selected || hovered {
                let bg = if *selected {
                    [
                        theme.accent[0] * 0.24,
                        theme.accent[1] * 0.24,
                        theme.accent[2] * 0.24,
                        0.90,
                    ]
                } else {
                    [
                        theme.accent[0] * 0.12,
                        theme.accent[1] * 0.12,
                        theme.accent[2] * 0.12,
                        0.60,
                    ]
                };
                frame.instances.push(custom_glass_instance(
                    bounds,
                    clip,
                    bg,
                    theme.accent,
                    4.0,
                    if *selected { 1.0 } else { 0.0 },
                    intensity,
                ));
            }

            let indent_offset = (*depth as f32) * 16.0;

            if *is_dir {
                let arrow_str = if *expanded { "▼" } else { "▶" };
                let arrow_bounds = [bounds[0] + 6.0 + indent_offset, bounds[1], 14.0, bounds[3]];
                frame.texts.push(text_spec(
                    arrow_str.to_string(),
                    arrow_bounds,
                    clip,
                    theme,
                    theme.accent,
                    TextAlign::Center,
                    TextRole::Caption,
                ));
            } else {
                let dot_bounds = [
                    bounds[0] + 11.0 + indent_offset,
                    bounds[1] + (bounds[3] - 4.0) * 0.5,
                    4.0,
                    4.0,
                ];
                frame.instances.push(custom_glass_instance(
                    dot_bounds,
                    clip,
                    theme.accent_secondary,
                    theme.accent_secondary,
                    2.0,
                    0.0,
                    0.0,
                ));
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
            frame.texts.push(text_spec(
                label.clone(),
                label_bounds,
                clip,
                theme,
                text_color,
                TextAlign::Left,
                TextRole::Small,
            ));
        }

        WidgetKind::ListItem {
            text,
            selected,
            badge,
            ..
        } => {
            let base = if *selected { theme.glow_intensity } else { 0.0 };
            let intensity = interactive_glow(base, hovered, pressed, theme);
            let bg = if *selected {
                [
                    theme.glass_bg[0] + 0.04,
                    theme.glass_bg[1] + 0.07,
                    theme.glass_bg[2] + 0.11,
                    0.85,
                ]
            } else {
                [
                    theme.glass_bg[0] * 0.60,
                    theme.glass_bg[1] * 0.60,
                    theme.glass_bg[2] * 0.60,
                    0.45,
                ]
            };
            frame.instances.push(glass_instance(
                bounds,
                clip,
                bg,
                theme.accent,
                intensity,
                theme,
            ));

            let text_color = if *selected {
                theme.text_color
            } else {
                theme.text_muted
            };
            let left_text_bounds = [bounds[0] + 12.0, bounds[1], bounds[2] - 90.0, bounds[3]];
            frame.texts.push(text_spec(
                text.clone(),
                left_text_bounds,
                clip,
                theme,
                text_color,
                TextAlign::Left,
                TextRole::Small,
            ));

            match badge {
                crate::kind::ListItemBadge::Success => {
                    let badge_size = 14.0;
                    let badge_bounds = [
                        bounds[0] + bounds[2] - badge_size - 14.0,
                        bounds[1] + (bounds[3] - badge_size) * 0.5,
                        badge_size,
                        badge_size,
                    ];
                    frame.instances.push(glass_instance(
                        badge_bounds,
                        clip,
                        theme.success,
                        theme.success,
                        0.12,
                        theme,
                    ));
                }
                crate::kind::ListItemBadge::Warning => {
                    let badge_size = 14.0;
                    let badge_bounds = [
                        bounds[0] + bounds[2] - badge_size - 14.0,
                        bounds[1] + (bounds[3] - badge_size) * 0.5,
                        badge_size,
                        badge_size,
                    ];
                    frame.instances.push(glass_instance(
                        badge_bounds,
                        clip,
                        theme.warning,
                        theme.warning,
                        0.12,
                        theme,
                    ));
                }
                crate::kind::ListItemBadge::Active(label) => {
                    let badge_w = 68.0;
                    let badge_h = 20.0;
                    let badge_bounds = [
                        bounds[0] + bounds[2] - badge_w - 12.0,
                        bounds[1] + (bounds[3] - badge_h) * 0.5,
                        badge_w,
                        badge_h,
                    ];
                    let badge_bg = [
                        theme.accent[0] * 0.12,
                        theme.accent[1] * 0.12,
                        theme.accent[2] * 0.12,
                        0.7,
                    ];
                    frame.instances.push(glass_instance(
                        badge_bounds,
                        clip,
                        badge_bg,
                        theme.accent,
                        0.18,
                        theme,
                    ));
                    frame.texts.push(text_spec(
                        format!("~ {label}"),
                        badge_bounds,
                        clip,
                        theme,
                        theme.accent,
                        TextAlign::Center,
                        TextRole::Caption,
                    ));
                }
                crate::kind::ListItemBadge::Custom {
                    text: badge_text,
                    color,
                } => {
                    let badge_w = 60.0;
                    let badge_h = 20.0;
                    let badge_bounds = [
                        bounds[0] + bounds[2] - badge_w - 12.0,
                        bounds[1] + (bounds[3] - badge_h) * 0.5,
                        badge_w,
                        badge_h,
                    ];
                    let badge_bg = [color[0] * 0.12, color[1] * 0.12, color[2] * 0.12, 0.7];
                    frame.instances.push(glass_instance(
                        badge_bounds,
                        clip,
                        badge_bg,
                        *color,
                        0.15,
                        theme,
                    ));
                    frame.texts.push(text_spec(
                        badge_text.clone(),
                        badge_bounds,
                        clip,
                        theme,
                        *color,
                        TextAlign::Center,
                        TextRole::Caption,
                    ));
                }
                crate::kind::ListItemBadge::None => {}
            }
        }

        WidgetKind::BreadcrumbItem { label, is_last, .. } => {
            if hovered && !*is_last {
                let bg = [
                    theme.accent[0] * 0.15,
                    theme.accent[1] * 0.15,
                    theme.accent[2] * 0.15,
                    0.60,
                ];
                frame.instances.push(custom_glass_instance(
                    bounds,
                    clip,
                    bg,
                    [0.0, 0.0, 0.0, 0.0],
                    4.0,
                    0.0,
                    0.0,
                ));
            }

            let text_color = if *is_last {
                [1.0, 1.0, 1.0, 1.0]
            } else if hovered {
                theme.accent
            } else {
                theme.text_muted
            };

            let text_w = if *is_last {
                bounds[2]
            } else {
                bounds[2] - 14.0
            };
            let text_bounds = [bounds[0], bounds[1], text_w, bounds[3]];
            let role = if *is_last {
                TextRole::Body
            } else {
                TextRole::Small
            };
            frame.texts.push(text_spec(
                label.clone(),
                text_bounds,
                clip,
                theme,
                text_color,
                TextAlign::Left,
                role,
            ));

            if !*is_last {
                let sep_bounds = [bounds[0] + bounds[2] - 12.0, bounds[1], 10.0, bounds[3]];
                frame.texts.push(text_spec(
                    "›".to_string(),
                    sep_bounds,
                    clip,
                    theme,
                    theme.text_muted,
                    TextAlign::Center,
                    TextRole::Caption,
                ));
            }
        }

        WidgetKind::PaginationItem {
            label,
            active,
            disabled,
            ..
        } => {
            let intensity = if *active {
                0.35
            } else if hovered && !*disabled {
                0.20
            } else {
                0.0
            };

            let bg = if *active {
                [
                    theme.accent[0] * 0.35,
                    theme.accent[1] * 0.35,
                    theme.accent[2] * 0.35,
                    0.95,
                ]
            } else if *disabled {
                [
                    theme.glass_bg[0] * 0.40,
                    theme.glass_bg[1] * 0.40,
                    theme.glass_bg[2] * 0.40,
                    0.30,
                ]
            } else if hovered {
                [
                    theme.accent[0] * 0.20,
                    theme.accent[1] * 0.20,
                    theme.accent[2] * 0.20,
                    0.70,
                ]
            } else {
                [
                    theme.glass_bg[0] * 0.60,
                    theme.glass_bg[1] * 0.60,
                    theme.glass_bg[2] * 0.60,
                    0.50,
                ]
            };

            let border_color = if *active {
                theme.accent
            } else if hovered && !*disabled {
                theme.accent_secondary
            } else {
                [0.0, 0.0, 0.0, 0.0]
            };

            frame.instances.push(custom_glass_instance(
                bounds,
                clip,
                bg,
                border_color,
                4.0,
                if *active || hovered { 1.0 } else { 0.0 },
                intensity,
            ));

            let text_color = if *active {
                [1.0, 1.0, 1.0, 1.0]
            } else if *disabled {
                [
                    theme.text_muted[0],
                    theme.text_muted[1],
                    theme.text_muted[2],
                    0.35,
                ]
            } else if hovered {
                theme.accent
            } else {
                theme.text_color
            };

            frame.texts.push(text_spec(
                label.clone(),
                bounds,
                clip,
                theme,
                text_color,
                TextAlign::Center,
                TextRole::Small,
            ));
        }

        WidgetKind::Badge { label, badge } => {
            let (accent_color, bg_tint) = match badge {
                crate::kind::ListItemBadge::Success => (
                    theme.success,
                    [
                        theme.success[0] * 0.15,
                        theme.success[1] * 0.15,
                        theme.success[2] * 0.15,
                        0.85,
                    ],
                ),
                crate::kind::ListItemBadge::Warning => (
                    theme.warning,
                    [
                        theme.warning[0] * 0.15,
                        theme.warning[1] * 0.15,
                        theme.warning[2] * 0.15,
                        0.85,
                    ],
                ),
                crate::kind::ListItemBadge::Active(_) => (
                    theme.accent,
                    [
                        theme.accent[0] * 0.15,
                        theme.accent[1] * 0.15,
                        theme.accent[2] * 0.15,
                        0.85,
                    ],
                ),
                crate::kind::ListItemBadge::Custom { color, .. } => (
                    *color,
                    [color[0] * 0.15, color[1] * 0.15, color[2] * 0.15, 0.85],
                ),
                crate::kind::ListItemBadge::None => (
                    [0.055, 0.085, 0.155, 0.35], // Subtle dark border
                    [0.030, 0.045, 0.090, 0.45], // Subtle dark recessed chip
                ),
            };

            let text_color = match badge {
                crate::kind::ListItemBadge::None => theme.text_color,
                _ => accent_color,
            };

            frame.instances.push(custom_glass_instance(
                bounds,
                clip,
                bg_tint,
                accent_color,
                theme.corner_radius.min(6.0),
                if matches!(badge, crate::kind::ListItemBadge::None) { 0.5 } else { theme.border_width },
                if matches!(badge, crate::kind::ListItemBadge::None) { 0.0 } else { 0.15 },
            ));
            frame.texts.push(text_spec(
                label.clone(),
                bounds,
                clip,
                theme,
                text_color,
                TextAlign::Center,
                TextRole::Caption,
            ));
        }

        WidgetKind::ColorSwatch { color, label, .. } => {
            let intensity = if hovered || pressed { 0.35 } else { 0.0 };
            let border_color = if hovered || pressed {
                theme.accent
            } else {
                theme.accent_secondary
            };

            let (box_bounds, text_bounds) = if let Some(_) = label {
                let box_h = (bounds[3] - 18.0).max(12.0);
                (
                    [bounds[0], bounds[1], bounds[2], box_h],
                    Some([bounds[0], bounds[1] + box_h + 2.0, bounds[2], 14.0]),
                )
            } else {
                (bounds, None)
            };

            // Outer glass frame
            frame.instances.push(custom_glass_instance(
                box_bounds,
                clip,
                *color,
                border_color,
                6.0,
                1.0,
                intensity,
            ));

            if let (Some(lbl), Some(tb)) = (label, text_bounds) {
                frame.texts.push(text_spec(
                    lbl.clone(),
                    tb,
                    clip,
                    theme,
                    theme.text_muted,
                    TextAlign::Center,
                    TextRole::Caption,
                ));
            }
        }

        WidgetKind::ColorPicker { color, space, .. } => {
            let col = crate::color::Color::from_array(*color);
            let (cur_h, cur_s, cur_v) = col.to_hsv();
            let intensity = if hovered || pressed { 0.25 } else { 0.0 };
            let pad = 10.0;

            // 1. Outer container glass card
            let bg = [
                theme.glass_bg[0] + 0.02,
                theme.glass_bg[1] + 0.03,
                theme.glass_bg[2] + 0.06,
                0.94,
            ];
            frame.instances.push(custom_glass_instance(
                bounds,
                clip,
                bg,
                theme.accent_secondary,
                12.0,
                1.2,
                intensity,
            ));

            // 2. Top Area: [Preview Swatch] (Left) + [2D SV Canvas] (Right)
            let top_h = (bounds[3] * 0.38).clamp(60.0, 90.0);
            let inner_w = (bounds[2] - 2.0 * pad).max(20.0);
            let swatch_w = (inner_w * 0.32).clamp(40.0, 85.0);
            let swatch_bounds = [bounds[0] + pad, bounds[1] + pad, swatch_w, top_h];
            frame.instances.push(custom_glass_instance(
                swatch_bounds,
                clip,
                *color,
                [1.0, 1.0, 1.0, 0.4],
                6.0,
                1.0,
                0.2,
            ));

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
                    frame.instances.push(custom_glass_instance(
                        cb,
                        clip,
                        cell_col,
                        [0.0, 0.0, 0.0, 0.0],
                        0.0,
                        0.0,
                        0.0,
                    ));
                }
            }
            // Canvas border overlay
            frame.instances.push(custom_glass_instance(
                sv_bounds,
                clip,
                [0.0, 0.0, 0.0, 0.0],
                [0.35, 0.45, 0.55, 0.7],
                6.0,
                1.0,
                0.0,
            ));

            // Reticle circle on 2D SV Canvas
            let reticle_r = 7.0;
            let rx = (sv_x + cur_s * sv_w - reticle_r).clamp(sv_x, sv_x + sv_w - reticle_r * 2.0);
            let ry = (sv_y + (1.0 - cur_v) * sv_h - reticle_r)
                .clamp(sv_y, sv_y + sv_h - reticle_r * 2.0);
            let reticle_bounds = [rx, ry, reticle_r * 2.0, reticle_r * 2.0];
            frame.instances.push(custom_glass_instance(
                reticle_bounds,
                clip,
                *color,
                [1.0, 1.0, 1.0, 1.0],
                reticle_r,
                2.0,
                0.4,
            ));

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
                frame.instances.push(custom_glass_instance(
                    sb,
                    clip,
                    seg_color,
                    [0.0, 0.0, 0.0, 0.0],
                    0.0,
                    0.0,
                    0.0,
                ));
            }
            frame.instances.push(custom_glass_instance(
                hue_bounds,
                clip,
                [0.0, 0.0, 0.0, 0.0],
                [0.35, 0.45, 0.55, 0.6],
                6.0,
                1.0,
                0.0,
            ));

            // Hue Thumb Indicator
            let hue_ratio = (cur_h / 360.0).clamp(0.0, 1.0);
            let thumb_r = 8.0;
            let thumb_x =
                (hue_x + hue_ratio * hue_w - thumb_r).clamp(hue_x, hue_x + hue_w - thumb_r * 2.0);
            let thumb_y = hue_y - 2.0;
            let thumb_bounds = [thumb_x, thumb_y, thumb_r * 2.0, thumb_r * 2.0];
            let pure_hue = crate::color::Color::from_hsv(cur_h, 1.0, 1.0, 1.0).to_array();
            frame.instances.push(custom_glass_instance(
                thumb_bounds,
                clip,
                pure_hue,
                [1.0, 1.0, 1.0, 1.0],
                thumb_r,
                2.0,
                0.5,
            ));

            // 4. Middle-Bottom Area: [HEX Box with Copy Indicator]
            let hex_y = hue_y + hue_h + 8.0;
            let hex_h = 28.0;
            let hex_x = bounds[0] + pad;
            let hex_w = bounds[2] - 2.0 * pad;
            let hex_bounds = [hex_x, hex_y, hex_w, hex_h];
            let hex_bg = [
                theme.glass_bg[0] * 0.7,
                theme.glass_bg[1] * 0.7,
                theme.glass_bg[2] * 0.7,
                0.6,
            ];
            frame.instances.push(custom_glass_instance(
                hex_bounds,
                clip,
                hex_bg,
                theme.accent_secondary,
                6.0,
                1.0,
                0.05,
            ));

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
            frame.texts.push(text_spec(
                "⎘".to_string(),
                copy_bounds,
                clip,
                theme,
                theme.accent,
                TextAlign::Center,
                TextRole::Body,
            ));

            // 5. Bottom Area: [4 Multi-space Mini-Cards] (RGB, CMYK, HSV, LAB)
            let cards_y = hex_y + hex_h + 6.0;
            let cards_h = (bounds[1] + bounds[3] - pad - cards_y).max(28.0);
            let card_gap = 6.0;
            let card_w = (bounds[2] - 2.0 * pad - 3.0 * card_gap) / 4.0;

            let (r, g, b) = col.to_rgb_u8();
            let (c, m, y, k) = col.to_cmyk();
            let (l, a_val, b_val) = col.to_lab();

            let metrics = [
                (
                    "RGB",
                    format!("{}, {}, {}", r, g, b),
                    *space == crate::color::ColorSpace::Rgb,
                ),
                (
                    "CMYK",
                    format!("{:.0}%, {:.0}%, {:.0}%, {:.0}%", c, m, y, k),
                    *space == crate::color::ColorSpace::Cmyk,
                ),
                (
                    "HSV",
                    format!(
                        "{:.0}°, {:.0}%, {:.0}%",
                        cur_h,
                        cur_s * 100.0,
                        cur_v * 100.0
                    ),
                    false,
                ),
                (
                    "LAB",
                    format!("{:.0}, {:+.0}, {:+.0}", l, a_val, b_val),
                    *space == crate::color::ColorSpace::Lab,
                ),
            ];

            for (idx, (m_title, m_val, is_sel)) in metrics.iter().enumerate() {
                let mx = bounds[0] + pad + idx as f32 * (card_w + card_gap);
                let mb = [mx, cards_y, card_w, cards_h];
                let m_bg = if *is_sel {
                    [
                        theme.accent[0] * 0.25,
                        theme.accent[1] * 0.25,
                        theme.accent[2] * 0.25,
                        0.9,
                    ]
                } else {
                    [
                        theme.glass_bg[0] * 0.5,
                        theme.glass_bg[1] * 0.5,
                        theme.glass_bg[2] * 0.5,
                        0.4,
                    ]
                };
                let m_border = if *is_sel {
                    theme.accent
                } else {
                    [
                        theme.accent_secondary[0] * 0.25,
                        theme.accent_secondary[1] * 0.25,
                        theme.accent_secondary[2] * 0.25,
                        0.35,
                    ]
                };
                frame.instances.push(custom_glass_instance(
                    mb,
                    clip,
                    m_bg,
                    m_border,
                    5.0,
                    1.0,
                    if *is_sel { 0.15 } else { 0.0 },
                ));

                let t_box = [mx + 4.0, cards_y + 2.0, card_w - 8.0, 12.0];
                frame.texts.push(text_spec(
                    m_title.to_string(),
                    t_box,
                    clip,
                    theme,
                    if *is_sel {
                        theme.accent
                    } else {
                        theme.text_muted
                    },
                    TextAlign::Center,
                    TextRole::Caption,
                ));

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

        WidgetKind::Media {
            kind,
            resource_id,
            fit,
            radius,
            ..
        } => {
            let r = radius.unwrap_or(theme.corner_radius);
            frame.media.push(MediaSpec {
                kind: *kind,
                bounds,
                resource_id: resource_id.clone(),
                fit: *fit,
                radius: r,
                clip,
            });
        }

        WidgetKind::VideoPlayer {
            resource_id,
            playing,
            progress,
            duration_sec,
            ..
        } => {
            // 1. Video frame media spec
            let r = theme.corner_radius;
            frame.media.push(MediaSpec {
                kind: crate::media::MediaKind("video"),
                bounds,
                resource_id: resource_id.clone(),
                fit: crate::media::MediaFit::Cover,
                radius: r,
                clip,
            });

            // 2. Glass frame border & ambient glow
            frame.instances.push(glass_instance(
                bounds,
                clip,
                [0.0, 0.0, 0.0, 0.0],
                theme.accent,
                if hovered { 0.20 } else { 0.05 },
                theme,
            ));

            // 3. Cyber Glass Bottom Transport Bar (h = 36px)
            let bar_h = 36.0;
            let bar_y = bounds[1] + bounds[3] - bar_h - 6.0;
            let bar_bounds = [bounds[0] + 8.0, bar_y, bounds[2] - 16.0, bar_h];
            let bar_bg = [
                theme.glass_bg[0] * 0.9,
                theme.glass_bg[1] * 0.9,
                theme.glass_bg[2] * 0.9,
                0.92,
            ];
            frame.instances.push(custom_glass_instance(
                bar_bounds,
                clip,
                bar_bg,
                theme.accent_secondary,
                6.0,
                1.0,
                0.15,
            ));

            // Play / Pause button glyph
            let play_glyph = if *playing { "⏸" } else { "▶" };
            let play_bounds = [bar_bounds[0] + 6.0, bar_y, 24.0, bar_h];
            frame.texts.push(text_spec(
                play_glyph.to_string(),
                play_bounds,
                clip,
                theme,
                theme.accent,
                TextAlign::Center,
                TextRole::Body,
            ));

            // Timecode labels: current / total
            let cur_sec = (progress * duration_sec).round() as u32;
            let tot_sec = duration_sec.round() as u32;
            let time_str = format!(
                "{:02}:{:02} / {:02}:{:02}",
                cur_sec / 60,
                cur_sec % 60,
                tot_sec / 60,
                tot_sec % 60
            );

            let time_w = 90.0;
            let time_bounds = [
                bar_bounds[0] + bar_bounds[2] - time_w - 8.0,
                bar_y,
                time_w,
                bar_h,
            ];
            frame.texts.push(text_spec(
                time_str,
                time_bounds,
                clip,
                theme,
                theme.text_muted,
                TextAlign::Right,
                TextRole::Caption,
            ));

            // Timeline Scrubber track between play button and timecode
            let track_x = bar_bounds[0] + 34.0;
            let track_w = (bar_bounds[2] - 34.0 - time_w - 14.0).max(10.0);
            let track_h = 4.0;
            let track_y = bar_y + (bar_h - track_h) * 0.5;
            let track_bounds = [track_x, track_y, track_w, track_h];
            frame.instances.push(glass_instance(
                track_bounds,
                clip,
                [0.15, 0.20, 0.30, 0.8],
                theme.accent_secondary,
                0.0,
                theme,
            ));

            // Scrubber fill progress
            let fill_w = (track_w * progress.clamp(0.0, 1.0)).max(2.0);
            let fill_bounds = [track_x, track_y, fill_w, track_h];
            frame.instances.push(glass_instance(
                fill_bounds,
                clip,
                theme.accent,
                theme.accent,
                0.4,
                theme,
            ));

            // Scrubber Thumb dot
            let thumb_size = 10.0;
            let thumb_x = track_x + fill_w - thumb_size * 0.5;
            let thumb_y = bar_y + (bar_h - thumb_size) * 0.5;
            let thumb_bounds = [thumb_x, thumb_y, thumb_size, thumb_size];
            frame.instances.push(glass_instance(
                thumb_bounds,
                clip,
                [1.0, 1.0, 1.0, 1.0],
                theme.accent,
                0.5,
                theme,
            ));
        }

        WidgetKind::AudioVisualizer { values, peak, .. } => {
            // Ambient container glass
            frame.instances.push(glass_instance(
                bounds,
                clip,
                theme.glass_bg,
                theme.accent_secondary,
                0.08,
                theme,
            ));

            let pad_x = 10.0;
            let pad_y = 10.0;
            let avail_w = (bounds[2] - pad_x * 2.0).max(1.0);
            let avail_h = (bounds[3] - pad_y * 2.0).max(1.0);

            if !values.is_empty() {
                let n = values.len();
                let gap = 3.0;
                let total_gaps = gap * (n.saturating_sub(1) as f32);
                let bar_w = ((avail_w - total_gaps) / n as f32).max(2.0);

                for (i, &val) in values.iter().enumerate() {
                    let ratio = (val / peak.max(1.0e-4)).clamp(0.05, 1.0);
                    let bar_h = avail_h * ratio;
                    let bar_x = bounds[0] + pad_x + i as f32 * (bar_w + gap);
                    let bar_y = bounds[1] + bounds[3] - pad_y - bar_h;
                    let bar_bounds = [bar_x, bar_y, bar_w, bar_h];

                    // Gradient color modulation from cyan to purple/pink
                    let t = (i as f32 / n as f32).clamp(0.0, 1.0);
                    let bar_color = [
                        theme.accent[0] * (1.0 - t) + theme.accent_secondary[0] * t,
                        theme.accent[1] * (1.0 - t) + theme.accent_secondary[1] * t,
                        theme.accent[2] * (1.0 - t) + theme.accent_secondary[2] * t,
                        0.90,
                    ];
                    frame.instances.push(custom_glass_instance(
                        bar_bounds, clip, bar_color, bar_color, 2.0, 0.5, 0.35,
                    ));
                }
            }
        }

        WidgetKind::CustomPaint { commands, .. } => {
            let origin_x = bounds[0];
            let origin_y = bounds[1];
            // Clip paint commands to the CustomPaint widget's own layout bounds
            let own_clip = [bounds[0], bounds[1], bounds[2], bounds[3]];
            let clip = crate::effective::intersect(clip, own_clip);

            for cmd in commands {
                match cmd {
                    crate::paint::PaintCommand::Line {
                        from,
                        to,
                        stroke_width,
                        color,
                    } => {
                        let p1 = [origin_x + from[0], origin_y + from[1]];
                        let p2 = [origin_x + to[0], origin_y + to[1]];
                        render_custom_paint_line(p1, p2, *stroke_width, *color, clip, frame);
                    }
                    crate::paint::PaintCommand::Rect {
                        bounds: r_bounds,
                        corner_radius,
                        fill,
                        stroke,
                    } => {
                        let rect_abs = [
                            origin_x + r_bounds[0],
                            origin_y + r_bounds[1],
                            r_bounds[2],
                            r_bounds[3],
                        ];
                        let bg = fill.unwrap_or([0.0, 0.0, 0.0, 0.0]);
                        let (stroke_color, border_w) =
                            stroke.unwrap_or(([0.0, 0.0, 0.0, 0.0], 0.0));
                        frame.instances.push(custom_glass_instance(
                            rect_abs,
                            clip,
                            bg,
                            stroke_color,
                            *corner_radius,
                            border_w,
                            0.1,
                        ));
                    }
                    crate::paint::PaintCommand::Circle {
                        center,
                        radius,
                        fill,
                        stroke,
                    } => {
                        let d = radius * 2.0;
                        let circle_abs = [
                            origin_x + center[0] - radius,
                            origin_y + center[1] - radius,
                            d,
                            d,
                        ];
                        let bg = fill.unwrap_or([0.0, 0.0, 0.0, 0.0]);
                        let (stroke_color, border_w) =
                            stroke.unwrap_or(([0.0, 0.0, 0.0, 0.0], 0.0));
                        frame.instances.push(custom_glass_instance(
                            circle_abs,
                            clip,
                            bg,
                            stroke_color,
                            *radius,
                            border_w,
                            0.2,
                        ));
                    }
                    crate::paint::PaintCommand::Bezier {
                        start,
                        ctrl1,
                        ctrl2,
                        end,
                        stroke_width,
                        color,
                    } => {
                        let p0 = [origin_x + start[0], origin_y + start[1]];
                        let p1 = [origin_x + ctrl1[0], origin_y + ctrl1[1]];
                        let p2 = [origin_x + ctrl2[0], origin_y + ctrl2[1]];
                        let p3 = [origin_x + end[0], origin_y + end[1]];

                        let chord = ((p3[0] - p0[0]).powi(2) + (p3[1] - p0[1]).powi(2)).sqrt();
                        let subdivisions = (chord / 5.0).clamp(20.0, 64.0) as usize;
                        let mut prev_pt = p0;
                        for i in 1..=subdivisions {
                            let t = i as f32 / subdivisions as f32;
                            let pt = crate::paint::eval_cubic_bezier(p0, p1, p2, p3, t);
                            render_custom_paint_line(
                                prev_pt,
                                pt,
                                *stroke_width,
                                *color,
                                clip,
                                frame,
                            );
                            prev_pt = pt;
                        }
                    }
                    crate::paint::PaintCommand::Polyline {
                        points,
                        stroke_width,
                        color,
                        closed,
                    } => {
                        if points.len() >= 2 {
                            for i in 0..(points.len() - 1) {
                                let p1 = [origin_x + points[i][0], origin_y + points[i][1]];
                                let p2 = [origin_x + points[i + 1][0], origin_y + points[i + 1][1]];
                                render_custom_paint_line(
                                    p1,
                                    p2,
                                    *stroke_width,
                                    *color,
                                    clip,
                                    frame,
                                );
                            }
                            if *closed {
                                let p1 = [
                                    origin_x + points[points.len() - 1][0],
                                    origin_y + points[points.len() - 1][1],
                                ];
                                let p2 = [origin_x + points[0][0], origin_y + points[0][1]];
                                render_custom_paint_line(
                                    p1,
                                    p2,
                                    *stroke_width,
                                    *color,
                                    clip,
                                    frame,
                                );
                            }
                        }
                    }
                    crate::paint::PaintCommand::Text {
                        position,
                        text,
                        font_size,
                        color,
                    } => {
                        let text_bounds = [
                            origin_x + position[0],
                            origin_y + position[1],
                            bounds[2] - position[0],
                            *font_size + 4.0,
                        ];
                        frame.texts.push(TextSpec {
                            text: text.clone(),
                            bounds: text_bounds,
                            font_size: *font_size,
                            color: *color,
                            align: TextAlign::Left,
                            weight: FontWeight::Normal,
                            clip,
                        });
                    }
                }
            }
        }

        WidgetKind::Knob {
            id: _,
            value,
            min,
            max,
            step: _,
            label,
            unit,
        } => {
            let is_hovered = hovered;
            let is_pressed = pressed;

            let cx = bounds[0] + bounds[2] * 0.5;
            let cy = bounds[1] + bounds[3] * 0.45;
            let radius = (bounds[2].min(bounds[3]) * 0.35).max(12.0);

            // Background Outer Dial Ring
            let ring_bounds = [cx - radius, cy - radius, radius * 2.0, radius * 2.0];
            frame.instances.push(custom_glass_instance(
                ring_bounds,
                clip,
                theme.card_bg(),
                theme.border_subtle(),
                radius,
                1.5,
                if is_hovered { 0.15 } else { 0.05 },
            ));

            // Inner Metallic Dial Cap
            let inner_radius = radius * 0.75;
            let cap_bounds = [
                cx - inner_radius,
                cy - inner_radius,
                inner_radius * 2.0,
                inner_radius * 2.0,
            ];
            let cap_bg = if is_pressed {
                [0.12, 0.16, 0.24, 0.95]
            } else if is_hovered {
                [0.10, 0.14, 0.20, 0.90]
            } else {
                [0.06, 0.08, 0.12, 0.85]
            };
            frame.instances.push(custom_glass_instance(
                cap_bounds,
                clip,
                cap_bg,
                if is_hovered { theme.accent } else { theme.border_subtle() },
                inner_radius,
                1.0,
                0.0,
            ));

            // Angle calculation: from -135 deg to +135 deg (270 deg range)
            let ratio = if *max > *min {
                ((*value - *min) / (*max - *min)).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let start_angle = -std::f32::consts::PI * 0.75;
            let total_sweep = std::f32::consts::PI * 1.5;
            let cur_angle = start_angle + ratio * total_sweep;

            // Indicator Needle Dot / Line
            let needle_dist = inner_radius * 0.65;
            let nx = cx + cur_angle.cos() * needle_dist;
            let ny = cy + cur_angle.sin() * needle_dist;
            let dot_r = 2.5;
            frame.instances.push(custom_glass_instance(
                [nx - dot_r, ny - dot_r, dot_r * 2.0, dot_r * 2.0],
                clip,
                theme.accent,
                theme.accent,
                dot_r,
                0.0,
                0.4,
            ));

            // Text Label and Value Display
            if let Some(lbl) = label {
                frame.texts.push(TextSpec {
                    text: lbl.clone(),
                    bounds: [bounds[0], bounds[1], bounds[2], 14.0],
                    font_size: 10.0,
                    color: theme.text_muted,
                    align: TextAlign::Center,
                    weight: FontWeight::Normal,
                    clip,
                });
            }

            let val_str = if let Some(u) = unit {
                format!("{:.1} {}", value, u)
            } else {
                format!("{:.1}", value)
            };
            frame.texts.push(TextSpec {
                text: val_str,
                bounds: [bounds[0], bounds[1] + bounds[3] - 16.0, bounds[2], 14.0],
                font_size: 11.0,
                color: theme.text_color,
                align: TextAlign::Center,
                weight: FontWeight::Bold,
                clip,
            });
        }

        WidgetKind::TimeSeriesChart {
            id: _,
            title,
            series,
            x_min,
            x_max,
            y_min,
            y_max,
            show_grid,
            show_legend: _,
            inspected_point,
        } => {
            // Chart Card Background
            frame.instances.push(custom_glass_instance(
                bounds,
                clip,
                theme.card_bg(),
                theme.border_subtle(),
                6.0,
                1.0,
                0.05,
            ));

            let pad_left = 36.0;
            let pad_right = 16.0;
            let pad_top = if title.is_some() { 28.0 } else { 12.0 };
            let pad_bottom = 24.0;

            if let Some(t) = title {
                frame.texts.push(TextSpec {
                    text: t.clone(),
                    bounds: [bounds[0] + 12.0, bounds[1] + 8.0, bounds[2] - 24.0, 16.0],
                    font_size: 12.0,
                    color: theme.text_color,
                    align: TextAlign::Left,
                    weight: FontWeight::Bold,
                    clip,
                });
            }

            let plot_x = bounds[0] + pad_left;
            let plot_y = bounds[1] + pad_top;
            let plot_w = (bounds[2] - pad_left - pad_right).max(10.0);
            let plot_h = (bounds[3] - pad_top - pad_bottom).max(10.0);
            let plot_clip = crate::effective::intersect(clip, [plot_x, plot_y, plot_w, plot_h]);

            // Grid Lines & Axis Ticks
            if *show_grid {
                for i in 0..=4 {
                    let gy = plot_y + (i as f32 / 4.0) * plot_h;
                    render_custom_paint_line(
                        [plot_x, gy],
                        [plot_x + plot_w, gy],
                        0.8,
                        [1.0, 1.0, 1.0, 0.05],
                        clip,
                        frame,
                    );
                    // Y-Axis Value Label
                    let y_val = y_max - (i as f32 / 4.0) * (y_max - y_min);
                    frame.texts.push(TextSpec {
                        text: format!("{:.0}", y_val),
                        bounds: [bounds[0] + 4.0, gy - 6.0, pad_left - 8.0, 12.0],
                        font_size: 9.0,
                        color: theme.text_muted,
                        align: TextAlign::Right,
                        weight: FontWeight::Normal,
                        clip,
                    });
                }
            }

            // Draw Series Polylines and Area Fills
            let dx_range = (*x_max - *x_min).max(0.001);
            let dy_range = (*y_max - *y_min).max(0.001);

            for (s_idx, s) in series.iter().enumerate() {
                if s.points.len() < 2 {
                    continue;
                }
                let mut screen_pts: Vec<[f32; 2]> = Vec::with_capacity(s.points.len());
                for pt in &s.points {
                    let nx = ((pt[0] - *x_min) / dx_range).clamp(0.0, 1.0);
                    let ny = ((pt[1] - *y_min) / dy_range).clamp(0.0, 1.0);
                    let sx = plot_x + nx * plot_w;
                    let sy = plot_y + (1.0 - ny) * plot_h;
                    screen_pts.push([sx, sy]);
                }

                // Area Fill under curve
                if s.filled {
                    let mut fill_color = s.color;
                    fill_color[3] *= 0.15;
                    for i in 0..(screen_pts.len() - 1) {
                        let p1 = screen_pts[i];
                        let p2 = screen_pts[i + 1];
                        let bottom_y = plot_y + plot_h;
                        let col_w = (p2[0] - p1[0]).max(1.0);
                        let avg_h = (bottom_y - (p1[1] + p2[1]) * 0.5).max(0.0);
                        let col_rect = [p1[0], bottom_y - avg_h, col_w, avg_h];
                        frame.instances.push(custom_glass_instance(
                            col_rect,
                            plot_clip,
                            fill_color,
                            [0.0, 0.0, 0.0, 0.0],
                            0.0,
                            0.0,
                            0.0,
                        ));
                    }
                }

                // Line Stroke
                for i in 0..(screen_pts.len() - 1) {
                    render_custom_paint_line(
                        screen_pts[i],
                        screen_pts[i + 1],
                        1.8,
                        s.color,
                        plot_clip,
                        frame,
                    );
                }

                // Inspection Point Highlight
                if let Some((insp_s, insp_p)) = inspected_point {
                    if *insp_s == s_idx && *insp_p < screen_pts.len() {
                        let ipt = screen_pts[*insp_p];
                        // Vertical guideline
                        render_custom_paint_line(
                            [ipt[0], plot_y],
                            [ipt[0], plot_y + plot_h],
                            1.0,
                            [1.0, 1.0, 1.0, 0.3],
                            plot_clip,
                            frame,
                        );
                        // Highlight dot
                        frame.instances.push(custom_glass_instance(
                            [ipt[0] - 4.0, ipt[1] - 4.0, 8.0, 8.0],
                            clip,
                            s.color,
                            [1.0, 1.0, 1.0, 1.0],
                            4.0,
                            1.5,
                            0.5,
                        ));
                    }
                }
            }
        }

        WidgetKind::NodeGraph {
            id: _,
            nodes,
            connections,
            pan,
            zoom: _,
            connecting_from: _,
        } => {
            // Node Graph Canvas Base with Subtle Grid
            frame.instances.push(custom_glass_instance(
                bounds,
                clip,
                [0.03, 0.04, 0.07, 0.95],
                theme.border_subtle(),
                4.0,
                1.0,
                0.0,
            ));
            let graph_clip = crate::effective::intersect(clip, bounds);

            let origin_x = bounds[0] + pan[0];
            let origin_y = bounds[1] + pan[1];

            // Render Connections as Smooth Bézier Cables
            for conn in connections {
                let from_node = nodes.iter().find(|n| n.id == conn.from_node);
                let to_node = nodes.iter().find(|n| n.id == conn.to_node);

                if let (Some(fn_node), Some(tn_node)) = (from_node, to_node) {
                    let out_idx = conn.from_socket;
                    let in_idx = conn.to_socket;

                    let p_start = [
                        origin_x + fn_node.pos[0] + fn_node.size[0],
                        origin_y + fn_node.pos[1] + 32.0 + (out_idx as f32 * 20.0) + 8.0,
                    ];
                    let p_end = [
                        origin_x + tn_node.pos[0],
                        origin_y + tn_node.pos[1] + 32.0 + (in_idx as f32 * 20.0) + 8.0,
                    ];

                    let dx = (p_end[0] - p_start[0]).abs().max(40.0) * 0.5;
                    let p_ctrl1 = [p_start[0] + dx, p_start[1]];
                    let p_ctrl2 = [p_end[0] - dx, p_end[1]];

                    let cable_color = conn.color.unwrap_or([0.0, 0.85, 1.0, 0.8]);
                    let subdivisions = 32;
                    let mut prev = p_start;
                    for i in 1..=subdivisions {
                        let t = i as f32 / subdivisions as f32;
                        let pt = crate::paint::eval_cubic_bezier(p_start, p_ctrl1, p_ctrl2, p_end, t);
                        render_custom_paint_line(prev, pt, 1.8, cable_color, graph_clip, frame);
                        prev = pt;
                    }
                }
            }

            // Render Nodes
            for node in nodes {
                let nx = origin_x + node.pos[0];
                let ny = origin_y + node.pos[1];
                let nw = node.size[0];
                let nh = node.size[1];
                let node_bounds = [nx, ny, nw, nh];

                // Card Body
                let border_col = if node.selected {
                    theme.accent
                } else {
                    theme.border_subtle()
                };
                frame.instances.push(custom_glass_instance(
                    node_bounds,
                    graph_clip,
                    theme.card_bg(),
                    border_col,
                    6.0,
                    if node.selected { 1.5 } else { 1.0 },
                    if node.selected { 0.2 } else { 0.05 },
                ));

                // Node Header Bar
                let header_bg = node.header_color.unwrap_or([0.15, 0.20, 0.30, 0.9]);
                frame.instances.push(custom_glass_instance(
                    [nx, ny, nw, 28.0],
                    graph_clip,
                    header_bg,
                    [0.0, 0.0, 0.0, 0.0],
                    6.0,
                    0.0,
                    0.0,
                ));
                frame.texts.push(TextSpec {
                    text: node.title.clone(),
                    bounds: [nx + 8.0, ny + 7.0, nw - 16.0, 14.0],
                    font_size: 11.0,
                    color: [1.0, 1.0, 1.0, 1.0],
                    align: TextAlign::Left,
                    weight: FontWeight::Bold,
                    clip: graph_clip,
                });

                // Input Sockets (Left side)
                for (i, sock) in node.inputs.iter().enumerate() {
                    let sy = ny + 32.0 + (i as f32 * 20.0);
                    let scol = sock.color.unwrap_or_else(|| sock.socket_type.default_color());
                    // Pin dot
                    frame.instances.push(custom_glass_instance(
                        [nx - 4.0, sy + 4.0, 8.0, 8.0],
                        graph_clip,
                        scol,
                        [0.0, 0.0, 0.0, 0.8],
                        4.0,
                        1.0,
                        0.3,
                    ));
                    // Socket Name
                    frame.texts.push(TextSpec {
                        text: sock.name.clone(),
                        bounds: [nx + 8.0, sy + 2.0, nw * 0.5 - 12.0, 12.0],
                        font_size: 9.0,
                        color: theme.text_muted,
                        align: TextAlign::Left,
                        weight: FontWeight::Normal,
                        clip: graph_clip,
                    });
                }

                // Output Sockets (Right side)
                for (i, sock) in node.outputs.iter().enumerate() {
                    let sy = ny + 32.0 + (i as f32 * 20.0);
                    let scol = sock.color.unwrap_or_else(|| sock.socket_type.default_color());
                    // Pin dot
                    frame.instances.push(custom_glass_instance(
                        [nx + nw - 4.0, sy + 4.0, 8.0, 8.0],
                        graph_clip,
                        scol,
                        [0.0, 0.0, 0.0, 0.8],
                        4.0,
                        1.0,
                        0.3,
                    ));
                    // Socket Name
                    frame.texts.push(TextSpec {
                        text: sock.name.clone(),
                        bounds: [nx + nw * 0.5, sy + 2.0, nw * 0.5 - 8.0, 12.0],
                        font_size: 9.0,
                        color: theme.text_muted,
                        align: TextAlign::Right,
                        weight: FontWeight::Normal,
                        clip: graph_clip,
                    });
                }
            }
        }

        WidgetKind::TagInput {
            id: _,
            tags,
            placeholder,
            active_tag: _,
        } => {
            let tag_clip = crate::effective::intersect(clip, bounds);

            // Container Box
            frame.instances.push(custom_glass_instance(
                bounds,
                clip,
                theme.glass_bg,
                theme.border_subtle(),
                4.0,
                1.0,
                0.0,
            ));

            let mut cur_x = bounds[0] + 6.0;
            let tag_h = (bounds[3] - 8.0).clamp(16.0, 22.0);
            let tag_y = bounds[1] + (bounds[3] - tag_h) * 0.5;
            let max_x = bounds[0] + bounds[2] - 6.0;

            let mut remaining_count = 0;
            for (i, tag) in tags.iter().enumerate() {
                let tag_text_w = (tag.len() as f32 * 6.5 + 18.0).max(36.0);
                if cur_x + tag_text_w > max_x - 30.0 && i < tags.len() - 1 {
                    remaining_count = tags.len() - i;
                    break;
                }
                if cur_x + tag_text_w > max_x {
                    remaining_count = tags.len() - i;
                    break;
                }

                let chip_bounds = [cur_x, tag_y, tag_text_w, tag_h];

                // Chip pill
                frame.instances.push(custom_glass_instance(
                    chip_bounds,
                    tag_clip,
                    [0.10, 0.18, 0.28, 0.9],
                    theme.accent,
                    tag_h * 0.5,
                    1.0,
                    0.05,
                ));

                // Tag text with cross
                frame.texts.push(TextSpec {
                    text: format!("{} ✕", tag),
                    bounds: [cur_x + 6.0, tag_y + 2.0, tag_text_w - 8.0, 12.0],
                    font_size: 9.5,
                    color: theme.accent,
                    align: TextAlign::Left,
                    weight: FontWeight::Normal,
                    clip: tag_clip,
                });

                cur_x += tag_text_w + 5.0;
            }

            if remaining_count > 0 {
                let badge_w = 26.0;
                let badge_bounds = [cur_x.min(max_x - badge_w), tag_y, badge_w, tag_h];
                frame.instances.push(custom_glass_instance(
                    badge_bounds,
                    tag_clip,
                    [0.15, 0.22, 0.35, 0.9],
                    theme.border_subtle(),
                    tag_h * 0.5,
                    1.0,
                    0.0,
                ));
                frame.texts.push(TextSpec {
                    text: format!("+{}", remaining_count),
                    bounds: [badge_bounds[0], tag_y + 2.0, badge_w, 12.0],
                    font_size: 9.0,
                    color: theme.text_muted,
                    align: TextAlign::Center,
                    weight: FontWeight::Bold,
                    clip: tag_clip,
                });
            }

            if tags.is_empty() {
                frame.texts.push(TextSpec {
                    text: placeholder.clone(),
                    bounds: [bounds[0] + 8.0, bounds[1] + 6.0, bounds[2] - 16.0, 14.0],
                    font_size: 10.5,
                    color: theme.text_muted,
                    align: TextAlign::Left,
                    weight: FontWeight::Normal,
                    clip: tag_clip,
                });
            }
        }

        WidgetKind::CodeEditor {
            id: _,
            text,
            language: _,
            line_numbers,
            focused,
            cursor: _,
            selection: _,
        } => {
            let editor_clip = crate::effective::intersect(clip, bounds);

            // Background Card Body
            let border_col = if *focused {
                theme.border_highlight()
            } else {
                theme.border_subtle()
            };
            frame.instances.push(custom_glass_instance(
                bounds,
                clip,
                [0.05, 0.07, 0.12, 0.95],
                border_col,
                4.0,
                1.0,
                if *focused { 0.12 } else { 0.02 },
            ));

            let gutter_w = if *line_numbers { 36.0 } else { 8.0 };

            // Gutter background
            if *line_numbers {
                frame.instances.push(custom_glass_instance(
                    [bounds[0], bounds[1], gutter_w, bounds[3]],
                    editor_clip,
                    [0.03, 0.04, 0.08, 0.98],
                    theme.border_subtle(),
                    4.0,
                    1.0,
                    0.0,
                ));
            }

            let lines: Vec<&str> = text.lines().collect();
            let line_h = 16.0;
            let start_y = bounds[1] + 6.0;

            for (i, line) in lines.iter().enumerate() {
                let cur_y = start_y + i as f32 * line_h;
                if cur_y + line_h > bounds[1] + bounds[3] {
                    break;
                }

                // Line Number
                if *line_numbers {
                    frame.texts.push(TextSpec {
                        text: format!("{}", i + 1),
                        bounds: [bounds[0] + 4.0, cur_y, gutter_w - 10.0, line_h],
                        font_size: 10.0,
                        color: [0.45, 0.55, 0.70, 0.8],
                        align: TextAlign::Right,
                        weight: FontWeight::Normal,
                        clip: editor_clip,
                    });
                }

                // Simple syntax coloring heuristic for code text
                let text_x = bounds[0] + gutter_w + 8.0;
                let text_w = bounds[2] - gutter_w - 16.0;

                let line_str = *line;
                let line_trimmed = line_str.trim_start();
                let line_color = if line_trimmed.starts_with("//") {
                    [0.45, 0.58, 0.65, 0.85] // Comments (Slate Green)
                } else if line_trimmed.starts_with("fn ") || line_trimmed.starts_with("pub fn ") {
                    [0.0, 0.9, 0.8, 1.0] // Function definitions (Cyan)
                } else if line_trimmed.starts_with("let ") || line_trimmed.starts_with("struct ") {
                    [0.75, 0.45, 0.95, 1.0] // Keywords / structs (Violet)
                } else if line_trimmed.starts_with("return ") {
                    [0.95, 0.55, 0.35, 1.0] // Control flow (Amber)
                } else {
                    theme.text_color
                };

                frame.texts.push(TextSpec {
                    text: line_str.to_string(),
                    bounds: [text_x, cur_y, text_w, line_h],
                    font_size: 11.0,
                    color: line_color,
                    align: TextAlign::Left,
                    weight: FontWeight::Normal,
                    clip: editor_clip,
                });
            }
        }

        WidgetKind::BarChart {
            id: _,
            title,
            bars,
            max_value,
            horizontal,
        } => {
            let chart_clip = crate::effective::intersect(clip, bounds);

            // Container Box
            frame.instances.push(custom_glass_instance(
                bounds,
                clip,
                theme.card_bg(),
                theme.border_subtle(),
                6.0,
                1.0,
                0.05,
            ));

            let pad_top = if title.is_some() { 24.0 } else { 10.0 };
            if let Some(t) = title {
                frame.texts.push(TextSpec {
                    text: t.clone(),
                    bounds: [bounds[0] + 10.0, bounds[1] + 6.0, bounds[2] - 20.0, 16.0],
                    font_size: 11.0,
                    color: theme.text_color,
                    align: TextAlign::Left,
                    weight: FontWeight::Bold,
                    clip: chart_clip,
                });
            }

            let max_val = max_value.unwrap_or_else(|| {
                bars.iter()
                    .map(|b| b.value)
                    .fold(1.0f32, |acc, v| acc.max(v))
            }).max(0.001);

            if *horizontal {
                let avail_h = bounds[3] - pad_top - 8.0;
                let bar_count = bars.len().max(1);
                let bar_h = (avail_h / bar_count as f32 - 6.0).clamp(12.0, 28.0);
                let max_bar_w = (bounds[2] - 120.0).max(20.0);

                for (i, bar) in bars.iter().enumerate() {
                    let by = bounds[1] + pad_top + i as f32 * (bar_h + 6.0);
                    let bw = (bar.value / max_val * max_bar_w).clamp(4.0, max_bar_w);

                    // Label
                    frame.texts.push(TextSpec {
                        text: bar.label.clone(),
                        bounds: [bounds[0] + 8.0, by + 2.0, 60.0, bar_h],
                        font_size: 10.0,
                        color: theme.text_muted,
                        align: TextAlign::Left,
                        weight: FontWeight::Normal,
                        clip: chart_clip,
                    });

                    // Bar Track
                    let bx = bounds[0] + 72.0;
                    frame.instances.push(custom_glass_instance(
                        [bx, by, max_bar_w, bar_h],
                        chart_clip,
                        [0.08, 0.12, 0.18, 0.8],
                        [0.0, 0.0, 0.0, 0.0],
                        bar_h * 0.4,
                        0.0,
                        0.0,
                    ));

                    // Bar Active Fill
                    frame.instances.push(custom_glass_instance(
                        [bx, by, bw, bar_h],
                        chart_clip,
                        bar.color,
                        bar.color,
                        bar_h * 0.4,
                        0.0,
                        0.25,
                    ));

                    // Value text
                    frame.texts.push(TextSpec {
                        text: format!("{:.0}", bar.value),
                        bounds: [bx + max_bar_w + 8.0, by + 2.0, 40.0, bar_h],
                        font_size: 10.0,
                        color: theme.text_color,
                        align: TextAlign::Left,
                        weight: FontWeight::Bold,
                        clip: chart_clip,
                    });
                }
            } else {
                let avail_w = bounds[2] - 20.0;
                let bar_count = bars.len().max(1);
                let col_w = (avail_w / bar_count as f32).max(16.0);
                let bar_w = (col_w - 8.0).clamp(12.0, 36.0);
                let plot_h = (bounds[3] - pad_top - 28.0).max(20.0);
                let base_y = bounds[1] + bounds[3] - 20.0;

                for (i, bar) in bars.iter().enumerate() {
                    let bx = bounds[0] + 10.0 + i as f32 * col_w + (col_w - bar_w) * 0.5;
                    let bh = (bar.value / max_val * plot_h).clamp(4.0, plot_h);
                    let by = base_y - bh;

                    // Bar Fill
                    frame.instances.push(custom_glass_instance(
                        [bx, by, bar_w, bh],
                        chart_clip,
                        bar.color,
                        bar.color,
                        4.0,
                        0.0,
                        0.2,
                    ));

                    // Label at bottom
                    frame.texts.push(TextSpec {
                        text: bar.label.clone(),
                        bounds: [bx - 6.0, base_y + 4.0, bar_w + 12.0, 12.0],
                        font_size: 9.0,
                        color: theme.text_muted,
                        align: TextAlign::Center,
                        weight: FontWeight::Normal,
                        clip: chart_clip,
                    });
                }
            }
        }

        WidgetKind::RadialMeter {
            id: _,
            label,
            value,
            min,
            max,
            unit,
            color,
        } => {
            let meter_clip = crate::effective::intersect(clip, bounds);
            let cx = bounds[0] + bounds[2] * 0.5;
            let cy = bounds[1] + bounds[3] * 0.45;
            let radius = (bounds[2].min(bounds[3]) * 0.38).max(14.0);
            let arc_color = color.unwrap_or(theme.accent);

            // Outer dial track
            frame.instances.push(custom_glass_instance(
                [cx - radius, cy - radius, radius * 2.0, radius * 2.0],
                meter_clip,
                [0.08, 0.12, 0.18, 0.85],
                theme.border_subtle(),
                radius,
                1.5,
                0.05,
            ));

            // Inner disc
            let inner_r = radius * 0.72;
            frame.instances.push(custom_glass_instance(
                [cx - inner_r, cy - inner_r, inner_r * 2.0, inner_r * 2.0],
                meter_clip,
                [0.04, 0.06, 0.10, 0.95],
                theme.border_subtle(),
                inner_r,
                1.0,
                0.0,
            ));

            // Angle calculation
            let ratio = if *max > *min {
                ((*value - *min) / (*max - *min)).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let cur_angle = -std::f32::consts::PI * 0.75 + ratio * std::f32::consts::PI * 1.5;

            // Needle Pip Indicator
            let needle_dist = inner_r * 0.7;
            let nx = cx + cur_angle.cos() * needle_dist;
            let ny = cy + cur_angle.sin() * needle_dist;
            frame.instances.push(custom_glass_instance(
                [nx - 3.0, ny - 3.0, 6.0, 6.0],
                meter_clip,
                arc_color,
                arc_color,
                3.0,
                0.0,
                0.4,
            ));

            // Central Value text
            let val_str = if let Some(u) = unit {
                format!("{:.0}{}", value, u)
            } else {
                format!("{:.0}", value)
            };
            frame.texts.push(TextSpec {
                text: val_str,
                bounds: [cx - inner_r, cy - 8.0, inner_r * 2.0, 16.0],
                font_size: 11.0,
                color: theme.text_color,
                align: TextAlign::Center,
                weight: FontWeight::Bold,
                clip: meter_clip,
            });

            // Bottom Label
            if let Some(lbl) = label {
                frame.texts.push(TextSpec {
                    text: lbl.clone(),
                    bounds: [bounds[0], bounds[1] + bounds[3] - 16.0, bounds[2], 14.0],
                    font_size: 10.0,
                    color: theme.text_muted,
                    align: TextAlign::Center,
                    weight: FontWeight::Normal,
                    clip: meter_clip,
                });
            }
        }

        WidgetKind::DropZone {
            id: _,
            label,
            hint,
            accepted_extensions,
            hovered: is_hovered,
            dropped_file,
        } => {
            let drop_clip = crate::effective::intersect(clip, bounds);
            let is_active = *is_hovered || hovered;

            // Background container
            let bg_color = if is_active {
                [theme.accent[0] * 0.15, theme.accent[1] * 0.15, theme.accent[2] * 0.15, 0.90]
            } else if dropped_file.is_some() {
                [0.05, 0.12, 0.08, 0.85] // Subtle success tint
            } else {
                theme.card_bg()
            };

            let border_color = if is_active {
                theme.accent
            } else if dropped_file.is_some() {
                [0.2, 0.85, 0.45, 1.0] // Success green
            } else {
                theme.border_subtle()
            };

            let glow = if is_active { 0.40 } else { 0.05 };

            frame.instances.push(custom_glass_instance(
                bounds,
                drop_clip,
                bg_color,
                border_color,
                8.0,
                if is_active { 2.0 } else { 1.2 },
                glow,
            ));

            let center_y = bounds[1] + bounds[3] * 0.35;

            // Header Icon / Glyph
            let icon_glyph = if is_active {
                "▼"
            } else if dropped_file.is_some() {
                "✓"
            } else {
                "◫"
            };

            let icon_color = if is_active {
                theme.accent
            } else if dropped_file.is_some() {
                [0.2, 0.85, 0.45, 1.0]
            } else {
                theme.text_muted
            };

            frame.texts.push(TextSpec {
                text: icon_glyph.to_string(),
                bounds: [bounds[0], center_y - 18.0, bounds[2], 22.0],
                font_size: 18.0,
                color: icon_color,
                align: TextAlign::Center,
                weight: FontWeight::Bold,
                clip: drop_clip,
            });

            // Primary Label
            frame.texts.push(TextSpec {
                text: label.clone(),
                bounds: [bounds[0] + 12.0, center_y + 8.0, bounds[2] - 24.0, 16.0],
                font_size: 12.0,
                color: theme.text_color,
                align: TextAlign::Center,
                weight: FontWeight::Bold,
                clip: drop_clip,
            });

            // Secondary Hint / Dropped File Info
            if let Some((fname, fsize)) = dropped_file {
                let size_str = if *fsize > 1024 * 1024 {
                    format!("{:.2} MB", *fsize as f32 / (1024.0 * 1024.0))
                } else {
                    format!("{:.1} KB", *fsize as f32 / 1024.0)
                };
                let info_text = format!("Imported: {} ({})", fname, size_str);
                frame.texts.push(TextSpec {
                    text: info_text,
                    bounds: [bounds[0] + 8.0, center_y + 26.0, bounds[2] - 16.0, 14.0],
                    font_size: 10.0,
                    color: [0.2, 0.85, 0.45, 1.0],
                    align: TextAlign::Center,
                    weight: FontWeight::Normal,
                    clip: drop_clip,
                });
            } else if let Some(h) = hint {
                frame.texts.push(TextSpec {
                    text: h.clone(),
                    bounds: [bounds[0] + 8.0, center_y + 26.0, bounds[2] - 16.0, 14.0],
                    font_size: 10.0,
                    color: theme.text_muted,
                    align: TextAlign::Center,
                    weight: FontWeight::Normal,
                    clip: drop_clip,
                });
            }

            // Accepted Extensions Badges at Bottom
            if !accepted_extensions.is_empty() && bounds[3] >= 75.0 {
                let badges_str = accepted_extensions
                    .iter()
                    .map(|ext| format!("[{}]", ext.to_uppercase()))
                    .collect::<Vec<_>>()
                    .join(" ");
                frame.texts.push(TextSpec {
                    text: badges_str,
                    bounds: [bounds[0] + 8.0, bounds[1] + bounds[3] - 18.0, bounds[2] - 16.0, 12.0],
                    font_size: 9.0,
                    color: theme.text_muted,
                    align: TextAlign::Center,
                    weight: FontWeight::Normal,
                    clip: drop_clip,
                });
            }
        }

        WidgetKind::Avatar {
            id: _,
            resource_id,
            initials,
            status,
            size: _,
            ring_color,
            glow,
        } => {
            let cx = bounds[0] + bounds[2] * 0.5;
            let cy = bounds[1] + bounds[3] * 0.5;
            let r = (bounds[2].min(bounds[3]) * 0.5).max(8.0);
            let avatar_clip = crate::effective::intersect(clip, bounds);

            let ring = ring_color.unwrap_or(theme.accent);
            let glow_val = if *glow || hovered { 0.45 } else { 0.08 };

            // Circular Glass Base & Ring Border
            let bg_col = if hovered {
                [0.10, 0.15, 0.22, 0.96]
            } else {
                [0.06, 0.09, 0.14, 0.94]
            };
            frame.instances.push(custom_glass_instance(
                [cx - r, cy - r, r * 2.0, r * 2.0],
                avatar_clip,
                bg_col,
                ring,
                r,
                if hovered { 2.0 } else { 1.5 },
                glow_val,
            ));

            // Content: Image Media or Initials Monogram
            if let Some(ref res_id) = resource_id {
                let inner_r = (r - 1.5).max(4.0);
                frame.media.push(crate::media::MediaSpec {
                    resource_id: res_id.clone(),
                    kind: crate::media::MediaKind("image"),
                    bounds: [cx - inner_r, cy - inner_r, inner_r * 2.0, inner_r * 2.0],
                    clip: avatar_clip,
                    fit: crate::media::MediaFit::Cover,
                    radius: inner_r,
                });
            } else {
                let text_str = initials.clone().unwrap_or_else(|| "U".to_string());
                let font_size = (r * 0.82).clamp(10.0, 26.0);
                frame.texts.push(TextSpec {
                    text: text_str,
                    bounds: [cx - r, cy - font_size * 0.55, r * 2.0, font_size * 1.2],
                    font_size,
                    color: [1.0, 1.0, 1.0, 1.0],
                    align: TextAlign::Center,
                    weight: FontWeight::Bold,
                    clip: avatar_clip,
                });
            }

            // Presence Status Indicator Pip (bottom-right)
            if let Some(status_col) = status.indicator_color() {
                let dot_r = (r * 0.28).clamp(3.5, 7.5);
                let dot_cx = cx + r * 0.65;
                let dot_cy = cy + r * 0.65;

                // Dark Cutout Bezel
                let cutout_r = dot_r + 1.5;
                frame.instances.push(custom_glass_instance(
                    [dot_cx - cutout_r, dot_cy - cutout_r, cutout_r * 2.0, cutout_r * 2.0],
                    clip,
                    [0.02, 0.04, 0.08, 1.0],
                    [0.02, 0.04, 0.08, 1.0],
                    cutout_r,
                    0.0,
                    0.0,
                ));

                // Glowing Status Dot
                frame.instances.push(custom_glass_instance(
                    [dot_cx - dot_r, dot_cy - dot_r, dot_r * 2.0, dot_r * 2.0],
                    clip,
                    status_col,
                    status_col,
                    dot_r,
                    0.0,
                    0.5,
                ));
            }
        }

        WidgetKind::Stepper {
            id: _,
            steps,
            current_step,
        } => {
            let n = steps.len();
            if n == 0 {
                return;
            }
            let step_w = bounds[2] / n as f32;
            let node_r = 12.0;
            let line_y = bounds[1] + node_r + 2.0;

            // Connecting Lines Between Nodes
            for i in 0..n.saturating_sub(1) {
                let x1 = bounds[0] + (i as f32 + 0.5) * step_w + node_r;
                let x2 = bounds[0] + (i as f32 + 1.5) * step_w - node_r;
                let is_past = i < *current_step;
                let line_col = if is_past {
                    [theme.accent[0], theme.accent[1], theme.accent[2], 0.85]
                } else {
                    [0.15, 0.22, 0.32, 0.50]
                };
                frame.instances.push(custom_glass_instance(
                    [x1, line_y - 1.0, (x2 - x1).max(2.0), 2.0],
                    clip,
                    line_col,
                    line_col,
                    1.0,
                    0.0,
                    if is_past { 0.2 } else { 0.0 },
                ));
            }

            // Step Nodes & Labels
            for (idx, step) in steps.iter().enumerate() {
                let cx = bounds[0] + (idx as f32 + 0.5) * step_w;
                let is_current = idx == *current_step;
                let is_done = idx < *current_step || step.state == crate::kind::StepState::Completed;
                let is_error = step.state == crate::kind::StepState::Error;

                let (bg_col, ring_col, glow_val) = if is_error {
                    ([0.25, 0.08, 0.10, 0.95], [1.0, 0.28, 0.32, 1.0], 0.5)
                } else if is_done {
                    ([0.08, 0.22, 0.15, 0.95], [0.15, 0.92, 0.45, 1.0], 0.4)
                } else if is_current {
                    ([0.06, 0.18, 0.28, 0.98], theme.accent, 0.6)
                } else {
                    ([0.04, 0.08, 0.14, 0.85], [0.20, 0.28, 0.40, 0.60], 0.05)
                };

                // Circle Node
                frame.instances.push(custom_glass_instance(
                    [cx - node_r, line_y - node_r, node_r * 2.0, node_r * 2.0],
                    clip,
                    bg_col,
                    ring_col,
                    node_r,
                    if is_current { 2.0 } else { 1.5 },
                    glow_val,
                ));

                // Step Glyph / Number
                let glyph_text = if is_error {
                    "!".to_string()
                } else if is_done {
                    "✓".to_string()
                } else {
                    (idx + 1).to_string()
                };

                frame.texts.push(TextSpec {
                    text: glyph_text,
                    bounds: [cx - node_r, line_y - 7.0, node_r * 2.0, 14.0],
                    font_size: 11.0,
                    color: if is_done { [0.15, 0.92, 0.45, 1.0] } else if is_current { [1.0, 1.0, 1.0, 1.0] } else { theme.text_muted },
                    align: TextAlign::Center,
                    weight: FontWeight::Bold,
                    clip,
                });

                // Step Title & Subtitle Below Node
                let label_w = (step_w - 8.0).max(40.0);
                frame.texts.push(TextSpec {
                    text: step.label.clone(),
                    bounds: [cx - label_w * 0.5, line_y + node_r + 4.0, label_w, 14.0],
                    font_size: 10.0,
                    color: if is_current { [1.0, 1.0, 1.0, 1.0] } else { theme.text_muted },
                    align: TextAlign::Center,
                    weight: if is_current { FontWeight::Bold } else { FontWeight::Normal },
                    clip,
                });

                if let Some(ref desc) = step.description {
                    frame.texts.push(TextSpec {
                        text: desc.clone(),
                        bounds: [cx - label_w * 0.5, line_y + node_r + 18.0, label_w, 12.0],
                        font_size: 9.0,
                        color: theme.text_muted,
                        align: TextAlign::Center,
                        weight: FontWeight::Normal,
                        clip,
                    });
                }
            }
        }

        WidgetKind::Kbd { text } => {
            let kbd_clip = crate::effective::intersect(clip, bounds);
            let bg = [0.06, 0.10, 0.16, 0.95];
            let border_col = [theme.accent[0] * 0.5, theme.accent[1] * 0.5, theme.accent[2] * 0.5, 0.70];

            // 3D Shadow Bottom Line
            frame.instances.push(custom_glass_instance(
                [bounds[0], bounds[1] + bounds[3] - 2.0, bounds[2], 2.0],
                kbd_clip,
                [0.0, 0.0, 0.0, 0.7],
                [0.0, 0.0, 0.0, 0.0],
                2.0,
                0.0,
                0.0,
            ));

            // Main Beveled Key Body
            frame.instances.push(custom_glass_instance(
                [bounds[0], bounds[1], bounds[2], (bounds[3] - 1.5).max(4.0)],
                kbd_clip,
                bg,
                border_col,
                4.0,
                1.0,
                0.08,
            ));

            frame.texts.push(TextSpec {
                text: text.clone(),
                bounds: [bounds[0] + 4.0, bounds[1], (bounds[2] - 8.0).max(4.0), bounds[3] - 2.0],
                font_size: 10.0,
                color: [0.90, 0.94, 1.0, 1.0],
                align: TextAlign::Center,
                weight: FontWeight::Bold,
                clip: kbd_clip,
            });
        }

        WidgetKind::Chip {
            id: _,
            label,
            icon,
            color_pip,
            selected,
            dismissible,
            variant,
        } => {
            let r = bounds[3] * 0.5;
            let chip_clip = crate::effective::intersect(clip, bounds);

            let (bg_col, border_col, text_col, glow_val) = match variant {
                crate::kind::ChipVariant::Primary => {
                    if *selected || hovered {
                        ([0.08, 0.25, 0.35, 0.95], theme.accent, [1.0, 1.0, 1.0, 1.0], 0.35)
                    } else {
                        ([0.05, 0.12, 0.20, 0.85], [theme.accent[0] * 0.6, theme.accent[1] * 0.6, theme.accent[2] * 0.6, 0.6], [0.85, 0.93, 1.0, 0.9], 0.08)
                    }
                }
                crate::kind::ChipVariant::Success => {
                    ([0.06, 0.20, 0.12, 0.90], [0.15, 0.92, 0.45, 0.85], [0.20, 0.95, 0.50, 1.0], if *selected { 0.35 } else { 0.08 })
                }
                crate::kind::ChipVariant::Warning => {
                    ([0.22, 0.16, 0.05, 0.90], [1.0, 0.78, 0.12, 0.85], [1.0, 0.82, 0.20, 1.0], if *selected { 0.35 } else { 0.08 })
                }
                crate::kind::ChipVariant::Danger => {
                    ([0.22, 0.06, 0.08, 0.90], [1.0, 0.28, 0.32, 0.85], [1.0, 0.40, 0.45, 1.0], if *selected { 0.35 } else { 0.08 })
                }
                crate::kind::ChipVariant::Outline => {
                    ([0.03, 0.05, 0.08, 0.60], theme.accent_secondary, theme.text_color, if *selected { 0.3 } else { 0.0 })
                }
                crate::kind::ChipVariant::Default => {
                    if *selected || hovered {
                        ([0.12, 0.18, 0.28, 0.95], theme.accent, [1.0, 1.0, 1.0, 1.0], 0.25)
                    } else {
                        ([0.06, 0.09, 0.15, 0.88], [0.18, 0.25, 0.36, 0.60], theme.text_muted, 0.02)
                    }
                }
            };

            frame.instances.push(custom_glass_instance(
                bounds,
                clip,
                bg_col,
                border_col,
                r,
                if *selected || hovered { 1.5 } else { 1.0 },
                glow_val,
            ));

            let mut cur_x = bounds[0] + 8.0;

            // Optional Leading Pip or Icon
            if let Some(pip_color) = color_pip {
                let pip_r = 3.5;
                let pip_cy = bounds[1] + bounds[3] * 0.5;
                frame.instances.push(custom_glass_instance(
                    [cur_x, pip_cy - pip_r, pip_r * 2.0, pip_r * 2.0],
                    chip_clip,
                    *pip_color,
                    *pip_color,
                    pip_r,
                    0.0,
                    0.3,
                ));
                cur_x += pip_r * 2.0 + 5.0;
            } else if let Some(ico) = icon {
                frame.texts.push(TextSpec {
                    text: ico.glyph().to_string(),
                    bounds: [cur_x, bounds[1], 14.0, bounds[3]],
                    font_size: 10.0,
                    color: border_col,
                    align: TextAlign::Center,
                    weight: FontWeight::Normal,
                    clip: chip_clip,
                });
                cur_x += 16.0;
            }

            let text_w = if *dismissible { (bounds[2] - (cur_x - bounds[0]) - 20.0).max(10.0) } else { (bounds[2] - (cur_x - bounds[0]) - 8.0).max(10.0) };

            frame.texts.push(TextSpec {
                text: label.clone(),
                bounds: [cur_x, bounds[1], text_w, bounds[3]],
                font_size: 10.5,
                color: text_col,
                align: TextAlign::Left,
                weight: if *selected { FontWeight::Bold } else { FontWeight::Normal },
                clip: chip_clip,
            });

            if *dismissible {
                let close_x = bounds[0] + bounds[2] - 18.0;
                frame.texts.push(TextSpec {
                    text: "✕".to_string(),
                    bounds: [close_x, bounds[1], 12.0, bounds[3]],
                    font_size: 9.0,
                    color: if hovered { [1.0, 0.4, 0.45, 1.0] } else { theme.text_muted },
                    align: TextAlign::Center,
                    weight: FontWeight::Bold,
                    clip: chip_clip,
                });
            }
        }

        WidgetKind::Skeleton { radius, shimmer } => {
            let r = radius.unwrap_or(theme.corner_radius.min(6.0));
            let bg = [0.08, 0.13, 0.20, 0.75];
            let border_col = if *shimmer {
                [theme.accent[0] * 0.4, theme.accent[1] * 0.4, theme.accent[2] * 0.4, 0.40]
            } else {
                [0.10, 0.15, 0.22, 0.30]
            };
            frame.instances.push(custom_glass_instance(
                bounds,
                clip,
                bg,
                border_col,
                r,
                1.0,
                if *shimmer { 0.15 } else { 0.0 },
            ));
        }

        WidgetKind::MultiProgressBar {
            id: _,
            segments,
            show_labels,
        } => {
            let total: f32 = segments.iter().map(|s| s.value).sum();
            let bar_h = if *show_labels { (bounds[3] - 16.0).max(6.0) } else { bounds[3] };
            let bar_clip = crate::effective::intersect(clip, [bounds[0], bounds[1], bounds[2], bar_h]);

            // Background Track
            frame.instances.push(custom_glass_instance(
                [bounds[0], bounds[1], bounds[2], bar_h],
                clip,
                [0.03, 0.06, 0.10, 0.95],
                [0.12, 0.18, 0.28, 0.60],
                4.0,
                1.0,
                0.05,
            ));

            if total > 0.0 {
                let mut cur_x = bounds[0];
                for seg in segments {
                    let seg_w = (seg.value / total) * bounds[2];
                    if seg_w > 0.5 {
                        frame.instances.push(custom_glass_instance(
                            [cur_x, bounds[1], seg_w, bar_h],
                            bar_clip,
                            seg.color,
                            seg.color,
                            0.0,
                            0.0,
                            0.25,
                        ));
                        cur_x += seg_w;
                    }
                }
            }

            if *show_labels && bounds[3] >= 24.0 {
                let legend_y = bounds[1] + bar_h + 4.0;
                let mut leg_x = bounds[0] + 4.0;
                for seg in segments {
                    let dot_r = 3.0;
                    frame.instances.push(custom_glass_instance(
                        [leg_x, legend_y + 3.0, dot_r * 2.0, dot_r * 2.0],
                        clip,
                        seg.color,
                        seg.color,
                        dot_r,
                        0.0,
                        0.2,
                    ));
                    leg_x += dot_r * 2.0 + 4.0;

                    let txt = format!("{}: {:.0}%", seg.label, if total > 0.0 { (seg.value / total) * 100.0 } else { 0.0 });
                    let txt_w = txt.len() as f32 * 6.0 + 8.0;
                    frame.texts.push(TextSpec {
                        text: txt,
                        bounds: [leg_x, legend_y, txt_w, 12.0],
                        font_size: 9.0,
                        color: theme.text_muted,
                        align: TextAlign::Left,
                        weight: FontWeight::Normal,
                        clip,
                    });
                    leg_x += txt_w + 8.0;
                }
            }
        }

        WidgetKind::Rating {
            id: _,
            value,
            max,
            glyph,
            readonly: _,
        } => {
            let m = (*max).max(1) as usize;
            let item_w = bounds[2] / m as f32;
            let font_size = (item_w.min(bounds[3]) * 0.75).clamp(12.0, 24.0);

            for i in 1..=m {
                let is_filled = i <= *value as usize;
                let cx = bounds[0] + (i as f32 - 0.5) * item_w;
                let glyph_str = glyph.glyph(is_filled);
                let col = if is_filled {
                    [1.0, 0.80, 0.15, 1.0] // Glowing Amber Gold
                } else {
                    [0.35, 0.42, 0.52, 0.60] // Slate Outline
                };

                frame.texts.push(TextSpec {
                    text: glyph_str.to_string(),
                    bounds: [cx - font_size * 0.6, bounds[1] + (bounds[3] - font_size) * 0.5, font_size * 1.2, font_size],
                    font_size,
                    color: col,
                    align: TextAlign::Center,
                    weight: FontWeight::Bold,
                    clip,
                });
            }
        }

        WidgetKind::Timeline { id: _, items } => {
            let line_x = bounds[0] + 16.0;
            let line_clip = crate::effective::intersect(clip, bounds);

            // Vertical Connecting Track
            if !items.is_empty() {
                frame.instances.push(custom_glass_instance(
                    [line_x - 1.0, bounds[1] + 10.0, 2.0, (bounds[3] - 20.0).max(4.0)],
                    line_clip,
                    [0.12, 0.18, 0.28, 0.70],
                    [0.12, 0.18, 0.28, 0.70],
                    1.0,
                    0.0,
                    0.0,
                ));
            }

            let item_h = (bounds[3] / items.len().max(1) as f32).max(36.0);
            for (idx, it) in items.iter().enumerate() {
                let node_y = bounds[1] + idx as f32 * item_h + 12.0;
                let node_r = 5.0;
                let node_col = it.status.color();

                // Glowing Event Node Pip
                frame.instances.push(custom_glass_instance(
                    [line_x - node_r, node_y - node_r, node_r * 2.0, node_r * 2.0],
                    line_clip,
                    node_col,
                    node_col,
                    node_r,
                    0.0,
                    0.4,
                ));

                // Time & Title
                let text_x = line_x + 16.0;
                let text_w = (bounds[2] - (text_x - bounds[0]) - 8.0).max(20.0);

                let header_str = format!("{} • {}", it.time, it.title);
                frame.texts.push(TextSpec {
                    text: header_str,
                    bounds: [text_x, node_y - 8.0, text_w, 14.0],
                    font_size: 11.0,
                    color: [0.92, 0.96, 1.0, 1.0],
                    align: TextAlign::Left,
                    weight: FontWeight::Bold,
                    clip: line_clip,
                });

                if let Some(ref desc) = it.description {
                    frame.texts.push(TextSpec {
                        text: desc.clone(),
                        bounds: [text_x, node_y + 6.0, text_w, 12.0],
                        font_size: 9.5,
                        color: theme.text_muted,
                        align: TextAlign::Left,
                        weight: FontWeight::Normal,
                        clip: line_clip,
                    });
                }
            }
        }
    }
}

fn render_custom_paint_line(
    from: [f32; 2],
    to: [f32; 2],
    stroke_width: f32,
    color: [f32; 4],
    clip: [f32; 4],
    frame: &mut Frame,
) {
    let dx = to[0] - from[0];
    let dy = to[1] - from[1];
    let len = (dx * dx + dy * dy).sqrt();
    let r = (stroke_width * 0.5).max(0.5);

    if len < 0.5 {
        let b = [
            from[0] - r,
            from[1] - r,
            stroke_width.max(1.0),
            stroke_width.max(1.0),
        ];
        frame
            .instances
            .push(custom_glass_instance(b, clip, color, color, r, 0.0, 0.0));
        return;
    }

    let step_size = (stroke_width * 0.35).clamp(0.6, 1.8);
    let steps = ((len / step_size).ceil() as usize).max(1);

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let x = from[0] + dx * t;
        let y = from[1] + dy * t;
        let b = [x - r, y - r, stroke_width.max(1.0), stroke_width.max(1.0)];
        frame
            .instances
            .push(custom_glass_instance(b, clip, color, color, r, 0.0, 0.0));
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
        'a'
        | 'b'
        | 'd'
        | 'g'
        | 'h'
        | 'n'
        | 'o'
        | 'p'
        | 'q'
        | 'u'
        | '0'..='9'
        | 'é'
        | 'è'
        | 'ê'
        | 'ë'
        | 'à'
        | 'â'
        | 'î'
        | 'ï'
        | 'ô'
        | 'ù'
        | 'û'
        | 'ç' => 0.50,
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
    text.chars()
        .map(|ch| estimate_char_width(ch, font_size))
        .sum()
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
