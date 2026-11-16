// A frame that stays on the graphics card, end to end: a real clip is decoded on the card, copied there, opened
// on wgpu's own device by the renderer's code, drawn, and read back; what is read is the frame the same decoder
// gives through memory, sample for sample.

use std::time::{Duration, Instant};

use super::shared_nv12;
use crate::{
  images::{ImageData, NativeImageBackend, SharedNv12Frames},
  media::Media,
  resources::{Loader, Source},
};

fn clip() -> std::path::PathBuf {
  std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../gallery/assets/media/sample.mp4")
}

/// A device on Direct3D 12 with NV12 textures, as the engine asks for one; `None` where the card has none.
fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
  let instance = wgpu::Instance::new(super::instance_descriptor(wgpu::Backends::DX12));
  let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).ok()?;
  let found = pollster::block_on(adapter.request_device(&super::device_descriptor(
    &adapter,
    wgpu::Features::empty(),
    &wgpu::Limits::default(),
  )))
  .ok()?;
  found
    .0
    .features()
    .contains(wgpu::Features::TEXTURE_FORMAT_NV12)
    .then_some(found)
}

/// The clip, opened and silenced, paused at its start.
fn opened() -> Media {
  let media = Media::open(&Loader::new(), &Source::file(clip()));
  media.set_muted(true);
  media
}

/// The image `media` gives once its first frame is shown, in frames of the card (`on_card`) or of memory.
fn first_frame(media: &Media, on_card: bool) -> ImageData {
  let started = Instant::now();
  loop {
    assert!(
      started.elapsed() < Duration::from_secs(15),
      "no first frame: {:?}",
      media.state()
    );
    if let Some(image) = media.frame() {
      let native = image.native().map(|native| native.backend());
      if on_card && native == Some(NativeImageBackend::SharedNv12) {
        return image;
      }
      // Memory's image is there from the start, black: its first frame is a later version of it.
      if !on_card && native.is_none() && image.version() > 0 {
        return image;
      }
    }
    assert_eq!(media.state().error, None);
    std::thread::sleep(Duration::from_millis(10));
  }
}

/// The frame `image` shows, of the card, drawn by the renderer's own code and read back.
fn drawn_from(device: &wgpu::Device, queue: &wgpu::Queue, image: &ImageData) -> Vec<u8> {
  let frames = image
    .native()
    .and_then(|native| native.payload::<SharedNv12Frames>())
    .expect("the frames of the card")
    .clone();
  let shown = frames.handles()[frames.shown()];
  let texture = shared_nv12::open(device, shown, image.width(), image.height())
    .expect("the renderer's device opens the frame");
  drawn(device, queue, &texture, image.width(), image.height())
}

/// How many pixels of `pixels` (a sample of gray and two of color each) are not `memory`'s frame, in gray and
/// in color, and the most one is off by.
fn differing(pixels: &[u8], memory: &[u8], wide: usize, high: usize) -> (usize, usize, u8) {
  let (mut gray, mut color, mut worst) = (0_usize, 0_usize, 0_u8);
  for y in 0..high {
    for x in 0..wide {
      let pixel = &pixels[(y * wide + x) * 4..][..4];
      let from = memory[y * wide + x];
      let at = wide * high + (y / 2) * wide + (x / 2) * 2;
      let (u, v) = (memory[at], memory[at + 1]);
      if pixel[0] != from {
        gray += 1;
        worst = worst.max(pixel[0].abs_diff(from));
      }
      if pixel[1] != u || pixel[2] != v {
        color += 1;
        worst = worst.max(pixel[1].abs_diff(u)).max(pixel[2].abs_diff(v));
      }
    }
  }
  (gray, color, worst)
}

