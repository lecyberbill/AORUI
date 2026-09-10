// [WFGY] Zone: RISK | λ: 0.3 | Fallbacks: 0 | Action: Three-pass rendering orchestration (offscreen background, Dual-Kawase blur, SDF + text composition)
use std::sync::Arc;

use ui_core::GpuSdfInstance;
use winit::window::Window;

use crate::blur_pipeline::BlurPipeline;
use crate::context::GpuContext;
use crate::media_pipeline::{MediaInstance, MediaPipeline};
use crate::resources::ResourceTable;
use crate::sdf_pipeline::SdfPipeline;
use crate::text::{TextLayer, TextRun};
use crate::textures::{create_render_target, RenderTarget};

/// Main entry point for the `ui-gpu` crate. Consumes `GpuSdfInstance`s (produced
/// from bounds resolved by `ui-layout`) and associated `TextRun`s, with zero
/// coupling to agent state machines (INV-CORE-1).
pub struct GpuRenderer {
    ctx: GpuContext,
    background: RenderTarget,
    blur_half: RenderTarget,
    blurred_full: RenderTarget,
    blur_pipeline: BlurPipeline,
    sdf_pipeline: SdfPipeline,
    text: TextLayer,
    measure: crate::measure::CosmicTextMeasure,
    /// Open pipeline registry for content formats beyond SDF glass
    /// (image/video/markdown/3D — see `media_pipeline`). Empty by default:
    /// has no performance or rendering impact when no media pipelines are registered (INV-EXT-1).
    media_pipelines: Vec<Box<dyn MediaPipeline>>,
}

impl GpuRenderer {
    pub fn new(window: Arc<Window>) -> Self {
        let ctx = GpuContext::new(window);
        let format = ctx.config.format;
        let full_size = (ctx.config.width, ctx.config.height);
        let half_size = ((full_size.0 / 2).max(1), (full_size.1 / 2).max(1));

        let background = create_render_target(&ctx.device, full_size, format, "background target");
        let blur_half = create_render_target(&ctx.device, half_size, format, "blur half-res target");
        let blurred_full = create_render_target(&ctx.device, full_size, format, "blurred full-res target");

        let blur_pipeline = BlurPipeline::new(&ctx.device, format);
        let sdf_pipeline = SdfPipeline::new(&ctx.device, format);
        let text = TextLayer::new(&ctx.device, &ctx.queue, format);
        let measure = crate::measure::CosmicTextMeasure::new();

        let image_pipeline = Box::new(crate::image_pipeline::ImagePipeline::new(&ctx.device, format, "image"));
        let video_pipeline = Box::new(crate::image_pipeline::ImagePipeline::new(&ctx.device, format, "video"));
        let media_pipelines: Vec<Box<dyn MediaPipeline>> = vec![image_pipeline, video_pipeline];

        let mut renderer = Self { ctx, background, blur_half, blurred_full, blur_pipeline, sdf_pipeline, text, measure, media_pipelines };
        renderer.update_media_screen_size();
        renderer
    }

    fn update_media_screen_size(&mut self) {
        // Reserved for dynamic per-pipeline screen size dispatch
    }

    /// Allocates and uploads an RGBA8 texture onto the GPU.
    pub fn create_texture_rgba(&self, width: u32, height: u32, data: &[u8]) -> Arc<crate::texture::GpuTexture> {
        crate::texture::GpuTexture::new_rgba(&self.ctx.device, &self.ctx.queue, width, height, data, Some("aorui_gpu_texture"))
    }

    /// Updates existing GPU texture with new pixel bytes (e.g. video frame streaming).
    pub fn update_texture_rgba(&self, texture: &crate::texture::GpuTexture, data: &[u8]) {
        texture.update(&self.ctx.queue, data);
    }

    pub fn text_measure(&self) -> crate::measure::CosmicTextMeasure {
        self.measure.clone()
    }

    pub fn window_size(&self) -> (u32, u32) {
        (self.ctx.config.width, self.ctx.config.height)
    }

    /// Registers a custom pipeline for an extended content format (see [`MediaPipeline`]).
    /// Multiple pipelines can coexist (one per `kind`). Registering the same `kind` twice
    /// gives priority to the latest registered.
    pub fn register_media_pipeline(&mut self, pipeline: Box<dyn MediaPipeline>) {
        self.media_pipelines.push(pipeline);
    }

