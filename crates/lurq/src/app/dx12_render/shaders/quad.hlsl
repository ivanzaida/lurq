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
};

VsOut vs_main(VsIn input)
{
  float aa_outset = 2.0;
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

float4 ps_main(VsOut input) : SV_TARGET
{
  float4 base_color = input.color;
  if (input.gradient_offset >= 0.0)
  {
    int off = (int)(input.gradient_offset + 0.5);
    if ((int)gradients[off].x >= 1)
    {
      base_color = sample_gradient(off, input.local, input.half_size);
    }
  }

  float2 radius = pick_radius(input.local, input.radii_h, input.radii_v);
  float outer_dist = sd_rounded_box(input.local, input.half_size, radius);

  float max_stroke = max(max(input.stroke.x, input.stroke.y), max(input.stroke.z, input.stroke.w));
  if (max_stroke <= 0.0)
  {
    float aa = max(fwidth(outer_dist), 0.001);
    float fill_alpha = saturate(0.5 - outer_dist / aa);
    return float4(base_color.rgb, base_color.a * fill_alpha);
  }

  float2 inner_half = max(float2(
    input.half_size.x - 0.5 * (input.stroke.y + input.stroke.w),
    input.half_size.y - 0.5 * (input.stroke.x + input.stroke.z)
  ), float2(0.0, 0.0));
  float2 inner_center = float2(
    0.5 * (input.stroke.w - input.stroke.y),
    0.5 * (input.stroke.x - input.stroke.z)
  );

  float2 inner_radius = max(radius - float2(max_stroke, max_stroke), float2(0.0, 0.0));
  float inner_dist = sd_rounded_box(input.local - inner_center, inner_half, inner_radius);
  float dist = max(outer_dist, -inner_dist);

  float aa = max(fwidth(dist), 0.001);
  float alpha = saturate(0.5 - dist / aa);
  return float4(input.color.rgb, input.color.a * alpha);
}
