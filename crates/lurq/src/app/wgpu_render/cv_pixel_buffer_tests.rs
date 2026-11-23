// A frame a decoder leaves in a `CVPixelBuffer` is drawn where it is, through CoreVideo's Metal textures
// (`create_macos_native_nv12_cached_image_texture`): what the renderer reads of it is what was written, byte for
// byte, though the buffer's rows are longer than its picture. A frame of red, green and blue too, and a frame the
// app's card writes is shown once the card signals it is done.

use std::sync::Arc;

use crate::{
  images::{CvPixelBufferFrames, ImageCmd, ImagePixelFormat, MacosCvPixelBuffer, NativeImageData},
  layout::quad::ClipRect,
};

/// A device on Metal, as the renderer has; `None` where there is none.
fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
  let instance = wgpu::Instance::new(super::instance_descriptor(
    wgpu::Backends::METAL,
    wgpu::InstanceFlags::from_build_config(),
  ));
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
  // A picture's: one view.
  let picture = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
      entry(2, wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)),
    ],
  });
  super::create_macos_native_nv12_cached_image_texture(device, &layout, &picture, &sampler, &globals, image)
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

// The colors of a frame: the matrix and the range a buffer says, and the shader's conversion of each.

/// R'G'B' of Y'CbCr by the standard's own weights of red and blue, `kr` and `kb`, with samples of 0 to 1 and
/// color about a half, in the full range or the video one.
fn standard_rgb(y: f32, cb: f32, cr: f32, kr: f32, kb: f32, full_range: bool) -> [f32; 3] {
  let (y, cb, cr) = if full_range {
    (y, cb - 0.5, cr - 0.5)
  } else {
    (
      (y - 16.0 / 255.0) * 255.0 / 219.0,
      (cb - 0.5) * 255.0 / 224.0,
      (cr - 0.5) * 255.0 / 224.0,
    )
  };
  let red = y + 2.0 * (1.0 - kr) * cr;
  let blue = y + 2.0 * (1.0 - kb) * cb;
  let green = (y - kr * red - kb * blue) / (1.0 - kr - kb);
  [red, green, blue]
}

/// What `yuv_to_rgb` gives for each case, `(y, cb, cr, matrix, full_range)`, drawn on the card one pixel a case.
fn converted(device: &wgpu::Device, queue: &wgpu::Queue, cases: &[[f32; 5]]) -> Vec<[f32; 3]> {
  let table = cases
    .iter()
    .map(|case| {
      format!(
        "array<f32, 5>({:?}, {:?}, {:?}, {:?}, {:?})",
        case[0], case[1], case[2], case[3], case[4]
      )
    })
    .collect::<Vec<_>>()
    .join(", ");
  let source = format!(
    "const CASES = array<array<f32, 5>, {count}>({table});
     @vertex fn vs(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {{
       var corners = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
       return vec4(corners[index], 0.0, 1.0);
     }}
     @fragment fn fs(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {{
       let sample = CASES[u32(at.x)];
       return vec4(yuv_to_rgb(sample[0], sample[1], sample[2], sample[3], sample[4]), 1.0);
     }}
     {yuv}",
    count = cases.len(),
    yuv = include_str!("shaders/yuv.wgsl"),
  );
  let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: Some("yuv"),
    source: wgpu::ShaderSource::Wgsl(source.into()),
  });
  let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: None,
    layout: None,
    vertex: wgpu::VertexState {
      module: &shader,
      entry_point: Some("vs"),
      buffers: &[],
      compilation_options: Default::default(),
    },
    fragment: Some(wgpu::FragmentState {
      module: &shader,
      entry_point: Some("fs"),
      targets: &[Some(wgpu::TextureFormat::Rgba32Float.into())],
      compilation_options: Default::default(),
    }),
    primitive: Default::default(),
    depth_stencil: None,
    multisample: Default::default(),
    multiview_mask: None,
    cache: None,
  });
  let width = cases.len() as u32;
  let target = device.create_texture(&wgpu::TextureDescriptor {
    label: None,
    size: wgpu::Extent3d {
      width,
      height: 1,
      depth_or_array_layers: 1,
    },
    mip_level_count: 1,
    sample_count: 1,
    dimension: wgpu::TextureDimension::D2,
    format: wgpu::TextureFormat::Rgba32Float,
    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    view_formats: &[],
  });
  let row = (width * 16).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
  let read = device.create_buffer(&wgpu::BufferDescriptor {
    label: None,
    size: u64::from(row),
    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
    mapped_at_creation: false,
  });
  let mut encoder = device.create_command_encoder(&Default::default());
  {
    let view = target.create_view(&Default::default());
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: None,
      color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        view: &view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
          load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
          store: wgpu::StoreOp::Store,
        },
      })],
      depth_stencil_attachment: None,
      timestamp_writes: None,
      occlusion_query_set: None,
      multiview_mask: None,
    });
    pass.set_pipeline(&pipeline);
    pass.draw(0..3, 0..1);
  }
  encoder.copy_texture_to_buffer(
    target.as_image_copy(),
    wgpu::TexelCopyBufferInfo {
      buffer: &read,
      layout: wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(row),
        rows_per_image: Some(1),
      },
    },
    wgpu::Extent3d {
      width,
      height: 1,
      depth_or_array_layers: 1,
    },
  );
  queue.submit([encoder.finish()]);
  read.slice(..).map_async(wgpu::MapMode::Read, |mapped| mapped.unwrap());
  device
    .poll(wgpu::PollType::wait_indefinitely())
    .expect("the card answers");
  let mapped = read.slice(..).get_mapped_range();
  let floats: &[f32] = bytemuck::cast_slice(&mapped[..(width * 16) as usize]);
  floats.chunks(4).map(|pixel| [pixel[0], pixel[1], pixel[2]]).collect()
}

