// [WFGY] Zone: SAFE | λ: 0.2 | Fallbacks: 0 | Action: Image & Video quad shader with SDF rounded corners & aspect fitting

struct Globals {
    screen_size: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var s_sampler: sampler;
@group(1) @binding(0) var t_texture: texture_2d<f32>;

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    // Instance attributes
    @location(0) bounds: vec4<f32>,       // [x, y, w, h] in screen pixels
    @location(1) clip_bounds: vec4<f32>,  // [x, y, w, h] in screen pixels
    @location(2) radius: f32,             // corner radius in px
    @location(3) fit_mode: u32,           // 0=Fill, 1=Contain, 2=Cover
    @location(4) aspect_ratio: f32,       // texture width / height
    @location(5) opacity: f32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) screen_pos: vec2<f32>,
    @location(1) @interpolate(flat) bounds: vec4<f32>,
    @location(2) @interpolate(flat) clip_bounds: vec4<f32>,
    @location(3) @interpolate(flat) radius: f32,
    @location(4) @interpolate(flat) fit_mode: u32,
    @location(5) @interpolate(flat) aspect_ratio: f32,
    @location(6) @interpolate(flat) opacity: f32,
};

// Generates 2 triangles (6 vertices) covering the instance quad bounds
@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    // Unit quad vertex offsets [0..1]
    var unit_pos = vec2<f32>(0.0, 0.0);
    switch in.vertex_index % 6u {
        case 0u: { unit_pos = vec2<f32>(0.0, 0.0); }
        case 1u: { unit_pos = vec2<f32>(1.0, 0.0); }
        case 2u: { unit_pos = vec2<f32>(0.0, 1.0); }
        case 3u: { unit_pos = vec2<f32>(0.0, 1.0); }
        case 4u: { unit_pos = vec2<f32>(1.0, 0.0); }
        default: { unit_pos = vec2<f32>(1.0, 1.0); }
    }

    let screen_x = in.bounds.x + unit_pos.x * in.bounds.z;
    let screen_y = in.bounds.y + unit_pos.y * in.bounds.w;
    out.screen_pos = vec2<f32>(screen_x, screen_y);

    // Convert screen coordinates to NDC [-1, 1]
    let ndc_x = (screen_x / globals.screen_size.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (screen_y / globals.screen_size.y) * 2.0;

    out.position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.bounds = in.bounds;
    out.clip_bounds = in.clip_bounds;
    out.radius = in.radius;
    out.fit_mode = in.fit_mode;
    out.aspect_ratio = in.aspect_ratio;
    out.opacity = in.opacity;

    return out;
}

// 2D Signed Distance Function for a rounded box
fn sd_rounded_box(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - b + vec2<f32>(r, r);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0, 0.0))) - r;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pos = in.screen_pos;

    // Viewport clipping (e.g. ScrollView bounds)
    if (pos.x < in.clip_bounds.x || pos.x > in.clip_bounds.x + in.clip_bounds.z ||
        pos.y < in.clip_bounds.y || pos.y > in.clip_bounds.y + in.clip_bounds.w) {
        discard;
    }

    // SDF rounded corner evaluation
    let half_size = in.bounds.zw * 0.5;
    let center = in.bounds.xy + half_size;
    let local_pos = pos - center;
    let r = min(in.radius, min(half_size.x, half_size.y));
    let dist = sd_rounded_box(local_pos, half_size, r);

    if (dist > 0.0) {
        discard;
    }
    let edge_alpha = 1.0 - smoothstep(-1.0, 0.0, dist);

    // Normalized UVs across the quad bounds [0..1]
    let u_norm = (pos.x - in.bounds.x) / max(in.bounds.z, 1.0);
    let v_norm = (pos.y - in.bounds.y) / max(in.bounds.w, 1.0);

    var uv = vec2<f32>(u_norm, v_norm);
    let quad_aspect = in.bounds.z / max(in.bounds.w, 1.0);
    let tex_aspect = max(in.aspect_ratio, 0.001);

    if (in.fit_mode == 1u) {
        // FitMode::Contain (Letterbox)
        if (quad_aspect > tex_aspect) {
            let scale = quad_aspect / tex_aspect;
            let u = (u_norm - 0.5) * scale + 0.5;
            if (u < 0.0 || u > 1.0) {
                discard;
            }
            uv = vec2<f32>(u, v_norm);
        } else {
            let scale = tex_aspect / quad_aspect;
            let v = (v_norm - 0.5) * scale + 0.5;
            if (v < 0.0 || v > 1.0) {
                discard;
            }
            uv = vec2<f32>(u_norm, v);
        }
    } else if (in.fit_mode == 2u) {
        // FitMode::Cover (Crop to fill)
        if (quad_aspect > tex_aspect) {
            let scale = tex_aspect / quad_aspect;
            uv = vec2<f32>(u_norm, (v_norm - 0.5) * scale + 0.5);
        } else {
            let scale = quad_aspect / tex_aspect;
            uv = vec2<f32>((u_norm - 0.5) * scale + 0.5, v_norm);
        }
    }

    let tex_color = textureSample(t_texture, s_sampler, uv);

    // Subtle glass border vignette
    let border_w = 1.2;
    let is_border = smoothstep(-border_w - 0.8, -border_w + 0.5, dist);
    let border_color = vec4<f32>(0.0, 0.85, 1.0, 0.35); // Cyber cyan subtle rim

    let final_color = mix(tex_color, border_color, is_border * 0.35);
    return vec4<f32>(final_color.rgb, final_color.a * in.opacity * edge_alpha);
}
