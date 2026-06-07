struct Globals {
    viewport:     vec4<f32>,
    clip_rect:    vec4<f32>,
    clip_radii_h: vec4<f32>,
    clip_radii_v: vec4<f32>,
    clip_active:  vec4<f32>,
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
    let viewport = globals.viewport.xy;
    let ndc_x = (in.position.x / viewport.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (in.position.y / viewport.y) * 2.0;

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

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    var clip_alpha = 1.0;
    if (globals.clip_active.x > 0.5) {
        let frag_pos = in.clip.xy;
        let cr = globals.clip_rect;
        let half = vec2<f32>(cr.z, cr.w) * 0.5;
        let centre = vec2<f32>(cr.x + half.x, cr.y + half.y);
        let local_clip = frag_pos - centre;
        let r = pick_radius(local_clip, globals.clip_radii_h, globals.clip_radii_v);
        let d = sd_rounded_box(local_clip, half, r);
        let aa = max(fwidth(d), 1.0);
        clip_alpha = clamp(0.5 - d / aa, 0.0, 1.0);
        if (clip_alpha <= 0.0) {
            discard;
        }
    }

    return vec4<f32>(in.color.rgb, in.color.a * clip_alpha);
}