#[test]
fn each_matrix_and_range_makes_the_colors_the_standard_does() {
  let Some((device, queue)) = device() else {
    eprintln!("skipped: no Metal device");
    return;
  };
  // (the shader's number, K_R, K_B): BT.709, BT.601, BT.2020.
  let matrices = [(0.0, 0.2126, 0.0722), (1.0, 0.299, 0.114), (2.0, 0.2627, 0.0593)];
  // A saturated orange, a dark blue, gray.
  let samples = [(0.55, 0.30, 0.75), (0.20, 0.70, 0.45), (0.50, 0.5, 0.5)];
  let mut cases = Vec::new();
  let mut expected = Vec::new();
  for (number, kr, kb) in matrices {
    for full_range in [false, true] {
      for (y, cb, cr) in samples {
        cases.push([y, cb, cr, number, if full_range { 1.0 } else { 0.0 }]);
        expected.push((number, full_range, standard_rgb(y, cb, cr, kr, kb, full_range)));
      }
    }
  }
  let drawn = converted(&device, &queue, &cases);
  for ((number, full_range, wanted), got) in expected.into_iter().zip(drawn) {
    let off = wanted.iter().zip(got).map(|(w, g)| (w - g).abs()).fold(0.0, f32::max);
    assert!(
      off < 0.002,
      "matrix {number}, full range {full_range}: {got:?}, the standard gives {wanted:?}"
    );
  }
}

#[test]
fn a_buffer_says_its_matrix_its_range_and_its_depth() {
  use core_foundation_sys::{base::kCFAllocatorDefault, string::CFStringRef};
  use core_video_sys::pixel_buffer::CVPixelBufferCreate;

  use super::video_color::{Matrix, VideoColor};

  #[link(name = "CoreVideo", kind = "framework")]
  unsafe extern "C" {
    static kCVImageBufferYCbCrMatrixKey: CFStringRef;
    static kCVImageBufferYCbCrMatrix_ITU_R_601_4: CFStringRef;
    static kCVImageBufferYCbCrMatrix_ITU_R_2020: CFStringRef;
    fn CVBufferSetAttachment(
      buffer: *mut std::ffi::c_void,
      key: CFStringRef,
      value: *const std::ffi::c_void,
      mode: u32,
    );
  }

  let buffer = |format: &[u8; 4], matrix: Option<CFStringRef>| unsafe {
    let mut buffer = std::ptr::null_mut();
    let status = CVPixelBufferCreate(
      kCFAllocatorDefault.cast(),
      16,
      16,
      u32::from_be_bytes(*format),
      std::ptr::null(),
      &mut buffer,
    );
    assert_eq!(status, 0, "{}", String::from_utf8_lossy(format));
    if let Some(matrix) = matrix {
      // 1 is kCVAttachmentMode_ShouldPropagate.
      CVBufferSetAttachment(buffer.cast(), kCVImageBufferYCbCrMatrixKey, matrix.cast(), 1);
    }
    // Held by the handle alone: the reference the creation gave is let go.
    let held = MacosCvPixelBuffer::retain(buffer.cast());
    core_foundation_sys::base::CFRelease(buffer.cast());
    held
  };
  let read = |buffer: &MacosCvPixelBuffer| unsafe { super::video_color::of(buffer.as_ptr()) };
  let color = |matrix, full_range, ten_bit| {
    Some(VideoColor {
      matrix,
      full_range,
      ten_bit,
    })
  };
  unsafe {
    assert_eq!(
      read(&buffer(b"420v", None)),
      color(Matrix::Bt709, false, false),
      "none said is BT.709"
    );
    assert_eq!(
      read(&buffer(b"420f", Some(kCVImageBufferYCbCrMatrix_ITU_R_601_4))),
      color(Matrix::Bt601, true, false)
    );
    assert_eq!(
      read(&buffer(b"x420", Some(kCVImageBufferYCbCrMatrix_ITU_R_2020))),
      color(Matrix::Bt2020, false, true)
    );
    assert_eq!(read(&buffer(b"xf20", None)), color(Matrix::Bt709, true, true));
    assert_eq!(read(&buffer(b"BGRA", None)), None, "not two planes");
  }
}

