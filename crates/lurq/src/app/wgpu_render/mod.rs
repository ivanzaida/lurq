mod vertex;

#[cfg(all(feature = "image", target_os = "macos"))]
use std::ffi::c_void;

#[cfg(all(feature = "image", target_os = "macos"))]
use core_foundation_sys::base::{CFAllocatorRef, CFRelease, OSStatus, kCFAllocatorDefault};
#[cfg(all(feature = "image", target_os = "macos"))]
use core_foundation_sys::dictionary::CFDictionaryRef;
#[cfg(all(feature = "image", target_os = "macos"))]
use core_video_sys::pixel_buffer::CVPixelBufferRef;
#[cfg(all(feature = "image", target_os = "macos"))]
use metal::foreign_types::ForeignType;
use raw_window_handle::{DisplayHandle, WindowHandle};
use std::time::Duration;
#[cfg(feature = "image")]
use vertex::ImageInstance;
use vertex::{Globals, GlyphInstance, QuadInstance, QuadVertex};
use wgpu::util::DeviceExt;

use crate::{
  app::{
    profiler::{RenderProfile, profile_elapsed, profile_if, profile_scope},
    render_engine::RenderEngine,
  },
  layout::render_list::RenderList,
};

struct DynamicBuffer {
  buffer: Option<wgpu::Buffer>,
  capacity: wgpu::BufferAddress,
  usage: wgpu::BufferUsages,
  label: &'static str,
  last_bytes: Vec<u8>,
}

fn changed_byte_range(previous: &[u8], next: &[u8]) -> Option<(usize, usize)> {
  if previous.len() != next.len() {
    return Some((0, next.len()));
  }

  let start = previous.iter().zip(next).position(|(a, b)| a != b)?;
  let end = previous
    .iter()
    .zip(next)
    .rposition(|(a, b)| a != b)
    .map(|index| index + 1)
    .unwrap_or(start + 1);
  let aligned_start = start & !3;
  let aligned_end = (end + 3).min(next.len()) & !3;
  Some((aligned_start, aligned_end.max(aligned_start + 4).min(next.len())))
}

impl DynamicBuffer {
  fn new(label: &'static str, usage: wgpu::BufferUsages) -> Self {
    Self {
      buffer: None,
      capacity: 0,
      usage,
      label,
      last_bytes: Vec::new(),
    }
  }

  fn clear(&mut self) {
    self.buffer = None;
    self.capacity = 0;
    self.last_bytes.clear();
  }

  fn write<T: bytemuck::Pod>(
    &mut self,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    data: &[T],
  ) -> Option<&wgpu::Buffer> {
    if data.is_empty() {
      return None;
    }

    let bytes = bytemuck::cast_slice(data);
    let required = bytes.len() as wgpu::BufferAddress;
    let mut recreated = false;
    if self.capacity < required {
      self.capacity = required.next_power_of_two().max(256);
      self.buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(self.label),
        size: self.capacity,
        usage: self.usage | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
      }));
      recreated = true;
    }

    let buffer = self.buffer.as_ref().unwrap();
    if recreated {
      queue.write_buffer(buffer, 0, bytes);
      self.last_bytes.clear();
      self.last_bytes.extend_from_slice(bytes);
    } else if let Some((start, end)) = changed_byte_range(&self.last_bytes, bytes) {
      queue.write_buffer(buffer, start as wgpu::BufferAddress, &bytes[start..end]);
      self.last_bytes.clear();
      self.last_bytes.extend_from_slice(bytes);
    }
    Some(buffer)
  }
}

struct PreparedDraw {
  start: usize,
  count: usize,
}

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

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct ClipGlobalsKey([u32; 20]);

#[cfg(feature = "image")]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
enum ImageClipFormat {
  Rgba,
  Nv12,
}

#[cfg(feature = "image")]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct ImageClipBindGroupKey {
  image_id: u64,
  frame_index: usize,
  clip: ClipGlobalsKey,
  format: ImageClipFormat,
}

#[cfg(feature = "image")]
struct CachedRgbaFrame {
  bind_group: wgpu::BindGroup,
  view: wgpu::TextureView,
  texture: wgpu::Texture,
}

#[cfg(feature = "image")]
enum CachedImageTexture {
  Rgba {
    bind_group: wgpu::BindGroup,
    view: wgpu::TextureView,
    texture: wgpu::Texture,
    animation_frames: Option<Vec<CachedRgbaFrame>>,
    width: u32,
    height: u32,
    frame_index: usize,
    version: u64,
  },
  Nv12 {
    bind_group: wgpu::BindGroup,
    y_view: wgpu::TextureView,
    uv_view: wgpu::TextureView,
    y_texture: wgpu::Texture,
    uv_texture: wgpu::Texture,
    #[cfg(target_os = "macos")]
    _macos_native: Option<MacosNativeNv12Texture>,
    width: u32,
    height: u32,
    frame_index: usize,
    version: u64,
  },
}

#[cfg(all(feature = "image", target_os = "macos"))]
struct MacosNativeNv12Texture {
  _native: crate::images::NativeImageData,
  _y_cv_texture: CvMetalTexture,
  _uv_cv_texture: CvMetalTexture,
}

#[cfg(all(feature = "image", target_os = "macos"))]
struct CvMetalTexture {
  ptr: CVMetalTextureRef,
}

#[cfg(all(feature = "image", target_os = "macos"))]
impl Drop for CvMetalTexture {
  fn drop(&mut self) {
    unsafe {
      CFRelease(self.ptr.cast());
    }
  }
}

#[cfg(all(feature = "image", target_os = "macos"))]
type CVMetalTextureCacheRef = *mut c_void;
#[cfg(all(feature = "image", target_os = "macos"))]
type CVMetalTextureRef = *mut c_void;

