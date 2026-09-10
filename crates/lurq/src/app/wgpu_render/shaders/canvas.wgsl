struct Globals { tile: vec4<f32>, surface: vec4<f32> };
@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var image: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;
struct Out { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) color: vec4<f32> };
@vertex fn vs_main(@location(0) position: vec2<f32>, @location(1) uv: vec2<f32>, @location(2) color: vec4<f32>) -> Out {
  let p = (position - globals.tile.xy) / globals.tile.zw;
  return Out(vec4<f32>(p.x * 2.0 - 1.0, 1.0 - p.y * 2.0, 0.0, 1.0), uv, color);
}
@vertex fn vs_seed(@builtin(vertex_index) index:u32) -> Out {
  let p = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
  return Out(vec4<f32>(p.x*2.0-1.0,1.0-p.y*2.0,0.0,1.0), (globals.tile.xy+p*globals.tile.zw)/globals.surface.xy, vec4<f32>(1.0));
}
@fragment fn fs_solid(v:Out) -> @location(0) vec4<f32> { return v.color; }
@fragment fn fs_image(v:Out) -> @location(0) vec4<f32> {
  let c = textureSample(image,image_sampler,v.uv);
  return vec4<f32>(c.rgb*c.a,c.a) * v.color;
}
@fragment fn fs_premul(v:Out) -> @location(0) vec4<f32> {
  return textureSample(image,image_sampler,v.uv) * v.color;
}
