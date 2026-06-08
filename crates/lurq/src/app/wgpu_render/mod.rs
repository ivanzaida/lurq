mod vertex;

use raw_window_handle::{DisplayHandle, WindowHandle};
use vertex::{Globals, GlyphInstance, QuadInstance, QuadVertex};
use wgpu::util::DeviceExt;

use crate::{
  app::{
    profiler::{ProfileScope, RenderProfile},
    render_engine::RenderEngine,
  },
  layout::render_list::RenderList,
};

#[cfg(feature = "image")]
struct CachedImageTexture {
  bind_group: wgpu::BindGroup,
  view: wgpu::TextureView,
  texture: wgpu::Texture,
  frame_index: usize,
  version: u64,
}

pub struct WgpuRenderEngine {
  instance: wgpu::Instance,
  adapter: Option<wgpu::Adapter>,
  device: Option<wgpu::Device>,
  queue: Option<wgpu::Queue>,
  quad_pipeline: Option<wgpu::RenderPipeline>,
  glyph_pipeline: Option<wgpu::RenderPipeline>,
  #[cfg(feature = "image")]
  image_pipeline: Option<wgpu::RenderPipeline>,
  #[cfg(feature = "svg")]
  svg_pipeline: Option<wgpu::RenderPipeline>,
  #[cfg(feature = "svg")]
  svg_bgl: Option<wgpu::BindGroupLayout>,
  surface: Option<wgpu::Surface<'static>>,
  surface_config: Option<wgpu::SurfaceConfiguration>,
  surface_format: Option<wgpu::TextureFormat>,
  quad_bgl: Option<wgpu::BindGroupLayout>,
  glyph_bgl: Option<wgpu::BindGroupLayout>,
  #[cfg(feature = "image")]
  image_bgl: Option<wgpu::BindGroupLayout>,
  #[cfg(feature = "image")]
  image_sampler: Option<wgpu::Sampler>,
  #[cfg(feature = "image")]
  image_texture_cache: std::collections::HashMap<u64, CachedImageTexture>,
  globals_buffer: Option<wgpu::Buffer>,
  gradient_buffer: Option<wgpu::Buffer>,
  atlas_texture: Option<wgpu::Texture>,
  atlas_view: Option<wgpu::TextureView>,
  atlas_sampler: Option<wgpu::Sampler>,
  atlas_size: (u32, u32),
  atlas_version: u64,
  last_profile: RenderProfile,
  profiling_enabled: bool,
  quad_bind_group: Option<wgpu::BindGroup>,
  glyph_bind_group: Option<wgpu::BindGroup>,
  vertex_buffer: Option<wgpu::Buffer>,
  index_buffer: Option<wgpu::Buffer>,
  width: u32,
  height: u32,
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
      #[cfg(feature = "image")]
      image_pipeline: None,
      #[cfg(feature = "svg")]
      svg_pipeline: None,
      #[cfg(feature = "svg")]
      svg_bgl: None,
      surface: None,
      surface_config: None,
      surface_format: None,
      quad_bgl: None,
      glyph_bgl: None,
      #[cfg(feature = "image")]
      image_bgl: None,
      #[cfg(feature = "image")]
      image_sampler: None,
      #[cfg(feature = "image")]
      image_texture_cache: std::collections::HashMap::new(),
      globals_buffer: None,
      gradient_buffer: None,
      atlas_texture: None,
      atlas_view: None,
      atlas_sampler: None,
      atlas_size: (0, 0),
      atlas_version: 0,
      last_profile: RenderProfile::default(),
      profiling_enabled: false,
      quad_bind_group: None,
      glyph_bind_group: None,
      vertex_buffer: None,
      index_buffer: None,
      width: 800,
      height: 600,
    }
  }

  fn release_gpu_resources(&mut self) {
    if let Some(device) = &self.device {
      let _ = device.poll(wgpu::Maintain::Poll);
    }

    #[cfg(feature = "image")]
    self.image_texture_cache.clear();

    self.quad_bind_group = None;
    self.glyph_bind_group = None;
    #[cfg(feature = "image")]
    {
      self.image_sampler = None;
      self.image_bgl = None;
      self.image_pipeline = None;
    }
    #[cfg(feature = "svg")]
    {
      self.svg_bgl = None;
      self.svg_pipeline = None;
    }

    self.vertex_buffer = None;
    self.index_buffer = None;
    self.globals_buffer = None;
    self.gradient_buffer = None;
    self.atlas_view = None;
    self.atlas_texture = None;
    self.atlas_sampler = None;

    self.quad_pipeline = None;
    self.glyph_pipeline = None;
    self.quad_bgl = None;
    self.glyph_bgl = None;

    self.surface_config = None;
    self.surface = None;
    self.surface_format = None;
    self.queue = None;
    self.device = None;
    self.adapter = None;
  }

  fn create_surface(&self, window: WindowHandle<'_>, display: DisplayHandle<'_>) -> wgpu::Surface<'static> {
    let surface_target =
      unsafe { wgpu::SurfaceTargetUnsafe::from_window(&WindowDisplayPair { window, display }) }.unwrap();
    unsafe { self.instance.create_surface_unsafe(surface_target) }.unwrap()
  }

  fn configure_surface(
    &mut self,
    surface: wgpu::Surface<'static>,
    adapter: &wgpu::Adapter,
    device: &wgpu::Device,
  ) -> wgpu::TextureFormat {
    let caps = surface.get_capabilities(adapter);
    let format = self
      .surface_format
      .filter(|format| caps.formats.contains(format))
      .or_else(|| caps.formats.iter().copied().find(wgpu::TextureFormat::is_srgb))
      .unwrap_or(caps.formats[0]);

    let config = wgpu::SurfaceConfiguration {
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      format,
      width: self.width.max(1),
      height: self.height.max(1),
      present_mode: wgpu::PresentMode::Fifo,
      alpha_mode: caps.alpha_modes[0],
      view_formats: vec![],
      desired_maximum_frame_latency: 2,
    };
    surface.configure(device, &config);
    self.surface = Some(surface);
    self.surface_config = Some(config);
    self.surface_format = Some(format);
    format
  }

  fn ensure_initialized(&mut self, window: WindowHandle<'_>, display: DisplayHandle<'_>) {
    if self.device.is_some() && self.surface.is_some() {
      return;
    }

    let surface = self.create_surface(window, display);

    if let (Some(adapter), Some(device)) = (self.adapter.clone(), self.device.clone()) {
      self.configure_surface(surface, &adapter, &device);
      return;
    }

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

    let format = self.configure_surface(surface, &adapter, &device);
    let config = self.surface_config.as_ref().unwrap();

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

    // --- Image pipeline ---
    #[cfg(feature = "image")]
    let (image_pipeline, image_bgl, image_sampler) = {
      use vertex::ImageInstance;
      let image_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("lurq_image_bgl"),
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
      let image_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("lurq_image_shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/image.wgsl").into()),
      });
      let image_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("lurq_image_pl"),
        bind_group_layouts: &[&image_bgl],
        push_constant_ranges: &[],
      });
      let image_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("lurq_image_pipeline"),
        layout: Some(&image_pipeline_layout),
        vertex: wgpu::VertexState {
          module: &image_shader,
          entry_point: Some("vs_main"),
          buffers: &[QuadVertex::desc(), ImageInstance::desc()],
          compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
          module: &image_shader,
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
      let image_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("lurq_image_sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        ..Default::default()
      });
      (image_pipeline, image_bgl, image_sampler)
    };

    // --- SVG pipeline ---
    #[cfg(feature = "svg")]
    let (svg_pipeline, svg_bgl) = {
      use vertex::SvgVertexGpu;
      let svg_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("lurq_svg_bgl"),
        entries: &[wgpu::BindGroupLayoutEntry {
          binding: 0,
          visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
          ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
          },
          count: None,
        }],
      });
      let svg_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("lurq_svg_shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/svg.wgsl").into()),
      });
      let svg_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("lurq_svg_pl"),
        bind_group_layouts: &[&svg_bgl],
        push_constant_ranges: &[],
      });
      let svg_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("lurq_svg_pipeline"),
        layout: Some(&svg_pipeline_layout),
        vertex: wgpu::VertexState {
          module: &svg_shader,
          entry_point: Some("vs_main"),
          buffers: &[SvgVertexGpu::desc()],
          compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
          module: &svg_shader,
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
      (svg_pipeline, svg_bgl)
    };

    // --- Persistent uniform/storage buffers ---
    let globals_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("lurq_globals"),
      contents: bytemuck::bytes_of(&Globals {
        viewport: [config.width as f32, config.height as f32, 0.0, 0.0],
        clip_rect: [0.0, 0.0, config.width as f32, config.height as f32],
        clip_radii_h: [0.0; 4],
        clip_radii_v: [0.0; 4],
        clip_active: [0.0; 4],
      }),
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let gradient_data: [f32; 4] = [0.0; 4];
    let gradient_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("lurq_gradients"),
      contents: bytemuck::cast_slice(&gradient_data),
      usage: wgpu::BufferUsages::STORAGE,
    });

    let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
      label: Some("lurq_atlas_sampler"),
      mag_filter: wgpu::FilterMode::Linear,
      min_filter: wgpu::FilterMode::Linear,
      ..Default::default()
    });

    let quad_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some("lurq_quad_bg"),
      layout: &quad_bgl,
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
    #[cfg(feature = "image")]
    {
      self.image_pipeline = Some(image_pipeline);
    }
    #[cfg(feature = "svg")]
    {
      self.svg_pipeline = Some(svg_pipeline);
      self.svg_bgl = Some(svg_bgl);
    }
    self.quad_bgl = Some(quad_bgl);
    self.glyph_bgl = Some(glyph_bgl);
    #[cfg(feature = "image")]
    {
      self.image_bgl = Some(image_bgl);
    }
    #[cfg(feature = "image")]
    {
      self.image_sampler = Some(image_sampler);
    }
    self.globals_buffer = Some(globals_buffer);
    self.gradient_buffer = Some(gradient_buffer);
    self.atlas_sampler = Some(atlas_sampler);
    self.quad_bind_group = Some(quad_bind_group);
    self.vertex_buffer = Some(vertex_buffer);
    self.index_buffer = Some(index_buffer);
  }
}

