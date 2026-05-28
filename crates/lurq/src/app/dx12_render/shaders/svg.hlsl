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

float4 ps_main(VsOut input) : SV_TARGET
{
  return input.color;
}