#[cfg(all(feature = "image", target_os = "macos"))]
#[link(name = "CoreVideo", kind = "framework")]
unsafe extern "C" {
  fn CVMetalTextureCacheCreate(
    allocator: CFAllocatorRef,
    cache_attributes: CFDictionaryRef,
    metal_device: *mut c_void,
    texture_attributes: CFDictionaryRef,
    cache_out: *mut CVMetalTextureCacheRef,
  ) -> OSStatus;

  fn CVMetalTextureCacheCreateTextureFromImage(
    allocator: CFAllocatorRef,
    texture_cache: CVMetalTextureCacheRef,
    source_image: CVPixelBufferRef,
    texture_attributes: CFDictionaryRef,
    pixel_format: metal::MTLPixelFormat,
    width: usize,
    height: usize,
    plane_index: usize,
    texture_out: *mut CVMetalTextureRef,
  ) -> OSStatus;

  fn CVMetalTextureGetTexture(image: CVMetalTextureRef) -> *mut c_void;
}

#[cfg(feature = "image")]
impl CachedImageTexture {
  fn is_compatible(&self, image: &crate::images::ImageCmd) -> bool {
    match self {
      Self::Rgba {
        width,
        height,
        animation_frames,
        ..
      } => {
        image.image_format == crate::images::ImagePixelFormat::Rgba8
          && *width == image.image_width
          && *height == image.image_height
          && animation_frames.as_ref().map(Vec::len) == image.animation_frames.as_ref().map(|frames| frames.len())
      }
      Self::Nv12 { width, height, .. } => {
        image.image_format == crate::images::ImagePixelFormat::Nv12
          && *width == image.image_width
          && *height == image.image_height
      }
    }
  }
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
  #[cfg(feature = "image")]
  nv12_image_pipeline: Option<wgpu::RenderPipeline>,
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
  nv12_image_bgl: Option<wgpu::BindGroupLayout>,
  #[cfg(feature = "image")]
  image_sampler: Option<wgpu::Sampler>,
  #[cfg(feature = "image")]
  image_texture_cache: std::collections::HashMap<u64, CachedImageTexture>,
  #[cfg(feature = "image")]
  image_clip_bind_groups: std::collections::HashMap<ImageClipBindGroupKey, wgpu::BindGroup>,
  globals_buffer: Option<wgpu::Buffer>,
  clip_globals_cache: std::collections::HashMap<ClipGlobalsKey, wgpu::Buffer>,
  quad_clip_bind_groups: std::collections::HashMap<ClipGlobalsKey, wgpu::BindGroup>,
  glyph_clip_bind_groups: std::collections::HashMap<ClipGlobalsKey, wgpu::BindGroup>,
  gradient_buffer: Option<wgpu::Buffer>,
  gradient_buffer_capacity: wgpu::BufferAddress,
  last_globals_bytes: Vec<u8>,
  last_gradient_bytes: Vec<u8>,
  atlas_texture: Option<wgpu::Texture>,
  atlas_view: Option<wgpu::TextureView>,
  atlas_sampler: Option<wgpu::Sampler>,
  atlas_size: (u32, u32),
  atlas_version: u64,
  #[cfg(feature = "perf_profile")]
  last_profile: RenderProfile,
  quad_bind_group: Option<wgpu::BindGroup>,
  glyph_bind_group: Option<wgpu::BindGroup>,
  vertex_buffer: Option<wgpu::Buffer>,
  index_buffer: Option<wgpu::Buffer>,
  rect_instance_buffer: DynamicBuffer,
  glyph_instance_buffer: DynamicBuffer,
  #[cfg(feature = "image")]
  image_instance_buffer: DynamicBuffer,
  scratch_rect_instances: Vec<QuadInstance>,
  scratch_rect_draws: Vec<PreparedDraw>,
  scratch_gradient_data: Vec<[f32; 4]>,
  scratch_glyph_instances: Vec<GlyphInstance>,
  #[cfg(feature = "image")]
  scratch_image_instances: Vec<ImageInstance>,
  scratch_ordered_draws: Vec<(usize, OrderedDraw)>,
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
      #[cfg(feature = "image")]
      nv12_image_pipeline: None,
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
      nv12_image_bgl: None,
      #[cfg(feature = "image")]
      image_sampler: None,
      #[cfg(feature = "image")]
      image_texture_cache: std::collections::HashMap::new(),
      #[cfg(feature = "image")]
      image_clip_bind_groups: std::collections::HashMap::new(),
      globals_buffer: None,
      clip_globals_cache: std::collections::HashMap::new(),
      quad_clip_bind_groups: std::collections::HashMap::new(),
      glyph_clip_bind_groups: std::collections::HashMap::new(),
      gradient_buffer: None,
      gradient_buffer_capacity: 0,
      last_globals_bytes: Vec::new(),
      last_gradient_bytes: Vec::new(),
      atlas_texture: None,
      atlas_view: None,
      atlas_sampler: None,
      atlas_size: (0, 0),
      atlas_version: 0,
      #[cfg(feature = "perf_profile")]
      last_profile: RenderProfile::default(),
      quad_bind_group: None,
      glyph_bind_group: None,
      vertex_buffer: None,
      index_buffer: None,
      rect_instance_buffer: DynamicBuffer::new("lurq_ordered_rect_instances", wgpu::BufferUsages::VERTEX),
      glyph_instance_buffer: DynamicBuffer::new("lurq_ordered_glyph_instances", wgpu::BufferUsages::VERTEX),
      #[cfg(feature = "image")]
      image_instance_buffer: DynamicBuffer::new("lurq_image_instances", wgpu::BufferUsages::VERTEX),
      scratch_rect_instances: Vec::new(),
      scratch_rect_draws: Vec::new(),
      scratch_gradient_data: Vec::new(),
      scratch_glyph_instances: Vec::new(),
      #[cfg(feature = "image")]
      scratch_image_instances: Vec::new(),
      scratch_ordered_draws: Vec::new(),
      width: 800,
      height: 600,
    }
  }

  fn release_gpu_resources(&mut self) {
    if let Some(device) = &self.device {
      let _ = device.poll(wgpu::Maintain::Poll);
    }

    #[cfg(feature = "image")]
    {
      self.image_texture_cache.clear();
      self.image_clip_bind_groups.clear();
    }
    self.clip_globals_cache.clear();
    self.quad_clip_bind_groups.clear();
    self.glyph_clip_bind_groups.clear();

    self.quad_bind_group = None;
    self.glyph_bind_group = None;
    #[cfg(feature = "image")]
    {
      self.image_sampler = None;
      self.image_bgl = None;
      self.nv12_image_bgl = None;
      self.image_pipeline = None;
      self.nv12_image_pipeline = None;
    }
    #[cfg(feature = "svg")]
    {
      self.svg_bgl = None;
      self.svg_pipeline = None;
    }

    self.vertex_buffer = None;
    self.index_buffer = None;
    self.rect_instance_buffer.clear();
    self.glyph_instance_buffer.clear();
    #[cfg(feature = "image")]
    self.image_instance_buffer.clear();
    self.globals_buffer = None;
    self.gradient_buffer = None;
    self.gradient_buffer_capacity = 0;
    self.last_globals_bytes.clear();
    self.last_gradient_bytes.clear();
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
      desired_maximum_frame_latency: 1,
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
    let (image_pipeline, nv12_image_pipeline, image_bgl, nv12_image_bgl, image_sampler) = {
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
      let nv12_image_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("lurq_nv12_image_bgl"),
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
            ty: wgpu::BindingType::Texture {
              sample_type: wgpu::TextureSampleType::Float { filterable: true },
              view_dimension: wgpu::TextureViewDimension::D2,
              multisampled: false,
            },
            count: None,
          },
          wgpu::BindGroupLayoutEntry {
            binding: 3,
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
      let nv12_image_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("lurq_nv12_image_shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/image_nv12.wgsl").into()),
      });
      let image_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("lurq_image_pl"),
        bind_group_layouts: &[&image_bgl],
        push_constant_ranges: &[],
      });
      let nv12_image_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("lurq_nv12_image_pl"),
        bind_group_layouts: &[&nv12_image_bgl],
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
      let nv12_image_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("lurq_nv12_image_pipeline"),
        layout: Some(&nv12_image_pipeline_layout),
        vertex: wgpu::VertexState {
          module: &nv12_image_shader,
          entry_point: Some("vs_main"),
          buffers: &[QuadVertex::desc(), ImageInstance::desc()],
          compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
          module: &nv12_image_shader,
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
      (
        image_pipeline,
        nv12_image_pipeline,
        image_bgl,
        nv12_image_bgl,
        image_sampler,
      )
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
      usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
    });
    let gradient_buffer_capacity = std::mem::size_of_val(&gradient_data) as wgpu::BufferAddress;

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
      self.nv12_image_pipeline = Some(nv12_image_pipeline);
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
      self.nv12_image_bgl = Some(nv12_image_bgl);
    }
    #[cfg(feature = "image")]
    {
      self.image_sampler = Some(image_sampler);
    }
    self.globals_buffer = Some(globals_buffer);
    self.gradient_buffer = Some(gradient_buffer);
    self.gradient_buffer_capacity = gradient_buffer_capacity;
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
    self.clip_globals_cache.clear();
    self.quad_clip_bind_groups.clear();
    self.glyph_clip_bind_groups.clear();
    #[cfg(feature = "image")]
    self.image_clip_bind_groups.clear();
    if let (Some(config), Some(device), Some(surface)) = (&mut self.surface_config, &self.device, &self.surface) {
      config.width = self.width;
      config.height = self.height;
      surface.configure(device, config);
    }
  }

  fn render(&mut self, list: &RenderList, window: WindowHandle<'_>, display: DisplayHandle<'_>) {
    let _total_start = profile_scope!();
    let _init_start = profile_scope!();
    self.ensure_initialized(window, display);
    let _init_dur = profile_elapsed!(_init_start);

    let device = self.device.as_ref().unwrap();
    let queue = self.queue.as_ref().unwrap();
    let surface = self.surface.as_ref().unwrap();
    let config = self.surface_config.as_ref().unwrap();
    let vtx_buf = self.vertex_buffer.as_ref().unwrap();
    let idx_buf = self.index_buffer.as_ref().unwrap();

    let _acquire_start = profile_scope!();
    let output = match surface.get_current_texture() {
      Ok(t) => t,
      Err(_) => {
        profile_if! {
          self.last_profile = RenderProfile {
            init: _init_dur,
            acquire: profile_elapsed!(_acquire_start),
            total: profile_elapsed!(_total_start),
            ..RenderProfile::default()
          };
        }
        return;
      }
    };
    let view = output.texture.create_view(&Default::default());
    let _acquire_dur = profile_elapsed!(_acquire_start);

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
    let _globals_start = profile_scope!();
    let globals_bytes = bytemuck::bytes_of(&globals);
    if self.last_globals_bytes.as_slice() != globals_bytes {
      queue.write_buffer(globals_buffer, 0, globals_bytes);
      self.last_globals_bytes.clear();
      self.last_globals_bytes.extend_from_slice(globals_bytes);
    }
    let _globals_dur = profile_elapsed!(_globals_start);

    // Atlas — recreate texture only if size changed
    let _atlas_start = profile_scope!();
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
      self.glyph_clip_bind_groups.clear();
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
    let _atlas_dur = profile_elapsed!(_atlas_start);

    let _encode_start = profile_scope!();
    let mut _buffer_upload_dur = Duration::default();
    let mut _image_texture_upload_dur = Duration::default();

    self.scratch_rect_instances.clear();
    self.scratch_rect_draws.clear();
    self.scratch_rect_draws.reserve(list.rects.len());
    self.scratch_gradient_data.clear();
    for r in &list.rects {
      let start = self.scratch_rect_instances.len();
      let gradient_offset = match &r.gradient {
        Some(gradient) => crate::layout::render_list::encode_gradient(&mut self.scratch_gradient_data, gradient),
        None => -1.0,
      };
      self.scratch_rect_instances.push(QuadInstance {
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
        self.scratch_rect_instances.push(QuadInstance {
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
      self.scratch_rect_draws.push(PreparedDraw {
        start,
        count: self.scratch_rect_instances.len() - start,
      });
    }
    let _rect_upload_start = profile_scope!();
    let rect_instance_buf = self
      .rect_instance_buffer
      .write(device, queue, &self.scratch_rect_instances);
    _buffer_upload_dur += profile_elapsed!(_rect_upload_start);

    // Storage buffers must be non-empty; a single zeroed vec4 is enough when
    // no rect uses a gradient (their `gradient_offset` is -1 and the shader
    // never reads it).
    if self.scratch_gradient_data.is_empty() {
      self.scratch_gradient_data.push([0.0; 4]);
    }
    let gradient_bytes = bytemuck::cast_slice(&self.scratch_gradient_data);
    let required_gradient_capacity = gradient_bytes.len() as wgpu::BufferAddress;
    let mut gradient_recreated = false;
    if self.gradient_buffer_capacity < required_gradient_capacity {
      self.gradient_buffer_capacity = required_gradient_capacity.next_power_of_two().max(256);
      let gradient_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("lurq_gradients"),
        size: self.gradient_buffer_capacity,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
      });
      let quad_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("lurq_quad_bg"),
        layout: self.quad_bgl.as_ref().unwrap(),
        entries: &[
          wgpu::BindGroupEntry {
            binding: 0,
            resource: self.globals_buffer.as_ref().unwrap().as_entire_binding(),
          },
          wgpu::BindGroupEntry {
            binding: 1,
            resource: gradient_buffer.as_entire_binding(),
          },
        ],
      });
      self.gradient_buffer = Some(gradient_buffer);
      self.quad_bind_group = Some(quad_bind_group);
      self.quad_clip_bind_groups.clear();
      gradient_recreated = true;
    }
    let gradient_buf = self.gradient_buffer.as_ref().unwrap();
    let _gradient_upload_start = profile_scope!();
    if gradient_recreated || self.last_gradient_bytes.as_slice() != gradient_bytes {
      queue.write_buffer(gradient_buf, 0, gradient_bytes);
      self.last_gradient_bytes.clear();
      self.last_gradient_bytes.extend_from_slice(gradient_bytes);
    }
    _buffer_upload_dur += profile_elapsed!(_gradient_upload_start);

    self.scratch_glyph_instances.clear();
    self.scratch_glyph_instances.reserve(list.glyphs.len());
    self
      .scratch_glyph_instances
      .extend(list.glyphs.iter().map(|g| GlyphInstance {
        pos: [g.x, g.y],
        size: [g.width, g.height],
        color: g.color,
        uv_min: g.uv_min,
        uv_max: g.uv_max,
        transform: g.transform,
        xf_origin: g.transform_origin,
        sharpness: g.sharpness,
      }));
    let _glyph_upload_start = profile_scope!();
    let glyph_instance_buf = self
      .glyph_instance_buffer
      .write(device, queue, &self.scratch_glyph_instances);
    _buffer_upload_dur += profile_elapsed!(_glyph_upload_start);

    #[cfg(feature = "image")]
    let _image_upload_start = profile_scope!();
    #[cfg(feature = "image")]
    let image_instance_buf = {
      self.scratch_image_instances.clear();
      self.scratch_image_instances.reserve(list.images.len());
      self
        .scratch_image_instances
        .extend(list.images.iter().map(|img| ImageInstance {
          pos: [img.x, img.y],
          size: [img.width, img.height],
          opacity: [1.0, 0.0, 0.0, 0.0],
          transform: img.transform,
          xf_origin: img.transform_origin,
          uv_min: img.uv_min,
          uv_max: img.uv_max,
          radii: img.radii,
        }));
      self
        .image_instance_buffer
        .write(device, queue, &self.scratch_image_instances)
    };
    #[cfg(feature = "image")]
    {
      _buffer_upload_dur += profile_elapsed!(_image_upload_start);
    }

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

      self.scratch_ordered_draws.clear();
      self.scratch_ordered_draws.reserve(
        list.rects.len()
          + list.glyphs.len()
          + {
            #[cfg(feature = "image")]
            {
              list.images.len()
            }
            #[cfg(not(feature = "image"))]
            {
              0
            }
          }
          + {
            #[cfg(feature = "svg")]
            {
              list.svgs.len()
            }
            #[cfg(not(feature = "svg"))]
            {
              0
            }
          },
      );
      for (index, rect) in list.rects.iter().enumerate() {
        self.scratch_ordered_draws.push((rect.order, OrderedDraw::Rect(index)));
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
        self.scratch_ordered_draws.push((
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
        self
          .scratch_ordered_draws
          .push((image.order, OrderedDraw::Image(index)));
      }
      #[cfg(feature = "svg")]
      for (index, svg) in list.svgs.iter().enumerate() {
        self.scratch_ordered_draws.push((svg.order, OrderedDraw::Svg(index)));
      }
      self.scratch_ordered_draws.sort_by_key(|(order, _)| *order);

      for (_, draw) in &self.scratch_ordered_draws {
        match draw {
          OrderedDraw::Rect(index) => {
            let r = &list.rects[*index];
            if !set_scissor(&mut pass, r.clip, vw, vh) {
              continue;
            }
            let prepared = &self.scratch_rect_draws[*index];
            let start = (prepared.start * std::mem::size_of::<QuadInstance>()) as wgpu::BufferAddress;
            let end = ((prepared.start + prepared.count) * std::mem::size_of::<QuadInstance>()) as wgpu::BufferAddress;
            pass.set_pipeline(self.quad_pipeline.as_ref().unwrap());
            if rounded_clip_needs_shader(r.clip) {
              let (clip_key, clip_globals) =
                globals_buffer_for_clip(&mut self.clip_globals_cache, device, r.clip, vw, vh);
              let clip_bind_group = self.quad_clip_bind_groups.entry(clip_key).or_insert_with(|| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
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
                })
              });
              pass.set_bind_group(0, &*clip_bind_group, &[]);
            } else {
              pass.set_bind_group(0, self.quad_bind_group.as_ref().unwrap(), &[]);
            }
            pass.set_vertex_buffer(0, vtx_buf.slice(..));
            pass.set_vertex_buffer(1, rect_instance_buf.unwrap().slice(start..end));
            pass.set_index_buffer(idx_buf.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..6, 0, 0..prepared.count as u32);
          }
          OrderedDraw::Glyph { start, count } => {
            let glyph_slice = &list.glyphs[*start..*start + *count];
            if glyph_slice.is_empty() || !set_scissor(&mut pass, glyph_slice[0].clip, vw, vh) {
              continue;
            }
            let start_byte = (*start * std::mem::size_of::<GlyphInstance>()) as wgpu::BufferAddress;
            let end_byte = ((*start + *count) * std::mem::size_of::<GlyphInstance>()) as wgpu::BufferAddress;
            pass.set_pipeline(self.glyph_pipeline.as_ref().unwrap());
            if rounded_clip_needs_shader(glyph_slice[0].clip) {
              let (clip_key, clip_globals) =
                globals_buffer_for_clip(&mut self.clip_globals_cache, device, glyph_slice[0].clip, vw, vh);
              let clip_bind_group = self.glyph_clip_bind_groups.entry(clip_key).or_insert_with(|| {
                device.create_bind_group(&wgpu::BindGroupDescriptor {
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
                })
              });
              pass.set_bind_group(0, &*clip_bind_group, &[]);
            } else {
              pass.set_bind_group(0, self.glyph_bind_group.as_ref().unwrap(), &[]);
            }
            pass.set_vertex_buffer(0, vtx_buf.slice(..));
            pass.set_vertex_buffer(1, glyph_instance_buf.unwrap().slice(start_byte..end_byte));
            pass.set_index_buffer(idx_buf.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..6, 0, 0..*count as u32);
          }
          #[cfg(feature = "image")]
          OrderedDraw::Image(index) => {
            let img = &list.images[*index];
            let image_bgl = self.image_bgl.as_ref().unwrap().clone();
            let nv12_image_bgl = self.nv12_image_bgl.as_ref().unwrap().clone();
            let image_sampler = self.image_sampler.as_ref().unwrap().clone();
            let image_pipeline = self.image_pipeline.as_ref().unwrap().clone();
            let nv12_image_pipeline = self.nv12_image_pipeline.as_ref().unwrap().clone();

            if !self
              .image_texture_cache
              .get(&img.image_id)
              .is_some_and(|cached| cached.is_compatible(img))
            {
              self.image_texture_cache.remove(&img.image_id);
              self.image_clip_bind_groups.clear();
            }

            if !self.image_texture_cache.contains_key(&img.image_id) {
              let _image_texture_upload_start = profile_scope!();
              let cached = match img.image_format {
                crate::images::ImagePixelFormat::Rgba8 => Some(create_rgba_cached_image_texture(
                  device,
                  queue,
                  &image_bgl,
                  &image_sampler,
                  globals_buffer,
                  img,
                )),
                crate::images::ImagePixelFormat::Nv12 => {
                  create_nv12_cached_image_texture(device, queue, &nv12_image_bgl, &image_sampler, globals_buffer, img)
                }
              };
              _image_texture_upload_dur += profile_elapsed!(_image_texture_upload_start);
              let Some(cached) = cached else {
                continue;
              };
              self.image_texture_cache.insert(img.image_id, cached);
              self.image_clip_bind_groups.clear();
            }

            let Some(cached) = self.image_texture_cache.get_mut(&img.image_id) else {
              continue;
            };
            match cached {
              CachedImageTexture::Rgba {
                texture,
                animation_frames,
                frame_index,
                version,
                ..
              } => {
                if animation_frames.is_some() {
                  *frame_index = img.frame_index;
                  *version = img.version;
                } else if *frame_index != img.frame_index || *version != img.version {
                  let _image_texture_upload_start = profile_scope!();
                  write_rgba_image_texture(queue, texture, img);
                  _image_texture_upload_dur += profile_elapsed!(_image_texture_upload_start);
                  *frame_index = img.frame_index;
                  *version = img.version;
                }
              }
              CachedImageTexture::Nv12 {
                y_texture,
                uv_texture,
                frame_index,
                version,
                ..
              } => {
                if *frame_index != img.frame_index || *version != img.version {
                  let _image_texture_upload_start = profile_scope!();
                  if !write_nv12_image_textures(queue, y_texture, uv_texture, img) {
                    _image_texture_upload_dur += profile_elapsed!(_image_texture_upload_start);
                    continue;
                  }
                  _image_texture_upload_dur += profile_elapsed!(_image_texture_upload_start);
                  *frame_index = img.frame_index;
                  *version = img.version;
                }
              }
            }

            if !set_scissor(&mut pass, img.clip, vw, vh) {
              continue;
            }

            match cached {
              CachedImageTexture::Rgba {
                bind_group,
                view,
                animation_frames,
                ..
              } => {
                let (bind_group, view) = animation_frames
                  .as_ref()
                  .and_then(|frames| frames.get(img.frame_index))
                  .map(|frame| (&frame.bind_group, &frame.view))
                  .unwrap_or((bind_group, view));
                pass.set_pipeline(&image_pipeline);
                if rounded_clip_needs_shader(img.clip) {
                  let (clip_key, clip_globals) =
                    globals_buffer_for_clip(&mut self.clip_globals_cache, device, img.clip, vw, vh);
                  let bind_key = ImageClipBindGroupKey {
                    image_id: img.image_id,
                    frame_index: img.frame_index,
                    clip: clip_key,
                    format: ImageClipFormat::Rgba,
                  };
                  let clip_bind_group = self.image_clip_bind_groups.entry(bind_key).or_insert_with(|| {
                    device.create_bind_group(&wgpu::BindGroupDescriptor {
                      label: Some("lurq_image_clip_bg"),
                      layout: &image_bgl,
                      entries: &[
                        wgpu::BindGroupEntry {
                          binding: 0,
                          resource: clip_globals.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                          binding: 1,
                          resource: wgpu::BindingResource::TextureView(view),
                        },
                        wgpu::BindGroupEntry {
                          binding: 2,
                          resource: wgpu::BindingResource::Sampler(&image_sampler),
                        },
                      ],
                    })
                  });
                  pass.set_bind_group(0, &*clip_bind_group, &[]);
                } else {
                  pass.set_bind_group(0, &*bind_group, &[]);
                }
              }
              CachedImageTexture::Nv12 {
                bind_group,
                y_view,
                uv_view,
                ..
              } => {
                pass.set_pipeline(&nv12_image_pipeline);
                if rounded_clip_needs_shader(img.clip) {
                  let (clip_key, clip_globals) =
                    globals_buffer_for_clip(&mut self.clip_globals_cache, device, img.clip, vw, vh);
                  let bind_key = ImageClipBindGroupKey {
                    image_id: img.image_id,
                    frame_index: img.frame_index,
                    clip: clip_key,
                    format: ImageClipFormat::Nv12,
                  };
                  let clip_bind_group = self.image_clip_bind_groups.entry(bind_key).or_insert_with(|| {
                    device.create_bind_group(&wgpu::BindGroupDescriptor {
                      label: Some("lurq_nv12_image_clip_bg"),
                      layout: &nv12_image_bgl,
                      entries: &[
                        wgpu::BindGroupEntry {
                          binding: 0,
                          resource: clip_globals.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                          binding: 1,
                          resource: wgpu::BindingResource::TextureView(y_view),
                        },
                        wgpu::BindGroupEntry {
                          binding: 2,
                          resource: wgpu::BindingResource::TextureView(uv_view),
                        },
                        wgpu::BindGroupEntry {
                          binding: 3,
                          resource: wgpu::BindingResource::Sampler(&image_sampler),
                        },
                      ],
                    })
                  });
                  pass.set_bind_group(0, &*clip_bind_group, &[]);
                } else {
                  pass.set_bind_group(0, &*bind_group, &[]);
                }
              }
            }
            let image_instance_stride = std::mem::size_of::<ImageInstance>() as wgpu::BufferAddress;
            let image_instance_start = *index as wgpu::BufferAddress * image_instance_stride;
            let image_instance_end = image_instance_start + image_instance_stride;
            pass.set_vertex_buffer(0, vtx_buf.slice(..));
            pass.set_vertex_buffer(
              1,
              image_instance_buf
                .unwrap()
                .slice(image_instance_start..image_instance_end),
            );
            pass.set_index_buffer(idx_buf.slice(..), wgpu::IndexFormat::Uint16);
            pass.draw_indexed(0..6, 0, 0..1);
          }
          #[cfg(feature = "svg")]
          OrderedDraw::Svg(index) => {
            use vertex::SvgVertexGpu;

            let svg_cmd = &list.svgs[*index];
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
    let _encode_dur = profile_elapsed!(_encode_start);

    let _submit_start = profile_scope!();
    queue.submit(std::iter::once(encoder.finish()));
    let _submit_dur = profile_elapsed!(_submit_start);

    let _present_start = profile_scope!();
    output.present();
    let _present_dur = profile_elapsed!(_present_start);

    profile_if! {
      self.last_profile = RenderProfile {
        init: _init_dur,
        acquire: _acquire_dur,
        globals_upload: _globals_dur,
        atlas_upload: _atlas_dur,
        buffer_upload: _buffer_upload_dur,
        image_upload: _image_texture_upload_dur,
        encode: _encode_dur,
        submit: _submit_dur,
        present: _present_dur,
        total: profile_elapsed!(_total_start),
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

  fn last_profile(&self) -> Option<RenderProfile> {
    #[cfg(feature = "perf_profile")]
    {
      Some(self.last_profile)
    }
    #[cfg(not(feature = "perf_profile"))]
    {
      None
    }
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

#[cfg(feature = "image")]
fn create_rgba_cached_image_texture(
  device: &wgpu::Device,
  queue: &wgpu::Queue,
  image_bgl: &wgpu::BindGroupLayout,
  image_sampler: &wgpu::Sampler,
  globals_buffer: &wgpu::Buffer,
  image: &crate::images::ImageCmd,
) -> CachedImageTexture {
  let frame = create_rgba_frame_texture(
    device,
    queue,
    image_bgl,
    image_sampler,
    globals_buffer,
    image.image_width,
    image.image_height,
    &image.data,
  );
  let animation_frames = image.animation_frames.as_ref().and_then(|frames| {
    (frames.len() > 1).then(|| {
      frames
        .iter()
        .map(|frame| {
          create_rgba_frame_texture(
            device,
            queue,
            image_bgl,
            image_sampler,
            globals_buffer,
            image.image_width,
            image.image_height,
            frame,
          )
        })
        .collect()
    })
  });

  CachedImageTexture::Rgba {
    bind_group: frame.bind_group,
    view: frame.view,
    texture: frame.texture,
    animation_frames,
    width: image.image_width,
    height: image.image_height,
    frame_index: image.frame_index,
    version: image.version,
  }
}

#[cfg(feature = "image")]
fn create_rgba_frame_texture(
  device: &wgpu::Device,
  queue: &wgpu::Queue,
  image_bgl: &wgpu::BindGroupLayout,
  image_sampler: &wgpu::Sampler,
  globals_buffer: &wgpu::Buffer,
  width: u32,
  height: u32,
  data: &[u8],
) -> CachedRgbaFrame {
  let texture = device.create_texture(&wgpu::TextureDescriptor {
    label: Some("lurq_img"),
    size: wgpu::Extent3d {
      width,
      height,
      depth_or_array_layers: 1,
    },
    mip_level_count: 1,
    sample_count: 1,
    dimension: wgpu::TextureDimension::D2,
    format: wgpu::TextureFormat::Rgba8UnormSrgb,
    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    view_formats: &[],
  });
  write_rgba_texture_data(queue, &texture, width, height, data);
  let view = texture.create_view(&Default::default());
  let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: Some("lurq_img_bg"),
    layout: image_bgl,
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
        resource: wgpu::BindingResource::Sampler(image_sampler),
      },
    ],
  });
  CachedRgbaFrame {
    bind_group,
    view,
    texture,
  }
}

#[cfg(feature = "image")]
fn create_nv12_cached_image_texture(
  device: &wgpu::Device,
  queue: &wgpu::Queue,
  nv12_image_bgl: &wgpu::BindGroupLayout,
  image_sampler: &wgpu::Sampler,
  globals_buffer: &wgpu::Buffer,
  image: &crate::images::ImageCmd,
) -> Option<CachedImageTexture> {
  if image.image_width == 0 || image.image_height == 0 || image.image_width % 2 != 0 || image.image_height % 2 != 0 {
    return None;
  }
  #[cfg(target_os = "macos")]
  if matches!(
    image.native.as_ref().map(crate::images::NativeImageData::backend),
    Some(crate::images::NativeImageBackend::MacosCvPixelBufferNv12)
  ) {
    return create_macos_native_nv12_cached_image_texture(device, nv12_image_bgl, image_sampler, globals_buffer, image);
  }

  let y_texture = device.create_texture(&wgpu::TextureDescriptor {
    label: Some("lurq_nv12_y"),
    size: wgpu::Extent3d {
      width: image.image_width,
      height: image.image_height,
      depth_or_array_layers: 1,
    },
    mip_level_count: 1,
    sample_count: 1,
    dimension: wgpu::TextureDimension::D2,
    format: wgpu::TextureFormat::R8Unorm,
    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    view_formats: &[],
  });
  let uv_texture = device.create_texture(&wgpu::TextureDescriptor {
    label: Some("lurq_nv12_uv"),
    size: wgpu::Extent3d {
      width: image.image_width / 2,
      height: image.image_height / 2,
      depth_or_array_layers: 1,
    },
    mip_level_count: 1,
    sample_count: 1,
    dimension: wgpu::TextureDimension::D2,
    format: wgpu::TextureFormat::Rg8Unorm,
    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    view_formats: &[],
  });
  if !write_nv12_image_textures(queue, &y_texture, &uv_texture, image) {
    return None;
  }
  let y_view = y_texture.create_view(&Default::default());
  let uv_view = uv_texture.create_view(&Default::default());
  let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: Some("lurq_nv12_img_bg"),
    layout: nv12_image_bgl,
    entries: &[
      wgpu::BindGroupEntry {
        binding: 0,
        resource: globals_buffer.as_entire_binding(),
      },
      wgpu::BindGroupEntry {
        binding: 1,
        resource: wgpu::BindingResource::TextureView(&y_view),
      },
      wgpu::BindGroupEntry {
        binding: 2,
        resource: wgpu::BindingResource::TextureView(&uv_view),
      },
      wgpu::BindGroupEntry {
        binding: 3,
        resource: wgpu::BindingResource::Sampler(image_sampler),
      },
    ],
  });
  Some(CachedImageTexture::Nv12 {
    bind_group,
    y_view,
    uv_view,
    y_texture,
    uv_texture,
    #[cfg(target_os = "macos")]
    _macos_native: None,
    width: image.image_width,
    height: image.image_height,
    frame_index: image.frame_index,
    version: image.version,
  })
}

