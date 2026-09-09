// [WFGY] Zone: RISK | delta_s: 0.6 | κ: High | η: Low | Action: Rendu instancie de cartes SDF (rounded-rect + halo neon + glass)
// Layout d'instance en bijection stricte avec ui_core::GpuSdfInstance (INV-GPU-1).
// bounds       : vec4<f32>  [x, y, width, height]  (espace ecran, pixels, origine haut-gauche)
// bg_color     : vec4<f32>  couleur de fond (translucidite glass dans .a)
// glow_color   : vec4<f32>  couleur du halo neon
// radius       : f32        rayon de coin arrondi (pixels)
// border_width : f32        epaisseur de bordure nette (pixels)
// glow_intensity: f32       intensite du halo (0 = pas de glow)
// blur_factor  : f32        poids d'echantillonnage de la texture de fond floutee
// clip_bounds  : vec4<f32>  [x, y, width, height] region hors de laquelle le quad est invisible (scroll)

struct Globals {
    screen_size: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var blurred_bg_tex: texture_2d<f32>;
@group(0) @binding(2) var blurred_bg_sampler: sampler;

struct InstanceInput {
    @location(0) bounds: vec4<f32>,
    @location(1) bg_color: vec4<f32>,
    @location(2) glow_color: vec4<f32>,
    @location(3) radius: f32,
    @location(4) border_width: f32,
    @location(5) glow_intensity: f32,
    @location(6) blur_factor: f32,
    @location(7) clip_bounds: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local_pos: vec2<f32>,   // pixels, relatif au centre du rect
    @location(1) half_size: vec2<f32>,   // demi-taille du rect (sans le padding de glow)
    @location(2) bg_color: vec4<f32>,
    @location(3) glow_color: vec4<f32>,
    @location(4) radius: f32,
    @location(5) border_width: f32,
    @location(6) glow_intensity: f32,
    @location(7) blur_factor: f32,
    @location(8) screen_uv: vec2<f32>,
    @location(9) glow_padding: f32,
    @location(10) clip_bounds: vec4<f32>,
};

// Marge en pixels ajoutee autour du rect pour laisser respirer le halo neon
// sans le clipper au bord du quad instancie.
const GLOW_PADDING_PER_UNIT: f32 = 18.0;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;

    let rect_origin = instance.bounds.xy;
    let rect_size = instance.bounds.zw;
    let half_size = rect_size * 0.5;
    let center = rect_origin + half_size;

    let padding = instance.glow_intensity * GLOW_PADDING_PER_UNIT + instance.border_width;
    let padded_half_size = half_size + vec2<f32>(padding, padding);

    // Quad plein ecran-instance via triangle-strip a 4 sommets, sans vertex buffer.
    var corners = array<vec2<f32>, 4>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(1.0, 1.0),
    );
    let corner = corners[vertex_index];

    let local_pixel_pos = corner * padded_half_size;
    let world_pos = center + local_pixel_pos;

    // Conversion pixels -> NDC (origine haut-gauche, Y vers le bas).
    let ndc_x = (world_pos.x / globals.screen_size.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (world_pos.y / globals.screen_size.y) * 2.0;

    out.clip_position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.local_pos = local_pixel_pos;
    out.half_size = half_size;
    out.bg_color = instance.bg_color;
    out.glow_color = instance.glow_color;
    out.radius = instance.radius;
    out.border_width = instance.border_width;
    out.glow_intensity = instance.glow_intensity;
    out.blur_factor = instance.blur_factor;
    out.screen_uv = world_pos / globals.screen_size;
    out.glow_padding = padding;
    out.clip_bounds = instance.clip_bounds;
    return out;
}

// Signed distance a un rectangle a coins arrondis, centre en (0,0).
// Reference: Inigo Quilez, "2D distance functions" (rounded box).
fn sd_rounded_box(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let q = abs(p) - half_size + vec2<f32>(radius, radius);
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Clipping (zones defilantes) : au-dela de clip_bounds, le fragment
    // n'existe simplement pas — coupe nette "overflow: hidden", y compris
    // pour le halo neon (qui ne doit pas deborder d'un viewport de scroll).
    let world_pos = in.screen_uv * globals.screen_size;
    let clip_min = in.clip_bounds.xy;
    let clip_max = in.clip_bounds.xy + in.clip_bounds.zw;
    if (world_pos.x < clip_min.x || world_pos.x > clip_max.x || world_pos.y < clip_min.y || world_pos.y > clip_max.y) {
        discard;
    }

    let dist = sd_rounded_box(in.local_pos, in.half_size, in.radius);

    // Glass : fond floute echantillonne, teinte par bg_color selon son alpha.
    let bg_sample = textureSample(blurred_bg_tex, blurred_bg_sampler, in.screen_uv).rgb;

    // Gradient vertical et brillance de surface (lumiere zenithale douce activee si glow_intensity > 0).
    // norm_y varie de 0.0 (haut du quad) a 1.0 (bas du quad).
    let norm_y = clamp((in.local_pos.y / in.half_size.y) * 0.5 + 0.5, 0.0, 1.0);
    let surface_gradient = ((1.0 - norm_y) * 0.10 - norm_y * 0.04) * in.glow_intensity;
    let surface_rgb = clamp(in.bg_color.rgb + vec3<f32>(surface_gradient), vec3<f32>(0.0), vec3<f32>(1.0));

    // Reflet speculaire fin sur le biseau superieur
    let top_rim = smoothstep(0.08, 0.0, norm_y) * smoothstep(-in.half_size.y, -in.border_width, in.local_pos.y);
    let specular = vec3<f32>(1.0, 1.0, 1.0) * top_rim * 0.12 * in.glow_intensity;

    let glass_rgb = mix(bg_sample, surface_rgb, in.bg_color.a) + specular;

    // Remplissage : anti-aliasing 1px sur le bord (smoothstep autour de dist = 0).
    let fill_alpha = 1.0 - smoothstep(-1.0, 1.0, dist);

    // Bordure nette avec arete superieure plus lumineuse (biseau eclairé).
    let border_raw = fill_alpha * smoothstep(-in.border_width - 0.75, -in.border_width + 0.75, dist);
    let border_alpha = select(0.0, border_raw, in.border_width > 0.0);
    let top_border_highlight = (1.0 - norm_y) * 0.35 * in.glow_intensity;
    let base_border_color = mix(in.bg_color.rgb, in.glow_color.rgb, 0.85);
    let border_color = clamp(base_border_color + vec3<f32>(top_border_highlight), vec3<f32>(0.0), vec3<f32>(1.0));

    // Lueur d'ambiance interne pour les elements actifs
    let inner_dist = max(-dist, 0.0);
    let inner_glow = exp(-0.55 * inner_dist) * in.glow_intensity * 0.10;
    let inner_rgb = in.glow_color.rgb * inner_glow;

    var color = mix(glass_rgb, border_color, border_alpha) + inner_rgb;
    var alpha = clamp(fill_alpha * in.bg_color.a + border_alpha * in.glow_color.a, 0.0, 1.0);

    // Halo neon : decroissance exponentielle continue et resserree
    let outside_dist = max(dist, 0.0);
    let glow_falloff = 0.35;
    let glow_raw = exp(-glow_falloff * outside_dist) * in.glow_intensity;
    let edge_fade = 1.0 - smoothstep(in.glow_padding * 0.7, in.glow_padding, outside_dist);
    let glow = glow_raw * edge_fade;
    color = color + in.glow_color.rgb * glow * (1.0 - fill_alpha);
    alpha = max(alpha, glow * in.glow_color.a);

    return vec4<f32>(color, alpha);
}
