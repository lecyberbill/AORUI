// [WFGY] Zone: TRANSIT | λ: 0.2 | Fallbacks: 1 (DefaultTextMeasure) | Action: Trait de mesure de texte subpixel découplé du moteur GPU
use crate::theme::FontFamily;

/// Primitives de mesure et géométrie de texte découplées de tout contexte de rendu ou GPU.
pub trait TextMeasure: Send + Sync {
    /// Mesure la largeur totale d'une ligne de texte pour une police et une taille données.
    fn measure_width(&self, text: &str, font_family: &FontFamily, font_size: f32) -> f32;

    /// Retourne la position horizontale X exacte (en pixels logiques subpixel) du caret
    /// pour un offset d'octets donné dans `text`.
    fn caret_x(&self, text: &str, font_family: &FontFamily, font_size: f32, byte_offset: usize) -> f32;

    /// Convertit des coordonnées locales `(x, y)` relatives au début du texte
    /// en offset d'octets le plus proche (hit-testing).
    fn hit_test(&self, text: &str, font_family: &FontFamily, font_size: f32, x: f32, y: f32) -> usize;

    /// Retourne les spans de sélection `(x_start, width)` pour chaque ligne entre `start` et `end`.
    fn selection_spans(
        &self,
        text: &str,
        font_family: &FontFamily,
        font_size: f32,
        start: usize,
        end: usize,
    ) -> Vec<(f32, f32)>;
}

/// Implémentation par défaut basée sur une estimation statistique de secours
/// lorsque aucun moteur vectoriel réel (ex: cosmic-text) n'est injecté.
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultTextMeasure;

impl TextMeasure for DefaultTextMeasure {
    fn measure_width(&self, text: &str, _font_family: &FontFamily, font_size: f32) -> f32 {
        crate::frame::estimate_text_width(text, font_size)
    }

    fn caret_x(&self, text: &str, _font_family: &FontFamily, font_size: f32, byte_offset: usize) -> f32 {
        let safe = byte_offset.min(text.len());
        crate::frame::estimate_text_width(&text[..safe], font_size)
    }

    fn hit_test(&self, text: &str, _font_family: &FontFamily, font_size: f32, x: f32, _y: f32) -> usize {
        crate::frame::find_cursor_index_in_line(text, x, font_size)
    }

    fn selection_spans(
        &self,
        text: &str,
        _font_family: &FontFamily,
        font_size: f32,
        start: usize,
        end: usize,
    ) -> Vec<(f32, f32)> {
        if start == end || text.is_empty() {
            return Vec::new();
        }
        let min_s = start.min(end).min(text.len());
        let max_s = start.max(end).min(text.len());
        let x1 = self.caret_x(text, _font_family, font_size, min_s);
        let x2 = self.caret_x(text, _font_family, font_size, max_s);
        vec![(x1, (x2 - x1).max(2.0))]
    }
}
