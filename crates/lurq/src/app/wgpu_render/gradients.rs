//! Where the quad shader reads gradient stops and the colors of a stroke's
//! sides from: a list of `vec4`s, one frame's worth, indexed by the instance.
//!
//! It is a storage buffer wherever the device has them. WebGL2 has none: there
//! the same `vec4`s are the texels of a float texture, [`TEXTURE_WIDTH`] to a
//! row, and the shader reads them with `textureLoad`. The shader is the same
//! but for the block between its `gradients` marks, which says how one `vec4`
//! is read.

/// Texels in a row of the texture the `vec4`s are kept in where there are no storage buffers.
const TEXTURE_WIDTH: u32 = 256;
const TEXEL_BYTES: u32 = 16;

const SHADER: &str = include_str!("shaders/quad.wgsl");
const MARK_START: &str = "// <gradients>";
const MARK_END: &str = "// </gradients>";

/// How the shader reads a `vec4` out of the texture.
const TEXTURE_READ: &str = "\
@group(0) @binding(1) var gradients: texture_2d<f32>;

fn gradient_at(index: i32) -> vec4<f32> {
    return textureLoad(gradients, vec2<i32>(index % 256, index / 256), 0);
}
";

pub(super) enum Gradients {
  Buffer {
    buffer: wgpu::Buffer,
    capacity: wgpu::BufferAddress,
  },
  Texture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    rows: u32,
    /// The frame's `vec4`s padded to whole rows.
    padded: Vec<[f32; 4]>,
  },
}

/// Whether `device` keeps the `vec4`s in a texture: it has no storage buffers a fragment shader reads.
pub(super) fn in_texture(device: &wgpu::Device) -> bool {
  device.limits().max_storage_buffers_per_shader_stage == 0
}

/// The quad shader, reading its `vec4`s from where `in_texture` says.
pub(super) fn quad_shader(in_texture: bool) -> std::borrow::Cow<'static, str> {
  if !in_texture {
    return SHADER.into();
  }
  let start = SHADER.find(MARK_START).expect("the quad shader marks its gradients");
  let end = SHADER.find(MARK_END).expect("the quad shader marks its gradients") + MARK_END.len();
  format!("{}{TEXTURE_READ}{}", &SHADER[..start], &SHADER[end..]).into()
}

/// The entry of the quad pipeline's bind group layout the `vec4`s are bound at.
pub(super) fn layout_entry(in_texture: bool) -> wgpu::BindGroupLayoutEntry {
  wgpu::BindGroupLayoutEntry {
    binding: 1,
    visibility: wgpu::ShaderStages::FRAGMENT,
    ty: if in_texture {
      wgpu::BindingType::Texture {
        sample_type: wgpu::TextureSampleType::Float { filterable: false },
        view_dimension: wgpu::TextureViewDimension::D2,
        multisampled: false,
      }
    } else {
      wgpu::BindingType::Buffer {
        ty: wgpu::BufferBindingType::Storage { read_only: true },
        has_dynamic_offset: false,
        min_binding_size: None,
      }
    },
    count: None,
  }
}

fn buffer(device: &wgpu::Device, capacity: wgpu::BufferAddress) -> wgpu::Buffer {
  device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("lurq_gradients"),
    size: capacity,
    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    mapped_at_creation: false,
  })
}

fn texture(device: &wgpu::Device, rows: u32) -> (wgpu::Texture, wgpu::TextureView) {
  let texture = device.create_texture(&wgpu::TextureDescriptor {
    label: Some("lurq_gradients"),
    size: wgpu::Extent3d {
      width: TEXTURE_WIDTH,
      height: rows,
      depth_or_array_layers: 1,
    },
    mip_level_count: 1,
    sample_count: 1,
    dimension: wgpu::TextureDimension::D2,
    format: wgpu::TextureFormat::Rgba32Float,
    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    view_formats: &[],
  });
  let view = texture.create_view(&Default::default());
  (texture, view)
}

impl Gradients {
  /// Room for one `vec4`: a binding must have something behind it, also in a frame with no gradient.
  pub(super) fn new(device: &wgpu::Device) -> Self {
    if in_texture(device) {
      let (texture, view) = texture(device, 1);
      Self::Texture {
        texture,
        view,
        rows: 1,
        padded: Vec::new(),
      }
    } else {
      let capacity = std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress;
      Self::Buffer {
        buffer: buffer(device, capacity),
        capacity,
      }
    }
  }

  /// What a bind group binds at the entry [`layout_entry`] describes.
  pub(super) fn binding(&self) -> wgpu::BindingResource<'_> {
    match self {
      Self::Buffer { buffer, .. } => buffer.as_entire_binding(),
      Self::Texture { view, .. } => wgpu::BindingResource::TextureView(view),
    }
  }

  /// Makes room for `count` `vec4`s. `true` when what is bound was replaced by a larger one: the bind groups
  /// made with the old one bind nothing the frame reads, and what it held is gone.
  pub(super) fn reserve(&mut self, device: &wgpu::Device, count: usize) -> bool {
    match self {
      Self::Buffer {
        buffer: bound,
        capacity,
      } => {
        let required = (count * std::mem::size_of::<[f32; 4]>()) as wgpu::BufferAddress;
        if *capacity >= required {
          return false;
        }
        *capacity = required.next_power_of_two().max(256);
        *bound = buffer(device, *capacity);
        true
      }
      Self::Texture {
        texture: bound,
        view,
        rows,
        ..
      } => {
        let required = (count as u32).div_ceil(TEXTURE_WIDTH).max(1);
        if *rows >= required {
          return false;
        }
        *rows = required.next_power_of_two();
        (*bound, *view) = texture(device, *rows);
        true
      }
    }
  }

  /// Uploads the frame's `vec4`s, which [`Self::reserve`] made room for.
  pub(super) fn write(&mut self, queue: &wgpu::Queue, data: &[[f32; 4]]) {
    match self {
      Self::Buffer { buffer, .. } => queue.write_buffer(buffer, 0, bytemuck::cast_slice(data)),
      Self::Texture { texture, padded, .. } => {
        let rows = (data.len() as u32).div_ceil(TEXTURE_WIDTH).max(1);
        padded.clear();
        padded.extend_from_slice(data);
        padded.resize((rows * TEXTURE_WIDTH) as usize, [0.0; 4]);
        queue.write_texture(
          wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
          },
          bytemuck::cast_slice(padded),
          wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(TEXTURE_WIDTH * TEXEL_BYTES),
            rows_per_image: Some(rows),
          },
          wgpu::Extent3d {
            width: TEXTURE_WIDTH,
            height: rows,
            depth_or_array_layers: 1,
          },
        );
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn the_shader_reads_a_texture_where_there_are_no_storage_buffers() {
    let storage = quad_shader(false);
    assert!(storage.contains("var<storage, read> gradients"));

    let texture = quad_shader(true);
    assert!(!texture.contains("var<storage"));
    assert!(texture.contains("var gradients: texture_2d<f32>"));
    assert!(!texture.contains(MARK_START) && !texture.contains(MARK_END));
    // Every read goes through the one function the two differ in.
    assert!(!texture.contains("gradients["));
    assert_eq!(
      storage.matches("gradient_at(").count(),
      texture.matches("gradient_at(").count()
    );
    // The row length the shader divides by is the texture's.
    assert!(TEXTURE_READ.contains(&format!("index % {TEXTURE_WIDTH}, index / {TEXTURE_WIDTH}")));
  }
}