// A frame of red, green and blue, and a frame shown once the app's card is done with it.

/// The bytes of a pixel of a picture of blue, green, red and alpha: none the same as the next.
fn bgra_at(x: usize, y: usize) -> [u8; 4] {
  [
    ((x * 11 + y * 3) % 256) as u8,
    ((x * 5 + y * 17 + 40) % 256) as u8,
    ((x * 23 + y * 7 + 90) % 256) as u8,
    255,
  ]
}

/// Draws `view` into a target of sRGB its size, each pixel read as it is (`textureLoad`), and reads it back: a view
/// of sRGB gives light, which the target encodes again, so what is read is the bytes that were written.
fn read_back(device: &wgpu::Device, queue: &wgpu::Queue, view: &wgpu::TextureView, width: u32, height: u32) -> Vec<u8> {
  let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::Wgsl(
      "@group(0) @binding(0) var picture: texture_2d<f32>;
       @vertex fn vs(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
         var corners = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
         return vec4(corners[index], 0.0, 1.0);
       }
       @fragment fn fs(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
         return textureLoad(picture, vec2<i32>(at.xy), 0);
       }"
      .into(),
    ),
  });
  let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: None,
    layout: None,
    vertex: wgpu::VertexState {
      module: &shader,
      entry_point: Some("vs"),
      buffers: &[],
      compilation_options: Default::default(),
    },
    fragment: Some(wgpu::FragmentState {
      module: &shader,
      entry_point: Some("fs"),
      targets: &[Some(wgpu::TextureFormat::Rgba8UnormSrgb.into())],
      compilation_options: Default::default(),
    }),
    primitive: Default::default(),
    depth_stencil: None,
    multisample: Default::default(),
    multiview_mask: None,
    cache: None,
  });
  let bound = device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: None,
    layout: &pipeline.get_bind_group_layout(0),
    entries: &[wgpu::BindGroupEntry {
      binding: 0,
      resource: wgpu::BindingResource::TextureView(view),
    }],
  });
  let size = wgpu::Extent3d {
    width,
    height,
    depth_or_array_layers: 1,
  };
  let target = device.create_texture(&wgpu::TextureDescriptor {
    label: None,
    size,
    mip_level_count: 1,
    sample_count: 1,
    dimension: wgpu::TextureDimension::D2,
    format: wgpu::TextureFormat::Rgba8UnormSrgb,
    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    view_formats: &[],
  });
  let row = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
  let read = device.create_buffer(&wgpu::BufferDescriptor {
    label: None,
    size: u64::from(row * height),
    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
    mapped_at_creation: false,
  });
  let mut encoder = device.create_command_encoder(&Default::default());
  {
    let target_view = target.create_view(&Default::default());
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: None,
      color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        view: &target_view,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
          load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
          store: wgpu::StoreOp::Store,
        },
      })],
      depth_stencil_attachment: None,
      timestamp_writes: None,
      occlusion_query_set: None,
      multiview_mask: None,
    });
    pass.set_pipeline(&pipeline);
    pass.set_bind_group(0, &bound, &[]);
    pass.draw(0..3, 0..1);
  }
  encoder.copy_texture_to_buffer(
    target.as_image_copy(),
    wgpu::TexelCopyBufferInfo {
      buffer: &read,
      layout: wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(row),
        rows_per_image: Some(height),
      },
    },
    size,
  );
  queue.submit([encoder.finish()]);
  read.slice(..).map_async(wgpu::MapMode::Read, |mapped| mapped.unwrap());
  device
    .poll(wgpu::PollType::wait_indefinitely())
    .expect("the card answers");
  let mapped = read.slice(..).get_mapped_range();
  mapped
    .chunks(row as usize)
    .flat_map(|line| line[..(width * 4) as usize].to_vec())
    .collect()
}

