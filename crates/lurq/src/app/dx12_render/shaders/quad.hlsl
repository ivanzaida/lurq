cbuffer Globals : register(b0)
{
  float4 viewport;
  float4 clip_rect;
  float4 clip_radii_h;
  float4 clip_radii_v;
  float4 clip_active;
};

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
};

VsOut vs_main(VsIn input)
{
  float2 local_px = input.corner * input.size;
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
  output.local = (input.corner - float2(0.5, 0.5)) * input.size;
  output.half_size = input.size * 0.5;
  output.radii_h = input.radii_h;
  output.radii_v = input.radii_v;
  output.stroke = input.stroke;
  return output;
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
    float gradient = max(length(pn / safe_radius), 1e-6);
    return (len_pn - 1.0) * len_pn / gradient;
  }
  return max(q.x - safe_radius.x, q.y - safe_radius.y);
}

float4 ps_main(VsOut input) : SV_TARGET
{
  float2 radius = pick_radius(input.local, input.radii_h, input.radii_v);
  float outer_dist = sd_rounded_box(input.local, input.half_size, radius);
  float outer_alpha = 1.0 - smoothstep(0.0, 1.0, outer_dist);

  float max_stroke = max(max(input.stroke.x, input.stroke.y), max(input.stroke.z, input.stroke.w));
  float alpha = outer_alpha;
  if (max_stroke > 0.0)
  {
    float2 inner_half = max(input.half_size - float2(max_stroke, max_stroke), float2(0.0, 0.0));
    float2 inner_radius = max(radius - float2(max_stroke, max_stroke), float2(0.0, 0.0));
    float inner_dist = sd_rounded_box(input.local, inner_half, inner_radius);
    alpha *= smoothstep(0.0, 1.0, inner_dist);
  }

  return float4(input.color.rgb, input.color.a * alpha);
}
