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

// A picture of two planes (NV12): the gray, and the two of color side by side at half the size. Compiled with
// `THREE_PLANES` for one of three (I420): the gray, and each of the two of color at half the size apart.
Texture2D<float> y_texture : register(t0);
#ifdef THREE_PLANES
Texture2D<float> u_texture : register(t1);
Texture2D<float> v_texture : register(t2);
#else
Texture2D<float2> uv_texture : register(t1);
#endif
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
  // The matrix and the range of the frame's samples (`yuv_to_rgb`), the instance's `opacity.zw`.
  float2 color : TEXCOORD5;
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

// The color of a sample of a video's frame: Y'CbCr made R'G'B' (gamma-encoded, 0 to 1), as the frame says it is,
// the same as `yuv_to_rgb` of the wgpu renderer's `yuv.wgsl`. `kind` is the matrix the frame was made with: 0 BT.709,
// 1 BT.601, 2 BT.2020 (of a constant luminance not). `full_range` is 1 where the samples take all of their range, 0
// where gray is from 16 to 235 and color from 16 to 240 of 255 (the video range, which a decoder gives unless told
// otherwise). Coefficients from the standards' K_R and K_B.
float3 yuv_to_rgb(float y, float cb_sample, float cr_sample, float kind, float full_range)
{
  float luma = y;
  float cb = cb_sample - 0.5;
  float cr = cr_sample - 0.5;
  if (full_range < 0.5) {
    luma = max(y - 16.0 / 255.0, 0.0) * (255.0 / 219.0);
    cb = cb * (255.0 / 224.0);
    cr = cr * (255.0 / 224.0);
  }
  // BT.709.
  float red_cr = 1.5748;
  float green_cb = 0.187324;
  float green_cr = 0.468124;
  float blue_cb = 1.8556;
  if (kind > 1.5) {
    red_cr = 1.4746;
    green_cb = 0.164553;
    green_cr = 0.571353;
    blue_cb = 1.8814;
  } else if (kind > 0.5) {
    red_cr = 1.402;
    green_cb = 0.344136;
    green_cr = 0.714136;
    blue_cb = 1.772;
  }
  return float3(luma + red_cr * cr, luma - green_cb * cb - green_cr * cr, luma + blue_cb * cb);
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
  // `viewport.zw` is the target's origin in window pixels (non-zero when
  // painting into an opacity layer).
  float2 target_px = world - viewport.zw;
  float2 ndc = float2((target_px.x / viewport.x) * 2.0 - 1.0, 1.0 - (target_px.y / viewport.y) * 2.0);

  VsOut output;
  output.position = float4(ndc, 0.0, 1.0);
  float2 uv_size = max(input.size, float2(1e-6, 1e-6));
  output.uv = input.uv_min + (local_px / uv_size) * (input.uv_max - input.uv_min);
  output.opacity = input.opacity.x;
  output.local_px = local_px;
  output.size = input.size;
  output.radii = input.radii;
  output.color = input.opacity.zw;
  return output;
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

// How much of the pixel at `frag_pos` (window pixels) the rounded clip at
// `rect` with these radii covers.
float rounded_clip_coverage(float2 frag_pos, float4 rect, float4 radii_h, float4 radii_v)
{
  float2 clip_half = rect.zw * 0.5;
  float2 clip_local = frag_pos - (rect.xy + clip_half);
  float clip_dist = sd_rounded_box(clip_local, clip_half, pick_clip_radius(clip_local, radii_h, radii_v));
  return saturate(0.5 - clip_dist / max(fwidth(clip_dist), 1.0));
}

float4 ps_main(VsOut input) : SV_TARGET
{
  float clip_alpha_value = 1.0;
  if (clip_active.x > 0.5) {
    clip_alpha_value = rounded_clip_coverage(input.position.xy, clip_rect, clip_radii_h, clip_radii_v);
  }
  // Under a second rounded clip, the draw is cut by both.
  if (clip_active.y > 0.5) {
    clip_alpha_value *= rounded_clip_coverage(input.position.xy, outer_clip_rect, outer_clip_radii, outer_clip_radii);
  }
  if (clip_alpha_value <= 0.0) {
    discard;
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
#ifdef THREE_PLANES
  float2 uv = float2(u_texture.Sample(image_sampler, input.uv), v_texture.Sample(image_sampler, input.uv));
#else
  float2 uv = uv_texture.Sample(image_sampler, input.uv).rg;
#endif
  float3 rgb = yuv_to_rgb(y, uv.x, uv.y, input.color.x, input.color.y);
  float4 color = float4(srgb_to_linear(saturate(rgb)), 1.0);
  color.a *= input.opacity * shape_alpha * clip_alpha_value;
  return encode_srgb(color);
}
