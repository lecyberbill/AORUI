// [WFGY] Zone: TRANSIT | delta_s: 0.45 | κ: Low | η: Low | Action: Registre de pipelines pour les formats de contenu au-dela des cartes SDF
use crate::resources::ResourceTable;

/// Une instance de contenu média à dessiner. Pendant volontaire avec
/// `ui_widgets::MediaSpec` (voir la note ci-dessous) plutôt qu'une
/// dépendance directe : `ui-gpu` reste utilisable sans `ui-widgets`, comme
/// `TextRun` (ci-dessus) reste indépendant de `ui_widgets::TextSpec`.
/// L'application fait le pont (voir `examples/widget_gallery.rs`).
#[derive(Debug, Clone, PartialEq)]
pub struct MediaInstance {
    /// Doit correspondre au `kind` déclaré par le [`MediaPipeline`] cible.
    pub kind: String,
    pub resource_id: String,
    pub bounds: [f32; 4],
    pub clip_bounds: [f32; 4],
    pub radius: f32,
    pub fit_mode: u32,
}

/// Point d'extension pour un format de contenu (image, vidéo, document
/// embarqué, viewport 3D...). Chaque implémentation s'enregistre auprès de
/// `GpuRenderer::register_media_pipeline` et ne reçoit, à chaque frame, que
/// les [`MediaInstance`] dont `kind` correspond à [`MediaPipeline::kind`].
///
/// INV-EXT-1 : un registre vide (aucun pipeline enregistré pour un `kind`
/// donné) ne doit jamais faire échouer le rendu — les `MediaInstance` sans
/// pipeline correspondant sont silencieusement ignorées. C'est ce qui rend
/// l'ajout d'un futur format non-cassant : une app qui ne connaît pas
/// encore le format "video" continue de fonctionner si un widget de ce type
/// apparaît (par erreur ou avant l'implémentation du pipeline).
///
/// [TODO: <aucune implémentation fournie ici (image/vidéo/3D restent à
/// écrire) — ce trait ne fait que réserver la place dans le pipeline de
/// rendu, conformément à la feuille de route "formats de contenu
/// extensibles" évoquée par l'utilisateur.>]
pub trait MediaPipeline: Send {
    /// Doit correspondre au `kind` utilisé côté `ui_widgets::MediaKind`
    /// (ex. `"image"`, `"video"`, `"markdown"`, `"viewport3d"`).
    fn kind(&self) -> &'static str;

    /// Met à jour les dimensions de la fenêtre pour la projection NDC.
    fn set_screen_size(&mut self, _queue: &wgpu::Queue, _width: f32, _height: f32) {}

    /// Dessine les instances de ce format dans la passe de composition,
    /// par-dessus les cartes SDF déjà rendues et sous le texte (voir l'ordre
    /// des passes dans `GpuRenderer::render`).
    fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        resources: &ResourceTable,
        instances: &[MediaInstance],
    );
}
