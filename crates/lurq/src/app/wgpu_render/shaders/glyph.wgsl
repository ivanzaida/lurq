// Glyph pipeline: instanced textured quads. The atlas is a single-
// channel `R8Unorm` mask; each glyph quad samples it and multiplies by
// the per-instance color (RGB + alpha). Premultiplied-alpha blending is
// done by the pipeline blend state.

// Per-clip-range uniform block; mirrors quad.wgsl's `Globals`. The
// fragment stage uses `clip_*` to discard glyphs that fall outside
// the active rounded clip — same SDF as the quad shader uses for
// its own corners, applied here purely as a mask.
struct Globals {
    viewport:     vec4<f32>,
    clip_rect:    vec4<f32>,
    clip_radii_h: vec4<f32>,
    clip_radii_v: vec4<f32>,
    clip_active:  vec4<f32>,
}

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;

/// Pick the (h, v) corner radius for whichever quadrant `p` lies in.
/// `p` is centre-relative pixel coords. Order: TL, TR, BR, BL.
fn pick_radius(p: vec2<f32>, rh: vec4<f32>, rv: vec4<f32>) -> vec2<f32> {
    if (p.y < 0.0) {
        if (p.x < 0.0) { return vec2<f32>(rh.x, rv.x); }
        else           { return vec2<f32>(rh.y, rv.y); }
    } else {
        if (p.x < 0.0) { return vec2<f32>(rh.w, rv.w); }
        else           { return vec2<f32>(rh.z, rv.z); }
    }
}

/// SDF of a rounded box with elliptical corners. Same code as
/// quad.wgsl — duplicated rather than shared because WGSL has no
/// cross-shader includes.
fn sd_rounded_box(p: vec2<f32>, half_size: vec2<f32>, r: vec2<f32>) -> f32 {
    let safe_r = max(r, vec2<f32>(0.0, 0.0));
    let q = abs(p) - half_size + safe_r;
    if (q.x > 0.0 && q.y > 0.0) {
        if (safe_r.x <= 0.0 || safe_r.y <= 0.0) {
            return max(q.x, q.y);
        }
        let pn = q / safe_r;
        let l = length(pn);
        let g = max(length(pn / safe_r), 1e-6);
        return (l - 1.0) * l / g;
    }
    return max(q.x - safe_r.x, q.y - safe_r.y);
}

struct VsIn {
    @location(0) corner: vec2<f32>,    // unit-quad corner in [0,1]
    @location(1) pos: vec2<f32>,       // top-left in physical pixels
    @location(2) size: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) uv_min: vec2<f32>,
    @location(5) uv_max: vec2<f32>,
    @location(6) transform: vec4<f32>, // 2x2 matrix: a, b, c, d
    @location(7) xf_origin: vec2<f32>, // transform origin relative to rect top-left
    @location(8) sharpness: f32,
}

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) sharpness: f32,
}

@vertex
fn vs_main(in: VsIn) -> VsOut {
    let local_px = in.corner * in.size;
    let centered = local_px - in.xf_origin;
    let rotated = vec2<f32>(
        in.transform.x * centered.x + in.transform.z * centered.y,
        in.transform.y * centered.x + in.transform.w * centered.y,
    );
    let world = in.pos + rotated + in.xf_origin;
    let viewport = globals.viewport.xy;
    let ndc_x = (world.x / viewport.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (world.y / viewport.y) * 2.0;
    let uv = mix(in.uv_min, in.uv_max, in.corner);

    var out: VsOut;
    out.clip = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.color = in.color;
    out.uv = uv;
    out.sharpness = in.sharpness;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    var clip_alpha = 1.0;
    // Rounded-clip discard. See quad.wgsl for the matching logic.
    if (globals.clip_active.x > 0.5) {
        let frag_pos = in.clip.xy;
        let cr = globals.clip_rect;
        let half = vec2<f32>(cr.z, cr.w) * 0.5;
        let centre = vec2<f32>(cr.x + half.x, cr.y + half.y);
        let local_clip = frag_pos - centre;
        let r = pick_radius(local_clip, globals.clip_radii_h, globals.clip_radii_v);
        let d = sd_rounded_box(local_clip, half, r);
        let aa = max(fwidth(d), 0.001);
        clip_alpha = clamp(0.5 - d / aa, 0.0, 1.0);
        if (clip_alpha <= 0.0) {
            discard;
        }
    }

    // Instance colors arrive in linear space and are written to an sRGB
    // surface, matching the quad pipeline.
    var coverage = textureSample(atlas, atlas_sampler, in.uv).r;
    coverage = clamp((coverage - 0.5) * max(in.sharpness, 1.0) + 0.5, 0.0, 1.0);
    return vec4<f32>(in.color.rgb, in.color.a * coverage * clip_alpha);
}
