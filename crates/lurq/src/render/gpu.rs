#![allow(dead_code)]

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadVertex {
  pub corner: [f32; 2],
}

impl QuadVertex {
  pub const CORNERS: [QuadVertex; 4] = [
    QuadVertex { corner: [0.0, 0.0] },
    QuadVertex { corner: [1.0, 0.0] },
    QuadVertex { corner: [1.0, 1.0] },
    QuadVertex { corner: [0.0, 1.0] },
  ];

  pub const INDICES: [u16; 6] = [0, 1, 2, 0, 2, 3];
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadInstance {
  pub pos: [f32; 2],
  pub size: [f32; 2],
  pub color: [f32; 4],
  pub radii_h: [f32; 4],
  pub radii_v: [f32; 4],
  pub stroke: [f32; 4],
  pub pattern: [f32; 4],
  pub transform: [f32; 4],
  pub xf_origin: [f32; 2],
  pub shadow_sigma: f32,
  pub gradient_offset: f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlyphInstance {
  pub pos: [f32; 2],
  pub size: [f32; 2],
  pub color: [f32; 4],
  pub uv_min: [f32; 2],
  pub uv_max: [f32; 2],
  pub transform: [f32; 4],
  pub xf_origin: [f32; 2],
}

#[cfg(feature = "image")]
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ImageInstance {
  pub pos: [f32; 2],
  pub size: [f32; 2],
  pub opacity: [f32; 4],
  pub transform: [f32; 4],
  pub xf_origin: [f32; 2],
  pub uv_min: [f32; 2],
  pub uv_max: [f32; 2],
  pub radii: [f32; 4],
}

#[cfg(feature = "svg")]
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SvgVertexGpu {
  pub position: [f32; 2],
  pub color: [f32; 4],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Globals {
  pub viewport: [f32; 4],
  pub clip_rect: [f32; 4],
  pub clip_radii_h: [f32; 4],
  pub clip_radii_v: [f32; 4],
  pub clip_active: [f32; 4],
}
