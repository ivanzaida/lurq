// The pass that puts a blurred backdrop back as glass (see `layout::glass`).
//
// The backdrop was copied out of the target and blurred like any backdrop
// blur's (`layer_blur.wgsl`); this reads the finished blur, once per pixel
// or once per colour, from where a ray through the glass comes out.
//
// The glass is the smooth union of up to eight rounded boxes. A pixel's
// distance to that union's edge says how much of it the glass covers, how
// deep inside it lies and, by the distance's gradient, which way the edge
// faces. Over the bezel the glass is curved, and a pixel there is read from
// further inside: by the whole bend at the very edge, by nothing where the
// glass is flat (`layout::glass::lens_pull`).
//
// As for a backdrop blur, the result replaces what the target held, with
// two draws of the same rect: `fs_glass_erase`, blended
// `dst * (1 - src.a)`, which also lays the shadow, then `fs_glass_add`,
// blended `dst + src`.
//
// Vertex buffer 0: unit quad corner in [0,1]^2.

struct Shape {
    box:     vec4<f32>,  // centre in target pixels, half size along its own axes
    inverse: vec4<f32>,  // 2x2 from a target-pixel offset to the box's own axes
    radii:   vec4<f32>,  // tl, tr, br, bl
    more:    vec4<f32>,  // x: target pixels per unit of the box's axes
};

struct Pass {
    rect:   vec4<f32>,  // rect drawn, in target pixels: x, y, w, h
    dest:   vec4<f32>,  // target size, then the offset added to a target pixel
    source: vec4<f32>,  // offset target pixel -> source texel scale, then 1 / source texture size
    limit:  vec4<f32>,  // lowest, then highest source texel centre a read may take
    clip:         vec4<f32>,  // the innermost rounded clip, as a shape's box
    clip_inverse: vec4<f32>,
    clip_radii:   vec4<f32>,
    outer_clip:       vec4<f32>,  // and the one outside it: upright
    outer_clip_radii: vec4<f32>,
    cut:    vec4<f32>,  // the clips' scale, the opacity, the number of shapes, the merge distance
    optics: vec4<f32>,  // bend, bezel, dispersion
    look:   vec4<f32>,  // saturation, brightness, rim light, inner glow
    tint:   vec4<f32>,  // premultiplied
    shadow: vec4<f32>,  // radius, distance down, alpha
    light:  vec4<f32>,  // unit vector toward the light, the rim's width, how much the light's side counts
    touch:  vec4<f32>,  // centre in target pixels, radius, strength
    shapes: array<Shape, 8>,
};

@group(0) @binding(0) var<uniform> glass: Pass;
@group(0) @binding(1) var backdrop: texture_2d<f32>;
@group(0) @binding(2) var backdrop_sampler: sampler;

struct VsOut {
    @builtin(position) clip: vec4<f32>,
};

@vertex
fn vs_main(@location(0) corner: vec2<f32>) -> VsOut {
    let px = glass.rect.xy + corner * glass.rect.zw;
    let size = glass.dest.xy;
    var out: VsOut;
    out.clip = vec4<f32>((px.x / size.x) * 2.0 - 1.0, 1.0 - (px.y / size.y) * 2.0, 0.0, 1.0);
    return out;
}

// The blurred backdrop under the target pixel `pixel`. Past the texels the
// copy holds, its edge repeats.
fn behind(pixel: vec2<f32>) -> vec4<f32> {
    let texel = (pixel + glass.dest.zw) * glass.source.xy;
    let inside = clamp(texel, glass.limit.xy, glass.limit.zw);
    return textureSampleLevel(backdrop, backdrop_sampler, inside * glass.source.zw, 0.0);
}

