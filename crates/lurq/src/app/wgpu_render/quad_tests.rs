// Pixel coverage of the real quad shader, read back from an offscreen target.
// A rect aligned to pixel edges must paint exactly its own pixels: a window-sized
// root rect otherwise leaves a faint line of clear colour on every window edge.

use wgpu::util::DeviceExt;

use super::vertex::{Globals, QuadInstance, QuadVertex};
use crate::node::color::Color;

const WIDTH: u32 = 64;
const HEIGHT: u32 = 48;
/// The production target: an sRGB surface rendered through its non-sRGB view.
fn target_format() -> wgpu::TextureFormat {
  super::render_target_format(wgpu::TextureFormat::Rgba8UnormSrgb)
}

/// One device for every test in this module: creating and dropping devices on
/// parallel test threads crashes some drivers.
fn device() -> &'static (wgpu::Device, wgpu::Queue) {
  static DEVICE: std::sync::OnceLock<(wgpu::Device, wgpu::Queue)> = std::sync::OnceLock::new();
  DEVICE.get_or_init(|| {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
      .expect("GPU adapter required for quad coverage tests");
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
      label: Some("quad coverage tests"),
      ..Default::default()
    }))
    .unwrap()
  })
}

fn fill(x: f32, y: f32, width: f32, height: f32, color: Color, radius: f32) -> QuadInstance {
  QuadInstance {
    pos: [x, y],
    size: [width, height],
    color: color.to_linear_f32_array(),
    radii_h: [radius; 4],
    radii_v: [radius; 4],
    stroke: [0.0; 4],
    pattern: [0.0; 4],
    transform: [1.0, 0.0, 0.0, 1.0],
    xf_origin: [0.0; 2],
    shadow_sigma: 0.0,
    gradient_offset: -1.0,
  }
}

