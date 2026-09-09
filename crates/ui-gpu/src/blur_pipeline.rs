// [WFGY] Zone: RISK | delta_s: 0.6 | κ: High | η: Low | Action: Passe Dual-Kawase (downsample/upsample) — Pass 2 du pipeline glassmorphism
use bytemuck::{Pod, Zeroable};

use crate::textures::create_linear_sampler;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct BlurParamsGpu {
    // xy = taille de texel source (1/w, 1/h), z = rayon d'échantillonnage, w = padding
    texel_size_and_radius: [f32; 4],
}

/// Encapsule les deux passes du flou Dual-Kawase (un niveau : downsample puis
/// upsample). Extensible à une chaîne multi-niveaux en rappelant `downsample`
/// / `upsample` sur des textures intermédiaires supplémentaires.
pub struct BlurPipeline {
    bind_group_layout: wgpu::BindGroupLayout,
    downsample_pipeline: wgpu::RenderPipeline,
    upsample_pipeline: wgpu::RenderPipeline,
    sampler: wgpu::Sampler,
    uniform_buffer: wgpu::Buffer,
}

impl BlurPipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blur_dual_kawase"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/blur_dual_kawase.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("blur bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("blur pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let make_pipeline = |entry_point: &str, label: &str| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_fullscreen",
                    buffers: &[],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point,
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            })
        };

        let downsample_pipeline = make_pipeline("fs_downsample", "blur downsample pipeline");
        let upsample_pipeline = make_pipeline("fs_upsample", "blur upsample pipeline");

        let sampler = create_linear_sampler(device, "blur sampler");

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("blur params uniform"),
            size: std::mem::size_of::<BlurParamsGpu>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self { bind_group_layout, downsample_pipeline, upsample_pipeline, sampler, uniform_buffer }
    }

    fn run_pass(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        pipeline: &wgpu::RenderPipeline,
        src_view: &wgpu::TextureView,
        src_size: (u32, u32),
        dst_view: &wgpu::TextureView,
        radius: f32,
    ) {
        let params = BlurParamsGpu {
            texel_size_and_radius: [1.0 / src_size.0 as f32, 1.0 / src_size.1 as f32, radius, 0.0],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&params));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("blur bind group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.uniform_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(src_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("blur pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: dst_view,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// Downsample : `src` (résolution pleine) -> `dst` (résolution réduite, typiquement 1/2).
    pub fn downsample(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        src_view: &wgpu::TextureView,
        src_size: (u32, u32),
        dst_view: &wgpu::TextureView,
    ) {
        self.run_pass(device, queue, encoder, &self.downsample_pipeline, src_view, src_size, dst_view, 1.0);
    }

    /// Upsample : `src` (résolution réduite) -> `dst` (résolution pleine).
    pub fn upsample(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        src_view: &wgpu::TextureView,
        src_size: (u32, u32),
        dst_view: &wgpu::TextureView,
    ) {
        self.run_pass(device, queue, encoder, &self.upsample_pipeline, src_view, src_size, dst_view, 1.0);
    }
}
