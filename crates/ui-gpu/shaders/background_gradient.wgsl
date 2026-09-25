// [WFGY] Zone: RISK | delta_s: 0.3 | Action: Atmospheric Procedural Gradient & Grid Background Shader
struct BackgroundUniforms {
    screen_size: vec2<f32>,
    grid_spacing: f32,
    grid_dot_size: f32,

    base_color: vec4<f32>,

    grad1_center: vec2<f32>,
    grad1_radius: vec2<f32>,
    grad1_color: vec4<f32>,

    grad2_center: vec2<f32>,
    grad2_radius: vec2<f32>,
    grad2_color: vec4<f32>,

    grid_opacity: f32,
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
};

@group(0) @binding(0) var<uniform> uniforms: BackgroundUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    // Fullscreen quad without vertex buffers
    var pos = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, 1.0),
    );
    let p = pos[vertex_index];
    out.position = vec4<f32>(p.x, p.y, 0.0, 1.0);
    // UV with origin at top-left
    out.uv = vec2<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let pixel_pos = in.uv * uniforms.screen_size;

    // 1. Base Dark Background Color
    var color = uniforms.base_color.rgb;

    // 2. Radial Gradient 1 (Top Zenith / Aurora)
    let c1 = uniforms.grad1_center * uniforms.screen_size;
    let r1 = uniforms.grad1_radius * uniforms.screen_size;
    let d1 = length((pixel_pos - c1) / max(r1, vec2<f32>(1.0, 1.0)));
    let falloff1 = smoothstep(1.0, 0.0, d1);
    color = color + uniforms.grad1_color.rgb * (falloff1 * uniforms.grad1_color.a);

    // 3. Radial Gradient 2 (Bottom Nadir / Cyan Ray)
    let c2 = uniforms.grad2_center * uniforms.screen_size;
    let r2 = uniforms.grad2_radius * uniforms.screen_size;
    let d2 = length((pixel_pos - c2) / max(r2, vec2<f32>(1.0, 1.0)));
    let falloff2 = smoothstep(1.0, 0.0, d2);
    color = color + uniforms.grad2_color.rgb * (falloff2 * uniforms.grad2_color.a);

    // 4. Subtle Technical Micro-Dot Grid
    if (uniforms.grid_opacity > 0.0 && uniforms.grid_spacing > 0.0) {
        let grid_uv = pixel_pos / uniforms.grid_spacing;
        let cell_coord = fract(grid_uv) - vec2<f32>(0.5, 0.5);
        let dot_dist = length(cell_coord) * uniforms.grid_spacing;
        let dot_mask = smoothstep(uniforms.grid_dot_size + 0.5, uniforms.grid_dot_size - 0.5, dot_dist);
        let grid_dot_color = vec3<f32>(0.28, 0.32, 0.42);
        color = mix(color, color + grid_dot_color, dot_mask * uniforms.grid_opacity);
    }

    return vec4<f32>(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
