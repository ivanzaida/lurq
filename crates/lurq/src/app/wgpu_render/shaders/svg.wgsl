struct Globals {
    viewport:     vec4<f32>,
    clip_rect:    vec4<f32>,
    clip_radii_h: vec4<f32>,
    clip_radii_v: vec4<f32>,
    clip_active:  vec4<f32>,
    // The rounded clip outside the innermost one: x, y, w, h, then radii
    // (tl, tr, br, bl). Cut by when `clip_active.y` is 1.
    outer_clip_rect:  vec4<f32>,
    outer_clip_radii: vec4<f32>,
}

@group(0) @binding(0) var<uniform> globals: Globals;

struct VsIn {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
}

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@vertex
fn vs_main(in: VsIn) -> VsOut {
    // `viewport.zw` is the target's origin in window pixels (non-zero when
    // painting into an opacity layer).
    let viewport = globals.viewport.xy;
    let target_px = in.position - globals.viewport.zw;
    let ndc_x = (target_px.x / viewport.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (target_px.y / viewport.y) * 2.0;

    var out: VsOut;
    out.clip = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.color = in.color;
    return out;
}

fn pick_radius(p: vec2<f32>, rh: vec4<f32>, rv: vec4<f32>) -> vec2<f32> {
    if (p.y < 0.0) {
        if (p.x < 0.0) { return vec2<f32>(rh.x, rv.x); }
        else           { return vec2<f32>(rh.y, rv.y); }
    } else {
        if (p.x < 0.0) { return vec2<f32>(rh.w, rv.w); }
        else           { return vec2<f32>(rh.z, rv.z); }
    }
}

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

// The render target is not sRGB, so the fixed-function blend mixes
// sRGB-encoded values, as CSS and design tools do. `fs_main` works in linear
// light like the rest of the pipeline and encodes each result it returns.
fn encode_srgb(color: vec4<f32>) -> vec4<f32> {
    let c = clamp(color.rgb, vec3<f32>(0.0), vec3<f32>(1.0));
    let low = c * 12.92;
    let high = 1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055;
    return vec4<f32>(select(high, low, c <= vec3<f32>(0.0031308)), color.a);
}

// How much of the pixel at `frag_pos` (window pixels) the rounded clip at
// `rect` with these radii covers.
fn rounded_clip_alpha(frag_pos: vec2<f32>, rect: vec4<f32>, radii_h: vec4<f32>, radii_v: vec4<f32>) -> f32 {
    let half = rect.zw * 0.5;
    let local_clip = frag_pos - (rect.xy + half);
    let r = pick_radius(local_clip, radii_h, radii_v);
    let d = sd_rounded_box(local_clip, half, r);
    let aa = max(fwidth(d), 1.0);
    return clamp(0.5 - d / aa, 0.0, 1.0);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    var clip_alpha = 1.0;
    if (globals.clip_active.x > 0.5) {
        clip_alpha = rounded_clip_alpha(in.clip.xy, globals.clip_rect, globals.clip_radii_h, globals.clip_radii_v);
    }
    // Under a second rounded clip, the draw is cut by both.
    if (globals.clip_active.y > 0.5) {
        clip_alpha *= rounded_clip_alpha(in.clip.xy, globals.outer_clip_rect, globals.outer_clip_radii, globals.outer_clip_radii);
    }
    if (clip_alpha <= 0.0) {
        discard;
    }

    return encode_srgb(vec4<f32>(in.color.rgb, in.color.a * clip_alpha));
}
