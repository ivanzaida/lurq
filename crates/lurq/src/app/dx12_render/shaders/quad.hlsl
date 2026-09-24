cbuffer Globals : register(b0)
{
  float4 viewport;
  float4 clip_rect;
  float4 clip_radii_h;
  float4 clip_radii_v;
  float4 clip_active;
};

// Gradient stop storage. Layout per gradient (each element is a float4):
//   [count, kind, flags, from_angle]
//   [dir.x, dir.y, center.x, center.y]
//   then per stop: [r, g, b, a], [position, _, _, _]
// kind: 0 = linear, 1 = radial (flags bit0 = ellipse), 2 = conic.
StructuredBuffer<float4> gradients : register(t0);

struct VsIn
{
  float2 corner : TEXCOORD0;
  float2 pos : TEXCOORD1;
  float2 size : TEXCOORD2;
  float4 color : TEXCOORD3;
  float4 radii_h : TEXCOORD4;
  float4 radii_v : TEXCOORD5;
  float4 stroke : TEXCOORD6;
  float4 pattern : TEXCOORD7;
  float4 transform : TEXCOORD8;
  float2 xf_origin : TEXCOORD9;
  float shadow_sigma : TEXCOORD10;
  float gradient_offset : TEXCOORD11;
};

struct VsOut
{
  float4 position : SV_POSITION;
  float4 color : COLOR0;
  float2 local : TEXCOORD0;
  float2 half_size : TEXCOORD1;
  float4 radii_h : TEXCOORD2;
  float4 radii_v : TEXCOORD3;
  float4 stroke : TEXCOORD4;
  float gradient_offset : TEXCOORD5;
  float4 pattern : TEXCOORD6;
  float shadow_sigma : TEXCOORD7;
};

// Box shadows. The same formula as `blurred_rounded_rect_coverage` in
// `layout/box_shadow.rs` and `quad.wgsl`; keep the three in step. A shadow
// instance has no stroke and `pattern.x` = 1 (outer) or 2 (inset); `radii_h`
// are the element box's radii, `radii_v` the shadow shape's, `pattern.yz` the
// shape offset, `pattern.w` the spread and `shadow_sigma` the Gaussian's
// standard deviation.
static const int SHADOW_Y_SAMPLES = 8;
static const float SHADOW_EXTENT_SIGMAS = 3.0;
static const float SHADOW_MIN_SIGMA = 0.1;

VsOut vs_main(VsIn input)
{
  float aa_outset = 2.0;
  float max_stroke = max(max(input.stroke.x, input.stroke.y), max(input.stroke.z, input.stroke.w));
  if (max_stroke <= 0.0 && input.pattern.x > 0.5 && input.pattern.x < 1.5)
  {
    // An outer shadow paints beyond its element box.
    aa_outset += max(input.pattern.w, 0.0) + max(abs(input.pattern.y), abs(input.pattern.z))
      + SHADOW_EXTENT_SIGMAS * input.shadow_sigma;
  }
  float2 local_px = input.corner * (input.size + float2(aa_outset * 2.0, aa_outset * 2.0))
    - float2(aa_outset, aa_outset);
  float2 centered = local_px - input.xf_origin;
  float2 transformed = float2(
    input.transform.x * centered.x + input.transform.z * centered.y,
    input.transform.y * centered.x + input.transform.w * centered.y
  );
  float2 px = input.pos + transformed + input.xf_origin;
  float2 ndc = float2((px.x / viewport.x) * 2.0 - 1.0, 1.0 - (px.y / viewport.y) * 2.0);

  VsOut output;
  output.position = float4(ndc, 0.0, 1.0);
  output.color = input.color;
  output.local = local_px - input.size * 0.5;
  output.half_size = input.size * 0.5;
  output.radii_h = input.radii_h;
  output.radii_v = input.radii_v;
  output.stroke = input.stroke;
  output.gradient_offset = input.gradient_offset;
  output.pattern = input.pattern;
  output.shadow_sigma = input.shadow_sigma;
  return output;
}

