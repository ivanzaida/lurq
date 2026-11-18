//! What the renderer's tests of a frame in two planes read back: each plane drawn as it is, with no colors made
//! of it, so that a frame drawn from where a decoder left it is compared byte for byte with the frame in memory.

/// Draws the planes `gray` and `color` (at half the size) into a target `width` by `height`, a sample of gray and
/// its two of color in each pixel, and reads the target back.
pub(super) fn drawn(
  device: &wgpu::Device,
  queue: &wgpu::Queue,
  gray: &wgpu::TextureView,
  color: &wgpu::TextureView,
  width: u32,
  height: u32,
) -> Vec<u8> {
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
        resource: wgpu::BindingResource::TextureView(gray),
      },
      wgpu::BindGroupEntry {
        binding: 1,
        resource: wgpu::BindingResource::TextureView(color),
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
