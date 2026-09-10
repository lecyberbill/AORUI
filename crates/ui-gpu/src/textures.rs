// [WFGY] Zone: TRANSIT | delta_s: 0.3 | κ: Low | η: Low | Action: Fabrique de textures offscreen (fond + niveaux de flou)
pub struct RenderTarget {
    // Conservee uniquement pour la propriete GPU : `view` en depend mais ne
    // la reference pas directement.
    #[allow(dead_code)]
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub size: (u32, u32),
}

pub fn create_render_target(
    device: &wgpu::Device,
    size: (u32, u32),
    format: wgpu::TextureFormat,
    label: &str,
) -> RenderTarget {
    let size = (size.0.max(1), size.1.max(1));
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    RenderTarget {
        texture,
        view,
        size,
    }
}

pub fn create_linear_sampler(device: &wgpu::Device, label: &str) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some(label),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::FilterMode::Linear,
        ..Default::default()
    })
}