#[cfg(all(feature = "image", target_os = "macos"))]
fn create_macos_native_nv12_cached_image_texture(
  device: &wgpu::Device,
  nv12_image_bgl: &wgpu::BindGroupLayout,
  image_sampler: &wgpu::Sampler,
  globals_buffer: &wgpu::Buffer,
  image: &crate::images::ImageCmd,
) -> Option<CachedImageTexture> {
  let native = image.native.as_ref()?.clone();
  let payload = native.payload::<crate::images::MacosCvPixelBufferNv12Image>()?;
  let (y_texture, y_cv_texture) = create_macos_native_plane_texture(
    device,
    payload.pixel_buffer.as_ptr(),
    image.image_width,
    image.image_height,
    0,
    metal::MTLPixelFormat::R8Unorm,
    wgpu::TextureFormat::R8Unorm,
    "lurq_macos_nv12_y",
  )?;
  let (uv_texture, uv_cv_texture) = create_macos_native_plane_texture(
    device,
    payload.pixel_buffer.as_ptr(),
    image.image_width / 2,
    image.image_height / 2,
    1,
    metal::MTLPixelFormat::RG8Unorm,
    wgpu::TextureFormat::Rg8Unorm,
    "lurq_macos_nv12_uv",
  )?;
  let y_view = y_texture.create_view(&Default::default());
  let uv_view = uv_texture.create_view(&Default::default());
  let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
    label: Some("lurq_macos_nv12_img_bg"),
    layout: nv12_image_bgl,
    entries: &[
      wgpu::BindGroupEntry {
        binding: 0,
        resource: globals_buffer.as_entire_binding(),
      },
      wgpu::BindGroupEntry {
        binding: 1,
        resource: wgpu::BindingResource::TextureView(&y_view),
      },
      wgpu::BindGroupEntry {
        binding: 2,
        resource: wgpu::BindingResource::TextureView(&uv_view),
      },
      wgpu::BindGroupEntry {
        binding: 3,
        resource: wgpu::BindingResource::Sampler(image_sampler),
      },
    ],
  });
  Some(CachedImageTexture::Nv12 {
    bind_group,
    y_view,
    uv_view,
    y_texture,
    uv_texture,
    _macos_native: Some(MacosNativeNv12Texture {
      _native: native,
      _y_cv_texture: y_cv_texture,
      _uv_cv_texture: uv_cv_texture,
    }),
    width: image.image_width,
    height: image.image_height,
    frame_index: image.frame_index,
    version: image.version,
  })
}

