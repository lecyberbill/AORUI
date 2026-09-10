// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0 | Action: Implémentation CosmicTextMeasure exacte pour caret_x, hit_test et shaping
use std::sync::{Arc, Mutex};

use glyphon::cosmic_text::{Attrs, Buffer, Cursor, Family, FontSystem, Metrics, Shaping};
use ui_widgets::{FontFamily, TextMeasure};

/// Moteur de mesure vectorielle subpixel basé sur `cosmic-text`.
/// Fournit un calcul à 0% de dérive pour le caret, la sélection et le hit-testing.
#[derive(Clone)]
pub struct CosmicTextMeasure {
    font_system: Arc<Mutex<FontSystem>>,
}

impl CosmicTextMeasure {
    pub fn new() -> Self {
        Self {
            font_system: Arc::new(Mutex::new(FontSystem::new())),
        }
    }

    fn shape_buffer(&self, text: &str, family: &FontFamily, font_size: f32) -> Buffer {
        let mut fs = self.font_system.lock().unwrap();
        let metrics = Metrics::new(font_size, font_size * 1.3);
        let mut buffer = Buffer::new(&mut fs, metrics);
        buffer.set_size(&mut fs, 10000.0, 1000.0);

        let cosmic_family = match family {
            FontFamily::SansSerif => Family::SansSerif,
            FontFamily::Serif => Family::Serif,
            FontFamily::Monospace => Family::Monospace,
            FontFamily::Named(name) => Family::Name(name),
        };
        let attrs = Attrs::new().family(cosmic_family);
        buffer.set_text(&mut fs, text, attrs, Shaping::Advanced);
        buffer.shape_until_scroll(&mut fs);
        buffer
    }
}

impl Default for CosmicTextMeasure {
    fn default() -> Self {
        Self::new()
    }
}

impl TextMeasure for CosmicTextMeasure {
    fn measure_width(&self, text: &str, font_family: &FontFamily, font_size: f32) -> f32 {
        if text.is_empty() {
            return 0.0;
        }
        let buffer = self.shape_buffer(text, font_family, font_size);
        buffer
            .layout_runs()
            .map(|run| run.line_w)
            .fold(0.0, f32::max)
    }

    fn caret_x(
        &self,
        text: &str,
        font_family: &FontFamily,
        font_size: f32,
        byte_offset: usize,
    ) -> f32 {
        if text.is_empty() || byte_offset == 0 {
            return 0.0;
        }
        let buffer = self.shape_buffer(text, font_family, font_size);
        let safe_offset = byte_offset.min(text.len());

        // Identifier la ligne et l'offset local dans cette ligne
        let mut line_idx = 0;
        let mut line_start = 0;
        for (i, buffer_line) in buffer.lines.iter().enumerate() {
            let line_len = buffer_line.text().len();
            if safe_offset <= line_start + line_len || i == buffer.lines.len() - 1 {
                line_idx = i;
                break;
            }
            line_start += line_len + 1; // +1 pour '\n'
        }
        let local_offset = safe_offset.saturating_sub(line_start);

        let cursor = Cursor::new(line_idx, local_offset);
        let layout_cursor = buffer.layout_cursor(&cursor);

        for run in buffer.layout_runs() {
            if run.line_i != layout_cursor.line {
                continue;
            }
            if let Some(glyph) = run.glyphs.get(layout_cursor.glyph) {
                return glyph.x;
            }
            if let Some(last) = run.glyphs.last() {
                return last.x + last.w;
            }
        }
        0.0
    }

    fn hit_test(
        &self,
        text: &str,
        font_family: &FontFamily,
        font_size: f32,
        x: f32,
        y: f32,
    ) -> usize {
        if text.is_empty() || x <= 0.0 {
            return 0;
        }
        let buffer = self.shape_buffer(text, font_family, font_size);
        if let Some(cursor) = buffer.hit(x, y) {
            let mut byte_offset = 0;
            for (i, l) in buffer.lines.iter().enumerate() {
                if i == cursor.line {
                    byte_offset += cursor.index.min(l.text().len());
                    break;
                }
                byte_offset += l.text().len() + 1;
            }
            byte_offset.min(text.len())
        } else {
            text.len()
        }
    }

    fn selection_spans(
        &self,
        text: &str,
        font_family: &FontFamily,
        font_size: f32,
        start: usize,
        end: usize,
    ) -> Vec<(f32, f32)> {
        if start == end || text.is_empty() {
            return Vec::new();
        }
        let min_s = start.min(end).min(text.len());
        let max_s = start.max(end).min(text.len());
        let x1 = self.caret_x(text, font_family, font_size, min_s);
        let x2 = self.caret_x(text, font_family, font_size, max_s);
        vec![(x1, (x2 - x1).max(2.0))]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosmic_caret_x_matches_width_at_end_of_string() {
        let measure = CosmicTextMeasure::new();
        let family = FontFamily::SansSerif;
        let text = "ici je peux saisir le texte sans décalage";
        let width = measure.measure_width(text, &family, 15.0);
        let caret_end = measure.caret_x(text, &family, 15.0, text.len());
        assert!(width > 100.0);
        // caret_end should match width within subpixel rounding
        assert!((width - caret_end).abs() < 1.0);
        assert_eq!(measure.caret_x(text, &family, 15.0, 0), 0.0);
    }

    #[test]
    fn cosmic_caret_x_advances_monotonically() {
        let measure = CosmicTextMeasure::new();
        let family = FontFamily::SansSerif;
        let text = "Hello AORUI";
        let mut prev_x = 0.0;
        for (idx, _) in text.char_indices().skip(1) {
            let x = measure.caret_x(text, &family, 15.0, idx);
            assert!(
                x > prev_x,
                "caret X must advance monotonically: {x} > {prev_x}"
            );
            prev_x = x;
        }
    }

    #[test]
    fn cosmic_hit_test_round_trip() {
        let measure = CosmicTextMeasure::new();
        let family = FontFamily::SansSerif;
        let text = "abcdef 123456";
        let x_at_3 = measure.caret_x(text, &family, 15.0, 3);
        let hit = measure.hit_test(text, &family, 15.0, x_at_3, 5.0);
        assert_eq!(hit, 3);

        let x_at_0 = measure.caret_x(text, &family, 15.0, 0);
        let hit_0 = measure.hit_test(text, &family, 15.0, x_at_0, 5.0);
        assert_eq!(hit_0, 0);
    }
}
