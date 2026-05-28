cbuffer Globals : register(b0)
{
  float4 viewport;
  float4 clip_rect;
  float4 clip_radii_h;
  float4 clip_radii_v;
  float4 clip_active;
};

Texture2D<float> atlas_texture : register(t0);
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
};

struct VsOut
{
  float4 position : SV_POSITION;
  float4 color : COLOR0;
  float2 uv : TEXCOORD0;
};

VsOut vs_main(VsIn input)
{
  float2 local_px = input.corner * input.size;
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
  output.uv = lerp(input.uv_min, input.uv_max, input.corner);
  return output;
}

float4 ps_main(VsOut input) : SV_TARGET
{
  float coverage = atlas_texture.Sample(atlas_sampler, input.uv);
  return float4(input.color.rgb, input.color.a * coverage);
}