#[cfg(all(feature = "image", target_os = "macos"))]
fn create_macos_native_plane_texture(
  device: &wgpu::Device,
  pixel_buffer: CVPixelBufferRef,
  width: u32,
  height: u32,
  plane_index: usize,
  metal_format: metal::MTLPixelFormat,
  wgpu_format: wgpu::TextureFormat,
  label: &'static str,
) -> Option<(wgpu::Texture, CvMetalTexture)> {
  let mut raw_device = None;
  unsafe {
    device.as_hal::<wgpu_hal::api::Metal, _, _>(|hal_device| {
      raw_device = hal_device.map(|device| device.raw_device().lock().clone());
    });
  }
  let raw_device = raw_device?;

  let mut cache = std::ptr::null_mut();
  let status = unsafe {
    CVMetalTextureCacheCreate(
      kCFAllocatorDefault,
      std::ptr::null(),
      raw_device.as_ptr().cast(),
      std::ptr::null(),
      &mut cache,
    )
  };
  if status != 0 || cache.is_null() {
    return None;
  }

  let mut cv_texture = std::ptr::null_mut();
  let status = unsafe {
    CVMetalTextureCacheCreateTextureFromImage(
      kCFAllocatorDefault,
      cache,
      pixel_buffer,
      std::ptr::null(),
      metal_format,
      width as usize,
      height as usize,
      plane_index,
      &mut cv_texture,
    )
  };
  unsafe {
    CFRelease(cache.cast());
  }
  if status != 0 || cv_texture.is_null() {
    return None;
  }

  let metal_texture_ptr = unsafe { CVMetalTextureGetTexture(cv_texture) };
  if metal_texture_ptr.is_null() {
    unsafe {
      CFRelease(cv_texture.cast());
    }
    return None;
  }
  let metal_texture = unsafe {
    core_foundation_sys::base::CFRetain(metal_texture_ptr.cast::<c_void>().cast::<std::ffi::c_void>());
    metal::Texture::from_ptr(metal_texture_ptr.cast())
  };
  let hal_texture = unsafe {
    wgpu_hal::metal::Device::texture_from_raw(
      metal_texture,
      wgpu_format,
      metal::MTLTextureType::D2,
      1,
      1,
      wgpu_hal::CopyExtent {
        width,
        height,
        depth: 1,
      },
    )
  };
  let texture = unsafe {
    device.create_texture_from_hal::<wgpu_hal::api::Metal>(
      hal_texture,
      &wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
          width,
          height,
          depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu_format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
      },
    )
  };
  Some((texture, CvMetalTexture { ptr: cv_texture }))
}

