// [WFGY] Zone: RISK | λ: 0.2 | Fallbacks: 0 | Action: Strict memory layout for GPU upload (bytemuck Pod/Zeroable)
//! Types with contractual memory layouts: uploaded verbatim into an instanced
//! `wgpu::Buffer`. Any field modification here must be mirrored in `sdf_card.wgsl`
//! (crate `ui-gpu`) to avoid silent memory misalignment (INV-GPU-1).

/// A single SDF quad instance (glass card, badge, neon border...) ready to
/// be copied into the `ui-gpu` pipeline instance buffer.
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuSdfInstance {
    /// [x, y, width, height] in normalized screen space.
    pub bounds: [f32; 4],
    /// [r, g, b, a] background color with glass translucency.
    pub bg_color: [f32; 4],
    /// [r, g, b, a] neon glow halo color.
    pub glow_color: [f32; 4],
    pub radius: f32,
    pub border_width: f32,
    pub glow_intensity: f32,
    /// Dual-Kawase blur sampling factor / texture weight (pass 2).
    pub blur_factor: f32,
    /// [x, y, width, height]: screen region outside of which this quad is
    /// completely clipped (`discard` in `sdf_card.wgsl`). Used for scrollable
    /// clipping regions (`ScrollView`) — a widget without a scrollable ancestor
    /// carries a bounding region covering the entire screen.
    pub clip_bounds: [f32; 4],
}
