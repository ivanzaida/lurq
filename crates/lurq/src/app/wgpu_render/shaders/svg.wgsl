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

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    if (globals.clip_active.x > 0.5) {
        let frag_pos = in.clip.xy;
        let cr = globals.clip_rect;
        let half = vec2<f32>(cr.z, cr.w) * 0.5;
        let centre = vec2<f32>(cr.x + half.x, cr.y + half.y);
        let local_clip = frag_pos - centre;
        let d = max(abs(local_clip.x) - half.x, abs(local_clip.y) - half.y);
        if (d > 0.5) {
            discard;
        }
    }

    return in.color;
}
