// [WFGY] Zone: TRANSIT | λ: 0.2 | Fallbacks: 0 | Action: Centralized theme and typography definitions (family, sizing hierarchy, font weight, color tokens)
/// Font family specification. Native to `ui-widgets` without dependencies
/// on `cosmic-text`/`fontdb` (extended INV-CORE-1) — translated by the renderer integration layer (`ui-gpu`).
#[derive(Debug, Clone, PartialEq)]
pub enum FontFamily {
    SansSerif,
    Serif,
    Monospace,
    /// Exact name of an installed font family (e.g. "Segoe UI").
    /// Falls back gracefully to system default if absent.
    Named(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontWeight {
    Normal,
    Bold,
}

/// Typography hierarchy: unified font family across the UI with distinct
/// sizes and weights configured by text semantic role.
#[derive(Debug, Clone, PartialEq)]
pub struct Typography {
    pub family: FontFamily,
    /// Window title typography size.
    pub title_size: f32,
    /// Body text size: labels, buttons, text fields.
    pub body_size: f32,
    /// Secondary compact text: tabs, list row items, menus.
    pub small_size: f32,
    /// Captions, subtitles, and descriptions.
    pub caption_size: f32,
    pub heading_weight: FontWeight,
    pub body_weight: FontWeight,
}

impl Typography {
    pub fn cyber_glass() -> Self {
        Self {
            family: FontFamily::SansSerif,
            title_size: 20.0,
            body_size: 15.0,
            small_size: 13.0,
            caption_size: 11.5,
            heading_weight: FontWeight::Bold,
            body_weight: FontWeight::Normal,
        }
    }
}

/// Visual parameters shared across glassmorphism UI elements (translucent surface + neon glow halo).
#[derive(Debug, Clone, PartialEq)]
pub struct Theme {
    pub glass_bg: [f32; 4],
    pub accent: [f32; 4],
    pub accent_secondary: [f32; 4],
    pub danger: [f32; 4],
    pub success: [f32; 4],
    pub warning: [f32; 4],
    pub text_color: [f32; 4],
    pub text_muted: [f32; 4],
    pub corner_radius: f32,
    pub border_width: f32,
    pub glow_intensity: f32,
    pub glow_intensity_hover: f32,
    pub typography: Typography,
}

impl Theme {
    /// Default cyber-glass theme: neon cyan accents on translucent dark glass.
    pub fn cyber_glass() -> Self {
        Self {
            glass_bg: [0.10, 0.13, 0.19, 0.78],
            accent: [0.0, 0.88, 0.98, 1.0],
            accent_secondary: [0.72, 0.38, 0.98, 1.0],
            danger: [1.0, 0.28, 0.38, 1.0],
            success: [0.18, 0.88, 0.48, 1.0],
            warning: [0.98, 0.75, 0.18, 1.0],
            text_color: [0.94, 0.97, 1.0, 1.0],
            text_muted: [0.60, 0.68, 0.78, 1.0],
            corner_radius: 14.0,
            border_width: 1.2,
            glow_intensity: 0.12,
            glow_intensity_hover: 0.35,
            typography: Typography::cyber_glass(),
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::cyber_glass()
    }
}