impl Drop for WgpuRenderEngine {
  fn drop(&mut self) {
    self.release_gpu_resources();
  }
}

impl RenderEngine for WgpuRenderEngine {
  fn resize(&mut self, width: u32, height: u32) {
    self.width = width.max(1);
    self.height = height.max(1);
    if let (Some(config), Some(device), Some(surface)) = (&mut self.surface_config, &self.device, &self.surface) {
      config.width = self.width;
      config.height = self.height;
      surface.configure(device, config);
    }
  }

  fn render(&mut self, list: &RenderList, window: WindowHandle<'_>, display: DisplayHandle<'_>) {
    let profiling_enabled = self.profiling_enabled;
    let total_start = ProfileScope::maybe_start(profiling_enabled);
    let init_start = ProfileScope::maybe_start(profiling_enabled);
    self.ensure_initialized(window, display);
    let init_dur = ProfileScope::elapsed_or_default(&init_start);

    let device = self.device.as_ref().unwrap();
    let queue = self.queue.as_ref().unwrap();
    let surface = self.surface.as_ref().unwrap();
    let config = self.surface_config.as_ref().unwrap();
    let vtx_buf = self.vertex_buffer.as_ref().unwrap();
    let idx_buf = self.index_buffer.as_ref().unwrap();

    let acquire_start = ProfileScope::maybe_start(profiling_enabled);
    let output = match surface.get_current_texture() {
      Ok(t) => t,
      Err(_) => {
        if profiling_enabled {
          self.last_profile = RenderProfile {
            init: init_dur,
            acquire: ProfileScope::elapsed_or_default(&acquire_start),
            total: ProfileScope::elapsed_or_default(&total_start),
            ..RenderProfile::default()
          };
        }
        return;
      }
    };
    let view = output.texture.create_view(&Default::default());
    let acquire_dur = ProfileScope::elapsed_or_default(&acquire_start);

    let vw = config.width as f32;
    let vh = config.height as f32;
    let globals_buffer = self.globals_buffer.as_ref().unwrap();

    let globals = Globals {
      viewport: [vw, vh, 0.0, 0.0],
      clip_rect: [0.0, 0.0, vw, vh],
      clip_radii_h: [0.0; 4],
      clip_radii_v: [0.0; 4],
      clip_active: [0.0; 4],
    };
    let globals_start = ProfileScope::maybe_start(profiling_enabled);
    queue.write_buffer(globals_buffer, 0, bytemuck::bytes_of(&globals));
    let globals_dur = ProfileScope::elapsed_or_default(&globals_start);

    // Atlas — recreate texture only if size changed
    let atlas_start = ProfileScope::maybe_start(profiling_enabled);
    let atlas = &list.atlas;
    let atlas_recreated = self.atlas_size != (atlas.width, atlas.height);
    if atlas_recreated {
      let texture = device.create_texture(&wgpu::TextureDescriptor {
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
      let view = texture.create_view(&Default::default());
      let glyph_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("lurq_glyph_bg"),
        layout: self.glyph_bgl.as_ref().unwrap(),
        entries: &[
          wgpu::BindGroupEntry {
            binding: 0,
            resource: globals_buffer.as_entire_binding(),
          },
          wgpu::BindGroupEntry {
            binding: 1,
            resource: wgpu::BindingResource::TextureView(&view),
          },
          wgpu::BindGroupEntry {
            binding: 2,
            resource: wgpu::BindingResource::Sampler(self.atlas_sampler.as_ref().unwrap()),
          },
        ],
      });
      self.atlas_texture = Some(texture);
      self.atlas_view = Some(view);
      self.glyph_bind_group = Some(glyph_bind_group);
      self.atlas_size = (atlas.width, atlas.height);
    }
    if atlas_recreated || self.atlas_version != atlas.version {
      queue.write_texture(
        wgpu::TexelCopyTextureInfo {
          texture: self.atlas_texture.as_ref().unwrap(),
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
      self.atlas_version = atlas.version;
    }
    let atlas_dur = ProfileScope::elapsed_or_default(&atlas_start);

    let encode_start = ProfileScope::maybe_start(profiling_enabled);
    struct PreparedDraw {
      start: usize,
      count: usize,
    }

    let mut rect_instances = Vec::new();
    let mut rect_draws = Vec::with_capacity(list.rects.len());
    let mut gradient_data: Vec<[f32; 4]> = Vec::new();
    for r in &list.rects {
      let start = rect_instances.len();
      let gradient_offset = match &r.gradient {
        Some(gradient) => crate::layout::render_list::encode_gradient(&mut gradient_data, gradient),
        None => -1.0,
      };
      rect_instances.push(QuadInstance {
        pos: [r.x, r.y],
        size: [r.width, r.height],
        color: r.color.to_linear_f32_array(),
        radii_h: r.radii,
        radii_v: r.radii,
        stroke: [0.0; 4],
        pattern: [0.0; 4],
        transform: r.transform,
        xf_origin: r.transform_origin,
        shadow_sigma: 0.0,
        gradient_offset,
      });
      if r.stroke.iter().any(|s| *s > 0.0) {
        rect_instances.push(QuadInstance {
          pos: [r.x, r.y],
          size: [r.width, r.height],
          color: r.stroke_color.to_linear_f32_array(),
          radii_h: r.radii,
          radii_v: r.radii,
          stroke: r.stroke,
          pattern: [0.0; 4],
          transform: r.transform,
          xf_origin: r.transform_origin,
          shadow_sigma: 0.0,
          gradient_offset: -1.0,
        });
      }
      rect_draws.push(PreparedDraw {
        start,
        count: rect_instances.len() - start,
      });
    }
    let rect_instance_buf = (!rect_instances.is_empty()).then(|| {
      device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("lurq_ordered_rect_instances"),
        contents: bytemuck::cast_slice(&rect_instances),
        usage: wgpu::BufferUsages::VERTEX,
      })
    });

    // Storage buffers must be non-empty; a single zeroed vec4 is enough when
    // no rect uses a gradient (their `gradient_offset` is -1 and the shader
    // never reads it).
    if gradient_data.is_empty() {
      gradient_data.push([0.0; 4]);
    }
    let gradient_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("lurq_gradients_frame"),
      contents: bytemuck::cast_slice(&gradient_data),
      usage: wgpu::BufferUsages::STORAGE,
    });
    let quad_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some("lurq_quad_bg_frame"),
      layout: self.quad_bgl.as_ref().unwrap(),
      entries: &[
        wgpu::BindGroupEntry {
          binding: 0,
          resource: self.globals_buffer.as_ref().unwrap().as_entire_binding(),
        },
        wgpu::BindGroupEntry {
          binding: 1,
          resource: gradient_buf.as_entire_binding(),
        },
      ],
    });

    let glyph_instances: Vec<GlyphInstance> = list
      .glyphs
      .iter()
      .map(|g| GlyphInstance {
        pos: [g.x, g.y],
        size: [g.width, g.height],
        color: g.color,
        uv_min: g.uv_min,
        uv_max: g.uv_max,
        transform: g.transform,
        xf_origin: g.transform_origin,
        sharpness: g.sharpness,
      })
      .collect();
    let glyph_instance_buf = (!glyph_instances.is_empty()).then(|| {
      device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("lurq_ordered_glyph_instances"),
        contents: bytemuck::cast_slice(&glyph_instances),
        usage: wgpu::BufferUsages::VERTEX,
      })
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
            load: wgpu::LoadOp::Clear(wgpu_clear_color(list.clear_color)),
            store: wgpu::StoreOp::Store,
          },
        })],
        ..Default::default()
      });

      enum OrderedDraw {
        Rect(usize),
        Glyph {
          start: usize,
          count: usize,
        },
        #[cfg(feature = "image")]
        Image(usize),
        #[cfg(feature = "svg")]
        Svg(usize),
      }

      let mut ordered_draws: Vec<(usize, OrderedDraw)> = Vec::new();
      for (index, rect) in list.rects.iter().enumerate() {
        ordered_draws.push((rect.order, OrderedDraw::Rect(index)));
      }
      let mut glyph_start = 0;
      while glyph_start < list.glyphs.len() {
        let order = list.glyphs[glyph_start].order;
        let clip = list.glyphs[glyph_start].clip;
        let mut glyph_end = glyph_start + 1;
        while glyph_end < list.glyphs.len()
          && list.glyphs[glyph_end].order == order
          && same_clip(list.glyphs[glyph_end].clip, clip)
        {
          glyph_end += 1;
        }
        ordered_draws.push((
          order,
          OrderedDraw::Glyph {
            start: glyph_start,
            count: glyph_end - glyph_start,
          },
        ));
        glyph_start = glyph_end;
      }
      #[cfg(feature = "image")]
      for (index, image) in list.images.iter().enumerate() {
        ordered_draws.push((image.order, OrderedDraw::Image(index)));
      }
      #[cfg(feature = "svg")]
      for (index, svg) in list.svgs.iter().enumerate() {
        ordered_draws.push((svg.order, OrderedDraw::Svg(index)));
      }
      ordered_draws.sort_by_key(|(order, _)| *order);

      for (_, draw) in ordered_draws {
        match draw {
          OrderedDraw::Rect(index) => {
            let r = &list.rects[index];
            if !set_scissor(&mut pass, r.clip, vw, vh) {
              continue;
            }
            let prepared = &rect_draws[index];
            let start = (prepared.start * std::mem::size_of::<QuadInstance>()) as wgpu::BufferAddress;
            let end = ((prepared.start + prepared.count) * std::mem::size_of::<QuadInstance>()) as wgpu::BufferAddress;
            pass.set_pipeline(self.quad_pipeline.as_ref().unwrap());
            if rounded_clip_needs_shader(r.clip) {
              let clip_globals = globals_buffer_for_clip(device, r.clip, vw, vh);
              let clip_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("lurq_quad_clip_bg"),
                layout: self.quad_bgl.as_ref().unwrap(),
                entries: &[
                  wgpu::BindGroupEntry {
                    binding: 0,
                    resource: clip_globals.as_entire_binding(),
                  },
                  wgpu::BindGroupEntry {
                    binding: 1,
                    resource: gradient_buf.as_entire_binding(),
                  },
                ],
              });
              pass.set_bind_group(0, &clip_bind_group, &[]);
            } else {
              pass.set_bind_group(0, &quad_bind_group, &[]);
            }
            pass.set_vertex_buffer(0, vtx_buf.slice(..));
            pass.set_vertex_buffer(1, rect_instance_buf.as_ref().unwrap().slice(start..end));
            pass.set_index_buffer(idx_buf.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..6, 0, 0..prepared.count as u32);
          }
          OrderedDraw::Glyph { start, count } => {
            let glyph_slice = &list.glyphs[start..start + count];
            if glyph_slice.is_empty() || !set_scissor(&mut pass, glyph_slice[0].clip, vw, vh) {
              continue;
            }
            let start_byte = (start * std::mem::size_of::<GlyphInstance>()) as wgpu::BufferAddress;
            let end_byte = ((start + count) * std::mem::size_of::<GlyphInstance>()) as wgpu::BufferAddress;
            pass.set_pipeline(self.glyph_pipeline.as_ref().unwrap());
            if rounded_clip_needs_shader(glyph_slice[0].clip) {
              let clip_globals = globals_buffer_for_clip(device, glyph_slice[0].clip, vw, vh);
              let clip_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("lurq_glyph_clip_bg"),
                layout: self.glyph_bgl.as_ref().unwrap(),
                entries: &[
                  wgpu::BindGroupEntry {
                    binding: 0,
                    resource: clip_globals.as_entire_binding(),
                  },
                  wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(self.atlas_view.as_ref().unwrap()),
                  },
                  wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(self.atlas_sampler.as_ref().unwrap()),
                  },
                ],
              });
              pass.set_bind_group(0, &clip_bind_group, &[]);
            } else {
              pass.set_bind_group(0, self.glyph_bind_group.as_ref().unwrap(), &[]);
            }
            pass.set_vertex_buffer(0, vtx_buf.slice(..));
            pass.set_vertex_buffer(1, glyph_instance_buf.as_ref().unwrap().slice(start_byte..end_byte));
            pass.set_index_buffer(idx_buf.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..6, 0, 0..count as u32);
          }
          #[cfg(feature = "image")]
          OrderedDraw::Image(index) => {
            use vertex::ImageInstance;

            let img = &list.images[index];
            let cached = self.image_texture_cache.entry(img.image_id).or_insert_with(|| {
              let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("lurq_img"),
                size: wgpu::Extent3d {
                  width: img.image_width,
                  height: img.image_height,
                  depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[],
              });
              queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                  texture: &texture,
                  mip_level: 0,
                  origin: wgpu::Origin3d::ZERO,
                  aspect: wgpu::TextureAspect::All,
                },
                &img.data,
                wgpu::TexelCopyBufferLayout {
                  offset: 0,
                  bytes_per_row: Some(img.image_width * 4),
                  rows_per_image: Some(img.image_height),
                },
                wgpu::Extent3d {
                  width: img.image_width,
                  height: img.image_height,
                  depth_or_array_layers: 1,
                },
              );
              let view = texture.create_view(&Default::default());
              let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("lurq_img_bg"),
                layout: self.image_bgl.as_ref().unwrap(),
                entries: &[
                  wgpu::BindGroupEntry {
                    binding: 0,
                    resource: globals_buffer.as_entire_binding(),
                  },
                  wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                  },
                  wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(self.image_sampler.as_ref().unwrap()),
                  },
                ],
              });
              CachedImageTexture {
                texture,
                view,
                bind_group,
                frame_index: img.frame_index,
                version: img.version,
              }
            });

            if cached.frame_index != img.frame_index || cached.version != img.version {
              queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                  texture: &cached.texture,
                  mip_level: 0,
                  origin: wgpu::Origin3d::ZERO,
                  aspect: wgpu::TextureAspect::All,
                },
                &img.data,
                wgpu::TexelCopyBufferLayout {
                  offset: 0,
                  bytes_per_row: Some(img.image_width * 4),
                  rows_per_image: Some(img.image_height),
                },
                wgpu::Extent3d {
                  width: img.image_width,
                  height: img.image_height,
                  depth_or_array_layers: 1,
                },
              );
              cached.frame_index = img.frame_index;
              cached.version = img.version;
            }

            if !set_scissor(&mut pass, img.clip, vw, vh) {
              continue;
            }

            let instance = ImageInstance {
              pos: [img.x, img.y],
              size: [img.width, img.height],
              opacity: [1.0, 0.0, 0.0, 0.0],
              transform: img.transform,
              xf_origin: img.transform_origin,
              uv_min: img.uv_min,
              uv_max: img.uv_max,
              radii: img.radii,
            };
            let instance_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
              label: Some("lurq_ii"),
              contents: bytemuck::cast_slice(&[instance]),
              usage: wgpu::BufferUsages::VERTEX,
            });
            pass.set_pipeline(self.image_pipeline.as_ref().unwrap());
            if rounded_clip_needs_shader(img.clip) {
              let clip_globals = globals_buffer_for_clip(device, img.clip, vw, vh);
              let clip_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("lurq_image_clip_bg"),
                layout: self.image_bgl.as_ref().unwrap(),
                entries: &[
                  wgpu::BindGroupEntry {
                    binding: 0,
                    resource: clip_globals.as_entire_binding(),
                  },
                  wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&cached.view),
                  },
                  wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(self.image_sampler.as_ref().unwrap()),
                  },
                ],
              });
              pass.set_bind_group(0, &clip_bind_group, &[]);
            } else {
              pass.set_bind_group(0, &cached.bind_group, &[]);
            }
            pass.set_vertex_buffer(0, vtx_buf.slice(..));
            pass.set_vertex_buffer(1, instance_buf.slice(..));
            pass.set_index_buffer(idx_buf.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..6, 0, 0..1);
          }
          #[cfg(feature = "svg")]
          OrderedDraw::Svg(index) => {
            use vertex::SvgVertexGpu;

            let svg_cmd = &list.svgs[index];
            if svg_cmd.mesh.vertices.is_empty() || svg_cmd.mesh.indices.is_empty() {
              continue;
            }

            let svg_globals = globals_for_clip(svg_cmd.clip, vw, vh);
            let svg_globals_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
              label: Some("lurq_svg_globals"),
              contents: bytemuck::bytes_of(&svg_globals),
              usage: wgpu::BufferUsages::UNIFORM,
            });

            let svg_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
              label: Some("lurq_svg_bg"),
              layout: self.svg_bgl.as_ref().unwrap(),
              entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: svg_globals_buf.as_entire_binding(),
              }],
            });

            if !set_scissor(&mut pass, svg_cmd.clip, vw, vh) {
              continue;
            }

            let gpu_verts: Vec<SvgVertexGpu> = svg_cmd
              .mesh
              .vertices
              .iter()
              .map(|v| SvgVertexGpu {
                position: [v.position[0] + svg_cmd.x, v.position[1] + svg_cmd.y],
                color: v.color,
              })
              .collect();

            let vb = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
              label: Some("lurq_svg_vb"),
              contents: bytemuck::cast_slice(&gpu_verts),
              usage: wgpu::BufferUsages::VERTEX,
            });
            let ib = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
              label: Some("lurq_svg_ib"),
              contents: bytemuck::cast_slice(&svg_cmd.mesh.indices),
              usage: wgpu::BufferUsages::INDEX,
            });

            pass.set_pipeline(self.svg_pipeline.as_ref().unwrap());
            pass.set_bind_group(0, &svg_bg, &[]);
            pass.set_vertex_buffer(0, vb.slice(..));
            pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..svg_cmd.mesh.indices.len() as u32, 0, 0..1);
          }
        }
      }
    }
    let encode_dur = ProfileScope::elapsed_or_default(&encode_start);

    let submit_start = ProfileScope::maybe_start(profiling_enabled);
    queue.submit(std::iter::once(encoder.finish()));
    let submit_dur = ProfileScope::elapsed_or_default(&submit_start);

    let present_start = ProfileScope::maybe_start(profiling_enabled);
    output.present();
    let present_dur = ProfileScope::elapsed_or_default(&present_start);

    if profiling_enabled {
      self.last_profile = RenderProfile {
        init: init_dur,
        acquire: acquire_dur,
        globals_upload: globals_dur,
        atlas_upload: atlas_dur,
        encode: encode_dur,
        submit: submit_dur,
        present: present_dur,
        total: ProfileScope::elapsed_or_default(&total_start),
      };
    }
  }

  fn release_window_surface(&mut self) {
    if let Some(device) = &self.device {
      let _ = device.poll(wgpu::Maintain::Poll);
    }
    self.surface_config = None;
    self.surface = None;
  }

  fn set_profiling_enabled(&mut self, enabled: bool) {
    self.profiling_enabled = enabled;
  }

  fn last_profile(&self) -> Option<RenderProfile> {
    Some(self.last_profile)
  }
}

