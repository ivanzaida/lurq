// `surface.zw` carries a composite's own parameters: the blend mode index and
// the layer alpha. A tile's own constants leave both at zero.
struct Globals { tile: vec4<f32>, surface: vec4<f32> };
@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;
// Only `fs_blend` reads this, and only its pipeline is bound with it.
@group(1) @binding(2) var backdrop: texture_2d<f32>;
struct Out { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) color: vec4<f32> };
@vertex fn vs_main(@location(0) position: vec2<f32>, @location(1) uv: vec2<f32>, @location(2) color: vec4<f32>) -> Out {
  let p = (position - globals.tile.xy) / globals.tile.zw;
  return Out(vec4<f32>(p.x * 2.0 - 1.0, 1.0 - p.y * 2.0, 0.0, 1.0), uv, color);
}
@vertex fn vs_seed(@builtin(vertex_index) index:u32) -> Out {
  let p = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
  return Out(vec4<f32>(p.x*2.0-1.0,1.0-p.y*2.0,0.0,1.0), (globals.tile.xy+p*globals.tile.zw)/globals.surface.xy, vec4<f32>(1.0));
}
// A whole tile sampled from a tile-sized texture: an isolated layer, or the
// saved copy of what was under it.
@vertex fn vs_tile(@builtin(vertex_index) index:u32) -> Out {
  let p = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
  return Out(vec4<f32>(p.x*2.0-1.0,1.0-p.y*2.0,0.0,1.0), p, vec4<f32>(globals.surface.w));
}
@fragment fn fs_solid(v:Out) -> @location(0) vec4<f32> { return v.color; }
@fragment fn fs_image(v:Out) -> @location(0) vec4<f32> {
  let c = textureSample(image,image_sampler,v.uv);
  return vec4<f32>(c.rgb*c.a,c.a) * v.color;
}
@fragment fn fs_premul(v:Out) -> @location(0) vec4<f32> {
  return textureSample(image,image_sampler,v.uv) * v.color;
}
// Restores a saved tile as the base of a pass: its own colour, not scaled.
@fragment fn fs_tile(v:Out) -> @location(0) vec4<f32> {
  return textureSample(image,image_sampler,v.uv);
}

// Gradients. `uv` carries the point in the gradient's own frame, so the three
// kinds differ only in the parameter they read from it, and `image` is the ramp.
const TAU: f32 = 6.28318530718;
fn ramp(t: f32, color: vec4<f32>) -> vec4<f32> {
  return textureSample(image, image_sampler, vec2<f32>(clamp(t, 0.0, 1.0), 0.5)) * color;
}
@fragment fn fs_gradient_linear(v:Out) -> @location(0) vec4<f32> { return ramp(v.uv.x * 0.5 + 0.5, v.color); }
@fragment fn fs_gradient_radial(v:Out) -> @location(0) vec4<f32> { return ramp(length(v.uv), v.color); }
@fragment fn fs_gradient_angular(v:Out) -> @location(0) vec4<f32> {
  return ramp(fract(atan2(v.uv.y, v.uv.x) / TAU), v.color);
}

// W3C compositing and blending level 1. The software backend runs the same
// formulas in Rust, and a pixel test compares the two.
fn straight(c: vec4<f32>) -> vec3<f32> {
  if (c.a <= 0.0) { return vec3<f32>(0.0); }
  return c.rgb / c.a;
}
fn lum(c: vec3<f32>) -> f32 { return dot(c, vec3<f32>(0.3, 0.59, 0.11)); }
fn clip_color(c: vec3<f32>) -> vec3<f32> {
  let l = lum(c);
  let n = min(c.r, min(c.g, c.b));
  let x = max(c.r, max(c.g, c.b));
  var r = c;
  if (n < 0.0) { r = l + (r - l) * l / max(l - n, 1e-7); }
  if (x > 1.0) { r = l + (r - l) * (1.0 - l) / max(x - l, 1e-7); }
  return r;
}
fn set_lum(c: vec3<f32>, l: f32) -> vec3<f32> { return clip_color(c + (l - lum(c))); }
fn sat(c: vec3<f32>) -> f32 { return max(c.r, max(c.g, c.b)) - min(c.r, min(c.g, c.b)); }
fn set_sat(c: vec3<f32>, s: f32) -> vec3<f32> {
  let mn = min(c.r, min(c.g, c.b));
  let mx = max(c.r, max(c.g, c.b));
  if (mx <= mn) { return vec3<f32>(0.0); }
  return (c - mn) / (mx - mn) * s;
}
fn hard_light(cb: f32, cs: f32) -> f32 {
  if (cs <= 0.5) { return cb * (2.0 * cs); }
  let s = 2.0 * cs - 1.0;
  return cb + s - cb * s;
}
fn separable(mode: i32, cb: f32, cs: f32) -> f32 {
  switch (mode) {
    case 1: { return min(cb, cs); }
    case 2: { return cb * cs; }
    case 3: { return max(cb + cs - 1.0, 0.0); }
    case 4: {
      if (cb >= 1.0) { return 1.0; }
      if (cs <= 0.0) { return 0.0; }
      return 1.0 - min((1.0 - cb) / cs, 1.0);
    }
    case 5: { return max(cb, cs); }
    case 6: { return cb + cs - cb * cs; }
    case 7: { return min(cb + cs, 1.0); }
    case 8: {
      if (cb <= 0.0) { return 0.0; }
      if (cs >= 1.0) { return 1.0; }
      return min(cb / (1.0 - cs), 1.0);
    }
    case 9: { return hard_light(cs, cb); }
    case 10: {
      var d = sqrt(max(cb, 0.0));
      if (cb <= 0.25) { d = ((16.0 * cb - 12.0) * cb + 4.0) * cb; }
      if (cs <= 0.5) { return cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb); }
      return cb + (2.0 * cs - 1.0) * (d - cb);
    }
    case 11: { return hard_light(cb, cs); }
    case 12: { return abs(cb - cs); }
    case 13: { return cb + cs - 2.0 * cb * cs; }
    default: { return cs; }
  }
}
fn blend_fn(mode: i32, cb: vec3<f32>, cs: vec3<f32>) -> vec3<f32> {
  switch (mode) {
    case 14: { return set_lum(set_sat(cs, sat(cb)), lum(cb)); }
    case 15: { return set_lum(set_sat(cb, sat(cs)), lum(cb)); }
    case 16: { return set_lum(cs, lum(cb)); }
    case 17: { return set_lum(cb, lum(cs)); }
    default: {
      return vec3<f32>(
        separable(mode, cb.r, cs.r),
        separable(mode, cb.g, cs.g),
        separable(mode, cb.b, cs.b),
      );
    }
  }
}
@fragment fn fs_blend(v:Out) -> @location(0) vec4<f32> {
  let source = textureSample(image, image_sampler, v.uv) * v.color;
  let back = textureSample(backdrop, image_sampler, v.uv);
  if (source.a <= 0.0) { return back; }
  let cb = straight(back);
  let cs = straight(source);
  let blended = blend_fn(i32(globals.surface.z + 0.5), cb, cs);
  let cr = (1.0 - back.a) * cs + back.a * blended;
  return vec4<f32>(source.a * cr + back.rgb * (1.0 - source.a), source.a + back.a * (1.0 - source.a));
}
