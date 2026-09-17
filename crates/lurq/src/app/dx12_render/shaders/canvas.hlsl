// `surface.zw` carries a composite's own parameters: the blend mode index and
// the layer alpha. A tile's own constants leave both at zero.
cbuffer Globals : register(b0) { float4 tile; float4 surface; };
Texture2D<float4> image_texture : register(t0);
// Only ps_blend reads this; every other pipeline leaves it bound to the same
// texture as t0, which keeps one root signature for the whole canvas.
Texture2D<float4> backdrop_texture : register(t1);
SamplerState image_sampler : register(s0);
struct Out { float4 position:SV_POSITION; float2 uv:TEXCOORD0; float4 color:TEXCOORD1; };
Out vs_main(float2 position:POSITION, float2 uv:TEXCOORD0, float4 color:COLOR0) {
  float2 p=(position-tile.xy)/tile.zw;
  Out o; o.position=float4(p.x*2-1,1-p.y*2,0,1);o.uv=uv;o.color=color;return o;
}
Out vs_seed(uint index:SV_VertexID) {
  float2 p=float2((index<<1)&2,index&2);
  Out o;o.position=float4(p.x*2-1,1-p.y*2,0,1);o.uv=(tile.xy+p*tile.zw)/surface.xy;o.color=1;return o;
}
// A whole tile sampled from a tile-sized texture: an isolated layer, or the
// saved copy of what was under it.
Out vs_tile(uint index:SV_VertexID) {
  float2 p=float2((index<<1)&2,index&2);
  Out o;o.position=float4(p.x*2-1,1-p.y*2,0,1);o.uv=p;o.color=surface.w;return o;
}
float4 ps_solid(Out v):SV_TARGET {return v.color;}
float4 ps_premul(Out v):SV_TARGET {return image_texture.Sample(image_sampler,v.uv)*v.color;}
// Restores a saved tile as the base of the draw that composites over it.
float4 ps_tile(Out v):SV_TARGET {return image_texture.Sample(image_sampler,v.uv);}

// Gradients. `uv` carries the point in the gradient's own frame, so the three
// kinds differ only in the parameter they read from it, and t0 is the ramp.
static const float TAU = 6.28318530718;
float4 ramp(float t, float4 color) {
  return image_texture.Sample(image_sampler, float2(clamp(t, 0.0, 1.0), 0.5)) * color;
}
float4 ps_gradient_linear(Out v):SV_TARGET { return ramp(v.uv.x * 0.5 + 0.5, v.color); }
float4 ps_gradient_radial(Out v):SV_TARGET { return ramp(length(v.uv), v.color); }
float4 ps_gradient_angular(Out v):SV_TARGET { return ramp(frac(atan2(v.uv.y, v.uv.x) / TAU), v.color); }

// W3C compositing and blending level 1. The software backend runs the same
// formulas in Rust, and a pixel test compares the two.
float3 straight(float4 c) { return c.a <= 0.0 ? float3(0,0,0) : c.rgb / c.a; }
float lum(float3 c) { return dot(c, float3(0.3, 0.59, 0.11)); }
float3 clip_color(float3 c) {
  float l = lum(c);
  float n = min(c.r, min(c.g, c.b));
  float x = max(c.r, max(c.g, c.b));
  float3 r = c;
  if (n < 0.0) { r = l + (r - l) * l / max(l - n, 1e-7); }
  if (x > 1.0) { r = l + (r - l) * (1.0 - l) / max(x - l, 1e-7); }
  return r;
}
float3 set_lum(float3 c, float l) { return clip_color(c + (l - lum(c))); }
float sat(float3 c) { return max(c.r, max(c.g, c.b)) - min(c.r, min(c.g, c.b)); }
float3 set_sat(float3 c, float s) {
  float mn = min(c.r, min(c.g, c.b));
  float mx = max(c.r, max(c.g, c.b));
  if (mx <= mn) { return float3(0,0,0); }
  return (c - mn) / (mx - mn) * s;
}
float hard_light(float cb, float cs) {
  if (cs <= 0.5) { return cb * (2.0 * cs); }
  float s = 2.0 * cs - 1.0;
  return cb + s - cb * s;
}
float separable(int mode, float cb, float cs) {
  if (mode == 1) { return min(cb, cs); }
  if (mode == 2) { return cb * cs; }
  if (mode == 3) { return max(cb + cs - 1.0, 0.0); }
  if (mode == 4) {
    if (cb >= 1.0) { return 1.0; }
    if (cs <= 0.0) { return 0.0; }
    return 1.0 - min((1.0 - cb) / cs, 1.0);
  }
  if (mode == 5) { return max(cb, cs); }
  if (mode == 6) { return cb + cs - cb * cs; }
  if (mode == 7) { return min(cb + cs, 1.0); }
  if (mode == 8) {
    if (cb <= 0.0) { return 0.0; }
    if (cs >= 1.0) { return 1.0; }
    return min(cb / (1.0 - cs), 1.0);
  }
  if (mode == 9) { return hard_light(cs, cb); }
  if (mode == 10) {
    float d = sqrt(max(cb, 0.0));
    if (cb <= 0.25) { d = ((16.0 * cb - 12.0) * cb + 4.0) * cb; }
    if (cs <= 0.5) { return cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb); }
    return cb + (2.0 * cs - 1.0) * (d - cb);
  }
  if (mode == 11) { return hard_light(cb, cs); }
  if (mode == 12) { return abs(cb - cs); }
  if (mode == 13) { return cb + cs - 2.0 * cb * cs; }
  return cs;
}
float3 blend_fn(int mode, float3 cb, float3 cs) {
  if (mode == 14) { return set_lum(set_sat(cs, sat(cb)), lum(cb)); }
  if (mode == 15) { return set_lum(set_sat(cb, sat(cs)), lum(cb)); }
  if (mode == 16) { return set_lum(cs, lum(cb)); }
  if (mode == 17) { return set_lum(cb, lum(cs)); }
  return float3(
    separable(mode, cb.r, cs.r),
    separable(mode, cb.g, cs.g),
    separable(mode, cb.b, cs.b));
}
float4 ps_blend(Out v):SV_TARGET {
  float4 source = image_texture.Sample(image_sampler, v.uv) * v.color;
  float4 back = backdrop_texture.Sample(image_sampler, v.uv);
  if (source.a <= 0.0) { return back; }
  float3 blended = blend_fn((int)(surface.z + 0.5), straight(back), straight(source));
  float3 cr = (1.0 - back.a) * straight(source) + back.a * blended;
  return float4(source.a * cr + back.rgb * (1.0 - source.a), source.a + back.a * (1.0 - source.a));
}
