// [WFGY] Zone: RISK | λ: 0.3 | Fallbacks: 0 | Action: WGPU/WGSL GPU rendering pipeline implementation (SDF + Dual-Kawase + glyphon)
//! GPU rendering engine of `AORUI` (An Other Rust UI).
//!
//! INV-CORE-1 (reminder): this crate must never be imported by `agent-runtime`.
//! It consumes already constructed [`ui_core::GpuSdfInstance`]s
//! (typically generated from bounds resolved by `ui-layout`).

mod blur_pipeline;
mod context;
pub mod image_pipeline;
pub mod measure;
mod media_pipeline;
mod renderer;
mod resources;
mod sdf_pipeline;
mod text;
pub mod texture;
mod textures;

pub use context::GpuContext;
pub use image_pipeline::ImagePipeline;
pub use measure::CosmicTextMeasure;
pub use media_pipeline::{MediaInstance, MediaPipeline};
pub use renderer::{GpuRenderer, RenderLayer};
pub use resources::ResourceTable;
pub use text::{TextLayer, TextRun};
pub use texture::GpuTexture;

#[cfg(test)]
mod tests {
    use ui_core::GpuSdfInstance;

    #[test]
    fn gpu_sdf_instance_matches_wgsl_instance_layout_size() {
        // INV-GPU-1: 20 x f32 = 80 bytes, matching the instance layout
        // in sdf_card.wgsl (bounds, bg_color, glow_color, radius,
        // border_width, glow_intensity, blur_factor, clip_bounds).
        assert_eq!(std::mem::size_of::<GpuSdfInstance>(), 80);
    }

    /// Offline healthcheck (without GPU/adapter): parses and validates each
    /// WGSL shader with naga to catch syntax/type errors that `cargo check` cannot see.
    fn validate_wgsl(source: &str) {
        let module = naga::front::wgsl::parse_str(source).expect("shader must parse successfully");
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        validator.validate(&module).expect("shader must pass naga validation");
    }

    #[test]
    fn sdf_card_shader_is_valid_wgsl() {
        validate_wgsl(include_str!("../shaders/sdf_card.wgsl"));
    }

    #[test]
    fn blur_dual_kawase_shader_is_valid_wgsl() {
        validate_wgsl(include_str!("../shaders/blur_dual_kawase.wgsl"));
    }

    #[test]
    fn image_quad_shader_is_valid_wgsl() {
        validate_wgsl(include_str!("shaders/image_quad.wgsl"));
    }
}
