mod vertex;

use raw_window_handle::{DisplayHandle, WindowHandle};
use vertex::{Globals, GlyphInstance, QuadInstance, QuadVertex};
use wgpu::util::DeviceExt;

use crate::{app::render_engine::RenderEngine, layout::render_list::RenderList};

pub struct WgpuRenderEngine {
  instance: wgpu::Instance,
  adapter: Option<wgpu::Adapter>,
  device: Option<wgpu::Device>,
  queue: Option<wgpu::Queue>,
  quad_pipeline: Option<wgpu::RenderPipeline>,
  glyph_pipeline: Option<wgpu::RenderPipeline>,
  surface: Option<wgpu::Surface<'static>>,
  surface_config: Option<wgpu::SurfaceConfiguration>,
  quad_bgl: Option<wgpu::BindGroupLayout>,
  glyph_bgl: Option<wgpu::BindGroupLayout>,
  vertex_buffer: Option<wgpu::Buffer>,
  index_buffer: Option<wgpu::Buffer>,
}

impl Default for WgpuRenderEngine {
  fn default() -> Self {
    WgpuRenderEngine::new()
  }
}

impl WgpuRenderEngine {
  pub fn new() -> Self {
    Self {
      instance: wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
      }),
      adapter: None,
      device: None,
      queue: None,
      quad_pipeline: None,
      glyph_pipeline: None,
      surface: None,
      surface_config: None,
      quad_bgl: None,
      glyph_bgl: None,
      vertex_buffer: None,
      index_buffer: None,
    }
  }

  fn ensure_initialized(&mut self, window: WindowHandle<'_>, display: DisplayHandle<'_>) {
    if self.device.is_some() {
      return;
    }

    let surface_target =
      unsafe { wgpu::SurfaceTargetUnsafe::from_window(&WindowDisplayPair { window, display }) }.unwrap();
    let surface = unsafe { self.instance.create_surface_unsafe(surface_target) }.unwrap();

    let adapter = pollster::block_on(self.instance.request_adapter(&wgpu::RequestAdapterOptions {
      power_preference: wgpu::PowerPreference::default(),
      compatible_surface: Some(&surface),
      force_fallback_adapter: false,
    }))
    .expect("no suitable GPU adapter found");

    let (device, queue) = pollster::block_on(adapter.request_device(
      &wgpu::DeviceDescriptor {
        label: Some("lurq"),
        ..Default::default()
      },
      None,
    ))
    .expect("failed to create device");

    let caps = surface.get_capabilities(&adapter);
    let format = caps.formats[0];

    let config = wgpu::SurfaceConfiguration {
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      format,
      width: 800,
      height: 600,
      present_mode: wgpu::PresentMode::Fifo,
      alpha_mode: caps.alpha_modes[0],
      view_formats: vec![],
      desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    // --- Quad pipeline ---
    let quad_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: Some("lurq_quad_bgl"),
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
        wgpu::BindGroupLayoutEntry {
          binding: 1,
          visibility: wgpu::ShaderStages::FRAGMENT,
          ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
          },
          count: None,
        },
      ],
    });

    let quad_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
      label: Some("lurq_quad_shader"),
      source: wgpu::ShaderSource::Wgsl(include_str!("shaders/quad.wgsl").into()),
    });

    let quad_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: Some("lurq_quad_pl"),
      bind_group_layouts: &[&quad_bgl],
      push_constant_ranges: &[],
    });

    let quad_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("lurq_quad_pipeline"),
      layout: Some(&quad_pipeline_layout),
      vertex: wgpu::VertexState {
        module: &quad_shader,
        entry_point: Some("vs_main"),
        buffers: &[QuadVertex::desc(), QuadInstance::desc()],
        compilation_options: Default::default(),
      },
      fragment: Some(wgpu::FragmentState {
        module: &quad_shader,
        entry_point: Some("fs_main"),
        targets: &[Some(wgpu::ColorTargetState {
          format,
          blend: Some(wgpu::BlendState::ALPHA_BLENDING),
          write_mask: wgpu::ColorWrites::ALL,
        })],
        compilation_options: Default::default(),
      }),
      primitive: wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        ..Default::default()
      },
      depth_stencil: None,
      multisample: wgpu::MultisampleState::default(),
      multiview: None,
      cache: None,
    });

    // --- Glyph pipeline ---
    let glyph_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
      label: Some("lurq_glyph_bgl"),
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
        wgpu::BindGroupLayoutEntry {
          binding: 1,
          visibility: wgpu::ShaderStages::FRAGMENT,
          ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
          },
          count: None,
        },
        wgpu::BindGroupLayoutEntry {
          binding: 2,
          visibility: wgpu::ShaderStages::FRAGMENT,
          ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
          count: None,
        },
      ],
    });

    let glyph_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
      label: Some("lurq_glyph_shader"),
      source: wgpu::ShaderSource::Wgsl(include_str!("shaders/glyph.wgsl").into()),
    });

    let glyph_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: Some("lurq_glyph_pl"),
      bind_group_layouts: &[&glyph_bgl],
      push_constant_ranges: &[],
    });

    let glyph_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
      label: Some("lurq_glyph_pipeline"),
      layout: Some(&glyph_pipeline_layout),
      vertex: wgpu::VertexState {
        module: &glyph_shader,
        entry_point: Some("vs_main"),
        buffers: &[QuadVertex::desc(), GlyphInstance::desc()],
        compilation_options: Default::default(),
      },
      fragment: Some(wgpu::FragmentState {
        module: &glyph_shader,
        entry_point: Some("fs_main"),
        targets: &[Some(wgpu::ColorTargetState {
          format,
          blend: Some(wgpu::BlendState::ALPHA_BLENDING),
          write_mask: wgpu::ColorWrites::ALL,
        })],
        compilation_options: Default::default(),
      }),
      primitive: wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        ..Default::default()
      },
      depth_stencil: None,
      multisample: wgpu::MultisampleState::default(),
      multiview: None,
      cache: None,
    });

    // --- Shared vertex/index buffers ---
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("lurq_quad_verts"),
      contents: bytemuck::cast_slice(&QuadVertex::CORNERS),
      usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("lurq_quad_idx"),
      contents: bytemuck::cast_slice(&QuadVertex::INDICES),
      usage: wgpu::BufferUsages::INDEX,
    });

    self.adapter = Some(adapter);
    self.device = Some(device);
    self.queue = Some(queue);
    self.quad_pipeline = Some(quad_pipeline);
    self.glyph_pipeline = Some(glyph_pipeline);
    self.surface = Some(surface);
    self.surface_config = Some(config);
    self.quad_bgl = Some(quad_bgl);
    self.glyph_bgl = Some(glyph_bgl);
    self.vertex_buffer = Some(vertex_buffer);
    self.index_buffer = Some(index_buffer);
  }
}

