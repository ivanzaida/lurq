cbuffer Globals : register(b0)
{
  float4 viewport;
  float4 clip_rect;
  float4 clip_radii_h;
  float4 clip_radii_v;
  float4 clip_active;
};

Texture2D<float4> atlas_texture : register(t0);
SamplerState atlas_sampler : register(s0);

struct VsIn
{
  float2 corner : TEXCOORD0;
  float2 pos : TEXCOORD1;
  float2 size : TEXCOORD2;
  float4 color : TEXCOORD3;
  float2 uv_min : TEXCOORD4;
  float2 uv_max : TEXCOORD5;
  float4 transform : TEXCOORD6;
  float2 xf_origin : TEXCOORD7;
  float sharpness : TEXCOORD8;
  float color_glyph : TEXCOORD9;
  float shadow_sigma : TEXCOORD10;
};

struct VsOut
{
  float4 position : SV_POSITION;
  float4 color : COLOR0;
  float2 uv : TEXCOORD0;
  float sharpness : TEXCOORD1;
  float color_glyph : TEXCOORD2;
  float shadow_sigma : TEXCOORD3;
  float4 uv_bounds : TEXCOORD4;
};

// Blur taps reach 2 * sigma texels; one extra texel keeps bilinear sampling
// at the edge inside the expanded region.
float shadow_pad(float sigma)
{
  return sigma > 0.0 ? ceil(sigma * 2.0) + 1.0 : 0.0;
}

VsOut vs_main(VsIn input)
{
  // Shadow instances grow beyond the glyph rect so the blur has room to
  // spill; uv is extrapolated through the same linear mapping.
  float pad = shadow_pad(input.shadow_sigma);
  float2 local_px = input.corner * (input.size + 2.0 * pad) - float2(pad, pad);
  float2 centered = local_px - input.xf_origin;
  float2 transformed = float2(
    input.transform.x * centered.x + input.transform.z * centered.y,
    input.transform.y * centered.x + input.transform.w * centered.y
  );
  float2 world = input.pos + transformed + input.xf_origin;
  float2 ndc = float2((world.x / viewport.x) * 2.0 - 1.0, 1.0 - (world.y / viewport.y) * 2.0);

  VsOut output;
  output.position = float4(ndc, 0.0, 1.0);
  output.color = input.color;
  output.uv = lerp(input.uv_min, input.uv_max, local_px / max(input.size, float2(1e-6, 1e-6)));
  output.sharpness = input.sharpness;
  output.color_glyph = input.color_glyph;
  output.shadow_sigma = input.shadow_sigma;
  output.uv_bounds = float4(input.uv_min, input.uv_max);
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

  float4 sample = atlas_texture.Sample(atlas_sampler, input.uv);

  // Text-shadow instances: Gaussian-blur the glyph's alpha coverage. Taps
  // outside the glyph's atlas rect are masked to zero so neighbouring atlas
  // entries never bleed in.
  if (input.shadow_sigma > 0.0)
  {
    float2 atlas_size;
    atlas_texture.GetDimensions(atlas_size.x, atlas_size.y);
    float2 texel = 1.0 / atlas_size;
    int radius = (int)ceil(input.shadow_sigma * 2.0);
    float inv_two_sigma2 = 1.0 / (2.0 * input.shadow_sigma * input.shadow_sigma);
    float sum = 0.0;
    float weight_sum = 0.0;
    for (int dy = -radius; dy <= radius; dy++)
    {
      for (int dx = -radius; dx <= radius; dx++)
      {
        float2 offset = float2(dx, dy);
        float weight = exp(-dot(offset, offset) * inv_two_sigma2);
        float2 tap_uv = input.uv + offset * texel;
        float inside = step(input.uv_bounds.x, tap_uv.x) * step(input.uv_bounds.y, tap_uv.y)
          * step(tap_uv.x, input.uv_bounds.z) * step(tap_uv.y, input.uv_bounds.w);
        sum += weight * atlas_texture.SampleLevel(atlas_sampler, tap_uv, 0.0).a * inside;
        weight_sum += weight;
      }
    }
    float shadow_coverage = sum / max(weight_sum, 1e-6);
    return float4(input.color.rgb, input.color.a * shadow_coverage * clip_alpha_value);
  }

  if (input.color_glyph > 0.5)
  {
    return float4(sample.rgb, sample.a * input.color.a * clip_alpha_value);
  }

  float coverage = sample.a;
  coverage = saturate((coverage - 0.5) * max(input.sharpness, 1.0) + 0.5);
  return float4(input.color.rgb, input.color.a * coverage * clip_alpha_value);
}
