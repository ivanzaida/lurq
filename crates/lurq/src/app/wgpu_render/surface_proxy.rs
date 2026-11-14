//! A texture a frame is painted into in place of the surface's, for a
//! surface whose texture cannot be copied from.
//!
//! A backdrop blur of the window ends the window's pass, copies what was
//! painted so far out of the frame and blurs the copy. Every surface but
//! WebGL's lets its texture be copied from. On WebGL a frame with such a blur
//! is painted into this texture, which does, and a last pass puts it onto the
//! surface pixel for pixel. A frame without one is painted straight onto the
//! surface, as everywhere else.

pub(super) struct SurfaceProxy {
  texture: wgpu::Texture,
  view: wgpu::TextureView,
  format: wgpu::TextureFormat,
  size: (u32, u32),
  pipeline: wgpu::RenderPipeline,
  bind_group: wgpu::BindGroup,
}

const SHADER: &str = "
@group(0) @binding(0) var frame: texture_2d<f32>;

// One triangle over the whole target.
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let x = f32(i32(index & 1u) * 4 - 1);
    let y = f32(i32(index >> 1u) * 4 - 1);
    return vec4<f32>(x, y, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    return textureLoad(frame, vec2<i32>(position.xy), 0);
}
";

impl SurfaceProxy {
  /// A texture of the surface's size, in the format the frame is rendered through.
  pub(super) fn new(device: &wgpu::Device, format: wgpu::TextureFormat, width: u32, height: u32) -> Self {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
      label: Some("lurq_surface_proxy"),
      size: wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format,
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT
        | wgpu::TextureUsages::COPY_SRC
        | wgpu::TextureUsages::TEXTURE_BINDING,
      view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: Some("lurq_surface_proxy_bgl"),
      entries: &[wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
          sample_type: wgpu::TextureSampleType::Float { filterable: false },
          view_dimension: wgpu::TextureViewDimension::D2,
          multisampled: false,
        },
        count: None,
      }],
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
      label: Some("lurq_surface_proxy_shader"),
      source: wgpu::ShaderSource::Wgsl(SHADER.into()),
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: Some("lurq_surface_proxy_pl"),
      bind_group_layouts: &[Some(&layout)],
      immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("lurq_surface_proxy_pipeline"),
      layout: Some(&pipeline_layout),
      vertex: wgpu::VertexState {
        module: &shader,
        entry_point: Some("vs_main"),
        buffers: &[],
        compilation_options: Default::default(),
      },
      fragment: Some(wgpu::FragmentState {
        module: &shader,
        entry_point: Some("fs_main"),
        targets: &[Some(wgpu::ColorTargetState {
          format,
          blend: None,
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
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some("lurq_surface_proxy_bg"),
      layout: &layout,
      entries: &[wgpu::BindGroupEntry {
        binding: 0,
        resource: wgpu::BindingResource::TextureView(&view),
      }],
    });
    Self {
      texture,
      view,
      format,
      size: (width, height),
      pipeline,
      bind_group,
    }
  }

  /// Whether this is the texture of a surface of that format and size.
  pub(super) fn fits(&self, format: wgpu::TextureFormat, width: u32, height: u32) -> bool {
    self.format == format && self.size == (width, height)
  }

  pub(super) fn texture(&self) -> &wgpu::Texture {
    &self.texture
  }

  pub(super) fn view(&self) -> &wgpu::TextureView {
    &self.view
  }

  /// The frame's last pass: what was painted into the texture, onto the surface.
  pub(super) fn put_onto(&self, encoder: &mut wgpu::CommandEncoder, surface: &wgpu::TextureView) {
    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
      label: Some("lurq_surface_proxy_pass"),
      color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        view: surface,
        depth_slice: None,
        resolve_target: None,
        ops: wgpu::Operations {
          load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
          store: wgpu::StoreOp::Store,
        },
      })],
      ..Default::default()
    });
    pass.set_pipeline(&self.pipeline);
    pass.set_bind_group(0, &self.bind_group, &[]);
    pass.draw(0..3, 0..1);
  }
}
