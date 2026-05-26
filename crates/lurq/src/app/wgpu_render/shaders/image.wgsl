// Image pipeline: instanced textured quads sampling an Rgba8UnormSrgb
// texture. Each instance covers one <img> element's content rect and
// maps the full [0,1]² UV range to the image texture.

struct Globals {
    viewport:     vec4<f32>,
    clip_rect:    vec4<f32>,
    clip_radii_h: vec4<f32>,
    clip_radii_v: vec4<f32>,
    clip_active:  vec4<f32>,
}

@group(0) @binding(0) var<uniform> globals: Globals;
@group(0) @binding(1) var img_tex: texture_2d<f32>;
@group(0) @binding(2) var img_sampler: sampler;

/// Pick the (h, v) corner radius for whichever quadrant `p` lies in.
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
        let g = max(length(pn / safe_r), 1e-6);
        return (l - 1.0) * l / g;
    }
    return max(q.x - safe_r.x, q.y - safe_r.y);
}

struct VsIn {
    @location(0) corner: vec2<f32>,
    @location(1) pos: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) opacity: vec4<f32>,
    @location(4) transform: vec4<f32>,  // 2x2 matrix: a, b, c, d
    @location(5) xf_origin: vec2<f32>,  // transform origin relative to rect top-left
}

struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) opacity: f32,
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

    var out: VsOut;
    out.clip = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.uv = in.corner;
    out.opacity = in.opacity.x;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // Rounded-clip discard.
    if (globals.clip_active.x > 0.5) {
        let frag_pos = in.clip.xy;
        let cr = globals.clip_rect;
        let half = vec2<f32>(cr.z, cr.w) * 0.5;
        let centre = vec2<f32>(cr.x + half.x, cr.y + half.y);
        let local_clip = frag_pos - centre;
        let r = pick_radius(local_clip, globals.clip_radii_h, globals.clip_radii_v);
        let d = sd_rounded_box(local_clip, half, r);
        if (d > 0.5) {
            discard;
        }
    }

    var color = textureSample(img_tex, img_sampler, in.uv);
    color.a *= in.opacity;
    // The surface is sRGB; the texture is Rgba8UnormSrgb so the
    // hardware already decoded to linear. The sRGB surface view
    // re-encodes on write. Just output linear RGBA.
    return color;
}