float4 sample_gradient(int off, float2 local, float2 half_size)
{
  float4 h0 = gradients[off];
  float4 h1 = gradients[off + 1];
  int count = (int)h0.x;
  int kind = (int)h0.y;
  const float PI = 3.14159265359;

  float t;
  if (kind == 0)
  {
    float2 dir = h1.xy;
    float hl = abs(half_size.x * dir.x) + abs(half_size.y * dir.y);
    t = (dot(local, dir) + hl) / (2.0 * max(hl, 1e-5));
  }
  else if (kind == 1)
  {
    float2 center = (h1.zw * 2.0 - float2(1.0, 1.0)) * half_size;
    if (h0.z > 0.5)
    {
      float2 cn = h1.zw * 2.0 - float2(1.0, 1.0);
      float2 p = (local - center) / max(half_size, float2(1e-3, 1e-3));
      float radius = max(
        max(length(float2(-1.0, -1.0) - cn), length(float2(1.0, -1.0) - cn)),
        max(length(float2(-1.0, 1.0) - cn), length(float2(1.0, 1.0) - cn)));
      t = length(p) / max(radius, 1e-5);
    }
    else
    {
      float radius = max(
        max(length(float2(-half_size.x, -half_size.y) - center), length(float2(half_size.x, -half_size.y) - center)),
        max(length(float2(-half_size.x, half_size.y) - center), length(float2(half_size.x, half_size.y) - center)));
      t = length(local - center) / max(radius, 1e-5);
    }
  }
  else
  {
    float2 center = (h1.zw * 2.0 - float2(1.0, 1.0)) * half_size;
    float2 d = local - center;
    float ang = (atan2(d.x, -d.y) - h0.w) / (2.0 * PI);
    t = ang - floor(ang);
  }

  if (kind != 2)
  {
    t = saturate(t);
  }

  int stop_base = off + 2;
  int last = count - 1;
  float4 color = gradients[stop_base + 2 * last];
  for (int i = 0; i < last; i = i + 1)
  {
    float pb = gradients[stop_base + 2 * (i + 1) + 1].x;
    if (t <= pb)
    {
      float pa = gradients[stop_base + 2 * i + 1].x;
      float4 ca = gradients[stop_base + 2 * i];
      float4 cb = gradients[stop_base + 2 * (i + 1)];
      float span = max(pb - pa, 1e-5);
      color = lerp(ca, cb, saturate((t - pa) / span));
      break;
    }
  }
  return color;
}

float2 pick_radius(float2 p, float4 radii_h, float4 radii_v)
{
  if (p.y < 0.0)
  {
    return p.x < 0.0 ? float2(radii_h.x, radii_v.x) : float2(radii_h.y, radii_v.y);
  }
  return p.x < 0.0 ? float2(radii_h.w, radii_v.w) : float2(radii_h.z, radii_v.z);
}

float sd_rounded_box(float2 p, float2 half_size, float2 radius)
{
  float2 safe_radius = max(radius, float2(0.0, 0.0));
  float2 q = abs(p) - half_size + safe_radius;
  if (q.x > 0.0 && q.y > 0.0)
  {
    if (safe_radius.x <= 0.0 || safe_radius.y <= 0.0)
    {
      return max(q.x, q.y);
    }
    float2 pn = q / safe_radius;
    float len_pn = length(pn);
    if (len_pn <= 1e-6)
    {
      return -min(safe_radius.x, safe_radius.y);
    }
    float gradient = max(length(pn / safe_radius), 1e-6);
    return (len_pn - 1.0) * len_pn / gradient;
  }
  return max(q.x - safe_radius.x, q.y - safe_radius.y);
}

float aa_width(float dist)
{
  return max(fwidth(dist), 1.0);
}

// Ramp width for one of the four half-pixel subsamples: half a pixel, so a
// pixel fully inside an axis-aligned edge gets full coverage. Matches quad.wgsl.
float subsample_aa_width(float dist)
{
  return 0.5 * aa_width(dist);
}

float rounded_clip_alpha(float2 frag_pos)
{
  if (clip_active.x <= 0.5)
  {
    return 1.0;
  }

  float2 half_size = clip_rect.zw * 0.5;
  float2 centre = clip_rect.xy + half_size;
  float2 local = frag_pos - centre;
  float dist = sd_rounded_box(local, half_size, pick_radius(local, clip_radii_h, clip_radii_v));
  return saturate(0.5 - dist / aa_width(dist));
}

