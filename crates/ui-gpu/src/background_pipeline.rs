// [WFGY] Zone: RISK | λ: 0.2 | Fallbacks: 0 | Action: GPU Background Gradient & Grid Pipeline
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable, Debug, PartialEq)]
pub struct BackgroundParams {
    pub screen_size: [f32; 2],
    pub grid_spacing: f32,
    pub grid_dot_size: f32,

    pub base_color: [f32; 4],

    pub grad1_center: [f32; 2],
    pub grad1_radius: [f32; 2],
    pub grad1_color: [f32; 4],

    pub grad2_center: [f32; 2],
    pub grad2_radius: [f32; 2],
    pub grad2_color: [f32; 4],

    pub grid_opacity: f32,
    pub _pad0: f32,
    pub _pad1: f32,
    pub _pad2: f32,
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn hex_to_linear(hex: &str) -> [f32; 4] {
    let clean = hex.trim().trim_start_matches('#');
    let srgb_to_lin = |v: u8| -> f32 { srgb_to_linear(v as f32 / 255.0) };
    match clean.len() {
        6 => {
            let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(0);
            [srgb_to_lin(r), srgb_to_lin(g), srgb_to_lin(b), 1.0]
        }
        8 => {
            let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(0);
            let a = u8::from_str_radix(&clean[6..8], 16).unwrap_or(255);
            [srgb_to_lin(r), srgb_to_lin(g), srgb_to_lin(b), a as f32 / 255.0]
        }
        _ => [0.0, 0.0, 0.0, 1.0],
    }
}

impl Default for BackgroundParams {
    fn default() -> Self {
        Self::aether_os(1380.0, 880.0)
    }
}

impl BackgroundParams {
    /// Flagship AetherOS Theme atmospheric gradient background matching screen.png & code.html.
    /// Deep midnight dark top-left with periwinkle zenith aurora shifted center-right and nadir cyan ray.
    pub fn aether_os(width: f32, height: f32) -> Self {
        Self {
            screen_size: [width, height],
            grid_spacing: 24.0,
            grid_dot_size: 1.0,
            base_color: hex_to_linear("#05080f"),         // Deep midnight oceanic black (linearized)
            grad1_center: [0.65, -0.25],                  // Shifted right/up: top-left remains deep dark
            grad1_radius: [0.55, 0.45],                   // Calibrated ellipse
            grad1_color: [
                srgb_to_linear(99.0 / 255.0),
                srgb_to_linear(102.0 / 255.0),
                srgb_to_linear(241.0 / 255.0),
                0.22,
            ], // Indigo/Periwinkle Aurora (#6366f1) in linear space
            grad2_center: [0.85, 0.95],                   // Nadir bottom right
            grad2_radius: [0.55, 0.45],                   // Ellipse 55% x 45%
            grad2_color: [
                srgb_to_linear(56.0 / 255.0),
                srgb_to_linear(189.0 / 255.0),
                srgb_to_linear(248.0 / 255.0),
                0.15,
            ], // Cyan Ray glow (#38bdf8) in linear space
            grid_opacity: 0.12,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        }
    }

    /// Cyber Glass theme: deep midnight navy with luminescent cyan ray.
    pub fn cyber_glass(width: f32, height: f32) -> Self {
        Self {
            screen_size: [width, height],
            grid_spacing: 28.0,
            grid_dot_size: 1.0,
            base_color: hex_to_linear("#030610"),
            grad1_center: [0.60, -0.15],
            grad1_radius: [0.50, 0.50],
            grad1_color: [
                srgb_to_linear(0.0),
                srgb_to_linear(0.88),
                srgb_to_linear(0.98),
                0.16,
            ],
            grad2_center: [0.85, 0.85],
            grad2_radius: [0.50, 0.50],
            grad2_color: [
                srgb_to_linear(0.72),
                srgb_to_linear(0.38),
                srgb_to_linear(0.98),
                0.12,
            ],
            grid_opacity: 0.10,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        }
    }

    /// Solid color background without gradients or grid.
    pub fn solid(width: f32, height: f32, color: [f32; 4]) -> Self {
        Self {
            screen_size: [width, height],
            grid_spacing: 0.0,
            grid_dot_size: 0.0,
            base_color: [
                srgb_to_linear(color[0]),
                srgb_to_linear(color[1]),
                srgb_to_linear(color[2]),
                color[3],
            ],
            grad1_center: [0.0, 0.0],
            grad1_radius: [1.0, 1.0],
            grad1_color: [0.0, 0.0, 0.0, 0.0],
            grad2_center: [0.0, 0.0],
            grad2_radius: [1.0, 1.0],
            grad2_color: [0.0, 0.0, 0.0, 0.0],
            grid_opacity: 0.0,
            _pad0: 0.0,
            _pad1: 0.0,
            _pad2: 0.0,
        }
    }
}

pub struct BackgroundPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
}

impl BackgroundPipeline {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("background_gradient_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/background_gradient.wgsl").into()),
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("background uniform buffer"),
            size: std::mem::size_of::<BackgroundParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("background bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("background bind group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("background pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("background render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Self {
            pipeline,
            bind_group,
            uniform_buffer,
        }
    }

    pub fn set_params(&self, queue: &wgpu::Queue, params: &BackgroundParams) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(params));
    }

    pub fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..4, 0..1);
    }
}
