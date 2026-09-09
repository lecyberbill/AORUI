// [WFGY] Zone: RISK | delta_s: 0.55 | κ: High | η: Low | Action: Flou Dual-Kawase (downsample + upsample) pour le glassmorphism
// Technique standard (Kawase / "Dual filtering") : chaque niveau echantillonne
// 4 (downsample) ou 8 (upsample) texels autour du centre avec un offset
// proportionnel a la taille de texel, ce qui approxime un flou gaussien large
// en O(1) sample-pattern par niveau, sans passes horizontales/verticales separees.

struct BlurParams {
    // xy: taille d'un texel de la texture source (1/largeur, 1/hauteur)
    // z : rayon d'echantillonnage (>= 1.0), w : reserve/alignement
    texel_size_and_radius: vec4<f32>,
};

@group(0) @binding(0) var<uniform> params: BlurParams;
@group(0) @binding(1) var src_tex: texture_2d<f32>;
@group(0) @binding(2) var src_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_fullscreen(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Triangle plein-ecran classique (3 sommets, pas de vertex buffer).
    var out: VertexOutput;
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let p = positions[vertex_index];
    out.clip_position = vec4<f32>(p, 0.0, 1.0);
    out.uv = vec2<f32>((p.x + 1.0) * 0.5, 1.0 - (p.y + 1.0) * 0.5);
    return out;
}

@fragment
fn fs_downsample(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = params.texel_size_and_radius.xy;
    let radius = params.texel_size_and_radius.z;
    let uv = in.uv;

    var sum = textureSample(src_tex, src_sampler, uv).rgb * 4.0;
    sum += textureSample(src_tex, src_sampler, uv - texel * radius).rgb;
    sum += textureSample(src_tex, src_sampler, uv + texel * radius).rgb;
    sum += textureSample(src_tex, src_sampler, uv + vec2<f32>(texel.x, -texel.y) * radius).rgb;
    sum += textureSample(src_tex, src_sampler, uv + vec2<f32>(-texel.x, texel.y) * radius).rgb;

    return vec4<f32>(sum / 8.0, 1.0);
}

@fragment
fn fs_upsample(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = params.texel_size_and_radius.xy;
    let radius = params.texel_size_and_radius.z;
    let uv = in.uv;

    var sum = textureSample(src_tex, src_sampler, uv + vec2<f32>(-texel.x * 2.0, 0.0) * radius).rgb;
    sum += textureSample(src_tex, src_sampler, uv + vec2<f32>(-texel.x, texel.y) * radius).rgb * 2.0;
    sum += textureSample(src_tex, src_sampler, uv + vec2<f32>(0.0, texel.y * 2.0) * radius).rgb;
    sum += textureSample(src_tex, src_sampler, uv + vec2<f32>(texel.x, texel.y) * radius).rgb * 2.0;
    sum += textureSample(src_tex, src_sampler, uv + vec2<f32>(texel.x * 2.0, 0.0) * radius).rgb;
    sum += textureSample(src_tex, src_sampler, uv + vec2<f32>(texel.x, -texel.y) * radius).rgb * 2.0;
    sum += textureSample(src_tex, src_sampler, uv + vec2<f32>(0.0, -texel.y * 2.0) * radius).rgb;
    sum += textureSample(src_tex, src_sampler, uv + vec2<f32>(-texel.x, -texel.y) * radius).rgb * 2.0;

    return vec4<f32>(sum / 12.0, 1.0);
}