float rounded_fill_alpha(float2 local, float2 half_size, float4 radii_h, float4 radii_v)
{
  float2 radius = pick_radius(local, radii_h, radii_v);
  float dist = sd_rounded_box(local, half_size, radius);
  return saturate(0.5 - dist / subsample_aa_width(dist));
}

float rounded_stroke_alpha(float2 local, float2 half_size, float4 radii_h, float4 radii_v, float4 stroke, float max_stroke)
{
  float2 radius = pick_radius(local, radii_h, radii_v);
  float outer_dist = sd_rounded_box(local, half_size, radius);
  float2 inner_half = max(float2(
    half_size.x - 0.5 * (stroke.y + stroke.w),
    half_size.y - 0.5 * (stroke.x + stroke.z)
  ), float2(0.0, 0.0));
  float2 inner_center = float2(
    0.5 * (stroke.w - stroke.y),
    0.5 * (stroke.x - stroke.z)
  );
  float2 inner_radius = max(radius - float2(max_stroke, max_stroke), float2(0.0, 0.0));
  float inner_dist = sd_rounded_box(local - inner_center, inner_half, inner_radius);
  float dist = max(outer_dist, -inner_dist);
  return saturate(0.5 - dist / subsample_aa_width(dist));
}

float supersampled_fill_alpha(float2 local, float2 half_size, float4 radii_h, float4 radii_v)
{
  return (
    rounded_fill_alpha(local + float2(-0.25, -0.25), half_size, radii_h, radii_v) +
    rounded_fill_alpha(local + float2(0.25, -0.25), half_size, radii_h, radii_v) +
    rounded_fill_alpha(local + float2(-0.25, 0.25), half_size, radii_h, radii_v) +
    rounded_fill_alpha(local + float2(0.25, 0.25), half_size, radii_h, radii_v)
  ) * 0.25;
}

float supersampled_stroke_alpha(float2 local, float2 half_size, float4 radii_h, float4 radii_v, float4 stroke, float max_stroke)
{
  return (
    rounded_stroke_alpha(local + float2(-0.25, -0.25), half_size, radii_h, radii_v, stroke, max_stroke) +
    rounded_stroke_alpha(local + float2(0.25, -0.25), half_size, radii_h, radii_v, stroke, max_stroke) +
    rounded_stroke_alpha(local + float2(-0.25, 0.25), half_size, radii_h, radii_v, stroke, max_stroke) +
    rounded_stroke_alpha(local + float2(0.25, 0.25), half_size, radii_h, radii_v, stroke, max_stroke)
  ) * 0.25;
}

float2 shadow_erf(float2 x)
{
  float2 s = sign(x);
  float2 a = abs(x);
  float2 r = 1.0 + (0.278393 + (0.230389 + 0.078108 * (a * a)) * a) * a;
  r = r * r;
  return s - s / (r * r);
}

float shadow_gaussian(float x, float sigma)
{
  return exp(-(x * x) / (2.0 * sigma * sigma)) / (2.5066283 * sigma);
}

// A row of the rounded rect at height `y`, convolved along x with the
// Gaussian: the difference of two `erf`s at the row's curved ends.
float shadow_blur_along_x(float x, float y, float sigma, float corner, float2 half_size)
{
  float delta = min(half_size.y - corner - abs(y), 0.0);
  float curved = half_size.x - corner + sqrt(max(corner * corner - delta * delta, 0.0));
  float2 integral = 0.5 + 0.5 * shadow_erf((float2(x, x) + float2(-curved, curved)) * (0.70710678 / sigma));
  return integral.y - integral.x;
}

