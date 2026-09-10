// [WFGY] Zone: TRANSIT | λ: 0.2 | Fallbacks: 0 | Action: Extension point for extended content formats beyond SDF cards (image, video, markdown, 3D...)

/// Identifier for a content format (`"image"`, `"video"`, `"markdown"`, `"viewport3d"`...).
/// Intentionally an open static string rather than a closed enum to allow zero-touch format extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MediaKind(pub &'static str);

/// Scaling and alignment mode for media content inside bounding boxes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MediaFit {
    /// Scale to fill bounds while preserving aspect ratio, cropping excess.
    #[default]
    Cover,
    /// Scale to fit entirely inside bounds while preserving aspect ratio, letterboxing if necessary.
    Contain,
    /// Stretch directly to fill bounds ignoring aspect ratio.
    Fill,
}

/// Declarative specification for non-SDF content (image, video frame, embedded document, 3D viewport...).
/// `ui-widgets` transports the format metadata, fitting options, and opaque resource key, allowing
/// the rendering backend (`ui-gpu` via registered `MediaPipeline`) to perform actual drawing.
#[derive(Debug, Clone, PartialEq)]
pub struct MediaSpec {
    pub kind: MediaKind,
    pub bounds: [f32; 4],
    pub resource_id: String,
    pub fit: MediaFit,
    pub radius: f32,
    pub clip: [f32; 4],
}
