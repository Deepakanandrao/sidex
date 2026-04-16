//! Core GPU renderer — owns the wgpu device, queue, surface, and all pipelines.
//!
//! The renderer now supports:
//! - **Camera transform** — scrolling is a camera offset applied in the uniform
//!   buffer rather than per-vertex. Set via [`set_camera`].
//! - **Scene-based rendering** — [`render_scene`] consumes a finished [`Scene`]
//!   and dispatches batched draw calls in draw order.
//! - **Multiple pipelines** — rect, shadow, underline, text (mask), subpixel text.

use std::sync::Arc;

use thiserror::Error;

use crate::color::Color;
use crate::pipeline;
use crate::scene::{self, PrimitiveBatch, Scene};
use crate::text_atlas::TextAtlas;
use crate::vertex::{RectVertex, TextVertex};

/// Errors that may occur during GPU initialisation or rendering.
#[derive(Debug, Error)]
pub enum GpuError {
    #[error("no suitable GPU adapter found")]
    NoAdapter,
    #[error("failed to request device: {0}")]
    RequestDevice(#[from] wgpu::RequestDeviceError),
    #[error("surface error: {0}")]
    Surface(#[from] wgpu::SurfaceError),
    #[error("failed to create surface: {0}")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),
}

/// Holds the surface texture and command encoder for a single frame.
pub struct FrameContext {
    pub surface_texture: wgpu::SurfaceTexture,
    pub view: wgpu::TextureView,
    pub encoder: wgpu::CommandEncoder,
}

/// Uniform data uploaded to the GPU each frame.
/// Contains the projection matrix and camera offset.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    projection: [f32; 16],
    camera_offset: [f32; 2],
    _pad: [f32; 2],
}

/// The primary GPU renderer.
pub struct GpuRenderer {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,

    // Bind group layouts
    pub uniform_bgl: wgpu::BindGroupLayout,
    pub atlas_bgl: wgpu::BindGroupLayout,

    // Pipelines
    pub text_pipeline: wgpu::RenderPipeline,
    pub subpixel_text_pipeline: wgpu::RenderPipeline,
    pub rect_pipeline: wgpu::RenderPipeline,
    pub shadow_pipeline: wgpu::RenderPipeline,
    pub underline_pipeline: wgpu::RenderPipeline,

    // Uniform buffer + bind group
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,

    // Camera state
    camera_x: f32,
    camera_y: f32,
}

impl GpuRenderer {
    /// Initialises the GPU renderer for the given window.
    #[allow(clippy::too_many_lines)]
    pub async fn new(window: Arc<winit::window::Window>) -> Result<Self, GpuError> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or(GpuError::NoAdapter)?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("sidex_device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    ..Default::default()
                },
                None,
            )
            .await?;

        let device = Arc::new(device);
        let queue = Arc::new(queue);

        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let uniform_bgl = pipeline::create_uniform_bind_group_layout(&device);
        let atlas_bgl = pipeline::create_atlas_bind_group_layout(&device);

        let text_pipeline =
            pipeline::create_text_pipeline(&device, format, &uniform_bgl, &atlas_bgl);
        let subpixel_text_pipeline =
            pipeline::create_subpixel_text_pipeline(&device, format, &uniform_bgl, &atlas_bgl);
        let rect_pipeline = pipeline::create_rect_pipeline(&device, format, &uniform_bgl);
        let shadow_pipeline = pipeline::create_shadow_pipeline(&device, format, &uniform_bgl);
        let underline_pipeline =
            pipeline::create_underline_pipeline(&device, format, &uniform_bgl);

