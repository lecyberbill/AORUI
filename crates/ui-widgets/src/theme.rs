// [WFGY] Zone: TRANSIT | λ: 0.2 | Fallbacks: 0 | Action: Centralized theme and typography definitions (family, sizing hierarchy, font weight, color tokens) with TOML/hex serde
use serde::{Deserialize, Serialize};
use std::path::Path;

pub mod color_serde {
    use serde::{de::Error, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(color: &[f32; 4], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let linear_to_srgb = |val: f32| -> f32 {
            if val <= 0.0031308 {
                val * 12.92
            } else {
                1.055 * val.powf(1.0 / 2.4) - 0.055
            }
        };

        let r = (linear_to_srgb(color[0].clamp(0.0, 1.0)) * 255.0).round() as u8;
        let g = (linear_to_srgb(color[1].clamp(0.0, 1.0)) * 255.0).round() as u8;
        let b = (linear_to_srgb(color[2].clamp(0.0, 1.0)) * 255.0).round() as u8;
        let a = (color[3].clamp(0.0, 1.0) * 255.0).round() as u8;
        if a == 255 {
            serializer.serialize_str(&format!("#{:02x}{:02x}{:02x}", r, g, b))
        } else {
            serializer.serialize_str(&format!("#{:02x}{:02x}{:02x}{:02x}", r, g, b, a))
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[f32; 4], D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ColorValue {
            FloatArray([f32; 4]),
            Hex(String),
        }

        match ColorValue::deserialize(deserializer)? {
            ColorValue::FloatArray(arr) => Ok(arr),
            ColorValue::Hex(s) => parse_hex_color(&s).map_err(D::Error::custom),
        }
    }

    pub fn parse_hex_color(s: &str) -> Result<[f32; 4], String> {
        let hex = s.strip_prefix('#').unwrap_or(s);
        let srgb_to_linear = |val: f32| -> f32 {
            if val <= 0.04045 {
                val / 12.92
            } else {
                ((val + 0.055) / 1.055).powf(2.4)
            }
        };

        match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).map_err(|e| format!("Invalid hex red '{s}': {e}"))?;
                let g = u8::from_str_radix(&hex[2..4], 16).map_err(|e| format!("Invalid hex green '{s}': {e}"))?;
                let b = u8::from_str_radix(&hex[4..6], 16).map_err(|e| format!("Invalid hex blue '{s}': {e}"))?;
                Ok([
                    srgb_to_linear(r as f32 / 255.0),
                    srgb_to_linear(g as f32 / 255.0),
                    srgb_to_linear(b as f32 / 255.0),
                    1.0,
                ])
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).map_err(|e| format!("Invalid hex red '{s}': {e}"))?;
                let g = u8::from_str_radix(&hex[2..4], 16).map_err(|e| format!("Invalid hex green '{s}': {e}"))?;
                let b = u8::from_str_radix(&hex[4..6], 16).map_err(|e| format!("Invalid hex blue '{s}': {e}"))?;
                let a = u8::from_str_radix(&hex[6..8], 16).map_err(|e| format!("Invalid hex alpha '{s}': {e}"))?;
                Ok([
                    srgb_to_linear(r as f32 / 255.0),
                    srgb_to_linear(g as f32 / 255.0),
                    srgb_to_linear(b as f32 / 255.0),
                    a as f32 / 255.0,
                ])
            }
            len => Err(format!("Expected 6 or 8 character hex color (got {len} characters in '{s}')")),
        }
    }
}

/// Font family specification. Native to `ui-widgets` without dependencies
/// on `cosmic-text`/`fontdb` (extended INV-CORE-1) — translated by the renderer integration layer (`ui-gpu`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FontFamily {
    SansSerif,
    Serif,
    Monospace,
    /// Exact name of an installed font family (e.g. "Segoe UI").
    /// Falls back gracefully to system default if absent.
    Named(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontWeight {
    Normal,
    Bold,
}

/// Typography hierarchy: unified font family across the UI with distinct
/// sizes and weights configured by text semantic role.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

    /// Compact, crisp typography for developer studios, IDEs and creator tools.
    pub fn studio_pro() -> Self {
        Self {
            family: FontFamily::SansSerif,
            title_size: 14.0,
            body_size: 12.0,
            small_size: 11.0,
            caption_size: 9.5,
            heading_weight: FontWeight::Bold,
            body_weight: FontWeight::Normal,
        }
    }
}