// Coverage of a rounded rect (half extent `half_size`, circular corner
// `radii`) blurred by `sigma`, at centre-relative `p`; the y convolution is a
// midpoint sum over +/- 3 sigma.
float blurred_rounded_box(float2 p, float2 half_size, float4 radii, float sigma)
{
  if (half_size.x <= 0.0 || half_size.y <= 0.0)
  {
    return 0.0;
  }
  float corner = max(min(pick_radius(p, radii, radii).x, min(half_size.x, half_size.y)), 0.0);
  float extent = SHADOW_EXTENT_SIGMAS * sigma;
  float start = clamp(-extent, p.y - half_size.y, p.y + half_size.y);
  float end = clamp(extent, p.y - half_size.y, p.y + half_size.y);
  float step = (end - start) / (float)SHADOW_Y_SAMPLES;
  float y = start + step * 0.5;
  float coverage = 0.0;
  [unroll]
  for (int i = 0; i < SHADOW_Y_SAMPLES; i = i + 1)
  {
    coverage += shadow_blur_along_x(p.x, p.y - y, sigma, corner, half_size) * shadow_gaussian(y, sigma) * step;
    y += step;
  }
  return saturate(coverage);
}

float shadow_shape_alpha(float2 p, float2 half_size, float4 radii, float sigma)
{
  if (sigma < SHADOW_MIN_SIGMA)
  {
    if (half_size.x <= 0.0 || half_size.y <= 0.0)
    {
      return 0.0;
    }
    return supersampled_fill_alpha(p, half_size, radii, radii);
  }
  return blurred_rounded_box(p, half_size, radii, sigma);
}

// An outer shadow covers its shape outside the element box; an inset one
// covers the box outside its shape.
float box_shadow_alpha(float2 local, float2 half_size, float4 box_radii, float4 shape_radii, float4 params, float sigma)
{
  float2 offset = params.yz;
  float spread = params.w;
  float box_alpha = supersampled_fill_alpha(local, half_size, box_radii, box_radii);
  if (params.x < 1.5)
  {
    float2 shape_half = max(half_size + float2(spread, spread), float2(0.0, 0.0));
    return shadow_shape_alpha(local - offset, shape_half, shape_radii, sigma) * (1.0 - box_alpha);
  }
  float2 shape_half = max(half_size - float2(spread, spread), float2(0.0, 0.0));
  return box_alpha * (1.0 - shadow_shape_alpha(local - offset, shape_half, shape_radii, sigma));
}

// The render target is not sRGB, so the fixed-function blend mixes
// sRGB-encoded values, as CSS and design tools do. `ps_main` works in linear
// light like the rest of the pipeline and encodes each result it returns.
float4 encode_srgb(float4 color)
{
  float3 c = saturate(color.rgb);
  float3 low = c * 12.92;
  float3 high = 1.055 * pow(c, 1.0 / 2.4) - 0.055;
  return float4(c <= 0.0031308 ? low : high, color.a);
}

float4 ps_main(VsOut input) : SV_TARGET
{
  float clip_alpha_value = rounded_clip_alpha(input.position.xy);
  if (clip_alpha_value <= 0.0)
  {
    discard;
  }

  float4 base_color = input.color;
  if (input.gradient_offset >= 0.0)
  {
    int off = (int)(input.gradient_offset + 0.5);
    if ((int)gradients[off].x >= 1)
    {
      base_color = sample_gradient(off, input.local, input.half_size);
    }
  }

  float max_stroke = max(max(input.stroke.x, input.stroke.y), max(input.stroke.z, input.stroke.w));
  if (max_stroke <= 0.0)
  {
    if (input.pattern.x > 0.5)
    {
      float shadow_alpha = box_shadow_alpha(input.local, input.half_size, input.radii_h, input.radii_v, input.pattern, input.shadow_sigma);
      if (shadow_alpha <= 0.0)
      {
        discard;
      }
      return encode_srgb(float4(base_color.rgb, base_color.a * shadow_alpha * clip_alpha_value));
    }
    float fill_alpha = supersampled_fill_alpha(input.local, input.half_size, input.radii_h, input.radii_v);
    return encode_srgb(float4(base_color.rgb, base_color.a * fill_alpha * clip_alpha_value));
  }

  float alpha = supersampled_stroke_alpha(input.local, input.half_size, input.radii_h, input.radii_v, input.stroke, max_stroke);
  return encode_srgb(float4(input.color.rgb, input.color.a * alpha * clip_alpha_value));
}