#[test]
fn a_frame_of_red_green_and_blue_in_a_pixel_buffer_is_drawn_where_it_is_as_a_picture() {
  let Some((device, queue)) = device() else {
    eprintln!("skipped: no Metal device");
    return;
  };
  // 37 wide: an odd width, and rows the system pads.
  let (width, height) = (37_usize, 20_usize);
  let buffer = MacosCvPixelBuffer::bgra(width as u32, height as u32).expect("a pixel buffer");
  assert!(!buffer.write_nv12(|_, _, _, _| ()), "one plane, not two");
  let mut stride = 0;
  assert!(buffer.write_bgra(|rows, row| {
    stride = row;
    for y in 0..height {
      for x in 0..width {
        rows[y * row + x * 4..][..4].copy_from_slice(&bgra_at(x, y));
      }
    }
  }));
  assert!(stride > width * 4, "rows longer than the picture: {stride}");
  let frames = CvPixelBufferFrames::new();
  frames.show(buffer);
  let native = NativeImageData::from_cv_pixel_buffer_frames(width as u32, height as u32, frames);
  let super::CachedImageTexture::CvRgba { view, .. } =
    opened(&device, &image_of(native, width as u32, height as u32, 1))
  else {
    panic!("a frame of red, green and blue is a picture, of one view");
  };
  let pixels = read_back(&device, &queue, &view, width as u32, height as u32);
  let mut wrong = Vec::new();
  for y in 0..height {
    for x in 0..width {
      let [blue, green, red, alpha] = bgra_at(x, y);
      let read = &pixels[(y * width + x) * 4..][..4];
      let off = [red, green, blue, alpha]
        .iter()
        .zip(read)
        .map(|(w, r)| w.abs_diff(*r))
        .max()
        .unwrap_or(0);
      if off > 1 {
        wrong.push((x, y, read.to_vec()));
      }
    }
  }
  assert!(
    wrong.is_empty(),
    "{} pixels are not the buffer's, the first: {:?}",
    wrong.len(),
    wrong.first()
  );
}

#[test]
fn a_frame_the_apps_card_writes_is_shown_once_its_event_reaches_its_value() {
  use objc2_metal::{MTLDevice as _, MTLSharedEvent as _};

  let Some((device, _queue)) = device() else {
    eprintln!("skipped: no Metal device");
    return;
  };
  // SAFETY: the device is wgpu's, of Metal, and lives through the test.
  let event = unsafe {
    let hal = device.as_hal::<wgpu_hal::api::Metal>().expect("a device of Metal");
    hal.raw_device().newSharedEvent().expect("a shared event")
  };
  let frame = |label: u8| {
    let buffer = MacosCvPixelBuffer::bgra(2, 2).expect("a pixel buffer");
    assert!(buffer.write_bgra(|rows, _| rows[0] = label));
    buffer
  };
  let label = |frames: &CvPixelBufferFrames| {
    frames.shown().map(|buffer| {
      let mut first = 0;
      assert!(buffer.write_bgra(|rows, _| first = rows[0]));
      first
    })
  };

  let frames = CvPixelBufferFrames::new();
  frames.show(frame(1));
  assert_eq!((label(&frames), frames.count()), (Some(1), 1));

  frames.show_after(frame(2), &event, 1);
  frames.show_after(frame(3), &event, 2);
  assert_eq!(
    label(&frames),
    Some(1),
    "the card writes the next two: the frame before is drawn"
  );
  assert_eq!(frames.count(), 1, "and the image is the same");

  event.setSignaledValue(1);
  assert_eq!((label(&frames), frames.count()), (Some(2), 2), "the first is written");

  event.setSignaledValue(2);
  assert_eq!((label(&frames), frames.count()), (Some(3), 3));

  // Written in the order the card finishes: the later frame ready first is shown, and the one before it never.
  frames.show_after(frame(4), &event, 4);
  frames.show_after(frame(5), &event, 3);
  event.setSignaledValue(3);
  assert_eq!(label(&frames), Some(5));
  event.setSignaledValue(4);
  assert_eq!(label(&frames), Some(5), "the earlier frame came too late");
}
