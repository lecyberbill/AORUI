// [WFGY] Zone: SAFE | λ: 0.1 | Fallbacks: 0 | Action: Bidirectional color conversions (RGB, HEX, HSV, CMYK, CIELAB)

/// Canonical RGBA color with channels normalized to `[0.0, 1.0]`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// Color space presentation modes for the ColorPicker widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
    Rgb,
    Hex,
    Lab,
    Cmyk,
}

impl Color {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    pub fn from_array(rgba: [f32; 4]) -> Self {
        Self {
            r: rgba[0].clamp(0.0, 1.0),
            g: rgba[1].clamp(0.0, 1.0),
            b: rgba[2].clamp(0.0, 1.0),
            a: rgba[3].clamp(0.0, 1.0),
        }
    }

    pub fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    // --- RGB (0..255) ---
    pub fn from_rgb_u8(r: u8, g: u8, b: u8, a: f32) -> Self {
        Self {
            r: (r as f32) / 255.0,
            g: (g as f32) / 255.0,
            b: (b as f32) / 255.0,
            a: a.clamp(0.0, 1.0),
        }
    }

    pub fn to_rgb_u8(self) -> (u8, u8, u8) {
        (
            (self.r.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.g.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.b.clamp(0.0, 1.0) * 255.0).round() as u8,
        )
    }

    // --- HEX (#RRGGBB / #RRGGBBAA) ---
    pub fn to_hex(self) -> String {
        let (r, g, b) = self.to_rgb_u8();
        format!("#{:02X}{:02X}{:02X}", r, g, b)
    }

    pub fn to_hex_with_alpha(self) -> String {
        let (r, g, b) = self.to_rgb_u8();
        let a = (self.a.clamp(0.0, 1.0) * 255.0).round() as u8;
        format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        let clean = hex.trim().trim_start_matches('#');
        match clean.len() {
            6 => {
                let r = u8::from_str_radix(&clean[0..2], 16).ok()?;
                let g = u8::from_str_radix(&clean[2..4], 16).ok()?;
                let b = u8::from_str_radix(&clean[4..6], 16).ok()?;
                Some(Self::from_rgb_u8(r, g, b, 1.0))
            }
            8 => {
                let r = u8::from_str_radix(&clean[0..2], 16).ok()?;
                let g = u8::from_str_radix(&clean[2..4], 16).ok()?;
                let b = u8::from_str_radix(&clean[4..6], 16).ok()?;
                let a = u8::from_str_radix(&clean[6..8], 16).ok()?;
                Some(Self::from_rgb_u8(r, g, b, (a as f32) / 255.0))
            }
            3 => {
                let r = u8::from_str_radix(&clean[0..1].repeat(2), 16).ok()?;
                let g = u8::from_str_radix(&clean[1..2].repeat(2), 16).ok()?;
                let b = u8::from_str_radix(&clean[2..3].repeat(2), 16).ok()?;
                Some(Self::from_rgb_u8(r, g, b, 1.0))
            }
            _ => None,
        }
    }

    // --- HSV (Hue: 0..360, Saturation: 0..1, Value: 0..1) ---
    pub fn to_hsv(self) -> (f32, f32, f32) {
        let r = self.r;
        let g = self.g;
        let b = self.b;

        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;

        let v = max;
        let s = if max > 0.0 { delta / max } else { 0.0 };

        let h = if delta.abs() < 1e-5 {
            0.0
        } else if (max - r).abs() < 1e-5 {
            60.0 * (((g - b) / delta) % 6.0)
        } else if (max - g).abs() < 1e-5 {
            60.0 * (((b - r) / delta) + 2.0)
        } else {
            60.0 * (((r - g) / delta) + 4.0)
        };

        let h = if h < 0.0 { h + 360.0 } else { h };
        (h, s, v)
    }

    pub fn from_hsv(h: f32, s: f32, v: f32, a: f32) -> Self {
        let h = (h % 360.0 + 360.0) % 360.0;
        let s = s.clamp(0.0, 1.0);
        let v = v.clamp(0.0, 1.0);

        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;

        let (r1, g1, b1) = match (h / 60.0) as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };

        Self {
            r: r1 + m,
            g: g1 + m,
            b: b1 + m,
            a: a.clamp(0.0, 1.0),
        }
    }

    // --- CMYK (C: 0..100%, M: 0..100%, Y: 0..100%, K: 0..100%) ---
    pub fn to_cmyk(self) -> (f32, f32, f32, f32) {
        let r = self.r.clamp(0.0, 1.0);
        let g = self.g.clamp(0.0, 1.0);
        let b = self.b.clamp(0.0, 1.0);

        let k = 1.0 - r.max(g).max(b);
        if (1.0 - k).abs() < 1e-5 {
            (0.0, 0.0, 0.0, 100.0)
        } else {
            let c = (1.0 - r - k) / (1.0 - k) * 100.0;
            let m = (1.0 - g - k) / (1.0 - k) * 100.0;
            let y = (1.0 - b - k) / (1.0 - k) * 100.0;
            (
                c.clamp(0.0, 100.0),
                m.clamp(0.0, 100.0),
                y.clamp(0.0, 100.0),
                (k * 100.0).clamp(0.0, 100.0),
            )
        }
    }

    pub fn from_cmyk(c: f32, m: f32, y: f32, k: f32, a: f32) -> Self {
        let c = (c / 100.0).clamp(0.0, 1.0);
        let m_val = (m / 100.0).clamp(0.0, 1.0);
        let y_val = (y / 100.0).clamp(0.0, 1.0);
        let k_val = (k / 100.0).clamp(0.0, 1.0);

        let r = (1.0 - c) * (1.0 - k_val);
        let g = (1.0 - m_val) * (1.0 - k_val);
        let b = (1.0 - y_val) * (1.0 - k_val);

        Self {
            r: r.clamp(0.0, 1.0),
            g: g.clamp(0.0, 1.0),
            b: b.clamp(0.0, 1.0),
            a: a.clamp(0.0, 1.0),
        }
    }

    // --- CIELAB (L*: 0..100, a*: -128..+127, b*: -128..+127 via sRGB D65) ---
    pub fn to_lab(self) -> (f32, f32, f32) {
        // 1. Linearize sRGB
        let to_linear = |c: f32| -> f32 {
            if c <= 0.04045 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };

        let rl = to_linear(self.r);
        let gl = to_linear(self.g);
        let bl = to_linear(self.b);

        // 2. sRGB -> CIE XYZ (D65)
        let x = rl * 0.4124564 + gl * 0.3575761 + bl * 0.1804375;
        let y = rl * 0.2126729 + gl * 0.7151522 + bl * 0.0721750;
        let z = rl * 0.0193339 + gl * 0.1191920 + bl * 0.9503041;

        // D65 reference white
        let xn = 0.95047;
        let yn = 1.00000;
        let zn = 1.08883;

        let f = |t: f32| -> f32 {
            if t > 0.008856 {
                t.cbrt()
            } else {
                7.787 * t + (16.0 / 116.0)
            }
        };

        let fx = f(x / xn);
        let fy = f(y / yn);
        let fz = f(z / zn);

        let l = (116.0 * fy) - 16.0;
        let a_val = 500.0 * (fx - fy);
        let b_val = 200.0 * (fy - fz);

        (
            l.clamp(0.0, 100.0),
            a_val.clamp(-128.0, 127.0),
            b_val.clamp(-128.0, 127.0),
        )
    }

    pub fn from_lab(l: f32, a_val: f32, b_val: f32, a: f32) -> Self {
        let yn = 1.00000;
        let xn = 0.95047;
        let zn = 1.08883;

        let fy = (l + 16.0) / 116.0;
        let fx = fy + (a_val / 500.0);
        let fz = fy - (b_val / 200.0);

        let finv = |t: f32| -> f32 {
            let t3 = t * t * t;
            if t3 > 0.008856 {
                t3
            } else {
                (t - (16.0 / 116.0)) / 7.787
            }
        };

        let x = xn * finv(fx);
        let y = yn * finv(fy);
        let z = zn * finv(fz);

        // XYZ -> Linear sRGB
        let rl = x * 3.2404542 - y * 1.5371385 - z * 0.4985314;
        let gl = -x * 0.9692660 + y * 1.8760108 + z * 0.0415560;
        let bl = x * 0.0556434 - y * 0.2040259 + z * 1.0572252;

        let to_gamma = |c: f32| -> f32 {
            if c <= 0.0031308 {
                12.92 * c
            } else {
                1.055 * c.powf(1.0 / 2.4) - 0.055
            }
        };

        Self {
            r: to_gamma(rl).clamp(0.0, 1.0),
            g: to_gamma(gl).clamp(0.0, 1.0),
            b: to_gamma(bl).clamp(0.0, 1.0),
            a: a.clamp(0.0, 1.0),
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self {
            r: 0.0,
            g: 0.85,
            b: 1.0,
            a: 1.0,
        } // Cyber Neon Cyan
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip() {
        let c = Color::from_hex("#00D9FF").unwrap();
        assert_eq!(c.to_rgb_u8(), (0, 217, 255));
        assert_eq!(c.to_hex(), "#00D9FF");

        let c_alpha = Color::from_hex("#FF55AA80").unwrap();
        assert_eq!(c_alpha.to_rgb_u8(), (255, 85, 170));
        assert!((c_alpha.a - 0.5019).abs() < 0.01);
    }

    #[test]
    fn hsv_round_trip() {
        let cyan = Color::from_hex("#00D9FF").unwrap();
        let (h, s, v) = cyan.to_hsv();
        assert!((h - 188.94).abs() < 1.0);
        assert!((s - 1.0).abs() < 0.01);
        assert!((v - 1.0).abs() < 0.01);

        let reconstructed = Color::from_hsv(h, s, v, 1.0);
        assert_eq!(reconstructed.to_rgb_u8(), (0, 217, 255));
    }

    #[test]
    fn cmyk_conversion() {
        let red = Color::new(1.0, 0.0, 0.0, 1.0);
        let (c, m, y, k) = red.to_cmyk();
        assert_eq!((c as u32, m as u32, y as u32, k as u32), (0, 100, 100, 0));

        let cyan = Color::new(0.0, 1.0, 1.0, 1.0);
        let (c, m, y, k) = cyan.to_cmyk();
        assert_eq!((c as u32, m as u32, y as u32, k as u32), (100, 0, 0, 0));
    }

    #[test]
    fn lab_conversion() {
        let white = Color::new(1.0, 1.0, 1.0, 1.0);
        let (l, a, b) = white.to_lab();
        assert!((l - 100.0).abs() < 0.5);
        assert!(a.abs() < 0.5);
        assert!(b.abs() < 0.5);

        let recon = Color::from_lab(l, a, b, 1.0);
        assert_eq!(recon.to_rgb_u8(), (255, 255, 255));
    }
}