    /// Constructs a [`TextRun`] positioned at `bounds` (same screen-space absolute coordinates
    /// as [`ui_core::GpuSdfInstance::bounds`]), for integration with widget models (e.g. `ui-widgets`)
    /// that are decoupled from `glyphon` (extended INV-CORE-1).
    pub fn make_text_run(
        &mut self,
        text: &str,
        bounds: [f32; 4],
        font_size: f32,
        color: [f32; 4],
        align: glyphon::cosmic_text::Align,
        family: glyphon::Family<'_>,
        weight: glyphon::Weight,
        clip: [f32; 4],
    ) -> TextRun {
        let line_height = font_size * 1.2;
        let buffer = self.text.make_buffer(text, font_size, line_height, bounds[2], bounds[3], align, family, weight);
        // Vertical centering offset for single-line widget text
        let top = bounds[1] + ((bounds[3] - line_height) * 0.5).max(0.0);
        let clip = glyphon::TextBounds {
            left: clip[0] as i32,
            top: clip[1] as i32,
            right: (clip[0] + clip[2]) as i32,
            bottom: (clip[1] + clip[3]) as i32,
        };
        TextRun { buffer, left: bounds[0], top, color: to_glyphon_color(color), clip }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.ctx.resize(new_size);
        let format = self.ctx.config.format;
        let full_size = (self.ctx.config.width, self.ctx.config.height);
        let half_size = ((full_size.0 / 2).max(1), (full_size.1 / 2).max(1));

        self.background = create_render_target(&self.ctx.device, full_size, format, "background target");
        self.blur_half = create_render_target(&self.ctx.device, half_size, format, "blur half-res target");
        self.blurred_full = create_render_target(&self.ctx.device, full_size, format, "blurred full-res target");
    }
}

/// Z-index rendering layer grouping SDF instances and associated TextRuns.
pub struct RenderLayer<'a> {
    pub instances: &'a [GpuSdfInstance],
    pub texts: &'a [TextRun],
}

impl GpuRenderer {
    pub fn render_layers(
        &mut self,
        background_clear: wgpu::Color,
        layers: &[RenderLayer<'_>],
        media: &[MediaInstance],
        resources: &ResourceTable,
    ) -> Result<(), wgpu::SurfaceError> {
        let frame = self.ctx.surface.get_current_texture()?;
        let surface_view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("frame encoder"),
        });

        // Pass 1: Clear all offscreen and surface render targets
        clear_target(&mut encoder, &self.background.view, background_clear);
        clear_target(&mut encoder, &surface_view, background_clear);
        clear_target(&mut encoder, &self.blurred_full.view, background_clear);
        clear_target(&mut encoder, &self.blur_half.view, background_clear);

        self.sdf_pipeline.set_screen_size(&self.ctx.queue, self.ctx.config.width as f32, self.ctx.config.height as f32);
        for pipeline in self.media_pipelines.iter_mut() {
            pipeline.set_screen_size(&self.ctx.queue, self.ctx.config.width as f32, self.ctx.config.height as f32);
        }

