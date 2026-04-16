//! Shader pipelines for the `SideX` GPU renderer.
//!
//! Pipelines:
//! - **Text** — monochrome glyph quads (alpha mask from R8 atlas).
//! - **Subpixel text** — subpixel-antialiased glyph quads (RGB coverage from Rgba8 atlas).
//! - **Rectangle** — filled/outlined rounded rectangles via SDF.
//! - **Shadow** — box shadow via SDF with blur.
//! - **Underline** — straight or wavy underlines/strikethroughs.

use crate::vertex::{RectVertex, TextVertex};

// ---------------------------------------------------------------------------
// WGSL sources
// ---------------------------------------------------------------------------

const TEXT_SHADER_SRC: &str = r"
struct Uniforms {
    projection: mat4x4<f32>,
    camera_offset: vec2<f32>,
    _pad: vec2<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var atlas_texture: texture_2d<f32>;
@group(1) @binding(1) var atlas_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = in.position - uniforms.camera_offset;
    out.clip_position = uniforms.projection * vec4<f32>(world_pos, 0.0, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let alpha = textureSample(atlas_texture, atlas_sampler, in.uv).r;
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
";

const SUBPIXEL_TEXT_SHADER_SRC: &str = r"
struct Uniforms {
    projection: mat4x4<f32>,
    camera_offset: vec2<f32>,
    _pad: vec2<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(1) @binding(0) var atlas_texture: texture_2d<f32>;
@group(1) @binding(1) var atlas_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = in.position - uniforms.camera_offset;
    out.clip_position = uniforms.projection * vec4<f32>(world_pos, 0.0, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let coverage = textureSample(atlas_texture, atlas_sampler, in.uv);
    return vec4<f32>(
        in.color.r * coverage.r,
        in.color.g * coverage.g,
        in.color.b * coverage.b,
        max(max(coverage.r, coverage.g), coverage.b) * in.color.a
    );
}
";

const RECT_SHADER_SRC: &str = r"
struct Uniforms {
    projection: mat4x4<f32>,
    camera_offset: vec2<f32>,
    _pad: vec2<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) rect_min: vec2<f32>,
    @location(2) rect_max: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) corner_radius: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) rect_min: vec2<f32>,
    @location(1) rect_max: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) corner_radius: f32,
    @location(4) pixel_pos: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = in.position - uniforms.camera_offset;
    out.clip_position = uniforms.projection * vec4<f32>(world_pos, 0.0, 1.0);
    out.rect_min = in.rect_min - uniforms.camera_offset;
    out.rect_max = in.rect_max - uniforms.camera_offset;
    out.color = in.color;
    out.corner_radius = in.corner_radius;
    out.pixel_pos = world_pos;
    return out;
}

fn rounded_rect_sdf(pixel: vec2<f32>, rect_min: vec2<f32>, rect_max: vec2<f32>, radius: f32) -> f32 {
    let half_size = (rect_max - rect_min) * 0.5;
    let center = rect_min + half_size;
    let r = min(radius, min(half_size.x, half_size.y));
    let q = abs(pixel - center) - half_size + vec2<f32>(r, r);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dist = rounded_rect_sdf(in.pixel_pos, in.rect_min, in.rect_max, in.corner_radius);
    let aa = fwidth(dist);
    let alpha = 1.0 - smoothstep(-aa, aa, dist);
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
";

const SHADOW_SHADER_SRC: &str = r"
struct Uniforms {
    projection: mat4x4<f32>,
    camera_offset: vec2<f32>,
    _pad: vec2<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) rect_min: vec2<f32>,
    @location(2) rect_max: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) corner_radius: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) rect_min: vec2<f32>,
    @location(1) rect_max: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) corner_radius: f32,
    @location(4) pixel_pos: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = in.position - uniforms.camera_offset;
    out.clip_position = uniforms.projection * vec4<f32>(world_pos, 0.0, 1.0);
    out.rect_min = in.rect_min - uniforms.camera_offset;
    out.rect_max = in.rect_max - uniforms.camera_offset;
    out.color = in.color;
    out.corner_radius = in.corner_radius;
    out.pixel_pos = world_pos;
    return out;
}