/// Draws the two planes of `texture` into a target of its size, a sample of gray and its two of color in each
/// pixel, and reads the target back.
fn drawn(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture, width: u32, height: u32) -> Vec<u8> {
  let (gray, color) = shared_nv12::planes(texture);
  let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: Some("planes"),
    source: wgpu::ShaderSource::Wgsl(
      "@group(0) @binding(0) var gray: texture_2d<f32>;
       @group(0) @binding(1) var color: texture_2d<f32>;
       @vertex fn vs(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
         var corners = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
         return vec4(corners[index], 0.0, 1.0);
       }
       @fragment fn fs(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
         let pixel = vec2<i32>(at.xy);
         return vec4(textureLoad(gray, pixel, 0).r, textureLoad(color, pixel / 2, 0).rg, 1.0);
       }"
        .into(),
    ),
  });
  let plane = |binding| wgpu::BindGroupLayoutEntry {
    binding,
    visibility: wgpu::ShaderStages::FRAGMENT,
    ty: wgpu::BindingType::Texture {
      sample_type: wgpu::TextureSampleType::Float { filterable: true },
      view_dimension: wgpu::TextureViewDimension::D2,
      multisampled: false,
    },
    count: None,
  };
  let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: None,
    entries: &[plane(0), plane(1)],
  });
  let bound = device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: None,
    layout: &layout,
    entries: &[
      wgpu::BindGroupEntry {
        binding: 0,
        resource: wgpu::BindingResource::TextureView(&gray),
      },
      wgpu::BindGroupEntry {
        binding: 1,
        resource: wgpu::BindingResource::TextureView(&color),
      },
    ],
  });
  let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: None,
    bind_group_layouts: &[Some(&layout)],
    immediate_size: 0,
  });
  let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: None,
    layout: Some(&pipeline_layout),
    vertex: wgpu::VertexState {
      module: &shader,
      entry_point: Some("vs"),
      buffers: &[],
      compilation_options: Default::default(),
    },
    fragment: Some(wgpu::FragmentState {
      module: &shader,
      entry_point: Some("fs"),
      targets: &[Some(wgpu::TextureFormat::Rgba8Unorm.into())],
      compilation_options: Default::default(),
    }),
    primitive: Default::default(),
    depth_stencil: None,
    multisample: Default::default(),
    multiview_mask: None,
    cache: None,
  });
  let target = device.create_texture(&wgpu::TextureDescriptor {
    label: None,
    size: wgpu::Extent3d {
      width,
      height,
      depth_or_array_layers: 1,
    },
    mip_level_count: 1,
    sample_count: 1,
    dimension: wgpu::TextureDimension::D2,
    format: wgpu::TextureFormat::Rgba8Unorm,
    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    view_formats: &[],
  });
  let view = target.create_view(&Default::default());
  let row = (width * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
  let read = device.create_buffer(&wgpu::BufferDescriptor {
    label: None,
    size: u64::from(row * height),
    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
    mapped_at_creation: false,
  });
  let mut encoder = device.create_command_encoder(&Default::default());
  {
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
    wgpu::Extent3d {
      width,
      height,
      depth_or_array_layers: 1,
    },
  );
  queue.submit([encoder.finish()]);
  read.slice(..).map_async(wgpu::MapMode::Read, |mapped| mapped.unwrap());
  device
    .poll(wgpu::PollType::wait_indefinitely())
    .expect("the card answers");
  let mapped = read.slice(..).get_mapped_range();
  let mut pixels = Vec::with_capacity((width * height * 4) as usize);
  for line in mapped.chunks(row as usize) {
    pixels.extend_from_slice(&line[..(width * 4) as usize]);
  }
  pixels
}

#[test]
fn a_frame_decoded_and_drawn_on_the_card_is_the_frame_memory_gives() {
  let Some((device, queue)) = device() else {
    eprintln!("skipped: this card has no Direct3D 12 device with NV12 textures");
    return;
  };
  // Nothing draws on a card yet: the frame goes through memory, as it always did.
  crate::images::set_shared_nv12_adapter(None);
  let early = opened();
  let in_memory = first_frame(&early, false);
  let (width, height) = (in_memory.width(), in_memory.height());
  assert_eq!((width, height), (640, 360));
  let memory = in_memory.data_arc();
  assert_eq!(memory.len(), (width * height * 3 / 2) as usize);
  let (wide, high) = (width as usize, height as usize);

  // The renderer says which card it draws on: the decoder decodes there, and its frames stay there.
  shared_nv12::announce(&device);
  assert!(
    crate::images::shared_nv12_adapter().is_some(),
    "a device with NV12 on Direct3D 12 says so"
  );
  let late = opened();
  let on_card = first_frame(&late, true);
  assert_eq!((on_card.width(), on_card.height()), (width, height));
  let pixels = drawn_from(&device, &queue, &on_card);
  // Every sample of gray, and the two of color of every two by two, are memory's.
  assert_eq!(
    differing(&pixels, &memory, wide, high),
    (0, 0, 0),
    "the pixels that differ in gray and in color of {}, and the most one is off by",
    wide * high
  );
  // The picture is a picture: the clip's first frame is not one flat color.
  let first = pixels[0];
  assert!(
    pixels.chunks(4).any(|pixel| pixel[0].abs_diff(first) > 32),
    "the frame has light and dark in it"
  );

  // The media that opened before anything drew on a card (a player on an app's first screen) starts over on
  // the card by itself, where it was, and its frame there is the same one.
  let moved = first_frame(&early, true);
  assert_eq!(
    differing(&drawn_from(&device, &queue, &moved), &memory, wide, high),
    (0, 0, 0),
    "the frame of the media that started over on the card"
  );
  let state = early.state();
  assert_eq!((state.error, state.playing, state.position), (None, false, 0.0));

  // Played, its frames go on coming on the card, each in another of its textures, and it is another picture.
  let frames = moved
    .native()
    .and_then(|native| native.payload::<SharedNv12Frames>())
    .expect("the frames of the card")
    .clone();
  let (before, slot) = (moved.version(), frames.shown());
  early.play();
  let started = Instant::now();
  let mut slots = std::collections::BTreeSet::from([slot]);
  while started.elapsed() < Duration::from_millis(600) {
    slots.insert(frames.shown());
    std::thread::sleep(Duration::from_millis(5));
  }
  early.pause();
  let shown = moved.version() - before;
  assert!((10..=22).contains(&shown), "about 18 frames of 30 a second in 0.6 s: {shown}");
  assert_eq!(slots.len(), frames.handles().len(), "every texture took its turn: {slots:?}");
  let later = drawn_from(&device, &queue, &moved);
  let (gray, ..) = differing(&later, &memory, wide, high);
  assert!(gray > wide * high / 100, "a frame half a second on is another picture: {gray} pixels differ");
  assert_eq!(early.state().error, None);
  crate::images::set_shared_nv12_adapter(None);
}