/// Draws `quads` over a `clear` background with the production quad shader and
/// target format and returns tight RGBA8 (sRGB-encoded) pixels.
fn render(clear: Color, quads: &[QuadInstance]) -> Vec<u8> {
  let (device, queue) = device();
  let uniform = wgpu::BindingType::Buffer {
    ty: wgpu::BufferBindingType::Uniform,
    has_dynamic_offset: false,
    min_binding_size: None,
  };
  let storage = wgpu::BindingType::Buffer {
    ty: wgpu::BufferBindingType::Storage { read_only: true },
    has_dynamic_offset: false,
    min_binding_size: None,
  };
  let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
    label: None,
    entries: &[
      wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: uniform,
        count: None,
      },
      wgpu::BindGroupLayoutEntry {
        binding: 1,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: storage,
        count: None,
      },
    ],
  });
  let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: None,
    source: wgpu::ShaderSource::Wgsl(include_str!("shaders/quad.wgsl").into()),
  });
  let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
    label: None,
    bind_group_layouts: &[Some(&bgl)],
    immediate_size: 0,
  });
  let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
    label: None,
    layout: Some(&layout),
    vertex: wgpu::VertexState {
      module: &shader,
      entry_point: Some("vs_main"),
      buffers: &[QuadVertex::desc(), QuadInstance::desc()],
      compilation_options: Default::default(),
    },
    fragment: Some(wgpu::FragmentState {
      module: &shader,
      entry_point: Some("fs_main"),
      targets: &[Some(wgpu::ColorTargetState {
        format: target_format(),
        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
        write_mask: wgpu::ColorWrites::ALL,
      })],
      compilation_options: Default::default(),
    }),
    primitive: wgpu::PrimitiveState::default(),
    depth_stencil: None,
    multisample: wgpu::MultisampleState::default(),
    multiview_mask: None,
    cache: None,
  });

  let (width, height) = (WIDTH as f32, HEIGHT as f32);
  let globals = Globals {
    viewport: [width, height, 0.0, 0.0],
    clip_rect: [0.0, 0.0, width, height],
    clip_radii_h: [0.0; 4],
    clip_radii_v: [0.0; 4],
    clip_active: [0.0; 4],
  };
  let buffer = |contents: &[u8], usage| {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: None,
      contents,
      usage,
    })
  };
  let globals = buffer(bytemuck::bytes_of(&globals), wgpu::BufferUsages::UNIFORM);
  let gradients = buffer(&[0; 16], wgpu::BufferUsages::STORAGE);
  let corners = buffer(bytemuck::cast_slice(&QuadVertex::CORNERS), wgpu::BufferUsages::VERTEX);
  let indices = buffer(bytemuck::cast_slice(&QuadVertex::INDICES), wgpu::BufferUsages::INDEX);
  let instances = buffer(bytemuck::cast_slice(quads), wgpu::BufferUsages::VERTEX);
  let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: None,
    layout: &bgl,
    entries: &[
      wgpu::BindGroupEntry {
        binding: 0,
        resource: globals.as_entire_binding(),
      },
      wgpu::BindGroupEntry {
        binding: 1,
        resource: gradients.as_entire_binding(),
      },
    ],
  });

  let target = device.create_texture(&wgpu::TextureDescriptor {
    label: None,
    size: wgpu::Extent3d {
      width: WIDTH,
      height: HEIGHT,
      depth_or_array_layers: 1,
    },
    mip_level_count: 1,
    sample_count: 1,
    dimension: wgpu::TextureDimension::D2,
    format: target_format(),
    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    view_formats: &[],
  });
  let view = target.create_view(&Default::default());
  let padded_row = (WIDTH * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
  let readback = device.create_buffer(&wgpu::BufferDescriptor {
    label: None,
    size: (padded_row * HEIGHT) as u64,
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
          load: wgpu::LoadOp::Clear(super::wgpu_clear_color(clear)),
          store: wgpu::StoreOp::Store,
        },
      })],
      depth_stencil_attachment: None,
      timestamp_writes: None,
      occlusion_query_set: None,
      multiview_mask: None,
    });
    pass.set_pipeline(&pipeline);
    pass.set_bind_group(0, &bind_group, &[]);
    pass.set_vertex_buffer(0, corners.slice(..));
    pass.set_vertex_buffer(1, instances.slice(..));
    pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint16);
    pass.draw_indexed(0..QuadVertex::INDICES.len() as u32, 0, 0..quads.len() as u32);
  }
  encoder.copy_texture_to_buffer(
    target.as_image_copy(),
    wgpu::TexelCopyBufferInfo {
      buffer: &readback,
      layout: wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(padded_row),
        rows_per_image: Some(HEIGHT),
      },
    },
    wgpu::Extent3d {
      width: WIDTH,
      height: HEIGHT,
      depth_or_array_layers: 1,
    },
  );
  queue.submit([encoder.finish()]);
  readback
    .slice(..)
    .map_async(wgpu::MapMode::Read, |result| result.unwrap());
  device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
  let data = readback.slice(..).get_mapped_range().to_vec();
  data
    .chunks_exact(padded_row as usize)
    .flat_map(|row| &row[..(WIDTH * 4) as usize])
    .copied()
    .collect()
}

fn pixel(pixels: &[u8], x: u32, y: u32) -> [u8; 3] {
  let index = ((y * WIDTH + x) * 4) as usize;
  [pixels[index], pixels[index + 1], pixels[index + 2]]
}

fn rgb(color: Color) -> [u8; 3] {
  [color.r(), color.g(), color.b()]
}

#[test]
#[ignore = "requires a GPU adapter; run explicitly"]
fn window_sized_rect_covers_every_edge_pixel() {
  let clear = Color::from_hex("#ffffff");
  let background = Color::from_hex("#101010");
  let pixels = render(clear, &[fill(0.0, 0.0, WIDTH as f32, HEIGHT as f32, background, 0.0)]);

  for x in 0..WIDTH {
    assert_eq!(pixel(&pixels, x, 0), rgb(background), "top edge x={x}");
    assert_eq!(pixel(&pixels, x, HEIGHT - 1), rgb(background), "bottom edge x={x}");
  }
  for y in 0..HEIGHT {
    assert_eq!(pixel(&pixels, 0, y), rgb(background), "left edge y={y}");
    assert_eq!(pixel(&pixels, WIDTH - 1, y), rgb(background), "right edge y={y}");
  }
}

