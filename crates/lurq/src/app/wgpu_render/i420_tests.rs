// A picture of memory of three planes (I420) is sent to three textures as it is laid out, and drawn by a shader
// made of the NV12 one: what the textures hold is each plane, byte for byte.

use std::sync::Arc;

use crate::{
  images::{ImageCmd, ImagePixelFormat},
  layout::quad::ClipRect,
};

/// The default device of this machine; `None` where there is none.
fn device() -> Option<(wgpu::Device, wgpu::Queue)> {
  let instance = wgpu::Instance::default();
  let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).ok()?;
  pollster::block_on(adapter.request_device(&Default::default())).ok()
}

fn image_of(data: Vec<u8>, width: u32, height: u32, version: u64) -> ImageCmd {
  ImageCmd {
    order: 0,
    x: 0.0,
    y: 0.0,
    width: width as f32,
    height: height as f32,
    image_id: 1,
    frame_index: 0,
    version,
    data: Arc::new(data),
    animation_frames: None,
    native: None,
    image_width: width,
    image_height: height,
    image_format: ImagePixelFormat::I420,
    uv_min: [0.0, 0.0],
    uv_max: [1.0, 1.0],
    radii: [0.0; 4],
    transform: [1.0, 0.0, 0.0, 1.0],
    transform_origin: [0.0; 2],
    clip: ClipRect::default(),
    opacity: 1.0,
  }
}

/// A frame whose three planes are each other bytes: one read from another plane, or another place, is seen.
fn frame(width: usize, height: usize, seed: usize) -> Vec<u8> {
  let gray = (0..width * height).map(|at| ((at * 7 + seed) % 251) as u8);
  let blue = (0..width * height / 4).map(|at| ((at * 13 + 3 + seed) % 241) as u8);
  let red = (0..width * height / 4).map(|at| ((at * 29 + 11 + seed) % 239) as u8);
  gray.chain(blue).chain(red).collect()
}

/// What the three textures hold, each pixel of the picture as (gray, blue-difference, red-difference).
fn held(
  device: &wgpu::Device,
  queue: &wgpu::Queue,
  views: [&wgpu::TextureView; 3],
  width: u32,
  height: u32,
) -> Vec<u8> {
  let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::Wgsl(
      "@group(0) @binding(0) var y: texture_2d<f32>;
       @group(0) @binding(1) var u: texture_2d<f32>;
       @group(0) @binding(2) var v: texture_2d<f32>;
       @vertex fn vs(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
         var corners = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
         return vec4(corners[index], 0.0, 1.0);
       }
       @fragment fn fs(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
         let pixel = vec2<i32>(at.xy);
         return vec4(textureLoad(y, pixel, 0).r, textureLoad(u, pixel / 2, 0).r, textureLoad(v, pixel / 2, 0).r, 1.0);
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
      targets: &[Some(wgpu::TextureFormat::Rgba8Unorm.into())],
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
    entries: &[0, 1, 2].map(|binding| wgpu::BindGroupEntry {
      binding,
      resource: wgpu::BindingResource::TextureView(views[binding as usize]),
    }),
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
    format: wgpu::TextureFormat::Rgba8Unorm,
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

/// The pixels of `held` that are not `frame`'s planes.
fn not_the_frames(held: &[u8], frame: &[u8], width: usize, height: usize) -> usize {
  let quarter = width * height / 4;
  let mut wrong = 0;
  for y in 0..height {
    for x in 0..width {
      let color = (y / 2) * (width / 2) + x / 2;
      let wanted = [
        frame[y * width + x],
        frame[width * height + color],
        frame[width * height + quarter + color],
      ];
      if held[(y * width + x) * 4..][..3] != wanted {
        wrong += 1;
      }
    }
  }
  wrong
}

#[test]
fn the_three_planes_of_a_picture_are_sent_to_the_card_as_they_are_and_its_shader_is_made() {
  let Some((device, queue)) = device() else {
    eprintln!("skipped: no device");
    return;
  };
  // The shader is the NV12 one with the color read from two textures, and it is valid.
  let source = super::i420_shader_source();
  assert!(source.contains("u_tex") && source.contains("v_tex") && !source.contains("uv_tex"));
  device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::Wgsl(source.into()),
  });

  let (width, height) = (38_u32, 22_u32);
  let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: None,
    entries: &[
      wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Buffer {
          ty: wgpu::BufferBindingType::Uniform,
          has_dynamic_offset: false,
          min_binding_size: None,
        },
        count: None,
      },
      plane(1),
      plane(2),
      plane(3),
      wgpu::BindGroupLayoutEntry {
        binding: 4,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
      },
    ],
  });
  let sampler = device.create_sampler(&Default::default());
  let globals = device.create_buffer(&wgpu::BufferDescriptor {
    label: None,
    size: 256,
    usage: wgpu::BufferUsages::UNIFORM,
    mapped_at_creation: false,
  });
  let first = frame(width as usize, height as usize, 0);
  let Some(super::CachedImageTexture::I420 {
    y_view,
    u_view,
    v_view,
    textures,
    ..
  }) = super::create_i420_cached_image_texture(
    &device,
    &queue,
    &layout,
    &sampler,
    &globals,
    &image_of(first.clone(), width, height, 1),
  )
  else {
    panic!("a picture of three planes is three textures");
  };
  let read = held(&device, &queue, [&y_view, &u_view, &v_view], width, height);
  assert_eq!(not_the_frames(&read, &first, width as usize, height as usize), 0);

  // The next frame goes into the same textures.
  let second = frame(width as usize, height as usize, 5);
  assert!(super::write_i420_image_textures(
    &queue,
    &textures,
    &image_of(second.clone(), width, height, 2)
  ));
  let read = held(&device, &queue, [&y_view, &u_view, &v_view], width, height);
  assert_eq!(not_the_frames(&read, &second, width as usize, height as usize), 0);
  assert_ne!(not_the_frames(&read, &first, width as usize, height as usize), 0);

  assert!(
    super::create_i420_cached_image_texture(
      &device,
      &queue,
      &layout,
      &sampler,
      &globals,
      &image_of(first, 37, 22, 1)
    )
    .is_none(),
    "an odd width has no plane of color of a whole size"
  );
}

fn plane(binding: u32) -> wgpu::BindGroupLayoutEntry {
  wgpu::BindGroupLayoutEntry {
    binding,
    visibility: wgpu::ShaderStages::FRAGMENT,
    ty: wgpu::BindingType::Texture {
      sample_type: wgpu::TextureSampleType::Float { filterable: true },
      view_dimension: wgpu::TextureViewDimension::D2,
      multisampled: false,
    },
    count: None,
  }
}
