// [WFGY] Zone: TRANSIT | delta_s: 0.45 | κ: Low | η: Low | Action: Rendu de texte vectoriel via glyphon/cosmic-text
use glyphon::{
    cosmic_text::Align, Attrs, Buffer, Color, Family, FontSystem, Metrics, Resolution, Shaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer as GlyphonRenderer, Weight,
};

/// Surimpression du texte vectoriel des cartes au-dessus de la composition
/// SDF. Famille/graisse/taille viennent de `ui_widgets::Theme::typography`
/// (centralisé côté appelant) — ce crate ne fait que les traduire vers
/// `cosmic-text`.
pub struct TextLayer {
    font_system: FontSystem,
    swash_cache: SwashCache,
    atlas: TextAtlas,
    renderer: GlyphonRenderer,
}

pub struct TextRun {
    pub buffer: Buffer,
    pub left: f32,
    pub top: f32,
    pub color: Color,
    /// Région écran hors de laquelle le texte est coupé (zones défilantes) —
    /// `TextBounds::default()` (non bornée) si le widget n'a pas d'ancêtre
    /// `ScrollView`.
    pub clip: TextBounds,
}

impl TextLayer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let mut atlas = TextAtlas::new(device, queue, format);
        let renderer = GlyphonRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);

        Self { font_system, swash_cache, atlas, renderer }
    }

    /// Crée un buffer de texte prêt à être positionné par [`TextRun`].
    ///
    /// `width`/`height` bornent la zone de wrap et de rendu du buffer. Sans
    /// appel à `set_size`, `cosmic-text` laisse `height` à 0 par défaut, ce
    /// qui fait calculer `maximum_lines = 0` dans son `LayoutRunIter` — donc
    /// *aucune* ligne n'est jamais dessinée, quel que soit le texte
    /// (bug constaté : tous les libellés invisibles à l'écran).
    pub fn make_buffer(
        &mut self,
        text: &str,
        font_size: f32,
        line_height: f32,
        width: f32,
        height: f32,
        align: Align,
        family: Family<'_>,
        weight: Weight,
    ) -> Buffer {
        let mut buffer = Buffer::new(&mut self.font_system, Metrics::new(font_size, line_height));
        buffer.set_size(&mut self.font_system, width.max(1.0), height.max(line_height));
        let attrs = Attrs::new().family(family).weight(weight);
        buffer.set_text(&mut self.font_system, text, attrs, Shaping::Advanced);
        for line in buffer.lines.iter_mut() {
            line.set_align(Some(align));
        }
        buffer.shape_until_scroll(&mut self.font_system);
        buffer
    }

    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen_size: (u32, u32),
        runs: &[TextRun],
    ) -> Result<(), glyphon::PrepareError> {
        let areas = runs.iter().map(|run| TextArea {
            buffer: &run.buffer,
            left: run.left,
            top: run.top,
            scale: 1.0,
            bounds: run.clip,
            default_color: run.color,
        });

        self.renderer.prepare(
            device,
            queue,
            &mut self.font_system,
            &mut self.atlas,
            Resolution { width: screen_size.0, height: screen_size.1 },
            areas,
            &mut self.swash_cache,
        )
    }

    pub fn render<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>) -> Result<(), glyphon::RenderError> {
        self.renderer.render(&self.atlas, pass)
    }

    pub fn trim_atlas(&mut self) {
        self.atlas.trim();
    }
}