fn same_clip(a: crate::layout::quad::ClipRect, b: crate::layout::quad::ClipRect) -> bool {
  a.active == b.active
    && a.x == b.x
    && a.y == b.y
    && a.width == b.width
    && a.height == b.height
    && a.border_radius == b.border_radius
}

fn wgpu_clear_color(color: crate::node::color::Color) -> wgpu::Color {
  let [r, g, b, a] = color.to_linear_f32_array();
  wgpu::Color {
    r: r as f64,
    g: g as f64,
    b: b as f64,
    a: a as f64,
  }
}

fn globals_for_clip(clip: crate::layout::quad::ClipRect, vw: f32, vh: f32) -> Globals {
  let radius = clip.border_radius.unwrap_or_default();
  let radii = radius.to_array();
  Globals {
    viewport: [vw, vh, 0.0, 0.0],
    clip_rect: if clip.active {
      [clip.x, clip.y, clip.width, clip.height]
    } else {
      [0.0, 0.0, vw, vh]
    },
    clip_radii_h: radii,
    clip_radii_v: radii,
    clip_active: if rounded_clip_needs_shader(clip) {
      [1.0, 0.0, 0.0, 0.0]
    } else {
      [0.0; 4]
    },
  }
}

fn rounded_clip_needs_shader(clip: crate::layout::quad::ClipRect) -> bool {
  let Some(radius) = clip.border_radius else {
    return false;
  };
  clip.active
    && (radius.top_left > 0.0 || radius.top_right > 0.0 || radius.bottom_right > 0.0 || radius.bottom_left > 0.0)
}

