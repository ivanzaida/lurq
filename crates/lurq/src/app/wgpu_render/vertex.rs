#[cfg(feature = "image")]
pub use crate::render::gpu::ImageInstance;
#[cfg(feature = "svg")]
pub use crate::render::gpu::SvgVertexGpu;
pub use crate::render::gpu::{Globals, GlyphInstance, QuadInstance, QuadVertex};

impl QuadVertex {
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
          format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
          offset: 64,
          shader_location: 6,
          format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
          offset: 80,
          shader_location: 7,
          format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
          offset: 96,
          shader_location: 8,
          format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
          offset: 112,
          shader_location: 9,
          format: wgpu::VertexFormat::Float32x2,
        },
        wgpu::VertexAttribute {
          offset: 120,
          shader_location: 10,
          format: wgpu::VertexFormat::Float32,
        },
        wgpu::VertexAttribute {
          offset: 124,
          shader_location: 11,
          format: wgpu::VertexFormat::Float32,
        },
      ],
    }
  }
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
          format: wgpu::VertexFormat::Float32x2,
        },
        wgpu::VertexAttribute {
          offset: 40,
          shader_location: 5,
          format: wgpu::VertexFormat::Float32x2,
        },
        wgpu::VertexAttribute {
          offset: 48,
          shader_location: 6,
          format: wgpu::VertexFormat::Float32x4,
        },
        wgpu::VertexAttribute {
          offset: 64,
          shader_location: 7,
          format: wgpu::VertexFormat::Float32x2,
        },
      ],
    }
  }
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
        wgpu::VertexAttribute {
          offset: 56,
          shader_location: 6,
          format: wgpu::VertexFormat::Float32x2,
        },
        wgpu::VertexAttribute {
          offset: 64,
          shader_location: 7,
          format: wgpu::VertexFormat::Float32x2,
        },
        wgpu::VertexAttribute {
          offset: 72,
          shader_location: 8,
          format: wgpu::VertexFormat::Float32x4,
        },
      ],
    }
  }
}

#[cfg(feature = "svg")]
impl SvgVertexGpu {
  pub fn desc() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
      array_stride: std::mem::size_of::<SvgVertexGpu>() as wgpu::BufferAddress,
      step_mode: wgpu::VertexStepMode::Vertex,
      attributes: &[
        wgpu::VertexAttribute {
          offset: 0,
          shader_location: 0,
          format: wgpu::VertexFormat::Float32x2,
        },
        wgpu::VertexAttribute {
          offset: 8,
          shader_location: 1,
          format: wgpu::VertexFormat::Float32x4,
        },
      ],
    }
  }
}
