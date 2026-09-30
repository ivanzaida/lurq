// Composites a finished opacity layer into its parent target.
//
// The layer texture holds premultiplied colour: it was cleared to
// transparent and painted with the regular pipelines, whose source-over
// blend onto a premultiplied destination keeps it premultiplied. Its texels
// map one to one onto target pixels (both are whole window pixels), so the
// fragment loads its texel instead of sampling: no filtering, no blur at any
// DPI scale. The pipeline blends premultiplied source-over.
//
// Vertex buffer 0: unit quad corner in [0,1]^2.

struct Composite {
    rect:    vec4<f32>,  // layer rect in target pixels: x, y, w, h
    dest:    vec4<f32>,  // target size, then target pixel -> layer texel offset
    opacity: vec4<f32>,  // x = layer opacity
};

@group(0) @binding(0) var<uniform> composite: Composite;
@group(0) @binding(1) var layer: texture_2d<f32>;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
};

@vertex
fn vs_main(@location(0) corner: vec2<f32>) -> VsOut {
    let px = composite.rect.xy + corner * composite.rect.zw;
    let size = composite.dest.xy;
    var out: VsOut;
    out.clip = vec4<f32>((px.x / size.x) * 2.0 - 1.0, 1.0 - (px.y / size.y) * 2.0, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let texel = vec2<i32>(floor(in.clip.xy + composite.dest.zw));
    return textureLoad(layer, texel, 0) * composite.opacity.x;
}
