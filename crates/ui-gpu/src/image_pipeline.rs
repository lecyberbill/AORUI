// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0 | Action: Image & Video GPU rendering pipeline implementing MediaPipeline
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;

use crate::media_pipeline::{MediaInstance, MediaPipeline};
use crate::resources::ResourceTable;
use crate::texture::GpuTexture;
use crate::textures::create_linear_sampler;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct GlobalsGpu {
    screen_size: [f32; 2],
    _pad: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct GpuImageInstance {
    pub bounds: [f32; 4],
    pub clip_bounds: [f32; 4],
    pub radius: f32,
    pub fit_mode: u32,
    pub aspect_ratio: f32,
    pub opacity: f32,
}

const INSTANCE_ATTRS: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
    0 => Float32x4, // bounds
    1 => Float32x4, // clip_bounds
    2 => Float32,   // radius
    3 => Uint32,    // fit_mode
    4 => Float32,   // aspect_ratio
    5 => Float32,   // opacity
];

pub struct ImagePipeline {
    kind_name: &'static str,
    pipeline: wgpu::RenderPipeline,
    globals_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    globals_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    sampler: wgpu::Sampler,
    screen_size: (f32, f32),
}

impl ImagePipeline {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        kind_name: &'static str,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("image_quad shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/image_quad.wgsl").into()),
        });

        let sampler = create_linear_sampler(device, "image linear sampler");

        let globals_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("image globals bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
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
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("image texture bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("image pipeline layout"),
            bind_group_layouts: &[&globals_layout, &texture_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("image render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GpuImageInstance>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &INSTANCE_ATTRS,
                }],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
        });

        let globals_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("image globals buffer"),
            size: std::mem::size_of::<GlobalsGpu>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let initial_capacity = 64;
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("image instance buffer"),
            size: (initial_capacity * std::mem::size_of::<GpuImageInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            kind_name,
            pipeline,
            globals_layout,
            texture_layout,
            globals_buffer,
            instance_buffer,
            sampler,
            screen_size: (1920.0, 1080.0),
        }
    }

    pub fn set_screen_size(&mut self, queue: &wgpu::Queue, width: f32, height: f32) {
        self.screen_size = (width, height);
        let globals = GlobalsGpu {
            screen_size: [width, height],
            _pad: [0.0, 0.0],
        };
        queue.write_buffer(&self.globals_buffer, 0, bytemuck::bytes_of(&globals));
    }

    fn create_texture_bind_group(
        &self,
        device: &wgpu::Device,
        view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("image texture bind group"),
            layout: &self.texture_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(view),
            }],
        })
    }
}

impl MediaPipeline for ImagePipeline {
    fn kind(&self) -> &'static str {
        self.kind_name
    }

    fn set_screen_size(&mut self, queue: &wgpu::Queue, width: f32, height: f32) {
        self.set_screen_size(queue, width, height);
    }

    fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        resources: &ResourceTable,
        instances: &[MediaInstance],
    ) {
        if instances.is_empty() {
            return;
        }

        let globals_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("image globals bind group"),
            layout: &self.globals_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.globals_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let mut batches = Vec::new();
        for inst in instances {
            let texture_opt: Option<&GpuTexture> = resources
                .get::<Arc<GpuTexture>>(&inst.resource_id)
                .map(|arc| arc.as_ref())
                .or_else(|| resources.get::<GpuTexture>(&inst.resource_id));

            if let Some(gpu_tex) = texture_opt {
                let bind_group = self.create_texture_bind_group(device, &gpu_tex.view);
                let gpu_instance = GpuImageInstance {
                    bounds: inst.bounds,
                    clip_bounds: inst.clip_bounds,
                    radius: inst.radius,
                    fit_mode: inst.fit_mode,
                    aspect_ratio: gpu_tex.aspect_ratio(),
                    opacity: 1.0,
                };
                batches.push((bind_group, gpu_instance));
            }
        }

        if batches.is_empty() {
            return;
        }

        // Render pass for media quads
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("image pipeline render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &globals_bind_group, &[]);

        for (bind_group, gpu_instance) in &batches {
            queue.write_buffer(&self.instance_buffer, 0, bytemuck::bytes_of(gpu_instance));
            pass.set_vertex_buffer(
                0,
                self.instance_buffer
                    .slice(..std::mem::size_of::<GpuImageInstance>() as u64),
            );
            pass.set_bind_group(1, bind_group, &[]);
            pass.draw(0..6, 0..1);
        }
    }
}
