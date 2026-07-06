// Glyph pipeline: instanced textured quads. The atlas stores RGBA pixels.
// Monochrome glyphs use white RGB plus alpha coverage and are tinted by the
// per-instance color. Color glyphs use their atlas RGB directly.

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
        if (l <= 1e-6) {
            return -min(safe_r.x, safe_r.y);
        }
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
    @location(9) color_glyph: f32,
    @location(10) shadow_sigma: f32,   // > 0 marks a blurred text-shadow instance
}

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) sharpness: f32,
    @location(3) color_glyph: f32,
    @location(4) shadow_sigma: f32,
    @location(5) uv_bounds: vec4<f32>, // glyph rect in atlas uv, for masking blur taps
}

/// Blur taps reach `2 * sigma` texels; the quad is padded one extra texel so
/// bilinear sampling at the edge stays inside the expanded region.
fn shadow_pad(sigma: f32) -> f32 {
    if (sigma <= 0.0) {
        return 0.0;
    }
    return ceil(sigma * 2.0) + 1.0;
}

@vertex
fn vs_main(in: VsIn) -> VsOut {
    // Shadow instances grow beyond the glyph rect so the blur has room to
    // spill; uv is extrapolated through the same linear mapping.
    let pad = shadow_pad(in.shadow_sigma);
    let local_px = in.corner * (in.size + 2.0 * pad) - vec2<f32>(pad, pad);
    let centered = local_px - in.xf_origin;
    let rotated = vec2<f32>(
        in.transform.x * centered.x + in.transform.z * centered.y,
        in.transform.y * centered.x + in.transform.w * centered.y,
    );
    let world = in.pos + rotated + in.xf_origin;
    let viewport = globals.viewport.xy;
    let ndc_x = (world.x / viewport.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (world.y / viewport.y) * 2.0;
    let uv = mix(in.uv_min, in.uv_max, local_px / max(in.size, vec2<f32>(1e-6, 1e-6)));

    var out: VsOut;
    out.clip = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.color = in.color;
    out.uv = uv;
    out.sharpness = in.sharpness;
    out.color_glyph = in.color_glyph;
    out.shadow_sigma = in.shadow_sigma;
    out.uv_bounds = vec4<f32>(in.uv_min, in.uv_max);
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
    // surface, matching the quad pipeline. Sampled before any non-uniform
    // branching because textureSample needs implicit derivatives.
    let sample = textureSample(atlas, atlas_sampler, in.uv);

    // Text-shadow instances: Gaussian-blur the glyph's alpha coverage. Taps
    // outside the glyph's atlas rect are masked to zero so neighbouring atlas
    // entries never bleed in. Uses textureSampleLevel because the loop is
    // non-uniform control flow.
    if (in.shadow_sigma > 0.0) {
        let texel = 1.0 / vec2<f32>(textureDimensions(atlas));
        let radius = i32(ceil(in.shadow_sigma * 2.0));
        let inv_two_sigma2 = 1.0 / (2.0 * in.shadow_sigma * in.shadow_sigma);
        var sum = 0.0;
        var weight_sum = 0.0;
        for (var dy = -radius; dy <= radius; dy = dy + 1) {
            for (var dx = -radius; dx <= radius; dx = dx + 1) {
                let offset = vec2<f32>(f32(dx), f32(dy));
                let weight = exp(-dot(offset, offset) * inv_two_sigma2);
                let tap_uv = in.uv + offset * texel;
                let inside = step(in.uv_bounds.x, tap_uv.x) * step(in.uv_bounds.y, tap_uv.y)
                    * step(tap_uv.x, in.uv_bounds.z) * step(tap_uv.y, in.uv_bounds.w);
                sum += weight * textureSampleLevel(atlas, atlas_sampler, tap_uv, 0.0).a * inside;
                weight_sum += weight;
            }
        }
        let shadow_coverage = sum / max(weight_sum, 1e-6);
        return vec4<f32>(in.color.rgb, in.color.a * shadow_coverage * clip_alpha);
    }

    if (in.color_glyph > 0.5) {
        return vec4<f32>(sample.rgb, sample.a * in.color.a * clip_alpha);
    }

    var coverage = sample.a;
    coverage = clamp((coverage - 0.5) * max(in.sharpness, 1.0) + 0.5, 0.0, 1.0);
    return vec4<f32>(in.color.rgb, in.color.a * coverage * clip_alpha);
}