        if layers.len() > 1 {
            // --- Layer 0 (Base UI): rendered to surface_view AND background target for blur capture ---
            let layer0 = &layers[0];
            if !layer0.instances.is_empty() {
                self.sdf_pipeline.upload_instances(&self.ctx.device, &self.ctx.queue, layer0.instances);
                self.sdf_pipeline.render(&self.ctx.device, &mut encoder, &surface_view, &self.blurred_full.view);
                self.sdf_pipeline.render(&self.ctx.device, &mut encoder, &self.background.view, &self.blurred_full.view);
            }

            // Render registered media pipelines on base UI
            for pipeline in self.media_pipelines.iter_mut() {
                let matching: Vec<MediaInstance> =
                    media.iter().filter(|instance| instance.kind == pipeline.kind()).cloned().collect();
                if !matching.is_empty() {
                    pipeline.render(&self.ctx.device, &self.ctx.queue, &mut encoder, &surface_view, resources, &matching);
                    pipeline.render(&self.ctx.device, &self.ctx.queue, &mut encoder, &self.background.view, resources, &matching);
                }
            }

            if !layer0.texts.is_empty() {
                if self.text.prepare(&self.ctx.device, &self.ctx.queue, (self.ctx.config.width, self.ctx.config.height), layer0.texts).is_ok() {
                    {
                        let mut text_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("text layer 0 pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &surface_view,
                                resolve_target: None,
                                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                            })],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });
                        let _ = self.text.render(&mut text_pass);
                    }

                    {
                        let mut text_pass_bg = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("text layer 0 bg pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &self.background.view,
                                resolve_target: None,
                                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                            })],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });
                        let _ = self.text.render(&mut text_pass_bg);
                    }
                }
            }

            // Submit Layer 0 pass to isolate glyphon vertex buffers
            self.ctx.queue.submit(std::iter::once(encoder.finish()));
            encoder = self.ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("overlay encoder"),
            });

            // Compute Dual-Kawase blur on base UI background
            self.blur_pipeline.downsample(
                &self.ctx.device,
                &self.ctx.queue,
                &mut encoder,
                &self.background.view,
                self.background.size,
                &self.blur_half.view,
            );
            self.blur_pipeline.upsample(
                &self.ctx.device,
                &self.ctx.queue,
                &mut encoder,
                &self.blur_half.view,
                self.blur_half.size,
                &self.blurred_full.view,
            );

            // --- Layer 1+ (Modal / Overlays): samples genuine blur of underlying UI ---
            for layer in &layers[1..] {
                if !layer.instances.is_empty() {
                    self.sdf_pipeline.upload_instances(&self.ctx.device, &self.ctx.queue, layer.instances);
                    self.sdf_pipeline.render(&self.ctx.device, &mut encoder, &surface_view, &self.blurred_full.view);
                }

                if !layer.texts.is_empty() {
                    if self.text.prepare(&self.ctx.device, &self.ctx.queue, (self.ctx.config.width, self.ctx.config.height), layer.texts).is_ok() {
                        let mut text_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("text layer overlay pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &surface_view,
                                resolve_target: None,
                                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                            })],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });
                        let _ = self.text.render(&mut text_pass);
                    }
                }
            }
        } else if let Some(layer) = layers.first() {
            if !layer.instances.is_empty() {
                self.sdf_pipeline.upload_instances(&self.ctx.device, &self.ctx.queue, layer.instances);
                self.sdf_pipeline.render(&self.ctx.device, &mut encoder, &surface_view, &self.blurred_full.view);
            }

            // Render registered media pipelines between SDF background quads and text overlays
            for pipeline in self.media_pipelines.iter_mut() {
                let matching: Vec<MediaInstance> =
                    media.iter().filter(|instance| instance.kind == pipeline.kind()).cloned().collect();
                if !matching.is_empty() {
                    pipeline.render(&self.ctx.device, &self.ctx.queue, &mut encoder, &surface_view, resources, &matching);
                }
            }

            if !layer.texts.is_empty() {
                if self.text.prepare(&self.ctx.device, &self.ctx.queue, (self.ctx.config.width, self.ctx.config.height), layer.texts).is_ok() {
                    let mut text_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("text overlay pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &surface_view,
                            resolve_target: None,
                            ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    let _ = self.text.render(&mut text_pass);
                }
            }
        }

        self.ctx.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
        self.text.trim_atlas();
        Ok(())
    }

    /// Single-layer frame rendering helper.
    pub fn render(
        &mut self,
        background_clear: wgpu::Color,
        instances: &[GpuSdfInstance],
        text_runs: &[TextRun],
        media: &[MediaInstance],
        resources: &ResourceTable,
    ) -> Result<(), wgpu::SurfaceError> {
        let layer = RenderLayer { instances, texts: text_runs };
        self.render_layers(background_clear, &[layer], media, resources)
    }
}

fn to_glyphon_color(color: [f32; 4]) -> glyphon::Color {
    let to_u8 = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
    glyphon::Color::rgba(to_u8(color[0]), to_u8(color[1]), to_u8(color[2]), to_u8(color[3]))
}

fn clear_target(encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView, color: wgpu::Color) {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("clear pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations { load: wgpu::LoadOp::Clear(color), store: wgpu::StoreOp::Store },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });
}