fn hex(s: &str) -> [f32; 4] {
    color_serde::parse_hex_color(s).expect("valid theme hex color definition")
}

/// Visual parameters shared across glassmorphism UI elements (translucent surface + neon glow halo).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Theme {
    #[serde(with = "color_serde")]
    pub glass_bg: [f32; 4],
    #[serde(with = "color_serde")]
    pub accent: [f32; 4],
    #[serde(with = "color_serde")]
    pub accent_secondary: [f32; 4],
    #[serde(with = "color_serde")]
    pub danger: [f32; 4],
    #[serde(with = "color_serde")]
    pub success: [f32; 4],
    #[serde(with = "color_serde")]
    pub warning: [f32; 4],
    #[serde(with = "color_serde")]
    pub text_color: [f32; 4],
    #[serde(with = "color_serde")]
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
            glass_bg: hex("#0f1624d9"),                  // #0f1624 with 85% alpha
            accent: hex("#00e0fa"),                      // Luminescent Neon Cyan
            accent_secondary: hex("#b861fa"),            // Electric Purple
            danger: hex("#ff4761"),                      // Coral Red
            success: hex("#2ee07a"),                     // Emerald Neon
            warning: hex("#fac02e"),                     // Amber Gold
            text_color: hex("#f0f7ff"),                  // Ice White
            text_muted: hex("#99add1"),                  // Muted Slate
            corner_radius: 14.0,
            border_width: 1.2,
            glow_intensity: 0.12,
            glow_intensity_hover: 0.35,
            typography: Typography::cyber_glass(),
        }
    }

    /// Studio Pro theme: Rich Midnight Navy Blue (#081026), vivid cyan accent (#2ee8d6), electric violet (#9447eb), 6-8px radius, micro-borders.
    pub fn studio_pro() -> Self {
        Self {
            glass_bg: hex("#081026fa"),                  // #081026 (Rich Midnight Navy Blue, 98% alpha)
            accent: hex("#2ee8d6"),                      // #2ee8d6 (Luminescent Bright Cyan)
            accent_secondary: hex("#9447eb"),            // #9447eb (Electric Violet/Purple)
            danger: hex("#ff5270"),                      // #ff5270 (Vibrant Coral Red)
            success: hex("#2eeb85"),                     // #2eeb85 (Bright Emerald Neon)
            warning: hex("#fcc72e"),                     // #fcc72e (Warm Amber Gold)
            text_color: hex("#f2f7ff"),                  // #f2f7ff (Crisp Ice White)
            text_muted: hex("#7a95be"),                  // #7a95be (Soft Slate Indigo Blue)
            corner_radius: 6.0,
            border_width: 1.0,
            glow_intensity: 0.08,
            glow_intensity_hover: 0.30,
            typography: Typography::studio_pro(),
        }
    }

    /// AetherOS Flagship Theme: Deep Midnight Navy Atmospheric Glassmorphism (#111624), Periwinkle (#c0c1ff), Cyan Ray (#7bd0ff), Emerald (#4edea3), Rose (#ffb4ab).
    pub fn aether_os() -> Self {
        Self {
            glass_bg: hex("#111624f2"),                  // #111624 with 95% alpha (Deep Midnight Navy Oceanic Slate)
            accent: hex("#c0c1ff"),                      // #c0c1ff (Periwinkle Indigo)
            accent_secondary: hex("#7bd0ff"),            // #7bd0ff (Cyan Ray)
            danger: hex("#ffb4ab"),                      // #ffb4ab (Error Rose)
            success: hex("#4edea3"),                     // #4edea3 (Emerald Sync)
            warning: hex("#f59e0b"),                     // #f59e0b (Amber Warning)
            text_color: hex("#dfe2f1"),                  // #dfe2f1 (Crisp Light Text)
            text_muted: hex("#8d99b2"),                  // #8d99b2 (Soft Slate Gray)
            corner_radius: 10.0,
            border_width: 1.0,
            glow_intensity: 0.05,
            glow_intensity_hover: 0.22,
            typography: Typography::studio_pro(),
        }
    }

    /// Canonical window background (Deep Obsidian Slate #070A12)
    pub fn window_bg(&self) -> [f32; 4] {
        hex("#070a12")
    }

    /// Clear color for wgpu render pass
    pub fn window_clear_color(&self) -> (f64, f64, f64) {
        let wb = self.window_bg();
        (wb[0] as f64, wb[1] as f64, wb[2] as f64)
    }

    /// Elevated card container surface (#151C2CF8)
    pub fn card_bg(&self) -> [f32; 4] {
        hex("#151c2cf8")
    }

    /// Translucent floating dock surface (#0d1322f2)
    pub fn floating_dock_bg(&self) -> [f32; 4] {
        hex("#0d1322f2")
    }

    /// Subtle 1px micro-border outline (#1e283cf0)
    pub fn border_subtle(&self) -> [f32; 4] {
        hex("#1e283cf0")
    }

    /// Focused / selected border outline
    pub fn border_highlight(&self) -> [f32; 4] {
        [self.accent_secondary[0], self.accent_secondary[1], self.accent_secondary[2], 0.9]
    }

    /// Parses a theme definition from a TOML string.
    pub fn from_toml(content: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(content)
    }

    /// Serializes the theme to a TOML formatted string.
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    /// Loads a theme definition from a TOML file path.
    pub fn from_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let theme = Self::from_toml(&content)?;
        Ok(theme)
    }

    /// Serializes and saves the theme to a file path.
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let content = self.to_toml()?;
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)?;
        Ok(())
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::aether_os()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color_6char() {
        let col = color_serde::parse_hex_color("#38d9d4").unwrap();
        // 0.22 sRGB -> 0.039 linear, 0.85 sRGB -> 0.689 linear, 0.83 sRGB -> 0.654 linear
        assert!((col[0] - 0.039).abs() < 0.02);
        assert!((col[1] - 0.689).abs() < 0.02);
        assert!((col[2] - 0.654).abs() < 0.02);
        assert_eq!(col[3], 1.0);
    }

    #[test]
    fn test_parse_hex_color_8char() {
        let col = color_serde::parse_hex_color("#0b132beb").unwrap();
        assert!((col[0] - 0.003).abs() < 0.01);
        assert!((col[1] - 0.007).abs() < 0.01);
        assert!((col[2] - 0.024).abs() < 0.01);
        assert!((col[3] - 0.9215).abs() < 0.01);
    }

    #[test]
    fn test_parse_hex_color_without_hash() {
        let col = color_serde::parse_hex_color("ffffff").unwrap();
        assert_eq!(col, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_parse_hex_color_invalid() {
        assert!(color_serde::parse_hex_color("#12345").is_err());
        assert!(color_serde::parse_hex_color("#xyz123").is_err());
    }

    #[test]
    fn test_theme_toml_roundtrip() {
        let studio = Theme::studio_pro();
        let toml_str = studio.to_toml().expect("serialize to toml");
        let parsed = Theme::from_toml(&toml_str).expect("deserialize from toml");
        assert_eq!(studio.corner_radius, parsed.corner_radius);
        assert_eq!(studio.border_width, parsed.border_width);
        assert_eq!(studio.typography, parsed.typography);
        for i in 0..4 {
            assert!((studio.glass_bg[i] - parsed.glass_bg[i]).abs() < 0.02);
            assert!((studio.accent[i] - parsed.accent[i]).abs() < 0.02);
        }
    }

    #[test]
    fn test_theme_toml_float_array_backward_compat() {
        let toml_str = r#"
            glass_bg = [0.10, 0.13, 0.19, 0.85]
            accent = [0.0, 0.88, 0.98, 1.0]
            accent_secondary = [0.72, 0.38, 0.98, 1.0]
            danger = [1.0, 0.28, 0.38, 1.0]
            success = [0.18, 0.88, 0.48, 1.0]
            warning = [0.98, 0.75, 0.18, 1.0]
            text_color = [0.94, 0.97, 1.0, 1.0]
            text_muted = [0.60, 0.68, 0.78, 1.0]
            corner_radius = 14.0
            border_width = 1.2
            glow_intensity = 0.12
            glow_intensity_hover = 0.35

            [typography]
            family = "SansSerif"
            title_size = 20.0
            body_size = 15.0
            small_size = 13.0
            caption_size = 11.5
            heading_weight = "Bold"
            body_weight = "Normal"
        "#;
        let theme = Theme::from_toml(toml_str).expect("deserialize float array toml");
        assert_eq!(theme.corner_radius, 14.0);
        assert_eq!(theme.accent, [0.0, 0.88, 0.98, 1.0]);
    }
}