#[cfg(feature = "image")]
fn write_rgba_image_texture(queue: &wgpu::Queue, texture: &wgpu::Texture, image: &crate::images::ImageCmd) {
  write_rgba_texture_data(queue, texture, image.image_width, image.image_height, &image.data);
}

#[cfg(feature = "image")]
fn write_rgba_texture_data(queue: &wgpu::Queue, texture: &wgpu::Texture, width: u32, height: u32, data: &[u8]) {
  queue.write_texture(
    wgpu::TexelCopyTextureInfo {
      texture,
      mip_level: 0,
      origin: wgpu::Origin3d::ZERO,
      aspect: wgpu::TextureAspect::All,
    },
    data,
    wgpu::TexelCopyBufferLayout {
      offset: 0,
      bytes_per_row: Some(width * 4),
      rows_per_image: Some(height),
    },
    wgpu::Extent3d {
      width,
      height,
      depth_or_array_layers: 1,
    },
  );
}

#[cfg(feature = "image")]
fn write_nv12_image_textures(
  queue: &wgpu::Queue,
  y_texture: &wgpu::Texture,
  uv_texture: &wgpu::Texture,
  image: &crate::images::ImageCmd,
) -> bool {
  let y_len = image.image_width as usize * image.image_height as usize;
  let uv_len = y_len / 2;
  if image.data.len() < y_len + uv_len {
    return false;
  }
  queue.write_texture(
    wgpu::TexelCopyTextureInfo {
      texture: y_texture,
      mip_level: 0,
      origin: wgpu::Origin3d::ZERO,
      aspect: wgpu::TextureAspect::All,
    },
    &image.data[..y_len],
    wgpu::TexelCopyBufferLayout {
      offset: 0,
      bytes_per_row: Some(image.image_width),
      rows_per_image: Some(image.image_height),
    },
    wgpu::Extent3d {
      width: image.image_width,
      height: image.image_height,
      depth_or_array_layers: 1,
    },
  );
  queue.write_texture(
    wgpu::TexelCopyTextureInfo {
      texture: uv_texture,
      mip_level: 0,
      origin: wgpu::Origin3d::ZERO,
      aspect: wgpu::TextureAspect::All,
    },
    &image.data[y_len..y_len + uv_len],
    wgpu::TexelCopyBufferLayout {
      offset: 0,
      bytes_per_row: Some(image.image_width),
      rows_per_image: Some(image.image_height / 2),
    },
    wgpu::Extent3d {
      width: image.image_width / 2,
      height: image.image_height / 2,
      depth_or_array_layers: 1,
    },
  );
  true
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

fn globals_buffer_for_clip<'a>(
  cache: &'a mut std::collections::HashMap<ClipGlobalsKey, wgpu::Buffer>,
  device: &wgpu::Device,
  clip: crate::layout::quad::ClipRect,
  vw: f32,
  vh: f32,
) -> (ClipGlobalsKey, &'a wgpu::Buffer) {
  let globals = globals_for_clip(clip, vw, vh);
  let key = clip_globals_key(&globals);
  let buffer = cache.entry(key).or_insert_with(|| {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("lurq_clip_globals"),
      contents: bytemuck::bytes_of(&globals),
      usage: wgpu::BufferUsages::UNIFORM,
    })
  });
  (key, buffer)
}

fn clip_globals_key(globals: &Globals) -> ClipGlobalsKey {
  ClipGlobalsKey(
    [
      globals.viewport[0],
      globals.viewport[1],
      globals.viewport[2],
      globals.viewport[3],
      globals.clip_rect[0],
      globals.clip_rect[1],
      globals.clip_rect[2],
      globals.clip_rect[3],
      globals.clip_radii_h[0],
      globals.clip_radii_h[1],
      globals.clip_radii_h[2],
      globals.clip_radii_h[3],
      globals.clip_radii_v[0],
      globals.clip_radii_v[1],
      globals.clip_radii_v[2],
      globals.clip_radii_v[3],
      globals.clip_active[0],
      globals.clip_active[1],
      globals.clip_active[2],
      globals.clip_active[3],
    ]
    .map(f32::to_bits),
  )
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