impl RenderEngine for WgpuRenderEngine {
  fn resize(&mut self, width: u32, height: u32) {
    if let (Some(config), Some(device), Some(surface)) = (&mut self.surface_config, &self.device, &self.surface) {
      config.width = width.max(1);
      config.height = height.max(1);
      surface.configure(device, config);
    }
  }

  fn render(&mut self, list: &RenderList, window: WindowHandle<'_>, display: DisplayHandle<'_>) {
    self.ensure_initialized(window, display);

    let device = self.device.as_ref().unwrap();
    let queue = self.queue.as_ref().unwrap();
    let surface = self.surface.as_ref().unwrap();
    let config = self.surface_config.as_ref().unwrap();
    let vtx_buf = self.vertex_buffer.as_ref().unwrap();
    let idx_buf = self.index_buffer.as_ref().unwrap();

    let output = match surface.get_current_texture() {
      Ok(t) => t,
      Err(_) => return,
    };
    let view = output.texture.create_view(&Default::default());

    let vw = config.width as f32;
    let vh = config.height as f32;

    let globals = Globals {
      viewport: [vw, vh, 0.0, 0.0],
      clip_rect: [0.0, 0.0, vw, vh],
      clip_radii_h: [0.0; 4],
      clip_radii_v: [0.0; 4],
      clip_active: [0.0; 4],
    };
    let globals_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("lurq_globals"),
      contents: bytemuck::bytes_of(&globals),
      usage: wgpu::BufferUsages::UNIFORM,
    });

    // --- Build quad batches grouped by clip ---
    struct QuadBatch {
      instances: Vec<QuadInstance>,
      clip: crate::layout::quad::ClipRect,
    }
    let mut quad_batches: Vec<QuadBatch> = Vec::new();

    for r in &list.rects {
      let batch = quad_batches.iter_mut().find(|b| {
        b.clip.active == r.clip.active
          && b.clip.x == r.clip.x
          && b.clip.y == r.clip.y
          && b.clip.width == r.clip.width
          && b.clip.height == r.clip.height
      });
      let batch = match batch {
        Some(b) => b,
        None => {
          quad_batches.push(QuadBatch {
            instances: Vec::new(),
            clip: r.clip,
          });
          quad_batches.last_mut().unwrap()
        }
      };

      batch.instances.push(QuadInstance {
        pos: [r.x, r.y],
        size: [r.width, r.height],
        color: r.color.to_f32_array(),
        radii_h: r.radii,
        radii_v: r.radii,
        stroke: [0.0; 4],
        pattern: [0.0; 4],
        transform: [1.0, 0.0, 0.0, 1.0],
        xf_origin: [0.0, 0.0],
        shadow_sigma: 0.0,
        gradient_offset: -1.0,
      });

      let has_stroke = r.stroke.iter().any(|s| *s > 0.0);
      if has_stroke {
        batch.instances.push(QuadInstance {
          pos: [r.x, r.y],
          size: [r.width, r.height],
          color: r.stroke_color.to_f32_array(),
          radii_h: r.radii,
          radii_v: r.radii,
          stroke: r.stroke,
          pattern: [0.0; 4],
          transform: [1.0, 0.0, 0.0, 1.0],
          xf_origin: [0.0, 0.0],
          shadow_sigma: 0.0,
          gradient_offset: -1.0,
        });
      }
    }

    // --- Build glyph batches grouped by clip ---
    struct GlyphBatch {
      instances: Vec<GlyphInstance>,
      clip: crate::layout::quad::ClipRect,
    }
    let mut glyph_batches: Vec<GlyphBatch> = Vec::new();

    for g in &list.glyphs {
      let batch = glyph_batches.iter_mut().find(|b| {
        b.clip.active == g.clip.active
          && b.clip.x == g.clip.x
          && b.clip.y == g.clip.y
          && b.clip.width == g.clip.width
          && b.clip.height == g.clip.height
      });
      let batch = match batch {
        Some(b) => b,
        None => {
          glyph_batches.push(GlyphBatch {
            instances: Vec::new(),
            clip: g.clip,
          });
          glyph_batches.last_mut().unwrap()
        }
      };

      batch.instances.push(GlyphInstance {
        pos: [g.x, g.y],
        size: [g.width, g.height],
        color: g.color,
        uv_min: g.uv_min,
        uv_max: g.uv_max,
        transform: [1.0, 0.0, 0.0, 1.0],
        xf_origin: [0.0, 0.0],
      });
    }

    let gradient_data: [f32; 4] = [0.0; 4];
    let gradient_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("lurq_gradients"),
      contents: bytemuck::cast_slice(&gradient_data),
      usage: wgpu::BufferUsages::STORAGE,
    });

    // Atlas setup
    let atlas = &list.atlas;
    let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
      label: Some("lurq_atlas"),
      size: wgpu::Extent3d {
        width: atlas.width,
        height: atlas.height,
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: wgpu::TextureFormat::R8Unorm,
      usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
      view_formats: &[],
    });
    queue.write_texture(
      wgpu::TexelCopyTextureInfo {
        texture: &atlas_texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
        aspect: wgpu::TextureAspect::All,
      },
      &atlas.data,
      wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(atlas.width),
        rows_per_image: Some(atlas.height),
      },
      wgpu::Extent3d {
        width: atlas.width,
        height: atlas.height,
        depth_or_array_layers: 1,
      },
    );
    let atlas_view = atlas_texture.create_view(&Default::default());
    let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
      mag_filter: wgpu::FilterMode::Linear,
      min_filter: wgpu::FilterMode::Linear,
      ..Default::default()
    });

    let mut encoder = device.create_command_encoder(&Default::default());

    // --- Single render pass with scissor-based clipping ---
    {
      let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("lurq_pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
          view: &view,
          resolve_target: None,
          ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
            store: wgpu::StoreOp::Store,
          },
        })],
        ..Default::default()
      });

      // Quad batches
      let quad_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("lurq_quad_bg"),
        layout: self.quad_bgl.as_ref().unwrap(),
        entries: &[
          wgpu::BindGroupEntry {
            binding: 0,
            resource: globals_buffer.as_entire_binding(),
          },
          wgpu::BindGroupEntry {
            binding: 1,
            resource: gradient_buffer.as_entire_binding(),
          },
        ],
      });

      pass.set_pipeline(self.quad_pipeline.as_ref().unwrap());
      pass.set_bind_group(0, &quad_bg, &[]);
      pass.set_vertex_buffer(0, vtx_buf.slice(..));
      pass.set_index_buffer(idx_buf.slice(..), wgpu::IndexFormat::Uint16);

      for batch in &quad_batches {
        if batch.instances.is_empty() {
          continue;
        }

        if batch.clip.active {
          let cx = batch.clip.x.max(0.0) as u32;
          let cy = batch.clip.y.max(0.0) as u32;
          let cw = (batch.clip.width as u32).min(vw as u32 - cx);
          let ch = (batch.clip.height as u32).min(vh as u32 - cy);
          pass.set_scissor_rect(cx, cy, cw.max(1), ch.max(1));
        } else {
          pass.set_scissor_rect(0, 0, vw as u32, vh as u32);
        }

        let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
          label: Some("lurq_qi"),
          contents: bytemuck::cast_slice(&batch.instances),
          usage: wgpu::BufferUsages::VERTEX,
        });
        pass.set_vertex_buffer(1, buf.slice(..));
        pass.draw_indexed(0..6, 0, 0..batch.instances.len() as u32);
      }

      // Glyph batches
      let glyph_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("lurq_glyph_bg"),
        layout: self.glyph_bgl.as_ref().unwrap(),
        entries: &[
          wgpu::BindGroupEntry {
            binding: 0,
            resource: globals_buffer.as_entire_binding(),
          },
          wgpu::BindGroupEntry {
            binding: 1,
            resource: wgpu::BindingResource::TextureView(&atlas_view),
          },
          wgpu::BindGroupEntry {
            binding: 2,
            resource: wgpu::BindingResource::Sampler(&atlas_sampler),
          },
        ],
      });

      pass.set_pipeline(self.glyph_pipeline.as_ref().unwrap());
      pass.set_bind_group(0, &glyph_bg, &[]);
      pass.set_vertex_buffer(0, vtx_buf.slice(..));
      pass.set_index_buffer(idx_buf.slice(..), wgpu::IndexFormat::Uint16);

      for batch in &glyph_batches {
        if batch.instances.is_empty() {
          continue;
        }

        if batch.clip.active {
          let cx = batch.clip.x.max(0.0) as u32;
          let cy = batch.clip.y.max(0.0) as u32;
          let cw = (batch.clip.width as u32).min(vw as u32 - cx);
          let ch = (batch.clip.height as u32).min(vh as u32 - cy);
          pass.set_scissor_rect(cx, cy, cw.max(1), ch.max(1));
        } else {
          pass.set_scissor_rect(0, 0, vw as u32, vh as u32);
        }

        let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
          label: Some("lurq_gi"),
          contents: bytemuck::cast_slice(&batch.instances),
          usage: wgpu::BufferUsages::VERTEX,
        });
        pass.set_vertex_buffer(1, buf.slice(..));
        pass.draw_indexed(0..6, 0, 0..batch.instances.len() as u32);
      }
    }

    queue.submit(std::iter::once(encoder.finish()));
    output.present();
  }
}

struct WindowDisplayPair<'a> {
  window: WindowHandle<'a>,
  display: DisplayHandle<'a>,
}

impl raw_window_handle::HasWindowHandle for WindowDisplayPair<'_> {
  fn window_handle(&self) -> Result<WindowHandle<'_>, raw_window_handle::HandleError> {
    Ok(self.window)
  }
}

impl raw_window_handle::HasDisplayHandle for WindowDisplayPair<'_> {
  fn display_handle(&self) -> Result<DisplayHandle<'_>, raw_window_handle::HandleError> {
    Ok(self.display)
  }
}