fn rounded_rect_sdf(pixel: vec2<f32>, rect_min: vec2<f32>, rect_max: vec2<f32>, radius: f32) -> f32 {
    let half_size = (rect_max - rect_min) * 0.5;
    let center = rect_min + half_size;
    let r = min(radius, min(half_size.x, half_size.y));
    let q = abs(pixel - center) - half_size + vec2<f32>(r, r);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - r;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dist = rounded_rect_sdf(in.pixel_pos, in.rect_min, in.rect_max, in.corner_radius);
    // corner_radius doubles as blur_radius for shadows
    let blur = max(in.corner_radius, 1.0);
    let alpha = 1.0 - smoothstep(-blur, blur, dist);
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
";

const UNDERLINE_SHADER_SRC: &str = r"
struct Uniforms {
    projection: mat4x4<f32>,
    camera_offset: vec2<f32>,
    _pad: vec2<f32>,
};
@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) rect_min: vec2<f32>,
    @location(2) rect_max: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) corner_radius: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) rect_min: vec2<f32>,
    @location(1) rect_max: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) corner_radius: f32,
    @location(4) pixel_pos: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_pos = in.position - uniforms.camera_offset;
    out.clip_position = uniforms.projection * vec4<f32>(world_pos, 0.0, 1.0);
    out.rect_min = in.rect_min - uniforms.camera_offset;
    out.rect_max = in.rect_max - uniforms.camera_offset;
    out.color = in.color;
    out.corner_radius = in.corner_radius;
    out.pixel_pos = world_pos;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // corner_radius > 0 means wavy underline
    if in.corner_radius > 0.0 {
        let wave_freq = 6.2832 / 6.0;
        let wave_amp = in.corner_radius;
        let center_y = (in.rect_min.y + in.rect_max.y) * 0.5;
        let wave_y = center_y + sin(in.pixel_pos.x * wave_freq) * wave_amp;
        let thickness = (in.rect_max.y - in.rect_min.y) * 0.5;
        let dist = abs(in.pixel_pos.y - wave_y) - thickness;
        let aa = fwidth(dist);
        let alpha = 1.0 - smoothstep(-aa, aa, dist);
        return vec4<f32>(in.color.rgb, in.color.a * alpha);
    }
    return in.color;
}
";

// ---------------------------------------------------------------------------
// Pipeline construction helpers
// ---------------------------------------------------------------------------

fn alpha_blend_state() -> wgpu::BlendState {
    wgpu::BlendState {
        color: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::SrcAlpha,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
        alpha: wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        },
    }
}

/// Creates the uniform bind group layout shared by all pipelines.
/// Now includes camera offset (vec2<f32>) alongside the projection matrix.
pub fn create_uniform_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("uniform_bind_group_layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    })
}

/// Creates the bind group layout for atlas texture + sampler.
pub fn create_atlas_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("atlas_bind_group_layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
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
    })
}

fn create_textured_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    uniform_bgl: &wgpu::BindGroupLayout,
    atlas_bgl: &wgpu::BindGroupLayout,
    shader_src: &str,
    label: &str,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(shader_src.into()),
    });

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{label}_layout")),
        bind_group_layouts: &[uniform_bgl, atlas_bgl],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[TextVertex::layout()],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(alpha_blend_state()),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        multiview: None,
        cache: None,
    })
}

fn create_rect_like_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    uniform_bgl: &wgpu::BindGroupLayout,
    shader_src: &str,
    label: &str,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(shader_src.into()),
    });

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(&format!("{label}_layout")),
        bind_group_layouts: &[uniform_bgl],
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[RectVertex::layout()],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(alpha_blend_state()),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        multiview: None,
        cache: None,
    })
}

/// Creates the monochrome text pipeline (alpha mask from R8 atlas).
pub fn create_text_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    uniform_bgl: &wgpu::BindGroupLayout,
    atlas_bgl: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_textured_pipeline(device, format, uniform_bgl, atlas_bgl, TEXT_SHADER_SRC, "text_pipeline")
}

/// Creates the subpixel-antialiased text pipeline (RGB coverage from Rgba8 atlas).
pub fn create_subpixel_text_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    uniform_bgl: &wgpu::BindGroupLayout,
    atlas_bgl: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_textured_pipeline(device, format, uniform_bgl, atlas_bgl, SUBPIXEL_TEXT_SHADER_SRC, "subpixel_text_pipeline")
}

/// Creates the render pipeline for drawing colored, rounded rectangles.
pub fn create_rect_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    uniform_bgl: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_rect_like_pipeline(device, format, uniform_bgl, RECT_SHADER_SRC, "rect_pipeline")
}

/// Creates the box shadow pipeline.
pub fn create_shadow_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    uniform_bgl: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_rect_like_pipeline(device, format, uniform_bgl, SHADOW_SHADER_SRC, "shadow_pipeline")
}

/// Creates the underline/strikethrough pipeline.
pub fn create_underline_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    uniform_bgl: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    create_rect_like_pipeline(device, format, uniform_bgl, UNDERLINE_SHADER_SRC, "underline_pipeline")
}
