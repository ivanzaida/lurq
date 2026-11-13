cbuffer Globals : register(b0)
{
  float4 viewport;
  float4 clip_rect;
  float4 clip_radii_h;
  float4 clip_radii_v;
  float4 clip_active;      // x: cut by the innermost rounded clip; y: by the one outside it too
  float4 outer_clip_rect;  // the rounded clip outside the innermost one: x, y, w, h
  float4 outer_clip_radii; // and its radii: tl, tr, br, bl
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
  // `viewport.zw` is the target's origin in window pixels (non-zero when
  // painting into an opacity layer).
  float2 target_px = input.position - viewport.zw;
  float2 ndc = float2((target_px.x / viewport.x) * 2.0 - 1.0, 1.0 - (target_px.y / viewport.y) * 2.0);

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

// How much of the pixel at `frag_pos` (window pixels) the rounded clip at
// `rect` with these radii covers.
float rounded_clip_coverage(float2 frag_pos, float4 rect, float4 radii_h, float4 radii_v)
{
  float2 half_size = rect.zw * 0.5;
  float2 local = frag_pos - (rect.xy + half_size);
  float dist = sd_rounded_box(local, half_size, pick_radius(local, radii_h, radii_v));
  return saturate(0.5 - dist / max(fwidth(dist), 1.0));
}

// How much of the pixel the rounded clips the draw is under cover: under
// two, both cut it.
float rounded_clip_alpha(float2 frag_pos)
{
  float alpha = 1.0;
  if (clip_active.x > 0.5)
  {
    alpha = rounded_clip_coverage(frag_pos, clip_rect, clip_radii_h, clip_radii_v);
  }
  if (clip_active.y > 0.5)
  {
    alpha *= rounded_clip_coverage(frag_pos, outer_clip_rect, outer_clip_radii, outer_clip_radii);
  }
  return alpha;
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
  return encode_srgb(float4(input.color.rgb, input.color.a * clip_alpha_value));
}
