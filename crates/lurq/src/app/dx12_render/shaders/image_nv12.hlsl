cbuffer Globals : register(b0)
{
  float4 viewport;
  float4 clip_rect;
  float4 clip_radii_h;
  float4 clip_radii_v;
  float4 clip_active;
};

Texture2D<float> y_texture : register(t0);
Texture2D<float2> uv_texture : register(t1);
SamplerState image_sampler : register(s0);

struct VsIn
{
  float2 corner : TEXCOORD0;
  float2 pos : TEXCOORD1;
  float2 size : TEXCOORD2;
  float4 opacity : TEXCOORD3;
  float4 transform : TEXCOORD4;
  float2 xf_origin : TEXCOORD5;
  float2 uv_min : TEXCOORD6;
  float2 uv_max : TEXCOORD7;
  float4 radii : TEXCOORD8;
};

struct VsOut
{
  float4 position : SV_POSITION;
  float2 uv : TEXCOORD0;
  float opacity : TEXCOORD1;
  float2 local_px : TEXCOORD2;
  float2 size : TEXCOORD3;
  float4 radii : TEXCOORD4;
};

float2 pick_radius(float2 p, float4 r)
{
  if (p.y < 0.0) {
    return p.x < 0.0 ? float2(r.x, r.x) : float2(r.y, r.y);
  }
  return p.x < 0.0 ? float2(r.w, r.w) : float2(r.z, r.z);
}

float2 pick_clip_radius(float2 p, float4 radii_h, float4 radii_v)
{
  if (p.y < 0.0) {
    return p.x < 0.0 ? float2(radii_h.x, radii_v.x) : float2(radii_h.y, radii_v.y);
  }
  return p.x < 0.0 ? float2(radii_h.w, radii_v.w) : float2(radii_h.z, radii_v.z);
}

float sd_rounded_box(float2 p, float2 half_size, float2 r)
{
  float2 safe_r = max(r, float2(0.0, 0.0));
  float2 q = abs(p) - half_size + safe_r;
  if (q.x > 0.0 && q.y > 0.0) {
    if (safe_r.x <= 0.0 || safe_r.y <= 0.0) {
      return max(q.x, q.y);
    }
    float2 pn = q / safe_r;
    float l = length(pn);
    if (l <= 1e-6) {
      return -min(safe_r.x, safe_r.y);
    }
    float g = max(length(pn / safe_r), 1e-6);
    return (l - 1.0) * l / g;
  }
  return max(q.x - safe_r.x, q.y - safe_r.y);
}

float3 srgb_to_linear(float3 color)
{
  float3 low = color / 12.92;
  float3 high = pow((color + 0.055) / 1.055, 2.4);
  return lerp(low, high, step(0.04045, color));
}

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
  float2 world = input.pos + transformed + input.xf_origin;
  float2 ndc = float2((world.x / viewport.x) * 2.0 - 1.0, 1.0 - (world.y / viewport.y) * 2.0);

  VsOut output;
  output.position = float4(ndc, 0.0, 1.0);
  float2 uv_size = max(input.size, float2(1e-6, 1e-6));
  output.uv = input.uv_min + (local_px / uv_size) * (input.uv_max - input.uv_min);
  output.opacity = input.opacity.x;
  output.local_px = local_px;
  output.size = input.size;
  output.radii = input.radii;
  return output;
}

float4 ps_main(VsOut input) : SV_TARGET
{
  float clip_alpha_value = 1.0;
  if (clip_active.x > 0.5) {
    float2 clip_half = clip_rect.zw * 0.5;
    float2 clip_centre = clip_rect.xy + clip_half;
    float2 clip_local = input.position.xy - clip_centre;
    float clip_dist = sd_rounded_box(clip_local, clip_half, pick_clip_radius(clip_local, clip_radii_h, clip_radii_v));
    clip_alpha_value = saturate(0.5 - clip_dist / max(fwidth(clip_dist), 1.0));
    if (clip_alpha_value <= 0.0) {
      discard;
    }
  }

  float2 half_size = input.size * 0.5;
  float2 local_clip = input.local_px - half_size;
  float d = sd_rounded_box(local_clip, half_size, pick_radius(local_clip, input.radii));
  float aa = max(fwidth(d), 0.001);
  float shape_alpha = saturate(0.5 - d / aa);
  if (shape_alpha <= 0.0) {
    discard;
  }

  float y = y_texture.Sample(image_sampler, input.uv);
  float2 uv = uv_texture.Sample(image_sampler, input.uv).rg;
  float c = max(y - 16.0 / 255.0, 0.0);
  float u = uv.x - 0.5;
  float v = uv.y - 0.5;
  float3 rgb = float3(
    1.164383 * c + 1.792741 * v,
    1.164383 * c - 0.213249 * u - 0.532909 * v,
    1.164383 * c + 2.112402 * u
  );
  float4 color = float4(srgb_to_linear(saturate(rgb)), 1.0);
  color.a *= input.opacity * shape_alpha * clip_alpha_value;
  return color;
}