#[test]
#[ignore = "requires a GPU adapter; run explicitly"]
fn pixel_aligned_rect_does_not_bleed_and_rounded_corners_stay_smooth() {
  let clear = Color::from_hex("#000000");
  let white = Color::from_hex("#ffffff");
  let pixels = render(
    clear,
    &[
      fill(8.0, 8.0, 16.0, 16.0, white, 0.0),
      fill(32.0, 8.0, 24.0, 24.0, white, 8.0),
    ],
  );

  // Square: exact inside, exact outside, on every side.
  for (inside, outside) in [
    ((8, 16), (7, 16)),
    ((23, 16), (24, 16)),
    ((16, 8), (16, 7)),
    ((16, 23), (16, 24)),
  ] {
    assert_eq!(pixel(&pixels, inside.0, inside.1), rgb(white), "inside {inside:?}");
    assert_eq!(pixel(&pixels, outside.0, outside.1), rgb(clear), "outside {outside:?}");
  }

  // Rounded corner (arc centre (40, 16), radius 8): the pixel the arc crosses
  // stays partially covered.
  let corner = pixel(&pixels, 34, 10)[0];
  assert!(corner > 0 && corner < 255, "anti-aliased arc pixel, got {corner}");
  assert_eq!(pixel(&pixels, 32, 32), rgb(clear), "outside the arc");
  assert_eq!(pixel(&pixels, 44, 20), rgb(white), "interior");
}

/// CSS source-over of a straight-alpha `source` onto an opaque `backdrop`,
/// computed on the sRGB-encoded channels as browsers and design tools do.
fn css_source_over(backdrop: Color, source: Color) -> [u8; 3] {
  let alpha = f32::from(source.a()) / 255.0;
  let channel = |back: u8, front: u8| (f32::from(front) * alpha + f32::from(back) * (1.0 - alpha)).round() as u8;
  [
    channel(backdrop.r(), source.r()),
    channel(backdrop.g(), source.g()),
    channel(backdrop.b(), source.b()),
  ]
}

fn assert_close(actual: [u8; 3], expected: [u8; 3], what: &str) {
  let close = actual.iter().zip(expected).all(|(a, e)| a.abs_diff(e) <= 1);
  assert!(close, "{what}: got {actual:?}, expected {expected:?}");
}

#[test]
#[ignore = "requires a GPU adapter; run explicitly"]
fn translucent_quads_blend_like_css() {
  let cases = [
    // A modal scrim over light text: #EEEEEE under #000000A6 is #535353.
    (Color::from_hex("#eeeeee"), Color::from_hex("#000000a6")),
    (Color::from_hex("#000000"), Color::from_hex("#ffffff80")),
    (Color::from_hex("#ffffff"), Color::from_hex("#3366cc80")),
    (Color::from_hex("#202830"), Color::from_hex("#f0a0404d")),
  ];
  for (backdrop, source) in cases {
    let pixels = render(backdrop, &[fill(0.0, 0.0, WIDTH as f32, HEIGHT as f32, source, 0.0)]);
    let expected = css_source_over(backdrop, source);
    assert_close(
      pixel(&pixels, WIDTH / 2, HEIGHT / 2),
      expected,
      &format!("{source:?} over {backdrop:?}"),
    );
  }
  assert_eq!(
    css_source_over(Color::from_hex("#eeeeee"), Color::from_hex("#000000a6")),
    [0x53; 3]
  );
}

#[test]
#[ignore = "requires a GPU adapter; run explicitly"]
fn opaque_quads_and_the_clear_keep_their_exact_colour() {
  let clear = Color::from_hex("#8a8f96");
  let fill_color = Color::from_hex("#1e5b3c");
  let pixels = render(clear, &[fill(0.0, 0.0, 16.0, 16.0, fill_color, 0.0)]);
  assert_eq!(pixel(&pixels, 8, 8), rgb(fill_color));
  assert_eq!(pixel(&pixels, 40, 30), rgb(clear));
}
