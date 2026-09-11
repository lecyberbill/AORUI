// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0 | Action: GPU RGBA Texture representation & dynamic streaming updates
use std::sync::Arc;

/// Handle to an allocated GPU 2D RGBA texture ready for shader sampling.
pub struct GpuTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
}

impl GpuTexture {
    /// Creates a new RGBA8 texture on the GPU and uploads initial pixel data.
    pub fn new_rgba(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
        data: &[u8],
        label: Option<&str>,
    ) -> Arc<Self> {
        let size = wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        if !data.is_empty() {
            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                data,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * width.max(1)),
                    rows_per_image: Some(height.max(1)),
                },
                size,
            );
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Arc::new(Self {
            texture,
            view,
            width: width.max(1),
            height: height.max(1),
        })
    }

    /// Wraps an existing GPU texture and view handle without reallocating.
    pub fn from_texture_and_view(
        texture: wgpu::Texture,
        view: wgpu::TextureView,
        width: u32,
        height: u32,
    ) -> Arc<Self> {
        Arc::new(Self {
            texture,
            view,
            width: width.max(1),
            height: height.max(1),
        })
    }

    /// Dynamically writes new RGBA pixel bytes to the texture (e.g. video/canvas stream).
    pub fn update(&self, queue: &wgpu::Queue, data: &[u8]) {
        let size = wgpu::Extent3d {
            width: self.width,
            height: self.height,
            depth_or_array_layers: 1,
        };
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * self.width),
                rows_per_image: Some(self.height),
            },
            size,
        );
    }

    /// Aspect ratio (width / height).
    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height.max(1) as f32
    }
}
