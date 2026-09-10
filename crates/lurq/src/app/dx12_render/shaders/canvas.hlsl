cbuffer Globals : register(b0) { float4 tile; float4 surface; };
Texture2D<float4> image_texture : register(t0);
SamplerState image_sampler : register(s0);
struct Out { float4 position:SV_POSITION; float2 uv:TEXCOORD0; float4 color:TEXCOORD1; };
Out vs_main(float2 position:POSITION, float2 uv:TEXCOORD0, float4 color:COLOR0) {
  float2 p=(position-tile.xy)/tile.zw;
  Out o; o.position=float4(p.x*2-1,1-p.y*2,0,1);o.uv=uv;o.color=color;return o;
}
Out vs_seed(uint index:SV_VertexID) {
  float2 p=float2((index<<1)&2,index&2);
  Out o;o.position=float4(p.x*2-1,1-p.y*2,0,1);o.uv=(tile.xy+p*tile.zw)/surface.xy;o.color=1;return o;
}
float4 ps_solid(Out v):SV_TARGET {return v.color;}
float4 ps_premul(Out v):SV_TARGET {return image_texture.Sample(image_sampler,v.uv)*v.color;}
