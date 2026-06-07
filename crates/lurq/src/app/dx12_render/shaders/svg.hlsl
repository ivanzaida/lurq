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
  float2 position : TEXCOORD0;
  float4 color : TEXCOORD1;
};

struct VsOut
{
  float4 position : SV_POSITION;
  float4 color : COLOR0;
};

VsOut vs_main(VsIn input)
{
  float2 ndc = float2((input.position.x / viewport.x) * 2.0 - 1.0, 1.0 - (input.position.y / viewport.y) * 2.0);

  VsOut output;
  output.position = float4(ndc, 0.0, 1.0);
  output.color = input.color;
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
    if (len_pn <= 1e-6)
    {
      return -min(safe_radius.x, safe_radius.y);
    }
    float gradient = max(length(pn / safe_radius), 1e-6);
    return (len_pn - 1.0) * len_pn / gradient;
  }
  return max(q.x - safe_radius.x, q.y - safe_radius.y);
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
  return saturate(0.5 - dist / max(fwidth(dist), 1.0));
}

float4 ps_main(VsOut input) : SV_TARGET
{
  float clip_alpha_value = rounded_clip_alpha(input.position.xy);
  if (clip_alpha_value <= 0.0)
  {
    discard;
  }
  return float4(input.color.rgb, input.color.a * clip_alpha_value);
}