fn globals_buffer_for_clip(
  device: &wgpu::Device,
  clip: crate::layout::quad::ClipRect,
  vw: f32,
  vh: f32,
) -> wgpu::Buffer {
  let globals = globals_for_clip(clip, vw, vh);
  device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
    label: Some("lurq_clip_globals"),
    contents: bytemuck::bytes_of(&globals),
    usage: wgpu::BufferUsages::UNIFORM,
  })
}

fn set_scissor(pass: &mut wgpu::RenderPass<'_>, clip: crate::layout::quad::ClipRect, vw: f32, vh: f32) -> bool {
  let Some((x, y, width, height)) = scissor_rect(clip, vw, vh) else {
    return false;
  };
  pass.set_scissor_rect(x, y, width, height);
  true
}

fn scissor_rect(clip: crate::layout::quad::ClipRect, vw: f32, vh: f32) -> Option<(u32, u32, u32, u32)> {
  let viewport_w = vw.ceil().max(1.0) as u32;
  let viewport_h = vh.ceil().max(1.0) as u32;

  if clip.active {
    let left = clip.x.floor().max(0.0);
    let top = clip.y.floor().max(0.0);
    let right = (clip.x + clip.width).ceil().clamp(0.0, viewport_w as f32);
    let bottom = (clip.y + clip.height).ceil().clamp(0.0, viewport_h as f32);
    if right <= left || bottom <= top {
      return None;
    }

    let cx = left as u32;
    let cy = top as u32;
    if cx >= viewport_w || cy >= viewport_h {
      return None;
    }
    let cw = ((right - left).ceil() as u32).min(viewport_w.saturating_sub(cx));
    let ch = ((bottom - top).ceil() as u32).min(viewport_h.saturating_sub(cy));
    if cw == 0 || ch == 0 {
      return None;
    }
    Some((cx, cy, cw, ch))
  } else {
    Some((0, 0, viewport_w, viewport_h))
  }
}

#[cfg(test)]
mod tests {
  use crate::{app::render_engine::RenderEngine, layout::quad::ClipRect};

  #[test]
  fn scissor_expands_fractional_clip_to_include_bottom_right_edge() {
    assert_eq!(
      super::scissor_rect(
        ClipRect {
          x: 10.6,
          y: 20.2,
          width: 30.1,
          height: 40.6,
          active: true,
          border_radius: None,
        },
        100.0,
        100.0,
      ),
      Some((10, 20, 31, 41))
    );
  }

  #[test]
  fn resize_before_initialization_sets_initial_surface_size() {
    let mut engine = super::WgpuRenderEngine::new();

    engine.resize(1440, 900);

    assert_eq!(engine.width, 1440);
    assert_eq!(engine.height, 900);
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
