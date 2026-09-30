// Composites a finished opacity layer into its parent target.
//
// The layer texture holds premultiplied colour: it was cleared to
// transparent and painted with the regular pipelines, whose source-over
// blend onto a premultiplied destination keeps it premultiplied. Its texels
// map one to one onto target pixels (both are whole window pixels), so the
// pixel shader loads its texel instead of sampling: no filtering, no blur at
// any DPI scale. The pipeline blends premultiplied source-over.

cbuffer Composite : register(b0)
{
  float4 rect;    // layer rect in target pixels: x, y, w, h
  float4 target;  // target size, then target pixel -> layer texel offset
  float4 opacity; // x = layer opacity
};

Texture2D<float4> layer_texture : register(t0);

struct VsOut
{
  float4 position : SV_POSITION;
};

VsOut vs_main(float2 corner : TEXCOORD0)
{
  float2 px = rect.xy + corner * rect.zw;
  VsOut output;
  output.position = float4((px.x / target.x) * 2.0 - 1.0, 1.0 - (px.y / target.y) * 2.0, 0.0, 1.0);
  return output;
}

float4 ps_main(VsOut input) : SV_TARGET
{
  int2 texel = int2(floor(input.position.xy + target.zw));
  return layer_texture.Load(int3(texel, 0)) * opacity.x;
}
