// A frame a decoder leaves in a `CVPixelBuffer` is drawn where it is, through CoreVideo's Metal textures
// (`create_macos_native_nv12_cached_image_texture`): what the renderer reads of it is what was written, byte for
// byte, though the buffer's rows are longer than its picture.

use std::sync::Arc;

use crate::{
  images::{CvPixelBufferFrames, ImageCmd, ImagePixelFormat, MacosCvPixelBuffer, NativeImageData},
  layout::quad::ClipRect,
};

/// A device on Metal, as the renderer has; `None` where there is none.
fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
  let instance = wgpu::Instance::new(super::instance_descriptor(wgpu::Backends::METAL));
  let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).ok()?;
  pollster::block_on(adapter.request_device(&super::device_descriptor(
    &adapter,
    wgpu::Features::empty(),
    &wgpu::Limits::default(),
  )))
  .ok()
}

/// The gray of a pixel, and the two of color of a pixel of the color plane: none the same as the next, so that a
/// row read from the wrong place, or a plane from the other, is seen.
fn gray_at(x: usize, y: usize) -> u8 {
  ((x * 7 + y * 13) % 251) as u8
}

fn color_at(x: usize, y: usize) -> (u8, u8) {
  (((x * 29 + y * 3) % 241) as u8, ((x * 5 + y * 31 + 17) % 239) as u8)
}

/// What the renderer draws `image` with: its two planes, as the renderer opened them.
fn opened(device: &wgpu::Device, image: &ImageCmd) -> super::CachedImageTexture {
  let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
    binding,
    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
    ty,
    count: None,
  };
  let plane = wgpu::BindingType::Texture {
    sample_type: wgpu::TextureSampleType::Float { filterable: true },
    view_dimension: wgpu::TextureViewDimension::D2,
    multisampled: false,
  };
  let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: None,
    entries: &[
      entry(
        0,
        wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Uniform,
          has_dynamic_offset: false,
          min_binding_size: None,
        },
      ),
      entry(1, plane),
      entry(2, plane),
      entry(3, wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)),
    ],
  });
  let sampler = device.create_sampler(&Default::default());
  let globals = device.create_buffer(&wgpu::BufferDescriptor {
    label: None,
    size: 256,
    usage: wgpu::BufferUsages::UNIFORM,
    mapped_at_creation: false,
  });
  super::create_macos_native_nv12_cached_image_texture(device, &layout, &sampler, &globals, image)
    .expect("the renderer opens the buffer")
}

fn image_of(native: NativeImageData, width: u32, height: u32, version: u64) -> ImageCmd {
  ImageCmd {
    order: 0,
    x: 0.0,
    y: 0.0,
    width: width as f32,
    height: height as f32,
    image_id: 1,
    frame_index: 0,
    version,
    data: Arc::new(Vec::new()),
    animation_frames: None,
    native: Some(native),
    image_width: width,
    image_height: height,
    image_format: ImagePixelFormat::Nv12,
    uv_min: [0.0, 0.0],
    uv_max: [1.0, 1.0],
    radii: [0.0; 4],
    transform: [1.0, 0.0, 0.0, 1.0],
    transform_origin: [0.0; 2],
    clip: ClipRect::default(),
    opacity: 1.0,
  }
}

/// Writes a frame into a buffer `buffer` large, shows the part of it `picture` large as a video shows its frames,
/// and gives the pixels the renderer read that are not the buffer's: where, and what was read.
fn not_the_buffers(buffer: (usize, usize), picture: (usize, usize)) -> Vec<(usize, usize, Vec<u8>)> {
  let Some((device, queue)) = device() else {
    eprintln!("skipped: no Metal device");
    return Vec::new();
  };
  let pixel_buffer = MacosCvPixelBuffer::nv12(buffer.0 as u32, buffer.1 as u32).expect("a pixel buffer");
  let mut strides = (0, 0);
  assert!(pixel_buffer.write_nv12(|gray, gray_stride, color, color_stride| {
    strides = (gray_stride, color_stride);
    for y in 0..buffer.1 {
      for x in 0..buffer.0 {
        gray[y * gray_stride + x] = gray_at(x, y);
      }
    }
    for y in 0..buffer.1 / 2 {
      for x in 0..buffer.0 / 2 {
        let (u, v) = color_at(x, y);
        color[y * color_stride + x * 2] = u;
        color[y * color_stride + x * 2 + 1] = v;
      }
    }
  }));
  assert!(strides.0 > buffer.0, "rows longer than the picture: {strides:?}");

  let (width, height) = picture;
  let frames = CvPixelBufferFrames::new();
  frames.show(pixel_buffer);
  let native = NativeImageData::from_cv_pixel_buffer_frames(width as u32, height as u32, frames);
  let super::CachedImageTexture::Nv12 {
    y_view,
    uv_view,
    _macos_native,
    ..
  } = opened(&device, &image_of(native, width as u32, height as u32, 1))
  else {
    panic!("the buffer is opened as two planes");
  };
  assert!(_macos_native.is_some(), "drawn where it is, not copied");

  let pixels = super::planes_drawn::drawn(&device, &queue, &y_view, &uv_view, width as u32, height as u32);
  let mut wrong = Vec::new();
  for y in 0..height {
    for x in 0..width {
      let pixel = &pixels[(y * width + x) * 4..][..3];
      let (u, v) = color_at(x / 2, y / 2);
      if pixel != [gray_at(x, y), u, v] {
        wrong.push((x, y, pixel.to_vec()));
      }
    }
  }
  wrong
}

#[test]
fn a_frame_in_a_pixel_buffer_is_drawn_from_where_the_decoder_left_it() {
  // 100 wide: CoreVideo pads each row to a multiple of 64 bytes, so the buffer's rows are longer than the picture.
  let wrong = not_the_buffers((100, 36), (100, 36));
  assert!(
    wrong.is_empty(),
    "{} pixels are not the buffer's, the first: {:?}",
    wrong.len(),
    wrong.first()
  );
}

#[test]
fn a_buffer_larger_than_its_picture_is_drawn_as_its_picture() {
  // As a codec's 1088 rows hold a picture of 1080.
  let wrong = not_the_buffers((100, 40), (100, 36));
  assert!(
    wrong.is_empty(),
    "{} pixels are not the buffer's, the first: {:?}",
    wrong.len(),
    wrong.first()
  );
}
