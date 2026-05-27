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

  pub fn desc() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
      array_stride: std::mem::size_of::<QuadVertex>() as wgpu::BufferAddress,
      step_mode: wgpu::VertexStepMode::Vertex,
      attributes: &[wgpu::VertexAttribute {
        offset: 0,
        shader_location: 0,
        format: wgpu::VertexFormat::Float32x2,
      }],
    }
  }
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

impl QuadInstance {
  pub fn desc() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
      array_stride: std::mem::size_of::<QuadInstance>() as wgpu::BufferAddress,
      step_mode: wgpu::VertexStepMode::Instance,
      attributes: &[
        wgpu::VertexAttribute {
          offset: 0,
          shader_location: 1,
          format: wgpu::VertexFormat::Float32x2,
        }, // pos
        wgpu::VertexAttribute {
          offset: 8,
          shader_location: 2,
          format: wgpu::VertexFormat::Float32x2,
        }, // size
        wgpu::VertexAttribute {
          offset: 16,
          shader_location: 3,
          format: wgpu::VertexFormat::Float32x4,
        }, // color
        wgpu::VertexAttribute {
          offset: 32,
          shader_location: 4,
          format: wgpu::VertexFormat::Float32x4,
        }, // radii_h
        wgpu::VertexAttribute {
          offset: 48,
          shader_location: 5,
          format: wgpu::VertexFormat::Float32x4,
        }, // radii_v
        wgpu::VertexAttribute {
          offset: 64,
          shader_location: 6,
          format: wgpu::VertexFormat::Float32x4,
        }, // stroke
        wgpu::VertexAttribute {
          offset: 80,
          shader_location: 7,
          format: wgpu::VertexFormat::Float32x4,
        }, // pattern
        wgpu::VertexAttribute {
          offset: 96,
          shader_location: 8,
          format: wgpu::VertexFormat::Float32x4,
        }, // transform
        wgpu::VertexAttribute {
          offset: 112,
          shader_location: 9,
          format: wgpu::VertexFormat::Float32x2,
        }, // xf_origin
        wgpu::VertexAttribute {
          offset: 120,
          shader_location: 10,
          format: wgpu::VertexFormat::Float32,
        }, // shadow_sigma
        wgpu::VertexAttribute {
          offset: 124,
          shader_location: 11,
          format: wgpu::VertexFormat::Float32,
        }, // gradient_offset
      ],
    }
  }
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

impl GlyphInstance {
  pub fn desc() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
      array_stride: std::mem::size_of::<GlyphInstance>() as wgpu::BufferAddress,
      step_mode: wgpu::VertexStepMode::Instance,
      attributes: &[
        wgpu::VertexAttribute {
          offset: 0,
          shader_location: 1,
          format: wgpu::VertexFormat::Float32x2,
        }, // pos
        wgpu::VertexAttribute {
          offset: 8,
          shader_location: 2,
          format: wgpu::VertexFormat::Float32x2,
        }, // size
        wgpu::VertexAttribute {
          offset: 16,
          shader_location: 3,
          format: wgpu::VertexFormat::Float32x4,
        }, // color
        wgpu::VertexAttribute {
          offset: 32,
          shader_location: 4,
          format: wgpu::VertexFormat::Float32x2,
        }, // uv_min
        wgpu::VertexAttribute {
          offset: 40,
          shader_location: 5,
          format: wgpu::VertexFormat::Float32x2,
        }, // uv_max
        wgpu::VertexAttribute {
          offset: 48,
          shader_location: 6,
          format: wgpu::VertexFormat::Float32x4,
        }, // transform
        wgpu::VertexAttribute {
          offset: 64,
          shader_location: 7,
          format: wgpu::VertexFormat::Float32x2,
        }, // xf_origin
      ],
    }
  }
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
}

#[cfg(feature = "image")]
impl ImageInstance {
  pub fn desc() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
      array_stride: std::mem::size_of::<ImageInstance>() as wgpu::BufferAddress,
      step_mode: wgpu::VertexStepMode::Instance,
      attributes: &[
        wgpu::VertexAttribute {
          offset: 0,
          shader_location: 1,
          format: wgpu::VertexFormat::Float32x2,
        },
        wgpu::VertexAttribute {
          offset: 8,
          shader_location: 2,
          format: wgpu::VertexFormat::Float32x2,
        },
        wgpu::VertexAttribute {
          offset: 16,
          shader_location: 3,
          format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
          offset: 32,
          shader_location: 4,
          format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
          offset: 48,
          shader_location: 5,
          format: wgpu::VertexFormat::Float32x2,
        },
      ],
    }
  }
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