// The distance from `pixel` to a rounded box's edge along the box's own
// axes: negative inside.
fn box_distance(pixel: vec2<f32>, box: vec4<f32>, inverse: vec4<f32>, radii: vec4<f32>) -> f32 {
    let offset = pixel - box.xy;
    let p = vec2<f32>(inverse.x * offset.x + inverse.z * offset.y, inverse.y * offset.x + inverse.w * offset.y);
    var radius = radii.x;
    if (p.x >= 0.0 && p.y < 0.0) {
        radius = radii.y;
    } else if (p.x >= 0.0) {
        radius = radii.z;
    } else if (p.y >= 0.0) {
        radius = radii.w;
    }
    let q = abs(p) - box.zw + vec2<f32>(radius);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

fn box_coverage(pixel: vec2<f32>, box: vec4<f32>, inverse: vec4<f32>, radii: vec4<f32>, scale: f32) -> f32 {
    return clamp(0.5 - box_distance(pixel, box, inverse, radii) * scale, 0.0, 1.0);
}

// How much of the result shows at `pixel`: the opacity, cut to the clips.
fn shown(pixel: vec2<f32>) -> f32 {
    let upright = vec4<f32>(1.0, 0.0, 0.0, 1.0);
    return glass.cut.y
        * box_coverage(pixel, glass.clip, glass.clip_inverse, glass.clip_radii, glass.cut.x)
        * box_coverage(pixel, glass.outer_clip, upright, glass.outer_clip_radii, glass.cut.x);
}

// The lower of two distances, rounded where they are within `k` of each
// other.
fn smooth_min(a: f32, b: f32, k: f32) -> f32 {
    if (k <= 0.0) {
        return min(a, b);
    }
    let h = max(k - abs(a - b), 0.0) / k;
    return min(a, b) - h * h * k * 0.25;
}

const FAR: f32 = 1.0e9;

// The distance from `pixel` to the glass's edge in target pixels, negative
// inside, and the bezel of the shape nearest to it: a shape narrower than
// two bezels is curved all the way to its middle.
fn field(pixel: vec2<f32>) -> vec2<f32> {
    var distance = FAR;
    var nearest = FAR;
    var bezel = glass.optics.y;
    let count = i32(glass.cut.z);
    for (var index = 0; index < 8; index++) {
        if (index >= count) {
            break;
        }
        let shape = glass.shapes[index];
        if (shape.box.z <= 0.0 || shape.box.w <= 0.0) {
            continue;
        }
        let own = box_distance(pixel, shape.box, shape.inverse, shape.radii) * shape.more.x;
        if (own < nearest) {
            nearest = own;
            bezel = min(glass.optics.y, min(shape.box.z, shape.box.w) * shape.more.x);
        }
        if (distance >= FAR) {
            distance = own;
        } else {
            distance = smooth_min(distance, own, glass.cut.w);
        }
    }
    return vec2<f32>(distance, max(bezel, 1.0));
}

// A rounded box's distance at `pixel`, as `box_distance` gives it, and the
// way it grows fastest: out of the box, straight away from its edge.
fn box_slope(pixel: vec2<f32>, box: vec4<f32>, inverse: vec4<f32>, radii: vec4<f32>) -> vec3<f32> {
    let offset = pixel - box.xy;
    let p = vec2<f32>(inverse.x * offset.x + inverse.z * offset.y, inverse.y * offset.x + inverse.w * offset.y);
    var radius = radii.x;
    if (p.x >= 0.0 && p.y < 0.0) {
        radius = radii.y;
    } else if (p.x >= 0.0) {
        radius = radii.z;
    } else if (p.y >= 0.0) {
        radius = radii.w;
    }
    let q = abs(p) - box.zw + vec2<f32>(radius);
    let outside = max(q, vec2<f32>(0.0));
    let far = length(outside);
    // Along the box's own axes: round a corner, or across the nearer side.
    var along = vec2<f32>(0.0, 1.0);
    if (far > 1.0e-5) {
        along = outside / far;
    } else if (q.x > q.y) {
        along = vec2<f32>(1.0, 0.0);
    }
    along *= vec2<f32>(select(-1.0, 1.0, p.x >= 0.0), select(-1.0, 1.0, p.y >= 0.0));
    // Back to the target's axes: the transpose of `inverse`.
    let out = vec2<f32>(inverse.x * along.x + inverse.y * along.y, inverse.z * along.x + inverse.w * along.y);
    var slope = vec2<f32>(0.0);
    if (dot(out, out) > 1.0e-12) {
        slope = normalize(out);
    }
    return vec3<f32>(far + min(max(q.x, q.y), 0.0) - radius, slope);
}

// `field` with the way the distance grows fastest, found with the distance
// in one pass over the shapes: the distance, the bezel, the way out.
fn field_slope(pixel: vec2<f32>) -> vec4<f32> {
    var distance = FAR;
    var nearest = FAR;
    var bezel = glass.optics.y;
    var slope = vec2<f32>(0.0);
    let count = i32(glass.cut.z);
    let merge = glass.cut.w;
    for (var index = 0; index < 8; index++) {
        if (index >= count) {
            break;
        }
        let shape = glass.shapes[index];
        if (shape.box.z <= 0.0 || shape.box.w <= 0.0) {
            continue;
        }
        let found = box_slope(pixel, shape.box, shape.inverse, shape.radii);
        let own = found.x * shape.more.x;
        if (own < nearest) {
            nearest = own;
            bezel = min(glass.optics.y, min(shape.box.z, shape.box.w) * shape.more.x);
        }
        if (distance >= FAR) {
            distance = own;
            slope = found.yz;
        } else {
            // The smooth minimum leans from the nearer shape's way toward the
            // other's by half of how near the two distances are.
            var lean = 0.0;
            if (merge > 0.0) {
                lean = 0.5 * max(merge - abs(distance - own), 0.0) / merge;
            }
            if (own < distance) {
                slope = mix(found.yz, slope, lean);
            } else {
                slope = mix(slope, found.yz, lean);
            }
            distance = smooth_min(distance, own, merge);
        }
    }
    return vec4<f32>(distance, max(bezel, 1.0), slope);
}

// How much of the bend the glass pulls the backdrop in by `u` of the way in
// over the bezel (see `layout::glass::lens_pull`).
fn lens_pull(u: f32) -> f32 {
    let inside = clamp(1.0 - u, 0.0, 1.0);
    return pow(inside, 1.1) * exp(-2.0 * (1.0 - inside));
}

@fragment
fn fs_glass_erase(in: VsOut) -> @location(0) vec4<f32> {
    let pixel = in.clip.xy;
    let coverage = clamp(0.5 - field(pixel).x, 0.0, 1.0);
    var shade = 0.0;
    if (glass.shadow.z > 0.0 && coverage < 1.0) {
        // The glass's shape moved down and blurred: half as dark at its
        // edge as under it, and gone three quarters of the radius out.
        let away = field(pixel - vec2<f32>(0.0, glass.shadow.y)).x;
        let reach = max(glass.shadow.x * 0.75, 1.0);
        shade = glass.shadow.z * (1.0 - smoothstep(-reach, reach, away));
    }
    return vec4<f32>(0.0, 0.0, 0.0, shown(pixel) * (coverage + (1.0 - coverage) * shade));
}

@fragment
fn fs_glass_add(in: VsOut) -> @location(0) vec4<f32> {
    let pixel = in.clip.xy;
    let here = field_slope(pixel);
    let coverage = clamp(0.5 - here.x, 0.0, 1.0);
    if (coverage <= 0.0) {
        return vec4<f32>(0.0);
    }
    // Which way the edge faces: the distance grows fastest straight out.
    let slant = here.zw;
    var outward = vec2<f32>(0.0);
    if (dot(slant, slant) > 1.0e-8) {
        outward = normalize(slant);
    }
    let depth = max(-here.x, 0.0);
    let bezel = here.y;
    let u = clamp(depth / bezel, 0.0, 1.0);

    let inside = 1.0 - u;
    let pull = glass.optics.x * lens_pull(u);
    let spread = glass.optics.z;
    var color: vec4<f32>;
    if (pull <= 0.01) {
        color = behind(pixel);
    } else if (spread > 0.0) {
        // Red is bent least and blue most.
        let red = behind(pixel - outward * pull * (1.0 - spread));
        let green = behind(pixel - outward * pull);
        let blue = behind(pixel - outward * pull * (1.0 + spread));
        color = vec4<f32>(red.r, green.g, blue.b, green.a);
    } else {
        color = behind(pixel - outward * pull);
    }

    // The material: saturation and light on what is behind, then the tint
    // over it. Colours are premultiplied.
    let grey = dot(color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    var rgb = mix(vec3<f32>(grey), color.rgb, glass.look.x) + vec3<f32>(glass.look.y * color.a);
    rgb = clamp(rgb, vec3<f32>(0.0), vec3<f32>(color.a));
    rgb = glass.tint.rgb + rgb * (1.0 - glass.tint.a);
    var alpha = glass.tint.a + color.a * (1.0 - glass.tint.a);

    // The light: along the rim, strongest where the edge faces the light
    // and, weaker, where it faces away from it; a glow that falls off over
    // the bezel; and a touch.
    // The rim's light is the same all the way round under a light from
    // straight above, as the system's is on a still device; a light from a
    // side adds to the edge that faces it and takes from the one that does
    // not.
    let rim = clamp(1.0 - depth / max(glass.light.z, 0.5), 0.0, 1.0);
    let lit = 1.0 + glass.light.w * dot(outward, glass.light.xy);
    var light = glass.look.z * rim * lit + glass.look.w * inside * inside * 0.5;
    if (glass.touch.w > 0.0) {
        let from_touch = (pixel - glass.touch.xy) / glass.touch.z;
        let near = exp(-dot(from_touch, from_touch));
        light += glass.touch.w * near * (0.28 + 0.6 * rim);
    }
    light = clamp(light, 0.0, 1.0);
    rgb = min(rgb + vec3<f32>(light), vec3<f32>(1.0));
    alpha = min(alpha + light * (1.0 - alpha), 1.0);

    return vec4<f32>(rgb, alpha) * (coverage * shown(pixel));
}