        let uniforms = Uniforms {
            projection: Self::orthographic_projection(
                surface_config.width,
                surface_config.height,
            ),
            camera_offset: [0.0, 0.0],
            _pad: [0.0, 0.0],
        };

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniform_buffer"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(
            &uniform_buffer,
            0,
            bytemuck::bytes_of(&uniforms),
        );

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("uniform_bind_group"),
            layout: &uniform_bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
            uniform_bgl,
            atlas_bgl,
            text_pipeline,
            subpixel_text_pipeline,
            rect_pipeline,
            shadow_pipeline,
            underline_pipeline,
            uniform_buffer,
            uniform_bind_group,
            camera_x: 0.0,
            camera_y: 0.0,
        })
    }

    /// Sets the camera offset (scroll position). Scrolling is just moving the
    /// camera — no per-vertex recalculation needed.
    pub fn set_camera(&mut self, x: f32, y: f32) {
        self.camera_x = x;
        self.camera_y = y;
        self.upload_uniforms();
    }

    /// Returns the current camera offset as `(x, y)`.
    pub fn camera(&self) -> (f32, f32) {
        (self.camera_x, self.camera_y)
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
        self.upload_uniforms();
    }

    pub fn begin_frame(&self) -> Result<FrameContext, GpuError> {
        let surface_texture = self.surface.get_current_texture()?;
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame_encoder"),
            });
        Ok(FrameContext {
            surface_texture,
            view,
            encoder,
        })
    }

    pub fn end_frame(&self, frame: FrameContext) {
        self.queue.submit(std::iter::once(frame.encoder.finish()));
        frame.surface_texture.present();
    }

    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.surface_config.format
    }

    pub fn surface_size(&self) -> (u32, u32) {
        (self.surface_config.width, self.surface_config.height)
    }

    // -----------------------------------------------------------------------
    // Scene-based rendering
    // -----------------------------------------------------------------------

    /// Renders a finished [`Scene`] into the given frame. This dispatches
    /// draw calls in draw order, switching pipelines as needed.
    ///
    /// The atlas must have all glyphs rasterized before calling this.
    #[allow(clippy::too_many_lines)]
    pub fn render_scene(
        &self,
        frame: &mut FrameContext,
        scene: &Scene,
        atlas: &TextAtlas,
        clear_color: Color,
    ) {
        let mask_bind_group = atlas.create_mask_bind_group(&self.device, &self.atlas_bgl);
        let color_bind_group = atlas.create_color_bind_group(&self.device, &self.atlas_bgl);

        let mut pass = frame.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("scene_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &frame.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: f64::from(clear_color.r),
                        g: f64::from(clear_color.g),
                        b: f64::from(clear_color.b),
                        a: f64::from(clear_color.a),
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_bind_group(0, &self.uniform_bind_group, &[]);

        for batch in scene.batches() {
            match batch {
                PrimitiveBatch::Shadows(shadows) => {
                    pass.set_pipeline(&self.shadow_pipeline);
                    Self::draw_shadow_batch(&self.device, &self.queue, &mut pass, shadows);
                }
                PrimitiveBatch::Quads(quads) => {
                    pass.set_pipeline(&self.rect_pipeline);
                    Self::draw_quad_batch(&self.device, &self.queue, &mut pass, quads);
                }
                PrimitiveBatch::Underlines(underlines) => {
                    pass.set_pipeline(&self.underline_pipeline);
                    Self::draw_underline_batch(
                        &self.device,
                        &self.queue,
                        &mut pass,
                        underlines,
                    );
                }
                PrimitiveBatch::MonochromeSprites(sprites) => {
                    pass.set_pipeline(&self.text_pipeline);
                    pass.set_bind_group(1, &mask_bind_group, &[]);
                    Self::draw_sprite_batch(&self.device, &self.queue, &mut pass, sprites);
                }
                PrimitiveBatch::SubpixelSprites(sprites) => {
                    pass.set_pipeline(&self.subpixel_text_pipeline);
                    pass.set_bind_group(1, &color_bind_group, &[]);
                    Self::draw_subpixel_sprite_batch(
                        &self.device,
                        &self.queue,
                        &mut pass,
                        sprites,
                    );
                }
                PrimitiveBatch::PolychromeSprites(sprites) => {
                    pass.set_pipeline(&self.text_pipeline);
                    pass.set_bind_group(1, &color_bind_group, &[]);
                    Self::draw_polychrome_sprite_batch(
                        &self.device,
                        &self.queue,
                        &mut pass,
                        sprites,
                    );
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Batch draw helpers
    // -----------------------------------------------------------------------

    fn draw_quad_batch(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pass: &mut wgpu::RenderPass<'_>,
        quads: &[scene::Quad],
    ) {
        let mut vertices = Vec::with_capacity(quads.len() * 4);
        let mut indices = Vec::with_capacity(quads.len() * 6);
        for q in quads {
            #[allow(clippy::cast_possible_truncation)]
            let base = vertices.len() as u32;
            let make = |vx: f32, vy: f32| RectVertex {
                x: vx,
                y: vy,
                rect_min: [q.x, q.y],
                rect_max: [q.x + q.width, q.y + q.height],
                color: q.color.to_array(),
                corner_radius: q.corner_radius,
                _pad: 0.0,
            };
            vertices.push(make(q.x, q.y));
            vertices.push(make(q.x + q.width, q.y));
            vertices.push(make(q.x + q.width, q.y + q.height));
            vertices.push(make(q.x, q.y + q.height));
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        Self::flush_indexed(device, queue, pass, &vertices, &indices);
    }

    fn draw_shadow_batch(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pass: &mut wgpu::RenderPass<'_>,
        shadows: &[scene::Shadow],
    ) {
        let mut vertices = Vec::with_capacity(shadows.len() * 4);
        let mut indices = Vec::with_capacity(shadows.len() * 6);
        for s in shadows {
            let expand = s.blur_radius + s.spread;
            let x = s.x - expand;
            let y = s.y - expand;
            let w = s.width + expand * 2.0;
            let h = s.height + expand * 2.0;
            #[allow(clippy::cast_possible_truncation)]
            let base = vertices.len() as u32;
            let make = |vx: f32, vy: f32| RectVertex {
                x: vx,
                y: vy,
                rect_min: [s.x, s.y],
                rect_max: [s.x + s.width, s.y + s.height],
                color: s.color.to_array(),
                corner_radius: s.blur_radius,
                _pad: 0.0,
            };
            vertices.push(make(x, y));
            vertices.push(make(x + w, y));
            vertices.push(make(x + w, y + h));
            vertices.push(make(x, y + h));
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        Self::flush_indexed(device, queue, pass, &vertices, &indices);
    }

    fn draw_underline_batch(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pass: &mut wgpu::RenderPass<'_>,
        underlines: &[scene::Underline],
    ) {
        let mut vertices = Vec::with_capacity(underlines.len() * 4);
        let mut indices = Vec::with_capacity(underlines.len() * 6);
        for u in underlines {
            #[allow(clippy::cast_possible_truncation)]
            let base = vertices.len() as u32;
            let wavy_expand = if u.wavy { u.thickness * 2.0 } else { 0.0 };
            let make = |vx: f32, vy: f32| RectVertex {
                x: vx,
                y: vy,
                rect_min: [u.x, u.y],
                rect_max: [u.x + u.width, u.y + u.thickness],
                color: u.color.to_array(),
                corner_radius: if u.wavy { u.thickness } else { 0.0 },
                _pad: 0.0,
            };
            vertices.push(make(u.x, u.y - wavy_expand));
            vertices.push(make(u.x + u.width, u.y - wavy_expand));
            vertices.push(make(u.x + u.width, u.y + u.thickness + wavy_expand));
            vertices.push(make(u.x, u.y + u.thickness + wavy_expand));
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        Self::flush_indexed(device, queue, pass, &vertices, &indices);
    }

    fn draw_sprite_batch(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pass: &mut wgpu::RenderPass<'_>,
        sprites: &[scene::MonochromeSprite],
    ) {
        let mut vertices = Vec::with_capacity(sprites.len() * 4);
        let mut indices = Vec::with_capacity(sprites.len() * 6);
        for s in sprites {
            #[allow(clippy::cast_possible_truncation)]
            let base = vertices.len() as u32;
            let color = s.color.to_array();
            vertices.push(TextVertex { x: s.x, y: s.y, uv_u: s.uv_left, uv_v: s.uv_top, color });
            vertices.push(TextVertex { x: s.x + s.width, y: s.y, uv_u: s.uv_right, uv_v: s.uv_top, color });
            vertices.push(TextVertex { x: s.x + s.width, y: s.y + s.height, uv_u: s.uv_right, uv_v: s.uv_bottom, color });
            vertices.push(TextVertex { x: s.x, y: s.y + s.height, uv_u: s.uv_left, uv_v: s.uv_bottom, color });
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        Self::flush_text_indexed(device, queue, pass, &vertices, &indices);
    }

    fn draw_subpixel_sprite_batch(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pass: &mut wgpu::RenderPass<'_>,
        sprites: &[scene::SubpixelSprite],
    ) {
        let mut vertices = Vec::with_capacity(sprites.len() * 4);
        let mut indices = Vec::with_capacity(sprites.len() * 6);
        for s in sprites {
            #[allow(clippy::cast_possible_truncation)]
            let base = vertices.len() as u32;
            let color = s.color.to_array();
            vertices.push(TextVertex { x: s.x, y: s.y, uv_u: s.uv_left, uv_v: s.uv_top, color });
            vertices.push(TextVertex { x: s.x + s.width, y: s.y, uv_u: s.uv_right, uv_v: s.uv_top, color });
            vertices.push(TextVertex { x: s.x + s.width, y: s.y + s.height, uv_u: s.uv_right, uv_v: s.uv_bottom, color });
            vertices.push(TextVertex { x: s.x, y: s.y + s.height, uv_u: s.uv_left, uv_v: s.uv_bottom, color });
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        Self::flush_text_indexed(device, queue, pass, &vertices, &indices);
    }

    fn draw_polychrome_sprite_batch(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pass: &mut wgpu::RenderPass<'_>,
        sprites: &[scene::PolychromeSprite],
    ) {
        let mut vertices = Vec::with_capacity(sprites.len() * 4);
        let mut indices = Vec::with_capacity(sprites.len() * 6);
        for s in sprites {
            #[allow(clippy::cast_possible_truncation)]
            let base = vertices.len() as u32;
            let color = [1.0, 1.0, 1.0, 1.0];
            vertices.push(TextVertex { x: s.x, y: s.y, uv_u: s.uv_left, uv_v: s.uv_top, color });
            vertices.push(TextVertex { x: s.x + s.width, y: s.y, uv_u: s.uv_right, uv_v: s.uv_top, color });
            vertices.push(TextVertex { x: s.x + s.width, y: s.y + s.height, uv_u: s.uv_right, uv_v: s.uv_bottom, color });
            vertices.push(TextVertex { x: s.x, y: s.y + s.height, uv_u: s.uv_left, uv_v: s.uv_bottom, color });
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        Self::flush_text_indexed(device, queue, pass, &vertices, &indices);
    }

    // -----------------------------------------------------------------------
    // Flush helpers
    // -----------------------------------------------------------------------

    fn flush_indexed(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pass: &mut wgpu::RenderPass<'_>,
        vertices: &[RectVertex],
        indices: &[u32],
    ) {
        if indices.is_empty() {
            return;
        }
        let vb = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("batch_vb"),
            size: std::mem::size_of_val(vertices) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vb, 0, bytemuck::cast_slice(vertices));
        let ib = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("batch_ib"),
            size: std::mem::size_of_val(indices) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&ib, 0, bytemuck::cast_slice(indices));
        #[allow(clippy::cast_possible_truncation)]
        let count = indices.len() as u32;
        pass.set_vertex_buffer(0, vb.slice(..));
        pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..count, 0, 0..1);
    }

    fn flush_text_indexed(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pass: &mut wgpu::RenderPass<'_>,
        vertices: &[TextVertex],
        indices: &[u32],
    ) {
        if indices.is_empty() {
            return;
        }
        let vb = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("text_batch_vb"),
            size: std::mem::size_of_val(vertices) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vb, 0, bytemuck::cast_slice(vertices));
        let ib = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("text_batch_ib"),
            size: std::mem::size_of_val(indices) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&ib, 0, bytemuck::cast_slice(indices));
        #[allow(clippy::cast_possible_truncation)]
        let count = indices.len() as u32;
        pass.set_vertex_buffer(0, vb.slice(..));
        pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..count, 0, 0..1);
    }

    // -----------------------------------------------------------------------
    // Internals
    // -----------------------------------------------------------------------

    fn upload_uniforms(&self) {
        let uniforms = Uniforms {
            projection: Self::orthographic_projection(
                self.surface_config.width,
                self.surface_config.height,
            ),
            camera_offset: [self.camera_x, self.camera_y],
            _pad: [0.0, 0.0],
        };
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));
    }

    #[allow(clippy::cast_precision_loss)]
    fn orthographic_projection(width: u32, height: u32) -> [f32; 16] {
        let w = width as f32;
        let h = height as f32;
        #[rustfmt::skip]
        let m = [
            2.0 / w,  0.0,       0.0, 0.0,
            0.0,     -2.0 / h,   0.0, 0.0,
            0.0,      0.0,       1.0, 0.0,
           -1.0,      1.0,       0.0, 1.0,
        ];
        m
    }
}
