mod app_errors;
#[cfg(feature = "canvas")]
mod canvas;
#[cfg(all(windows, feature = "raster"))]
mod card_copies;
#[cfg(all(windows, feature = "raster"))]
mod card_frames;
#[cfg(all(test, windows, feature = "raster", feature = "media"))]
mod card_frames_tests;
mod device_loss;
mod extension;
#[cfg(feature = "perf_profile")]
mod gpu_timing;
mod gradients;
mod layers;
#[cfg(test)]
mod quad_tests;
mod surface_proxy;
mod vertex;
#[cfg(all(test, windows, feature = "screenshot"))]
mod viewport_tests;

#[cfg(all(feature = "raster", any(target_os = "macos", target_os = "ios")))]
use std::ffi::c_void;
use std::time::Duration;

#[cfg(all(feature = "raster", any(target_os = "macos", target_os = "ios")))]
use core_foundation_sys::base::{CFAllocatorRef, CFRelease, OSStatus, kCFAllocatorDefault};
#[cfg(all(feature = "raster", any(target_os = "macos", target_os = "ios")))]
use core_foundation_sys::dictionary::CFDictionaryRef;
#[cfg(all(feature = "raster", any(target_os = "macos", target_os = "ios")))]
use core_video_sys::pixel_buffer::CVPixelBufferRef;
use extension::WgpuFrameExtensionEntry;
pub use extension::{SharedWgpuContext, WgpuAppError, WgpuFrameExtension, WgpuFrameInfo, WgpuViewportRect};
#[cfg(all(feature = "raster", any(target_os = "macos", target_os = "ios")))]
use objc2::{rc::Retained, runtime::ProtocolObject};
use raw_window_handle::{DisplayHandle, WindowHandle};
#[cfg(feature = "raster")]
use vertex::ImageInstance;
use vertex::{Globals, GlyphInstance, QuadInstance, QuadVertex};
use wgpu::util::DeviceExt;
pub use wgpu::{
  Backend as WgpuBackend, Backends as WgpuBackends, Features as WgpuFeatures, Limits as WgpuLimits,
  PresentMode as WgpuPresentMode,
};

#[cfg(feature = "perf_profile")]
use crate::app::profile_types::RenderProfile;
#[cfg(feature = "screenshot")]
use crate::app::render_engine::RenderFrameCapture;
use crate::{
  app::{
    profile_support::{profile_elapsed, profile_if, profile_scope},
    render_engine::RenderEngine,
  },
  layout::{
    backdrop::BackdropRegion,
    backdrop_reuse::{BackdropInputs, PaintedDraw, backdrop_regions},
    opacity_layer::{LayerPlan, LayerStep, TargetSpace},
    render_list::RenderList,
  },
  render::glyph_batch::{GlyphRun, batch_glyph_runs, glyph_runs},
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
  if previous == next {
    return None;
  }

  let start = previous.iter().zip(next).position(|(a, b)| a != b).unwrap_or(0);
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
  Glyph(GlyphRun),
  #[cfg(feature = "raster")]
  Image(usize),
}

impl OrderedDraw {
  /// The draw as the inputs of backdrop blurs are told from it.
  fn painted(&self) -> PaintedDraw {
    match *self {
      Self::Rect(index) => PaintedDraw::Rect(index),
      Self::Glyph(GlyphRun { start, count, .. }) => PaintedDraw::Glyphs { start, count },
      #[cfg(feature = "raster")]
      Self::Image(index) => PaintedDraw::Image(index),
    }
  }

  fn glyph_run(&self) -> Option<GlyphRun> {
    match self {
      Self::Glyph(run) => Some(*run),
      _ => None,
    }
  }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ActivePipeline {
  Quad,
  Glyph,
  #[cfg(feature = "raster")]
  Image,
  #[cfg(feature = "raster")]
  Nv12Image,
  Layer,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ActiveVertexBuffer {
  CommonQuad,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ActiveIndexBuffer {
  CommonU16,
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct ClipGlobalsKey([u32; 28]);

#[cfg(feature = "raster")]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
enum ImageClipFormat {
  Rgba,
  Nv12,
}

#[cfg(feature = "raster")]
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct ImageClipBindGroupKey {
  image_id: u64,
  frame_index: usize,
  clip: ClipGlobalsKey,
  format: ImageClipFormat,
}

#[cfg(feature = "raster")]
struct CachedRgbaFrame {
  bind_group: wgpu::BindGroup,
  view: wgpu::TextureView,
  texture: wgpu::Texture,
}

#[cfg(feature = "raster")]
enum CachedImageTexture {
  ExternalRgba {
    bind_group: wgpu::BindGroup,
    view: wgpu::TextureView,
    width: u32,
    height: u32,
    version: u64,
  },
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
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    _macos_native: Option<MacosNativeNv12Texture>,
    width: u32,
    height: u32,
    frame_index: usize,
    version: u64,
  },
  /// The frames of a video that stay on the card: each of its surfaces opened once, and the one that holds the
  /// frame to draw drawn.
  #[cfg(windows)]
  Card(card_frames::Opened),
}

#[cfg(all(feature = "raster", any(target_os = "macos", target_os = "ios")))]
struct MacosNativeNv12Texture {
  _native: crate::images::NativeImageData,
  /// The buffer the textures are of: held for as long as they are drawn, so that what decodes does not write the
  /// next frame into it.
  _buffer: crate::images::MacosCvPixelBuffer,
  _y_cv_texture: CvMetalTexture,
  _uv_cv_texture: CvMetalTexture,
}

#[cfg(all(feature = "raster", any(target_os = "macos", target_os = "ios")))]
struct CvMetalTexture {
  ptr: CVMetalTextureRef,
}

#[cfg(all(feature = "raster", any(target_os = "macos", target_os = "ios")))]
impl Drop for CvMetalTexture {
  fn drop(&mut self) {
    unsafe {
      CFRelease(self.ptr.cast());
    }
  }
}

/// The cache CoreVideo makes Metal's textures of a buffer with, for one device.
#[cfg(all(feature = "raster", any(target_os = "macos", target_os = "ios")))]
struct CvMetalTextureCache {
  ptr: CVMetalTextureCacheRef,
}

#[cfg(all(feature = "raster", any(target_os = "macos", target_os = "ios")))]
impl Drop for CvMetalTextureCache {
  fn drop(&mut self) {
    unsafe {
      CFRelease(self.ptr.cast());
    }
  }
}

#[cfg(all(feature = "raster", any(target_os = "macos", target_os = "ios")))]
thread_local! {
  /// The cache of the device that draws on this thread, with the device it is of.
  static CV_TEXTURE_CACHE: std::cell::RefCell<Option<(usize, CvMetalTextureCache)>> =
    const { std::cell::RefCell::new(None) };
}

#[cfg(all(feature = "raster", any(target_os = "macos", target_os = "ios")))]
type CVMetalTextureCacheRef = *mut c_void;
#[cfg(all(feature = "raster", any(target_os = "macos", target_os = "ios")))]
type CVMetalTextureRef = *mut c_void;

#[cfg(all(feature = "raster", any(target_os = "macos", target_os = "ios")))]
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
    pixel_format: objc2_metal::MTLPixelFormat,
    width: usize,
    height: usize,
    plane_index: usize,
    texture_out: *mut CVMetalTextureRef,
  ) -> OSStatus;

  fn CVMetalTextureGetTexture(image: CVMetalTextureRef) -> *mut c_void;
}

/// The part of its texture `image` is drawn from: the part it asks for, within the picture's part of the
/// texture where its frames stay on the card and the picture is not all of it.
#[cfg(feature = "raster")]
fn drawn_part(
  cache: &std::collections::HashMap<u64, CachedImageTexture>,
  image: &crate::images::ImageCmd,
) -> ([f32; 2], [f32; 2]) {
  #[cfg(windows)]
  if let Some(CachedImageTexture::Card(opened)) = cache.get(&image.image_id) {
    return opened.part(image.uv_min, image.uv_max);
  }
  let _ = cache;
  (image.uv_min, image.uv_max)
}

#[cfg(feature = "raster")]
impl CachedImageTexture {
  /// Whether it is the textures of a frame that stays on the card, and `image` shows another frame now.
  #[cfg(any(target_os = "macos", target_os = "ios"))]
  fn is_another_frame_of_the_card(&self, image: &crate::images::ImageCmd) -> bool {
    matches!(self, Self::Nv12 { _macos_native: Some(_), version, .. } if *version != image.version)
  }

  fn is_compatible(&self, image: &crate::images::ImageCmd) -> bool {
    match self {
      Self::ExternalRgba { .. } => false,
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
          && !is_card_frames(image)
      }
      // Frames of memory are not frames of the card, though both are NV12.
      Self::Nv12 { width, height, .. } => {
        image.image_format == crate::images::ImagePixelFormat::Nv12
          && *width == image.image_width
          && *height == image.image_height
          && !is_card_frames(image)
      }
      #[cfg(windows)]
      Self::Card(opened) => {
        card_frames::of(image).is_some_and(|frames| opened.is_of(frames, (image.image_width, image.image_height)))
      }
    }
  }
}

/// Whether `image` is frames that stay on the card.
#[cfg(feature = "raster")]
fn is_card_frames(image: &crate::images::ImageCmd) -> bool {
  #[cfg(windows)]
  {
    image
      .native
      .as_ref()
      .is_some_and(|native| native.backend() == crate::images::NativeImageBackend::CardFrames)
  }
  #[cfg(not(windows))]
  {
    let _ = image;
    false
  }
}

pub struct WgpuRenderEngine {
  #[cfg(feature = "canvas")]
  canvases: Vec<crate::canvas::CanvasHandle>,
  #[cfg(feature = "canvas")]
  canvas_renderer: Option<canvas::Renderer>,
  #[cfg(feature = "canvas")]
  canvas_asset_budget: crate::canvas::CanvasAssetBudget,
  instance: Option<wgpu::Instance>,
  backends: wgpu::Backends,
  /// A device made ahead of the engine, which draws with it instead of asking for one; see [`WgpuDevice`]. Or
  /// the device of the app's other windows, which this shares.
  prepared: Option<WgpuDevice>,
  /// Whether the device is this engine's alone, and not the one the engines of the app's other windows share.
  own_device: bool,
  /// The window's physical pixels to one logical pixel, which the extensions are told.
  scale_factor: f32,
  /// The errors of the app's own calls on the device that no extension was told of yet.
  untold: app_errors::Untold,
  /// Whether the device is lost: the card was reset or taken away.
  lost: std::sync::Arc<std::sync::atomic::AtomicBool>,
  /// How many frames to let pass before a device that was not given is asked for again.
  device_wait: u32,
  /// The device's number in the order of devices and of the views apps give.
  device_number: u64,
  /// Whether the device is of another card than the one a device was lost on, which made none in time.
  displaced: bool,
  /// Whether the device was made while something held the lost one still: it is not a whole one.
  made_while_held: bool,
  /// When that card was last asked whether it makes a device again, and how long until it is asked next.
  #[cfg(not(target_arch = "wasm32"))]
  card_asked: Option<(std::time::Instant, std::time::Duration)>,
  /// Whether that card makes a device again: this engine is to be replaced by one that draws on it.
  card_is_back: bool,
  /// The slots whose views this drew: where the device is lost, their views are let go of.
  #[cfg(feature = "raster")]
  external_slots: Vec<crate::images::WgpuExternalImageWeak>,
  adapter: Option<wgpu::Adapter>,
  device: Option<wgpu::Device>,
  queue: Option<wgpu::Queue>,
  optional_device_features: wgpu::Features,
  optional_device_limits: wgpu::Limits,
  requested_present_mode: wgpu::PresentMode,
  quad_pipeline: Option<wgpu::RenderPipeline>,
  glyph_pipeline: Option<wgpu::RenderPipeline>,
  #[cfg(feature = "raster")]
  image_pipeline: Option<wgpu::RenderPipeline>,
  #[cfg(feature = "raster")]
  nv12_image_pipeline: Option<wgpu::RenderPipeline>,
  surface: Option<wgpu::Surface<'static>>,
  surface_config: Option<wgpu::SurfaceConfiguration>,
  surface_format: Option<wgpu::TextureFormat>,
  quad_bgl: Option<wgpu::BindGroupLayout>,
  glyph_bgl: Option<wgpu::BindGroupLayout>,
  #[cfg(feature = "raster")]
  image_bgl: Option<wgpu::BindGroupLayout>,
  #[cfg(feature = "raster")]
  nv12_image_bgl: Option<wgpu::BindGroupLayout>,
  #[cfg(feature = "raster")]
  image_sampler: Option<wgpu::Sampler>,
  #[cfg(feature = "raster")]
  image_texture_cache: std::collections::HashMap<u64, CachedImageTexture>,
  /// The textures of the frames of a video that were drawn in the frame before: let go once this one is drawn.
  #[cfg(any(target_os = "macos", target_os = "ios"))]
  retired_card_frames: Vec<CachedImageTexture>,
  /// The leases of the card's frames that are drawn no more, until the card has finished with them.
  #[cfg(all(feature = "raster", windows))]
  card_leases: card_frames::Leases,
  /// A device on each of the machine's cards, for frames the renderer's own card does not open.
  #[cfg(all(feature = "raster", windows))]
  card_readers: card_copies::Readers,
  #[cfg(feature = "raster")]
  image_clip_bind_groups: std::collections::HashMap<ImageClipBindGroupKey, wgpu::BindGroup>,
  globals_buffer: Option<wgpu::Buffer>,
  clip_globals: ClipGlobalsCache,
  quad_clip_bind_groups: std::collections::HashMap<ClipGlobalsKey, wgpu::BindGroup>,
  glyph_clip_bind_groups: std::collections::HashMap<ClipGlobalsKey, wgpu::BindGroup>,
  gradients: Option<gradients::Gradients>,
  last_globals_bytes: Vec<u8>,
  last_gradient_bytes: Vec<u8>,
  atlas_texture: Option<wgpu::Texture>,
  atlas_view: Option<wgpu::TextureView>,
  atlas_sampler: Option<wgpu::Sampler>,
  atlas_size: (u32, u32),
  atlas_version: u64,
  #[cfg(feature = "perf_profile")]
  last_profile: RenderProfile,
  #[cfg(feature = "perf_profile")]
  last_layer_profile: crate::app::profile_types::LayerProfile,
  #[cfg(feature = "perf_profile")]
  profile_context: Option<crate::app::profiler::ProfileContext>,
  /// GPU time of frames rendered while a profiling session is active.
  #[cfg(feature = "perf_profile")]
  gpu_timer: gpu_timing::GpuTimer,
  quad_bind_group: Option<wgpu::BindGroup>,
  glyph_bind_group: Option<wgpu::BindGroup>,
  vertex_buffer: Option<wgpu::Buffer>,
  index_buffer: Option<wgpu::Buffer>,
  rect_instance_buffer: DynamicBuffer,
  glyph_instance_buffer: DynamicBuffer,
  #[cfg(feature = "raster")]
  image_instance_buffer: DynamicBuffer,
  scratch_rect_instances: Vec<QuadInstance>,
  scratch_rect_draws: Vec<PreparedDraw>,
  scratch_gradient_data: Vec<[f32; 4]>,
  scratch_glyph_instances: Vec<GlyphInstance>,
  #[cfg(feature = "raster")]
  scratch_image_instances: Vec<ImageInstance>,
  scratch_ordered_draws: Vec<(usize, OrderedDraw)>,
  layer_plan: LayerPlan,
  /// Offscreen targets of faded subtrees; created by the first frame that
  /// has one.
  layer_compositor: Option<layers::LayerCompositor>,
  scratch_layer_slots: Vec<Option<usize>>,
  scratch_blur_slots: Vec<Option<layers::BlurSlots>>,
  scratch_backdrop_slots: Vec<Option<layers::BackdropSlots>>,
  scratch_backdrop_regions: Vec<Option<BackdropRegion>>,
  /// What each backdrop blur of the frame reads, to tell whether its kept
  /// result is still the blur of it.
  backdrop_inputs: BackdropInputs,
  width: u32,
  height: u32,
  frame_extensions: Vec<WgpuFrameExtensionEntry>,
  /// Whether the surface's texture can be copied from; `false` on WebGL.
  surface_copies: bool,
  /// A test draws as on such a surface; see `without_surface_copies`.
  #[cfg(test)]
  deny_surface_copies: bool,
  /// What a frame that is read back is painted into where it cannot; see `surface_proxy`.
  surface_proxy: Option<surface_proxy::SurfaceProxy>,
  #[cfg(feature = "screenshot")]
  pending_frame_capture: Option<RenderFrameCapture>,
}

fn default_backends() -> WgpuBackends {
  #[cfg(windows)]
  {
    WgpuBackends::DX12
  }
  #[cfg(not(windows))]
  {
    WgpuBackends::all()
  }
}

fn instance_descriptor(backends: WgpuBackends) -> wgpu::InstanceDescriptor {
  let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
  descriptor.backends = backends;
  // lurq waits for the frame latency object itself; see `wait_for_frame_latency`.
  descriptor.backend_options.dx12.latency_waitable_object = wgpu::Dx12UseFrameLatencyWaitableObject::DontWait;
  descriptor
}

/// The instances of wgpu the program has, by the backends each was made among.
#[cfg(not(target_arch = "wasm32"))]
static INSTANCES: std::sync::Mutex<Vec<(WgpuBackends, wgpu::Instance)>> = std::sync::Mutex::new(Vec::new());

/// The instance of wgpu among `backends`: one for the program, which every engine and every device made
/// ahead shares.
///
/// Each engine had its own, and so each window. Under Vulkan a program with two of them alive ended with an
/// access violation when one was let go of, on the card this was run on, each time it was tried; one
/// instance with a device for each window does not. Under Direct3D 12 either way holds.
#[cfg(not(target_arch = "wasm32"))]
fn new_instance(backends: WgpuBackends) -> wgpu::Instance {
  let mut instances = INSTANCES.lock().unwrap();
  if let Some((_, instance)) = instances.iter().find(|(among, _)| *among == backends) {
    return instance.clone();
  }
  let instance = wgpu::Instance::new(instance_descriptor(backends));
  instances.push((backends, instance.clone()));
  instance
}

/// After a device is lost, an instance of Direct3D 12 alone is made anew for what takes the device's place,
/// as it was before the instance was shared: what it knows of the cards is from before the loss. One that
/// may draw through Vulkan is kept, for a second one beside it is what ends the program there.
#[cfg(not(target_arch = "wasm32"))]
fn forget_instances_after_a_loss() {
  INSTANCES
    .lock()
    .unwrap()
    .retain(|(among, _)| among.contains(WgpuBackends::VULKAN));
}

#[cfg(target_arch = "wasm32")]
fn new_instance(backends: WgpuBackends) -> wgpu::Instance {
  wgpu::Instance::new(instance_descriptor(backends))
}

/// The instance of a page. Whether a browser has WebGPU is known only once it was asked for an adapter, and an
/// instance is of one kind from the start: with the `webgl` feature the browser is asked first, and the instance
/// is WebGL's when it has no WebGPU.
#[cfg(target_arch = "wasm32")]
async fn new_page_instance(backends: WgpuBackends) -> wgpu::Instance {
  #[cfg(feature = "webgl")]
  return wgpu::util::new_instance_with_webgpu_detection(instance_descriptor(backends)).await;
  #[cfg(not(feature = "webgl"))]
  new_instance(backends)
}

/// The device an engine asks `adapter` for: the optional features and limits as far as the adapter has them.
fn device_descriptor(
  adapter: &wgpu::Adapter,
  optional_features: wgpu::Features,
  optional_limits: &wgpu::Limits,
) -> wgpu::DeviceDescriptor<'static> {
  // A frame of a video that stays on the card is an NV12 texture, where the card has them.
  #[cfg(all(windows, feature = "raster"))]
  let optional_features = optional_features
    | wgpu::Features::TEXTURE_FORMAT_NV12
    // Under Vulkan such a frame is a texture of Direct3D given to it by its handle.
    | wgpu::Features::VULKAN_EXTERNAL_MEMORY_WIN32;
  #[cfg_attr(not(feature = "perf_profile"), allow(unused_mut))]
  let mut required_features = adapter.features() & optional_features;
  // Measures GPU time while a profiling session is active.
  #[cfg(feature = "perf_profile")]
  {
    required_features |= gpu_timing::GpuTimer::requested_features(adapter);
  }
  wgpu::DeviceDescriptor {
    label: Some("lurq"),
    required_features,
    required_limits: optional_limits.clone().or_worse_values_from(&adapter.limits()),
    ..Default::default()
  }
}

/// How long after an engine went to another card the card a device was lost on is first asked whether it
/// makes a device again, and the longest between two such questions: each takes the time of making a device.
#[cfg(not(target_arch = "wasm32"))]
const CARD_ASKED_FIRST: std::time::Duration = std::time::Duration::from_secs(3);
#[cfg(not(target_arch = "wasm32"))]
const CARD_ASKED_AT_MOST: std::time::Duration = std::time::Duration::from_secs(60);

/// A GPU device made ahead of the engine that draws with it.
///
/// An engine asks for its device when it draws the first frame and waits for
/// the answer. A browser answers through a promise, which a page cannot wait
/// for: there the app awaits [`WgpuDevice::request`] before it starts its
/// window, and gives the device to each engine with
/// [`WgpuRenderEngine::with_device`]. Elsewhere this is optional: it moves
/// the wait off the first frame.
///
/// In a browser the device is the one of the page's canvas, the canvas the
/// window then draws to: WebGPU's where the browser has it, and with the
/// `webgl` feature WebGL2's where it has not. [`WgpuDevice::backend`] says
/// which.
///
/// A device that is lost (its card was reset or taken away) is made again, once for all the engines that
/// share it: the engine that takes the place of a lost one asks this for its device, and is given another.
/// Not in a browser, where a device is not waited for.
///
/// ```no_run
/// # async fn start() {
/// use lurq::app::{
///   Tree,
///   wgpu_render::{WgpuDevice, WgpuRenderEngine},
/// };
///
/// let device = WgpuDevice::request().await.expect("a GPU device");
/// let mut tree = Tree::new();
/// tree.set_render_engine_factory(move || Box::new(WgpuRenderEngine::new().with_device(device.clone())));
/// # }
/// ```
#[derive(Clone)]
pub struct WgpuDevice {
  /// The device as it is now: another one after a loss, and none while the card gives none.
  made: std::sync::Arc<parking_lot::Mutex<Option<MadeDevice>>>,
  /// Whether the card the device was lost on makes a device again, as an engine on another card found: the
  /// device of the other card is let go of for one of that card.
  back: std::sync::Arc<std::sync::atomic::AtomicBool>,
  /// Among which the device was asked for, and is asked for again, with these features where the card has them
  /// and these limits.
  backends: WgpuBackends,
  features: wgpu::Features,
  limits: wgpu::Limits,
  backend: WgpuBackend,
  /// The surface of the page's canvas, which the adapter was asked for, until the engine that draws to the
  /// canvas takes it.
  #[cfg(target_arch = "wasm32")]
  surface: std::rc::Rc<std::cell::RefCell<Option<wgpu::Surface<'static>>>>,
}

/// The number of a device that was just made, in the order of devices and of the views apps give: a view
/// given before the device was made is not of it.
fn device_number() -> u64 {
  #[cfg(feature = "raster")]
  {
    crate::images::next_in_order()
  }
  #[cfg(not(feature = "raster"))]
  {
    0
  }
}

/// A device with what it was made of, and what says that it is lost.
#[derive(Clone)]
struct MadeDevice {
  instance: wgpu::Instance,
  adapter: wgpu::Adapter,
  device: wgpu::Device,
  queue: wgpu::Queue,
  lost: std::sync::Arc<std::sync::atomic::AtomicBool>,
  /// The device's number in the order of devices and of the views apps give.
  number: u64,
  /// Whether it is of another card than the one a device was lost on, which made none in time.
  displaced: bool,
  /// Whether it was made while something held the lost device still: it is not a whole one.
  while_held: bool,
  /// The errors of the app's own calls on it that no extension was told of yet.
  untold: app_errors::Untold,
}

/// Why [`WgpuDevice::request`] has no device: no adapter (a browser without WebGPU), or one that refused the device.
#[derive(Clone, Debug)]
pub struct WgpuDeviceError(String);

impl std::fmt::Display for WgpuDeviceError {
  fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    formatter.write_str(&self.0)
  }
}

impl std::error::Error for WgpuDeviceError {}

impl WgpuDevice {
  /// Asks for a device as an engine does for its first frame, with the default backends, features and limits.
  pub async fn request() -> Result<Self, WgpuDeviceError> {
    Self::request_with(default_backends()).await
  }

  /// [`Self::request`] among `backends` only. In a browser, `WgpuBackends::GL` asks for WebGL2 also where there
  /// is WebGPU, and `WgpuBackends::BROWSER_WEBGPU` for WebGPU alone.
  pub async fn request_with(backends: WgpuBackends) -> Result<Self, WgpuDeviceError> {
    let (features, limits) = (wgpu::Features::empty(), wgpu::Limits::default());
    #[cfg(not(target_arch = "wasm32"))]
    let made = Self::ask(backends, features, &limits).await?;
    #[cfg(target_arch = "wasm32")]
    let (made, surface) = Self::ask(backends).await?;
    Ok(Self {
      backend: made.adapter.get_info().backend,
      // In a browser nothing of wgpu's goes to another thread, and this is shared as on any system.
      #[allow(clippy::arc_with_non_send_sync)]
      made: std::sync::Arc::new(parking_lot::Mutex::new(Some(made))),
      back: std::sync::Arc::default(),
      backends,
      features,
      limits,
      #[cfg(target_arch = "wasm32")]
      surface: std::rc::Rc::new(std::cell::RefCell::new(Some(surface))),
    })
  }

  /// The device an engine made for itself, to be shared with the engines of the app's other windows.
  #[cfg(not(target_arch = "wasm32"))]
  fn of_engine(made: MadeDevice, backends: WgpuBackends, features: wgpu::Features, limits: wgpu::Limits) -> Self {
    Self {
      backend: made.adapter.get_info().backend,
      made: std::sync::Arc::new(parking_lot::Mutex::new(Some(made))),
      back: std::sync::Arc::default(),
      backends,
      features,
      limits,
    }
  }

  /// Asks for an adapter among `backends` and its device, as an engine does for its first frame.
  #[cfg(not(target_arch = "wasm32"))]
  async fn ask(
    backends: WgpuBackends,
    features: wgpu::Features,
    limits: &wgpu::Limits,
  ) -> Result<MadeDevice, WgpuDeviceError> {
    let instance = new_instance(backends);
    // After a loss, the card the device was lost on is asked first, and waited for a little.
    let describe = |adapter: &wgpu::Adapter| device_descriptor(adapter, features, limits);
    let (adapter, device, queue, displaced, while_held) =
      match device_loss::the_card_again(&instance, None, describe).await {
        device_loss::Again::Card(adapter, device, queue, held) => (adapter, device, queue, false, held),
        device_loss::Again::Wait => {
          return Err(WgpuDeviceError(
            "the card a device was lost on makes none yet".to_owned(),
          ));
        }
        device_loss::Again::Elsewhere(adapter, device, queue) => (adapter, device, queue, true, false),
        device_loss::Again::Unasked => {
          let (adapter, device, queue) = Self::ask_of(&instance, None, features, limits).await?;
          (adapter, device, queue, false, false)
        }
      };
    let lost = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let untold = app_errors::Untold::default();
    // Where it is lost, the engines that draw with it are replaced, and are given another.
    device_loss::watch(&device, &lost, &adapter, &untold);
    Ok(MadeDevice {
      instance,
      adapter,
      device,
      queue,
      lost,
      number: device_number(),
      displaced,
      while_held,
      untold,
    })
  }

  /// Asks for an adapter among `backends` for the page's canvas, and its device.
  #[cfg(target_arch = "wasm32")]
  async fn ask(backends: WgpuBackends) -> Result<(MadeDevice, wgpu::Surface<'static>), WgpuDeviceError> {
    // WebGL gives a device for a canvas, so the page's canvas is there before the device is.
    #[cfg(target_arch = "wasm32")]
    let (instance, surface) = {
      let instance = new_page_instance(backends).await;
      let canvas = crate::app::web_canvas::page_canvas()
        .ok_or_else(|| WgpuDeviceError("the page has no document to put a canvas in".to_owned()))?;
      // The handles are given as a window's are: wgpu's own implementation takes no canvas without a display
      // handle, which `SurfaceTarget::Canvas` leaves out. The surface keeps its own reference to the canvas.
      let value: &wasm_bindgen::JsValue = &canvas;
      let window = raw_window_handle::WebCanvasWindowHandle::new(std::ptr::NonNull::from(value).cast());
      let display = raw_window_handle::WebDisplayHandle::new();
      let surface = unsafe {
        instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
          raw_display_handle: Some(display.into()),
          raw_window_handle: window.into(),
        })
      }
      .map_err(|error| WgpuDeviceError(format!("the page's canvas cannot be drawn to: {error}")))?;
      (instance, surface)
    };
    let (adapter, device, queue) = Self::ask_of(
      &instance,
      Some(&surface),
      wgpu::Features::empty(),
      &wgpu::Limits::default(),
    )
    .await?;
    let lost = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let untold = app_errors::Untold::default();
    // A device lost here is not made again; what the app does wrong on it ends nothing.
    device_loss::watch(&device, &lost, &adapter, &untold);
    let made = MadeDevice {
      instance,
      adapter,
      device,
      queue,
      lost,
      number: device_number(),
      displaced: false,
      while_held: false,
      untold,
    };
    Ok((made, surface))
  }

  /// The adapter of `instance` that draws to `surface`, or any where there is none to draw to yet, and its
  /// device.
  async fn ask_of(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'static>>,
    features: wgpu::Features,
    limits: &wgpu::Limits,
  ) -> Result<(wgpu::Adapter, wgpu::Device, wgpu::Queue), WgpuDeviceError> {
    let adapter = instance
      .request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::default(),
        compatible_surface: surface,
        force_fallback_adapter: false,
      })
      .await
      .map_err(|error| WgpuDeviceError(format!("no suitable GPU adapter found: {error}")))?;
    let (device, queue) = adapter
      .request_device(&device_descriptor(&adapter, features, limits))
      .await
      .map_err(|error| WgpuDeviceError(format!("failed to create device: {error}")))?;
    Ok((adapter, device, queue))
  }

  /// The device as it is now. One that is lost is let go of and another asked for, once for all that share
  /// this; `None` while the card gives none, as Direct3D says it does not while anything holds the lost one.
  /// One of another card, taken because the first card made none in time, is let go of the same way once
  /// an engine found that the first card makes a device again.
  fn current(&self) -> Option<MadeDevice> {
    #[cfg_attr(target_arch = "wasm32", allow(unused_mut))]
    let mut made = self.made.lock();
    #[cfg(not(target_arch = "wasm32"))]
    {
      let back = self.back.swap(false, std::sync::atomic::Ordering::AcqRel);
      if made.as_ref().is_some_and(|made| {
        device_loss::is_lost(&made.device, &made.lost) || (back && (made.displaced || made.while_held))
      }) {
        *made = None;
      }
      if made.is_none() {
        *made = pollster::block_on(Self::ask(self.backends, self.features, &self.limits)).ok();
      }
    }
    made.clone()
  }

  /// What the device draws through: `WgpuBackend::BrowserWebGpu` or `WgpuBackend::Gl` in a browser.
  pub fn backend(&self) -> WgpuBackend {
    self.backend
  }
}

/// A device the engines of an app's windows share: one an engine on the thread that draws them asked for itself,
/// as long as one of them draws with it.
#[cfg(not(target_arch = "wasm32"))]
struct SharedDevice {
  backends: WgpuBackends,
  features: wgpu::Features,
  limits: wgpu::Limits,
  backend: WgpuBackend,
  made: std::sync::Weak<parking_lot::Mutex<Option<MadeDevice>>>,
  back: std::sync::Weak<std::sync::atomic::AtomicBool>,
}

#[cfg(not(target_arch = "wasm32"))]
thread_local! {
  /// The devices the engines on this thread share, by what they were asked for with.
  static SHARED_DEVICES: std::cell::RefCell<Vec<SharedDevice>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(not(target_arch = "wasm32"))]
impl WgpuDevice {
  /// The device another engine on this thread draws with, asked for among `backends` with `features` and
  /// `limits`, while one does.
  fn shared(backends: WgpuBackends, features: wgpu::Features, limits: &wgpu::Limits) -> Option<Self> {
    SHARED_DEVICES.with_borrow_mut(|shared| {
      shared.retain(|device| device.made.strong_count() > 0);
      let device = shared
        .iter()
        .find(|device| device.backends == backends && device.features == features && device.limits == *limits)?;
      Some(Self {
        made: device.made.upgrade()?,
        back: device.back.upgrade()?,
        backends,
        features,
        limits: limits.clone(),
        backend: device.backend,
      })
    })
  }

  /// Offers this device to the engines made on this thread after it, while one draws with it.
  fn share(&self) {
    SHARED_DEVICES.with_borrow_mut(|shared| {
      shared.push(SharedDevice {
        backends: self.backends,
        features: self.features,
        limits: self.limits.clone(),
        backend: self.backend,
        made: std::sync::Arc::downgrade(&self.made),
        back: std::sync::Arc::downgrade(&self.back),
      })
    });
  }
}

/// Runs `call` on an extension as the app's: as the engine of the device numbered `number` (a view it gives a slot
/// is of that device), and in error scopes of `device` where there is one that lives, whose errors the extension
/// is told of after the call.
fn run_extension(
  entry: &mut WgpuFrameExtensionEntry,
  device: Option<&wgpu::Device>,
  untold: &app_errors::Untold,
  number: u64,
  call: impl FnOnce(&mut dyn WgpuFrameExtension),
) {
  let extension = &mut *entry.extension;
  #[cfg(feature = "raster")]
  let within = |run: &mut dyn FnMut()| crate::images::for_device(number, run);
  #[cfg(not(feature = "raster"))]
  let within = |run: &mut dyn FnMut()| {
    let _ = number;
    run()
  };
  let mut call = Some(call);
  let mut run = || {
    if let Some(call) = call.take() {
      call(&mut *extension)
    }
  };
  let errors = match device {
    Some(device) => app_errors::caught(device, untold, || within(&mut run)).1,
    None => {
      app_errors::apps(|| within(&mut run));
      Vec::new()
    }
  };
  for error in &errors {
    app_errors::apps(|| entry.extension.error(error));
  }
}

impl Default for WgpuRenderEngine {
  fn default() -> Self {
    WgpuRenderEngine::new()
  }
}

impl WgpuRenderEngine {
  pub fn new() -> Self {
    Self {
      #[cfg(feature = "canvas")]
      canvases: Vec::new(),
      #[cfg(feature = "canvas")]
      canvas_renderer: None,
      #[cfg(feature = "canvas")]
      canvas_asset_budget: Default::default(),
      instance: None,
      backends: default_backends(),
      prepared: None,
      own_device: false,
      scale_factor: 1.0,
      untold: app_errors::Untold::default(),
      lost: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
      device_wait: 0,
      device_number: 0,
      displaced: false,
      made_while_held: false,
      #[cfg(not(target_arch = "wasm32"))]
      card_asked: None,
      card_is_back: false,
      #[cfg(feature = "raster")]
      external_slots: Vec::new(),
      adapter: None,
      device: None,
      queue: None,
      optional_device_features: wgpu::Features::empty(),
      optional_device_limits: wgpu::Limits::default(),
      requested_present_mode: wgpu::PresentMode::Fifo,
      quad_pipeline: None,
      glyph_pipeline: None,
      #[cfg(feature = "raster")]
      image_pipeline: None,
      #[cfg(feature = "raster")]
      nv12_image_pipeline: None,
      surface: None,
      surface_config: None,
      surface_format: None,
      quad_bgl: None,
      glyph_bgl: None,
      #[cfg(feature = "raster")]
      image_bgl: None,
      #[cfg(feature = "raster")]
      nv12_image_bgl: None,
      #[cfg(feature = "raster")]
      image_sampler: None,
      #[cfg(feature = "raster")]
      image_texture_cache: std::collections::HashMap::new(),
      #[cfg(any(target_os = "macos", target_os = "ios"))]
      retired_card_frames: Vec::new(),
      #[cfg(all(feature = "raster", windows))]
      card_leases: card_frames::Leases::default(),
      #[cfg(all(feature = "raster", windows))]
      card_readers: card_copies::Readers::default(),
      #[cfg(feature = "raster")]
      image_clip_bind_groups: std::collections::HashMap::new(),
      globals_buffer: None,
      clip_globals: ClipGlobalsCache::default(),
      quad_clip_bind_groups: std::collections::HashMap::new(),
      glyph_clip_bind_groups: std::collections::HashMap::new(),
      gradients: None,
      last_globals_bytes: Vec::new(),
      last_gradient_bytes: Vec::new(),
      atlas_texture: None,
      atlas_view: None,
      atlas_sampler: None,
      atlas_size: (0, 0),
      atlas_version: 0,
      #[cfg(feature = "perf_profile")]
      last_profile: RenderProfile::default(),
      #[cfg(feature = "perf_profile")]
      last_layer_profile: Default::default(),
      #[cfg(feature = "perf_profile")]
      profile_context: None,
      #[cfg(feature = "perf_profile")]
      gpu_timer: gpu_timing::GpuTimer::new(),
      quad_bind_group: None,
      glyph_bind_group: None,
      vertex_buffer: None,
      index_buffer: None,
      rect_instance_buffer: DynamicBuffer::new("lurq_ordered_rect_instances", wgpu::BufferUsages::VERTEX),
      glyph_instance_buffer: DynamicBuffer::new("lurq_ordered_glyph_instances", wgpu::BufferUsages::VERTEX),
      #[cfg(feature = "raster")]
      image_instance_buffer: DynamicBuffer::new("lurq_image_instances", wgpu::BufferUsages::VERTEX),
      scratch_rect_instances: Vec::new(),
      scratch_rect_draws: Vec::new(),
      scratch_gradient_data: Vec::new(),
      scratch_glyph_instances: Vec::new(),
      #[cfg(feature = "raster")]
      scratch_image_instances: Vec::new(),
      scratch_ordered_draws: Vec::new(),
      layer_plan: LayerPlan::default(),
      layer_compositor: None,
      scratch_layer_slots: Vec::new(),
      scratch_blur_slots: Vec::new(),
      scratch_backdrop_slots: Vec::new(),
      scratch_backdrop_regions: Vec::new(),
      backdrop_inputs: BackdropInputs::default(),
      width: 800,
      height: 600,
      frame_extensions: Vec::new(),
      surface_copies: true,
      #[cfg(test)]
      deny_surface_copies: false,
      surface_proxy: None,
      #[cfg(feature = "screenshot")]
      pending_frame_capture: None,
    }
  }

  /// Opacity-layer textures the engine keeps pooled.
  #[cfg(all(test, feature = "screenshot"))]
  pub(crate) fn layer_texture_count(&self) -> usize {
    self
      .layer_compositor
      .as_ref()
      .map_or(0, layers::LayerCompositor::texture_count)
  }

  /// Backdrop blurs the last frame put back from a kept texture, then the
  /// textures kept.
  #[cfg(all(test, feature = "screenshot"))]
  pub(crate) fn backdrop_reuse(&self) -> (usize, usize) {
    self.layer_compositor.as_ref().map_or((0, 0), |compositor| {
      let (reused, kept, _) = compositor.backdrop_stats();
      (reused, kept)
    })
  }

  /// Select backends before rendering the first frame. Instance creation is lazy,
  /// so excluded backend loaders are never initialized by this engine.
  ///
  /// Windows defaults to DX12: independent Vulkan instances can race in the
  /// platform loader during concurrent window teardown. Other platforms retain
  /// wgpu's full backend selection. Vulkan remains an explicit opt-in on Windows.
  /// Panics if this engine has already initialized a GPU instance.
  pub fn with_backends(mut self, backends: WgpuBackends) -> Self {
    assert!(self.instance.is_none(), "select WGPU backends before rendering");
    self.backends = backends;
    self
  }

  /// Draws with a device made ahead ([`WgpuDevice::request`]) instead of
  /// asking for one at the first frame: what a browser needs, where the
  /// engine cannot wait for the device. The backends, optional features and
  /// limits are then the device's, not this engine's.
  /// Panics if this engine has already initialized a GPU instance.
  pub fn with_device(mut self, device: WgpuDevice) -> Self {
    assert!(self.instance.is_none(), "give the WGPU device before rendering");
    self.prepared = Some(device);
    self
  }

  /// Draws as on a surface whose texture cannot be copied from, as WebGL's: a frame that is read back is
  /// painted beside the surface.
  #[cfg(all(test, feature = "screenshot"))]
  pub(crate) fn without_surface_copies(mut self) -> Self {
    self.deny_surface_copies = true;
    self
  }

  /// Draws with a device of this engine's own, where it would share the one the engines of the app's other windows
  /// draw with. An engine made without [`with_device`](Self::with_device) takes the device of another engine on
  /// its thread that was asked for among the same backends with the same optional features and limits, while one
  /// draws with it, and asks for one otherwise: so a view of one device can be shown in every window of the app.
  /// A device of its own is for a window that must not wait on the others' work, or that draws on another card.
  /// In a browser there is one window and its device is given.
  pub fn with_own_device(mut self) -> Self {
    self.own_device = true;
    self
  }

  pub fn with_frame_extension(mut self, extension: impl WgpuFrameExtension + 'static) -> Self {
    self.add_frame_extension(extension);
    self
  }

  pub fn with_optional_device_features(mut self, features: WgpuFeatures) -> Self {
    self.optional_device_features |= features;
    self
  }

  pub fn with_optional_device_limits(mut self, limits: WgpuLimits) -> Self {
    self.optional_device_limits = limits;
    self
  }

  pub fn with_present_mode(mut self, present_mode: WgpuPresentMode) -> Self {
    self.requested_present_mode = present_mode;
    self
  }

  /// How many charged bytes of uploaded Canvas images and text this renderer
  /// keeps between frames; 64 MiB by default. See [`CanvasAssetBudget`] for
  /// how a frame that needs more is handled.
  ///
  /// [`CanvasAssetBudget`]: crate::canvas::CanvasAssetBudget
  #[cfg(feature = "canvas")]
  pub fn with_canvas_asset_budget(mut self, budget: crate::canvas::CanvasAssetBudget) -> Self {
    self.canvas_asset_budget = budget;
    self
  }

  pub fn add_frame_extension(&mut self, extension: impl WgpuFrameExtension + 'static) {
    self
      .frame_extensions
      .push(WgpuFrameExtensionEntry::new(Box::new(extension)));
  }

  fn shared_context(&self) -> Option<SharedWgpuContext> {
    Some(SharedWgpuContext {
      instance: self.instance.clone()?,
      adapter: self.adapter.clone()?,
      device: self.device.clone()?,
      queue: self.queue.clone()?,
      surface_format: self.surface_format?,
      number: self.device_number,
    })
  }

  fn prepare_frame_extensions(&mut self, list: &RenderList) {
    if self.frame_extensions.is_empty() {
      return;
    }
    let Some(gpu) = self.shared_context() else {
      return;
    };
    let frame = WgpuFrameInfo::new(list, self.width, self.height, self.scale_factor);
    // What the app did wrong on the device outside the extensions' calls since the last frame.
    let untold = self.untold.take();
    for entry in &mut self.frame_extensions {
      if !entry.initialized {
        run_extension(
          entry,
          Some(&gpu.device),
          &self.untold,
          self.device_number,
          |extension| extension.initialize(&gpu),
        );
        entry.initialized = true;
      }
      for error in &untold {
        app_errors::apps(|| entry.extension.error(error));
      }
      run_extension(
        entry,
        Some(&gpu.device),
        &self.untold,
        self.device_number,
        |extension| extension.prepare(&gpu, frame),
      );
    }
  }

  /// Tells each extension that was initialized that its device is lost, once, and lets go of the views of
  /// that device in the slots this drew: an extension that was told lets go of what it made, and what it
  /// left in a slot is held by nothing of the app's.
  fn tell_extensions_of_loss(&mut self) {
    for entry in &mut self.frame_extensions {
      if entry.initialized && !entry.told_lost {
        entry.told_lost = true;
        // Nothing is asked of a device that is lost: no scope is pushed on it.
        run_extension(entry, None, &self.untold, self.device_number, |extension| {
          extension.device_lost()
        });
      }
    }
    #[cfg(feature = "raster")]
    for slot in self.external_slots.drain(..) {
      if let Some(slot) = slot.upgrade() {
        slot.forget_view_of(self.device_number);
      }
    }
  }

  fn release_gpu_resources(&mut self) {
    if let Some(device) = &self.device {
      device_loss::asked(device, Some(&self.lost), wgpu::PollType::Poll);
    }

    // An engine whose device is lost is let go of before it draws again: its extensions are told here.
    let lost = self.device_is_lost();
    if lost {
      self.tell_extensions_of_loss();
    }
    let device = self.device.clone().filter(|_| !lost);
    for entry in &mut self.frame_extensions {
      if entry.initialized {
        run_extension(entry, device.as_ref(), &self.untold, self.device_number, |extension| {
          extension.shutdown()
        });
        entry.initialized = false;
      }
    }

    #[cfg(feature = "raster")]
    {
      self.image_texture_cache.clear();
      self.image_clip_bind_groups.clear();
    }
    self.clip_globals.clear();
    self.quad_clip_bind_groups.clear();
    self.glyph_clip_bind_groups.clear();

    self.quad_bind_group = None;
    self.glyph_bind_group = None;
    self.layer_compositor = None;
    #[cfg(feature = "raster")]
    {
      self.image_sampler = None;
      self.image_bgl = None;
      self.nv12_image_bgl = None;
      self.image_pipeline = None;
      self.nv12_image_pipeline = None;
    }

    self.vertex_buffer = None;
    self.index_buffer = None;
    self.rect_instance_buffer.clear();
    self.glyph_instance_buffer.clear();
    #[cfg(feature = "raster")]
    self.image_instance_buffer.clear();
    self.globals_buffer = None;
    self.gradients = None;
    self.surface_proxy = None;
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
    #[cfg(feature = "canvas")]
    {
      self.canvas_renderer = None;
    }
    #[cfg(feature = "perf_profile")]
    self.gpu_timer.release();
    self.queue = None;
    self.device = None;
    self.adapter = None;
  }

  fn create_surface(&self, window: WindowHandle<'_>, display: DisplayHandle<'_>) -> wgpu::Surface<'static> {
    // The page's canvas has its surface since the device was asked for.
    #[cfg(target_arch = "wasm32")]
    if let Some(surface) = self
      .prepared
      .as_ref()
      .and_then(|prepared| prepared.surface.borrow_mut().take())
    {
      return surface;
    }
    let handles = WindowDisplayPair { window, display };
    let surface_target = unsafe { wgpu::SurfaceTargetUnsafe::from_display_and_window(&handles, &handles) }.unwrap();
    unsafe { self.instance.as_ref().unwrap().create_surface_unsafe(surface_target) }.unwrap()
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
      .or_else(|| caps.formats.iter().copied().find(|format| !format.is_srgb()))
      .unwrap_or(caps.formats[0]);
    let target_format = render_target_format(format);

    // A backdrop blur and a capture copy the frame out of the surface's texture, where that can be copied
    // from. WebGL's cannot: there a frame that is read is painted beside the surface; see `surface_proxy`.
    self.surface_copies = caps.usages.contains(wgpu::TextureUsages::COPY_SRC);
    #[cfg(test)]
    if self.deny_surface_copies {
      self.surface_copies = false;
    }
    let config = wgpu::SurfaceConfiguration {
      usage: if self.surface_copies {
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC
      } else {
        wgpu::TextureUsages::RENDER_ATTACHMENT
      },
      format,
      width: self.width.max(1),
      height: self.height.max(1),
      present_mode: supported_present_mode(self.requested_present_mode, &caps.present_modes),
      alpha_mode: caps.alpha_modes[0],
      view_formats: if target_format == format {
        vec![]
      } else {
        vec![target_format]
      },
      desired_maximum_frame_latency: SURFACE_FRAME_LATENCY,
    };
    surface.configure(device, &config);
    self.surface = Some(surface);
    self.surface_config = Some(config);
    self.surface_format = Some(format);
    target_format
  }

  /// Whether the next `render` reconfigures the surface to the latest `resize`.
  fn surface_size_pending(&self) -> bool {
    self
      .surface_config
      .as_ref()
      .is_some_and(|config| (config.width, config.height) != (self.width, self.height))
  }

  /// Reconfigures the surface once for however many `resize` calls arrived since the last frame.
  fn apply_pending_surface_size(&mut self) {
    if !self.surface_size_pending() {
      return;
    }
    if let (Some(config), Some(device), Some(surface)) = (&mut self.surface_config, &self.device, &self.surface) {
      config.width = self.width;
      config.height = self.height;
      surface.configure(device, config);
    }
    // A window of another size has other pixels behind its glass.
    if let Some(compositor) = self.layer_compositor.as_mut() {
      compositor.forget_backdrops();
    }
  }

  /// On another card than the one a device was lost on, asks that card from time to time whether it makes a
  /// device again, less and less often. Once it does, this engine is to be replaced: the one that takes its
  /// place draws on that card, where the frames of a decoder on it stay, and the window is drawn by the
  /// card it was drawn by before the loss.
  ///
  /// And with a device made while something held the lost one still, which is not a whole one, sees whether
  /// the lost one is gone: this engine is to be replaced then too, by one with a device made after it.
  #[cfg(not(target_arch = "wasm32"))]
  fn look_for_the_card(&mut self) {
    if self.card_is_back {
      return;
    }
    if self.made_while_held && !device_loss::a_lost_device_is_held() {
      self.card_is_back = true;
      if let Some(given) = &self.prepared {
        given.back.store(true, std::sync::atomic::Ordering::Release);
      }
      return;
    }
    if !self.displaced {
      return;
    }
    let Some((asked, every)) = self.card_asked else {
      return;
    };
    if asked.elapsed() < every {
      return;
    }
    self.card_asked = Some((std::time::Instant::now(), (every * 2).min(CARD_ASKED_AT_MOST)));
    let Some(instance) = self.instance.clone() else {
      return;
    };
    let (features, limits) = (self.optional_device_features, self.optional_device_limits.clone());
    let describe = move |adapter: &wgpu::Adapter| device_descriptor(adapter, features, &limits);
    if pollster::block_on(device_loss::the_card_gives(&instance, describe)) {
      self.card_is_back = true;
      // A device this was given is let go of for one of that card, for all that share it.
      if let Some(given) = &self.prepared {
        given.back.store(true, std::sync::atomic::Ordering::Release);
      }
    }
  }

  /// Whether the device this draws with is lost.
  fn device_is_lost(&self) -> bool {
    self
      .device
      .as_ref()
      .is_some_and(|device| device_loss::is_lost(device, &self.lost))
  }

  /// Asks for the adapter that draws to `surface` and its device, and waits for them. `None` where a device
  /// was lost in this program and the card gives none yet: it is asked for again some frames later.
  #[cfg(not(target_arch = "wasm32"))]
  fn request_device(&mut self, surface: &wgpu::Surface<'static>) -> Option<(wgpu::Adapter, wgpu::Device, wgpu::Queue)> {
    // After a loss, the card the device was lost on is asked first, and waited for a little: asked for any
    // card, the window would go to another one while something of the app holds the lost device still.
    let (features, limits) = (self.optional_device_features, self.optional_device_limits.clone());
    let describe = move |adapter: &wgpu::Adapter| device_descriptor(adapter, features, &limits);
    let instance = self.instance.clone().unwrap();
    match pollster::block_on(device_loss::the_card_again(&instance, Some(surface), describe)) {
      device_loss::Again::Card(adapter, device, queue, held) => {
        self.made_while_held = held;
        return Some((adapter, device, queue));
      }
      device_loss::Again::Wait => return None,
      device_loss::Again::Elsewhere(adapter, device, queue) => {
        self.displaced = true;
        return Some((adapter, device, queue));
      }
      device_loss::Again::Unasked => {}
    }
    let adapter = pollster::block_on(
      self
        .instance
        .as_ref()
        .unwrap()
        .request_adapter(&wgpu::RequestAdapterOptions {
          power_preference: wgpu::PowerPreference::default(),
          compatible_surface: Some(surface),
          force_fallback_adapter: false,
        }),
    );
    if adapter.is_err() && device_loss::happened() {
      return None;
    }
    let adapter = adapter.expect("no suitable GPU adapter found");
    let made = pollster::block_on(adapter.request_device(&device_descriptor(
      &adapter,
      self.optional_device_features,
      &self.optional_device_limits,
    )));
    if made.is_err() && device_loss::happened() {
      return None;
    }
    let (device, queue) = made.expect("failed to create device");
    Some((adapter, device, queue))
  }

  /// A browser answers through a promise, which a frame cannot wait for.
  #[cfg(target_arch = "wasm32")]
  fn request_device(
    &mut self,
    _surface: &wgpu::Surface<'static>,
  ) -> Option<(wgpu::Adapter, wgpu::Device, wgpu::Queue)> {
    panic!(
      "in a browser the GPU device comes from a promise: await `WgpuDevice::request()` before the window starts and \
       give the device with `WgpuRenderEngine::with_device`"
    );
  }

  fn ensure_initialized(&mut self, window: WindowHandle<'_>, display: DisplayHandle<'_>) {
    if self.device.is_some() && self.surface.is_some() {
      return;
    }

    // A device that was not given is not asked for in every frame.
    if self.device_wait > 0 {
      self.device_wait -= 1;
      return;
    }

    // The device of the app's other windows, where one draws with a device asked for as this one would be.
    #[cfg(not(target_arch = "wasm32"))]
    let shared = self.prepared.is_none() && !self.own_device && self.device.is_none() && {
      self.prepared = WgpuDevice::shared(
        self.backends,
        self.optional_device_features,
        &self.optional_device_limits,
      );
      self.prepared.is_some()
    };
    // A device this was given, as it is now: another one where the first was lost.
    let given = match &self.prepared {
      Some(prepared) if self.device.is_none() => match prepared.current() {
        Some(made) => Some(made),
        None => {
          self.device_wait = 15;
          return;
        }
      },
      _ => None,
    };
    if self.instance.is_none() {
      self.instance = Some(match &given {
        Some(made) => made.instance.clone(),
        None => new_instance(self.backends),
      });
    }
    let surface = self.create_surface(window, display);
    // A device of another window whose card cannot draw to this one's: this window asks for one of its own.
    #[cfg(not(target_arch = "wasm32"))]
    if shared
      && given
        .as_ref()
        .is_some_and(|made| !made.adapter.is_surface_supported(&surface))
    {
      drop(surface);
      self.prepared = None;
      self.instance = None;
      self.own_device = true;
      return self.ensure_initialized(window, display);
    }

    if let (Some(adapter), Some(device)) = (self.adapter.clone(), self.device.clone()) {
      self.configure_surface(surface, &adapter, &device);
      return;
    }

    let (adapter, device, queue) = match given {
      Some(made) => {
        // All that share the device lose it together, and are told of the app's errors on it together.
        self.lost = made.lost;
        self.untold = made.untold;
        self.device_number = made.number;
        self.displaced = made.displaced;
        self.made_while_held = made.while_held;
        (made.adapter, made.device, made.queue)
      }
      None => {
        let Some(made) = self.request_device(&surface) else {
          self.device_wait = 15;
          return;
        };
        // The device is this engine's own: where it is lost, another engine takes this one's place.
        device_loss::watch(&made.1, &self.lost, &made.0, &self.untold);
        self.device_number = device_number();
        // And the device of the engines of the app's other windows made after this one.
        #[cfg(not(target_arch = "wasm32"))]
        if !self.own_device {
          let device = WgpuDevice::of_engine(
            MadeDevice {
              instance: self.instance.clone().expect("the instance the device was asked of"),
              adapter: made.0.clone(),
              device: made.1.clone(),
              queue: made.2.clone(),
              lost: self.lost.clone(),
              number: self.device_number,
              displaced: self.displaced,
              while_held: self.made_while_held,
              untold: self.untold.clone(),
            },
            self.backends,
            self.optional_device_features,
            self.optional_device_limits.clone(),
          );
          device.share();
          self.prepared = Some(device);
        }
        made
      }
    };
    // On another card than the one a device was lost on: that card is looked for from here.
    #[cfg(not(target_arch = "wasm32"))]
    if self.displaced {
      self.card_asked = Some((std::time::Instant::now(), CARD_ASKED_FIRST));
    }
    // What decodes a video decodes on the card this draws on, and its frames stay there.
    #[cfg(all(windows, feature = "raster"))]
    if !card_frames::announce(&device) {
      card_frames::announce_vulkan(&device, &adapter);
    }
    // On Metal the buffer the system's decoder gives is drawn where it is.
    #[cfg(all(feature = "raster", any(target_os = "macos", target_os = "ios")))]
    crate::images::set_cv_pixel_buffer_frames_drawn(unsafe { device.as_hal::<wgpu_hal::api::Metal>() }.is_some());

    let format = self.configure_surface(surface, &adapter, &device);
    let config = self.surface_config.as_ref().unwrap();

    // --- Quad pipeline ---
    // The gradients are a storage buffer, or a texture where the device has no storage buffers.
    let gradients_in_texture = gradients::in_texture(&device);
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
        gradients::layout_entry(gradients_in_texture),
      ],
    });

    let quad_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
      label: Some("lurq_quad_shader"),
      source: wgpu::ShaderSource::Wgsl(gradients::quad_shader(gradients_in_texture)),
    });

    let quad_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
      label: Some("lurq_quad_pl"),
      bind_group_layouts: &[Some(&quad_bgl)],
      immediate_size: 0,
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
      multiview_mask: None,
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
      bind_group_layouts: &[Some(&glyph_bgl)],
      immediate_size: 0,
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
      multiview_mask: None,
      cache: None,
    });

    // --- Image pipeline ---
    #[cfg(feature = "raster")]
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
        bind_group_layouts: &[Some(&image_bgl)],
        immediate_size: 0,
      });
      let nv12_image_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("lurq_nv12_image_pl"),
        bind_group_layouts: &[Some(&nv12_image_bgl)],
        immediate_size: 0,
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
        multiview_mask: None,
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
        multiview_mask: None,
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

    // --- Persistent uniform/storage buffers ---
    let globals_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
      label: Some("lurq_globals"),
      contents: bytemuck::bytes_of(&Globals::unclipped(config.width as f32, config.height as f32)),
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let mut gradients = gradients::Gradients::new(&device);
    gradients.write(&queue, &[[0.0; 4]]);

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
          resource: gradients.binding(),
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

    #[cfg(feature = "perf_profile")]
    self.gpu_timer.configure(&adapter, &device, &queue);
    self.adapter = Some(adapter);
    self.device = Some(device);
    self.queue = Some(queue);
    self.quad_pipeline = Some(quad_pipeline);
    self.glyph_pipeline = Some(glyph_pipeline);
    #[cfg(feature = "raster")]
    {
      self.image_pipeline = Some(image_pipeline);
      self.nv12_image_pipeline = Some(nv12_image_pipeline);
    }
    self.quad_bgl = Some(quad_bgl);
    self.glyph_bgl = Some(glyph_bgl);
    #[cfg(feature = "raster")]
    {
      self.image_bgl = Some(image_bgl);
      self.nv12_image_bgl = Some(nv12_image_bgl);
    }
    #[cfg(feature = "raster")]
    {
      self.image_sampler = Some(image_sampler);
    }
    self.globals_buffer = Some(globals_buffer);
    self.gradients = Some(gradients);
    self.atlas_sampler = Some(atlas_sampler);
    self.quad_bind_group = Some(quad_bind_group);
    self.vertex_buffer = Some(vertex_buffer);
    self.index_buffer = Some(index_buffer);
  }

  /// The texture a frame is drawn into when the surface gives none and a capture waits for that frame: AppKit hands
  /// out no drawable while the window is occluded, and the capture would otherwise wait until it shows again.
  #[cfg_attr(not(feature = "screenshot"), allow(unused_variables))]
  fn offscreen_capture_target(
    &self,
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
  ) -> Option<FrameTarget> {
    #[cfg(feature = "screenshot")]
    if self.pending_frame_capture.is_some() {
      return Some(FrameTarget::Offscreen(device.create_texture(
        &wgpu::TextureDescriptor {
          label: Some("lurq_offscreen_capture_frame"),
          size: wgpu::Extent3d {
            width: config.width,
            height: config.height,
            depth_or_array_layers: 1,
          },
          mip_level_count: 1,
          sample_count: 1,
          dimension: wgpu::TextureDimension::D2,
          format: config.format,
          usage: config.usage,
          view_formats: &config.view_formats,
        },
      )));
    }
    None
  }
}

/// How many frames may wait for the display. One keeps input closest to the screen. On iOS that gives the layer
/// two drawables, and a frame then waits for the one on screen to come back: measured on an iPhone, every
/// present took two refreshes, 30 frames a second on a 60 Hz display.
#[cfg(not(target_os = "ios"))]
const SURFACE_FRAME_LATENCY: u32 = 1;
#[cfg(target_os = "ios")]
const SURFACE_FRAME_LATENCY: u32 = 2;

fn supported_present_mode(requested: wgpu::PresentMode, supported: &[wgpu::PresentMode]) -> wgpu::PresentMode {
  match requested {
    wgpu::PresentMode::AutoVsync | wgpu::PresentMode::AutoNoVsync => requested,
    requested if supported.contains(&requested) => requested,
    _ => wgpu::PresentMode::Fifo,
  }
}

/// Waits until a DX12 swap chain is ready for the next frame, before `get_current_texture`.
///
/// With wgpu's default `Dx12UseFrameLatencyWaitableObject::Wait`, `get_current_texture` waits on the
/// swap chain's frame latency object while it holds the device's fence lock. That wait lasts until
/// vblank, and `Queue::submit` needs the same lock, so no other thread can submit work meanwhile.
/// The instance is created with `DontWait`, and this waits on the same object with no wgpu lock held.
/// It does nothing on other backends, or when the surface has no swap chain yet.
fn wait_for_frame_latency(surface: &wgpu::Surface<'_>) {
  #[cfg(windows)]
  {
    use windows::Win32::System::Threading::WaitForSingleObject;

    // wgpu-core's own frame timeout for `get_current_texture`.
    const FRAME_LATENCY_WAIT_TIMEOUT_MS: u32 = 1000;

    // SAFETY: the HAL surface is only read. The handle belongs to the swap chain, which lurq
    // reconfigures only on this thread, so it stays open through the wait below.
    let handle = unsafe { surface.as_hal::<wgpu::hal::api::Dx12>() }.and_then(|hal| unsafe { hal.waitable_handle() });
    if let Some(handle) = handle {
      // SAFETY: `handle` is the swap chain's frame latency waitable object, a valid handle.
      unsafe { WaitForSingleObject(handle, FRAME_LATENCY_WAIT_TIMEOUT_MS) };
    }
  }
  #[cfg(not(windows))]
  let _ = surface;
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

impl Drop for WgpuRenderEngine {
  fn drop(&mut self) {
    let _own = app_errors::OwnWork::begin();
    self.release_gpu_resources();
  }
}

#[cfg(test)]
mod extension_tests {
  use super::{WgpuFrameExtension, WgpuRenderEngine};
  use crate::app::render_engine::RenderEngine;

  struct ContinuousExtension;

  impl WgpuFrameExtension for ContinuousExtension {
    fn wants_redraw(&self) -> bool {
      true
    }
  }

  #[test]
  fn frame_extension_can_drive_host_redraws() {
    let renderer = WgpuRenderEngine::new().with_frame_extension(ContinuousExtension);
    assert!(renderer.wants_redraw());
  }

  /// Writes down what it is told, in the order it is told it.
  struct Told(std::sync::Arc<std::sync::Mutex<Vec<&'static str>>>);

  impl WgpuFrameExtension for Told {
    fn device_lost(&mut self) {
      self.0.lock().unwrap().push("device lost");
    }

    fn shutdown(&mut self) {
      self.0.lock().unwrap().push("shutdown");
    }
  }

  /// An engine with a device and an extension that was initialized with it, and what the extension is told.
  fn with_a_device() -> Option<(WgpuRenderEngine, std::sync::Arc<std::sync::Mutex<Vec<&'static str>>>)> {
    let instance = super::new_instance(super::default_backends());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).ok()?;
    let (device, queue) = pollster::block_on(adapter.request_device(&super::device_descriptor(
      &adapter,
      wgpu::Features::empty(),
      &wgpu::Limits::default(),
    )))
    .ok()?;
    let told = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut engine = WgpuRenderEngine::new().with_frame_extension(Told(told.clone()));
    (engine.device, engine.queue) = (Some(device), Some(queue));
    engine.frame_extensions[0].initialized = true;
    Some((engine, told))
  }

  #[test]
  fn an_extension_is_told_once_that_its_device_is_lost_before_it_is_shut_down() {
    let Some((mut engine, told)) = with_a_device() else {
      eprintln!("skipped: no card to make a device on");
      return;
    };
    // The loss is found, as its card or wgpu says it.
    engine.lost.store(true, std::sync::atomic::Ordering::Release);
    assert!(engine.lost());
    engine.tell_extensions_of_loss();
    engine.tell_extensions_of_loss();
    assert_eq!(*told.lock().unwrap(), ["device lost"], "told once");
    drop(engine);
    assert_eq!(*told.lock().unwrap(), ["device lost", "shutdown"]);
  }

  #[test]
  fn an_extension_is_told_of_the_loss_by_an_engine_let_go_of_before_it_draws_again() {
    let Some((engine, told)) = with_a_device() else {
      eprintln!("skipped: no card to make a device on");
      return;
    };
    engine.lost.store(true, std::sync::atomic::Ordering::Release);
    // The tree lets go of a lost engine in the pass that finds it, with no frame between.
    drop(engine);
    assert_eq!(*told.lock().unwrap(), ["device lost", "shutdown"]);
  }

  #[test]
  fn an_extension_whose_device_lives_is_only_shut_down() {
    let Some((engine, told)) = with_a_device() else {
      eprintln!("skipped: no card to make a device on");
      return;
    };
    drop(engine);
    assert_eq!(*told.lock().unwrap(), ["shutdown"]);
  }

  #[test]
  fn an_extension_that_was_never_initialized_is_told_nothing() {
    let told = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let engine = WgpuRenderEngine::new().with_frame_extension(Told(told.clone()));
    engine.lost.store(true, std::sync::atomic::Ordering::Release);
    drop(engine);
    assert!(told.lock().unwrap().is_empty());
  }
}

impl RenderEngine for WgpuRenderEngine {
  fn profile_backend(&self) -> &'static str {
    "wgpu"
  }
  #[cfg(feature = "perf_profile")]
  fn set_profile_context(&mut self, context: crate::app::profiler::ProfileContext) {
    self.profile_context = Some(context);
  }
  #[cfg(feature = "canvas")]
  fn prepare_canvases(&mut self, canvases: &[crate::canvas::CanvasHandle]) {
    self.canvases.clear();
    self
      .canvases
      .extend(canvases.iter().filter(|c| !c.status().software).cloned());
  }
  fn resize(&mut self, width: u32, height: u32) {
    self.width = width.max(1);
    self.height = height.max(1);
    self.clip_globals.clear();
    self.quad_clip_bind_groups.clear();
    self.glyph_clip_bind_groups.clear();
    #[cfg(feature = "raster")]
    self.image_clip_bind_groups.clear();
    // The surface is reconfigured by the next `render`, not here. Reconfiguring waits for the GPU and
    // recreates the swapchain, and a live resize can deliver hundreds of size events per frame (Windows
    // replays the ones queued during a modal sizing loop back to back), which stalled for seconds.
  }

  fn set_scale_factor(&mut self, scale_factor: f32) {
    self.scale_factor = scale_factor;
  }

  fn draws_tooling(&mut self) {
    // The app's extensions are the app's windows': they are never initialized for this one.
    self.frame_extensions.clear();
  }

  fn render(&mut self, list: &RenderList, window: WindowHandle<'_>, display: DisplayHandle<'_>) -> bool {
    // An error of what is asked of wgpu from here is lurq's, but in the extensions' calls.
    let _own = app_errors::OwnWork::begin();
    // A device that is lost is asked nothing more: what it answers is errors.
    if self.device_is_lost() {
      self.tell_extensions_of_loss();
      #[cfg(feature = "screenshot")]
      if let Some(capture) = self.pending_frame_capture.take() {
        capture.target.fail("the graphics device was lost");
      }
      return false;
    }
    #[cfg(not(target_arch = "wasm32"))]
    self.look_for_the_card();
    let _total_start = profile_scope!();
    let _init_start = profile_scope!();
    #[cfg(feature = "perf_profile")]
    let _init_phase = self
      .profile_context
      .as_ref()
      .map(|context| context.phase(crate::app::profiler::Phase::RenderInit));
    self.ensure_initialized(window, display);
    // After a loss the card may give no device yet: the frame is not drawn, and is asked for again.
    if self.device.is_none() || self.surface.is_none() {
      #[cfg(feature = "screenshot")]
      if let Some(capture) = self.pending_frame_capture.take() {
        capture.target.fail("the graphics device was lost");
      }
      return false;
    }
    self.apply_pending_surface_size();
    let _init_dur = profile_elapsed!(_init_start);
    #[cfg(feature = "perf_profile")]
    drop(_init_phase);
    // Queries only while a session is active; otherwise this does no GPU work.
    #[cfg(feature = "perf_profile")]
    self
      .gpu_timer
      .begin_frame(self.device.as_ref().unwrap(), self.profile_context.as_ref());
    self.prepare_frame_extensions(list);
    // What the frame before this one no longer drew has been drawn by the card by now.
    #[cfg(all(feature = "raster", any(target_os = "macos", target_os = "ios")))]
    self.retired_card_frames.clear();
    #[cfg(feature = "perf_profile")]
    let mut _canvas_profile = crate::app::profile_types::CanvasProfile::default();
    #[cfg(feature = "canvas")]
    if !self.canvases.is_empty() || self.canvas_renderer.is_some() {
      let _canvas_start = profile_scope!();
      let device = self.device.as_ref().unwrap();
      let queue = self.queue.as_ref().unwrap();
      let renderer = self
        .canvas_renderer
        .get_or_insert_with(|| canvas::Renderer::new(device, queue).with_asset_budget(self.canvas_asset_budget));
      #[cfg(feature = "perf_profile")]
      {
        renderer.profile_context = self.profile_context.clone();
        renderer.gpu_marks = self.gpu_timer.canvas_marks();
      }
      renderer.process(device, queue, &self.canvases);
      #[cfg(feature = "perf_profile")]
      {
        self.gpu_timer.canvas_done(renderer.gpu_marks.take());
        _canvas_profile = renderer.profile;
        _canvas_profile.total = profile_elapsed!(_canvas_start);
      }
    }

    #[cfg(feature = "raster")]
    {
      // Resized and removed canvases release their old image IDs. Match DX12's
      // per-frame ownership so those backing textures cannot accumulate.
      let live_images: std::collections::HashSet<u64> = list.images.iter().map(|image| image.image_id).collect();
      self.image_texture_cache.retain(|id, _| live_images.contains(id));
      self
        .image_clip_bind_groups
        .retain(|key, _| live_images.contains(&key.image_id));
    }

    let device = self.device.as_ref().unwrap();
    let queue = self.queue.as_ref().unwrap();
    let surface = self.surface.as_ref().unwrap();
    let config = self.surface_config.as_ref().unwrap();
    let vtx_buf = self.vertex_buffer.as_ref().unwrap();
    let idx_buf = self.index_buffer.as_ref().unwrap();

    let _acquire_start = profile_scope!();
    #[cfg(feature = "perf_profile")]
    let _acquire_phase = self
      .profile_context
      .as_ref()
      .map(|context| context.phase(crate::app::profiler::Phase::RenderAcquire));
    wait_for_frame_latency(surface);
    let output = match surface.get_current_texture() {
      wgpu::CurrentSurfaceTexture::Success(texture) | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
        FrameTarget::Surface(texture)
      }
      // The surface fell out of sync with the window — common on Vulkan during a
      // live resize (VK_ERROR_OUT_OF_DATE_KHR). Reconfigure to the current size
      // and retry once; without this the frame is skipped and the window stalls
      // (and stays stalled) while resizing.
      wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
        surface.configure(device, config);
        wait_for_frame_latency(surface);
        match surface.get_current_texture() {
          wgpu::CurrentSurfaceTexture::Success(texture) | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => {
            FrameTarget::Surface(texture)
          }
          _ => match self.offscreen_capture_target(device, config) {
            Some(target) => target,
            None => return false,
          },
        }
      }
      _ => match self.offscreen_capture_target(device, config) {
        Some(target) => target,
        None => {
          profile_if! {
            self.last_profile = RenderProfile {
              init: _init_dur,
              acquire: profile_elapsed!(_acquire_start),
              total: profile_elapsed!(_total_start),
              ..RenderProfile::default()
            };
          }
          return false;
        }
      },
    };
    let view = output.texture().create_view(&wgpu::TextureViewDescriptor {
      format: Some(render_target_format(config.format)),
      ..Default::default()
    });
    let _acquire_dur = profile_elapsed!(_acquire_start);
    #[cfg(feature = "perf_profile")]
    drop(_acquire_phase);

    let vw = config.width as f32;
    let vh = config.height as f32;
    let globals_buffer = self.globals_buffer.as_ref().unwrap();

    let globals = Globals::unclipped(vw, vh);
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
    let mut _atlas_upload_bytes = 0usize;
    let mut _atlas_upload_rects = 0usize;
    let mut _atlas_full_uploads = 0usize;
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
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
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
      // The atlas is shared across windows; this engine's texture may be
      // several versions behind, so apply the log's rects newer than *our*
      // uploaded version — or the full atlas when the log no longer covers
      // the gap.
      let pending = if atlas_recreated {
        None
      } else {
        crate::app::glyph_engine::atlas_rects_to_apply(atlas, self.atlas_version)
      };
      let upload_full_atlas = pending.is_none();
      if upload_full_atlas {
        _atlas_upload_bytes = wgpu_staged_texture_bytes(atlas.width * 4, atlas.height);
        _atlas_upload_rects = 1;
        _atlas_full_uploads = 1;
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
            bytes_per_row: Some(atlas.width * 4),
            rows_per_image: Some(atlas.height),
          },
          wgpu::Extent3d {
            width: atlas.width,
            height: atlas.height,
            depth_or_array_layers: 1,
          },
        );
      } else {
        for rect in pending.unwrap_or_default() {
          if rect.width == 0 || rect.height == 0 || rect.x >= atlas.width || rect.y >= atlas.height {
            continue;
          }
          let width = rect.width.min(atlas.width - rect.x);
          let height = rect.height.min(atlas.height - rect.y);
          let row_bytes = atlas.width as usize * 4;
          let rect_x_bytes = rect.x as usize * 4;
          let width_bytes = width as usize * 4;
          let start = rect.y as usize * row_bytes + rect_x_bytes;
          let end = start + (height as usize - 1) * row_bytes + width_bytes;
          if end > atlas.data.len() {
            continue;
          }
          _atlas_upload_bytes += wgpu_staged_texture_bytes(width * 4, height);
          _atlas_upload_rects += 1;
          queue.write_texture(
            wgpu::TexelCopyTextureInfo {
              texture: self.atlas_texture.as_ref().unwrap(),
              mip_level: 0,
              origin: wgpu::Origin3d {
                x: rect.x,
                y: rect.y,
                z: 0,
              },
              aspect: wgpu::TextureAspect::All,
            },
            &atlas.data[start..end],
            wgpu::TexelCopyBufferLayout {
              offset: 0,
              bytes_per_row: Some(atlas.width * 4),
              rows_per_image: Some(height),
            },
            wgpu::Extent3d {
              width,
              height,
              depth_or_array_layers: 1,
            },
          );
        }
      }
      self.atlas_version = atlas.version;
    }
    let _atlas_dur = profile_elapsed!(_atlas_start);

    let _encode_start = profile_scope!();
    #[cfg(feature = "perf_profile")]
    let _encode_phase = self
      .profile_context
      .as_ref()
      .map(|context| context.phase(crate::app::profiler::Phase::RenderEncode));
    let mut _buffer_upload_dur = Duration::default();
    let mut _image_texture_upload_dur = Duration::default();

    self.scratch_rect_instances.clear();
    self.scratch_rect_draws.clear();
    self.scratch_rect_draws.reserve(list.rects.len());
    self.scratch_gradient_data.clear();
    for r in &list.rects {
      let start = self.scratch_rect_instances.len();
      if let Some(shadow) = &r.shadow {
        self.scratch_rect_instances.push(QuadInstance::box_shadow(r, shadow));
        self.scratch_rect_draws.push(PreparedDraw { start, count: 1 });
        continue;
      }
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
          // A stroke's pattern: its dash, its gap, and the edge its radii are at.
          pattern: [
            0.0,
            r.stroke_style.dash[0],
            r.stroke_style.dash[1],
            r.stroke_style.radii_at,
          ],
          transform: r.transform,
          xf_origin: r.transform_origin,
          shadow_sigma: 0.0,
          gradient_offset: crate::layout::render_list::encode_stroke_colors(
            &mut self.scratch_gradient_data,
            &r.stroke_style,
          ),
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
    let gradients = self.gradients.as_mut().unwrap();
    let gradient_recreated = gradients.reserve(device, self.scratch_gradient_data.len());
    if gradient_recreated {
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
            resource: gradients.binding(),
          },
        ],
      });
      self.quad_bind_group = Some(quad_bind_group);
      self.quad_clip_bind_groups.clear();
    }
    let _gradient_upload_start = profile_scope!();
    if gradient_recreated || self.last_gradient_bytes.as_slice() != gradient_bytes {
      gradients.write(queue, &self.scratch_gradient_data);
      self.last_gradient_bytes.clear();
      self.last_gradient_bytes.extend_from_slice(gradient_bytes);
    }
    let gradients = self.gradients.as_ref().unwrap();
    _buffer_upload_dur += profile_elapsed!(_gradient_upload_start);

    self.scratch_glyph_instances.clear();
    self.scratch_glyph_instances.reserve(list.glyphs.len());
    let atlas_inverse = [
      1.0 / list.atlas.width.max(1) as f32,
      1.0 / list.atlas.height.max(1) as f32,
    ];
    self
      .scratch_glyph_instances
      .extend(list.glyphs.iter().map(|g| GlyphInstance {
        pos: [g.x, g.y],
        size: [g.width, g.height],
        color: g.color,
        uv_min: [g.atlas_min[0] * atlas_inverse[0], g.atlas_min[1] * atlas_inverse[1]],
        uv_max: [g.atlas_max[0] * atlas_inverse[0], g.atlas_max[1] * atlas_inverse[1]],
        transform: g.transform,
        xf_origin: g.transform_origin,
        sharpness: g.sharpness,
        color_glyph: if g.color_glyph { 1.0 } else { 0.0 },
        shadow_sigma: g.shadow_sigma,
      }));
    let _glyph_upload_start = profile_scope!();
    let glyph_instance_buf = self
      .glyph_instance_buffer
      .write(device, queue, &self.scratch_glyph_instances);
    _buffer_upload_dur += profile_elapsed!(_glyph_upload_start);

    #[cfg(feature = "raster")]
    let _image_upload_start = profile_scope!();
    // The frames that stay on the card: their surfaces are opened once, and the frame to draw is taken now, for
    // the part of its texture an image is drawn from is the frame's to say. A frame of another card is sent
    // from memory here, before anything is drawn.
    #[cfg(all(feature = "raster", windows))]
    if let (Some(image_bgl), Some(nv12_image_bgl), Some(image_sampler)) = (
      self.image_bgl.as_ref(),
      self.nv12_image_bgl.as_ref(),
      self.image_sampler.as_ref(),
    ) {
      let bind = |first: &wgpu::TextureView, second: Option<&wgpu::TextureView>| {
        let globals = wgpu::BindGroupEntry {
          binding: 0,
          resource: globals_buffer.as_entire_binding(),
        };
        let view = |binding, view| wgpu::BindGroupEntry {
          binding,
          resource: wgpu::BindingResource::TextureView(view),
        };
        let sampler = |binding| wgpu::BindGroupEntry {
          binding,
          resource: wgpu::BindingResource::Sampler(image_sampler),
        };
        match second {
          Some(second) => device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lurq_card_frame_planes_bg"),
            layout: nv12_image_bgl,
            entries: &[globals, view(1, first), view(2, second), sampler(3)],
          }),
          None => device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lurq_card_frame_bg"),
            layout: image_bgl,
            entries: &[globals, view(1, first), sampler(2)],
          }),
        }
      };
      for img in &list.images {
        let Some(frames) = card_frames::of(img) else {
          continue;
        };
        let kept = self
          .image_texture_cache
          .get(&img.image_id)
          .is_some_and(|cached| cached.is_compatible(img));
        if !kept {
          self.image_texture_cache.remove(&img.image_id);
          self.image_clip_bind_groups.clear();
          let size = (img.image_width, img.image_height);
          // Kept though nothing opens them: they are not asked for again in each frame.
          let opened =
            card_frames::Opened::open(device, frames, size, &self.card_leases, &mut self.card_readers, &bind);
          self
            .image_texture_cache
            .insert(img.image_id, CachedImageTexture::Card(opened));
        }
        if let Some(CachedImageTexture::Card(opened)) = self.image_texture_cache.get_mut(&img.image_id) {
          opened.pick(queue);
        }
      }
    }

    #[cfg(feature = "raster")]
    let image_instance_buf = {
      self.scratch_image_instances.clear();
      self.scratch_image_instances.reserve(list.images.len());
      let cache = &self.image_texture_cache;
      self.scratch_image_instances.extend(list.images.iter().map(|img| {
        let (uv_min, uv_max) = drawn_part(cache, img);
        ImageInstance {
          pos: [img.x, img.y],
          size: [img.width, img.height],
          opacity: [img.opacity, canvas_image_flag(&img.native), 0.0, 0.0],
          transform: img.transform,
          xf_origin: img.transform_origin,
          uv_min,
          uv_max,
          radii: img.radii,
        }
      }));
      self
        .image_instance_buffer
        .write(device, queue, &self.scratch_image_instances)
    };
    #[cfg(feature = "raster")]
    {
      _buffer_upload_dur += profile_elapsed!(_image_upload_start);
    }

    // Draws in paint order, across the pipelines.
    self.scratch_ordered_draws.clear();
    self.scratch_ordered_draws.reserve(
      list.rects.len() + list.glyphs.len() + {
        #[cfg(feature = "raster")]
        {
          list.images.len()
        }
        #[cfg(not(feature = "raster"))]
        {
          0
        }
      },
    );
    for (index, rect) in list.rects.iter().enumerate() {
      self.scratch_ordered_draws.push((rect.order, OrderedDraw::Rect(index)));
    }
    glyph_runs(
      &list.glyphs,
      |clip| {
        scissor_rect(clip, vw, vh)
          .map(|(x, y, width, height)| [x as f32, y as f32, (x + width) as f32, (y + height) as f32])
      },
      |order, run| self.scratch_ordered_draws.push((order, OrderedDraw::Glyph(run))),
    );
    #[cfg(feature = "raster")]
    for (index, image) in list.images.iter().enumerate() {
      self
        .scratch_ordered_draws
        .push((image.order, OrderedDraw::Image(index)));
    }
    self.scratch_ordered_draws.sort_by_key(|(order, _)| *order);
    let ordered_draws = std::mem::take(&mut self.scratch_ordered_draws);

    // Faded subtrees paint into offscreen layers first, each before the
    // target it is composited into; the window's pass comes last. Without
    // layers the plan is the window's pass alone.
    let mut layer_plan = std::mem::take(&mut self.layer_plan);
    layer_plan.build(
      ordered_draws.len(),
      |index| ordered_draws[index].0,
      &list.layers,
      &list.backdrops,
    );
    self.scratch_layer_slots.clear();
    self.scratch_blur_slots.clear();
    self.scratch_backdrop_slots.clear();
    if layer_plan.has_layers() || layer_plan.has_backdrops() {
      let format = render_target_format(config.format);
      if self
        .layer_compositor
        .as_ref()
        .is_none_or(|compositor| compositor.format() != format)
      {
        self.layer_compositor = Some(layers::LayerCompositor::new(device, format));
      }
    }
    if let Some(compositor) = self.layer_compositor.as_mut() {
      compositor.begin_frame();
      for target in layer_plan.targets() {
        self.scratch_layer_slots.push(target.layer.map(|layer| {
          let (_, [width, height]) = list.layers[layer].painted();
          compositor.acquire(device, width, height)
        }));
        self.scratch_blur_slots.push(
          target
            .layer
            .and_then(|layer| list.layers[layer].blur)
            .map(|blur| compositor.acquire_blur(device, &blur)),
        );
      }
      // A backdrop blur reads the target its step is in. One whose kept
      // result is still the blur of what it reads takes that; the others run.
      let window = [config.width, config.height];
      backdrop_regions(list, &layer_plan, window, &mut self.scratch_backdrop_regions);
      self.backdrop_inputs.describe(
        list,
        &layer_plan,
        ordered_draws.len(),
        |index| ordered_draws[index].1.painted(),
        &self.scratch_backdrop_regions,
        window,
      );
      compositor.acquire_backdrops(
        device,
        &self.scratch_backdrop_regions,
        &self.backdrop_inputs,
        window,
        &mut self.scratch_backdrop_slots,
      );
    }

    // A frame that is read back, by a backdrop blur of the window or a capture, is painted beside a surface
    // whose texture cannot be copied from, and put onto it in a last pass; see `surface_proxy`.
    let mut frame_is_read = false;
    if !self.surface_copies {
      frame_is_read = layer_plan
        .targets()
        .iter()
        .filter(|target| target.layer.is_none())
        .flat_map(|target| layer_plan.steps(target))
        .any(|step| {
          matches!(step, LayerStep::Backdrop(index)
            if self.scratch_backdrop_slots.get(*index).copied().flatten().is_some_and(|slots| slots.runs()))
        });
      #[cfg(feature = "screenshot")]
      {
        frame_is_read |= self.pending_frame_capture.is_some();
      }
    }
    if frame_is_read {
      let format = render_target_format(config.format);
      if !self
        .surface_proxy
        .as_ref()
        .is_some_and(|proxy| proxy.fits(format, config.width, config.height))
      {
        self.surface_proxy = Some(surface_proxy::SurfaceProxy::new(
          device,
          format,
          config.width,
          config.height,
        ));
      }
    }
    let surface_proxy = self.surface_proxy.as_ref().filter(|_| frame_is_read);
    let (window_texture, window_view) = match surface_proxy {
      Some(proxy) => (proxy.texture(), proxy.view()),
      None => (output.texture(), &view),
    };

    let mut encoder = device.create_command_encoder(&Default::default());
    #[cfg(feature = "perf_profile")]
    let target_count = layer_plan.targets().len();
    #[cfg(feature = "perf_profile")]
    let mut _draw_calls = 0usize;
    #[cfg(feature = "perf_profile")]
    let mut _layers = crate::app::profile_types::LayerProfile::default();
    for (target_index, target) in layer_plan.targets().iter().enumerate() {
      let layer_slot = self.scratch_layer_slots.get(target_index).copied().flatten();
      let (space, target_view, clear) = match (target.layer, layer_slot, self.layer_compositor.as_ref()) {
        (Some(layer), Some(slot), Some(compositor)) => (
          compositor.space(slot, &list.layers[layer]),
          compositor.view(slot).clone(),
          wgpu::Color::TRANSPARENT,
        ),
        _ => (
          TargetSpace::window(vw, vh),
          window_view.clone(),
          wgpu_clear_color(list.clear_color),
        ),
      };
      // A backdrop blur that runs ends the target's pass, reads what it
      // painted and starts another that keeps it, so a target is painted in
      // one pass more than it has backdrop blurs that run.
      #[cfg_attr(
        not(feature = "perf_profile"),
        allow(unused_variables, unused_assignments, unused_mut)
      )]
      let mut backdrops_left = layer_plan
        .steps(target)
        .iter()
        .filter(|step| {
          matches!(step, LayerStep::Backdrop(index)
            if self.scratch_backdrop_slots.get(*index).copied().flatten().is_some_and(|slots| slots.runs()))
        })
        .count();
      macro_rules! begin_target_pass {
        ($load:expr, $first:expr) => {
          encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some(if target.layer.is_some() {
              "lurq_layer_pass"
            } else {
              "lurq_pass"
            }),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
              view: &target_view,
              depth_slice: None,
              resolve_target: None,
              ops: wgpu::Operations {
                load: $load,
                store: wgpu::StoreOp::Store,
              },
            })],
            // The UI's GPU time: the first pass's start to the end of the
            // window's last pass.
            #[cfg(feature = "perf_profile")]
            timestamp_writes: self.gpu_timer.ui_pass_writes(
              $first && target_index == 0,
              target_index + 1 == target_count && backdrops_left == 0,
            ),
            ..Default::default()
          })
        };
      }
      let mut pass = begin_target_pass!(wgpu::LoadOp::Clear(clear), true);
      profile_if! { _layers.render_passes += 1; }

      let mut active_pipeline = None;
      let mut active_vertex_buffer = None;
      let mut active_index_buffer = None;

      for step in layer_plan.steps(target) {
        let range = match step {
          LayerStep::Draws(range) => range.clone(),
          LayerStep::Backdrop(index) => {
            let (Some(backdrop), Some(compositor)) = (
              self.scratch_backdrop_slots.get(*index).copied().flatten(),
              self.layer_compositor.as_ref(),
            ) else {
              continue;
            };
            if backdrop.runs() {
              let (target_texture, target_origin) = match (target.layer, layer_slot) {
                (Some(layer), Some(slot)) => (compositor.texture(slot), list.layers[layer].painted().0),
                _ => (window_texture, [0, 0]),
              };
              drop(pass);
              let _blur_passes = compositor.prepare_backdrop(
                queue,
                &mut encoder,
                vtx_buf,
                idx_buf,
                target_texture,
                target_origin,
                &backdrop,
              );
              backdrops_left -= 1;
              pass = begin_target_pass!(wgpu::LoadOp::Load, false);
              profile_if! {
                _layers.render_passes += _blur_passes + 1;
                _layers.blur_draws += _blur_passes;
                _layers.target_copies += 1;
                _layers.backdrops_blurred += 1;
              }
              // A new pass starts with nothing bound.
              active_vertex_buffer = None;
              active_index_buffer = None;
            } else {
              // Its kept result is put back in the pass, as it is.
              profile_if! { _layers.backdrops_reused += 1; }
            }
            if active_vertex_buffer != Some(ActiveVertexBuffer::CommonQuad) {
              pass.set_vertex_buffer(0, vtx_buf.slice(..));
              active_vertex_buffer = Some(ActiveVertexBuffer::CommonQuad);
            }
            if active_index_buffer != Some(ActiveIndexBuffer::CommonU16) {
              pass.set_index_buffer(idx_buf.slice(..), wgpu::IndexFormat::Uint16);
              active_index_buffer = Some(ActiveIndexBuffer::CommonU16);
            }
            active_pipeline = Some(ActivePipeline::Layer);
            if set_scissor(&mut pass, space.layer_rect(backdrop.bounds()), &space) {
              compositor.composite_backdrop(queue, &mut pass, &backdrop, &space);
              profile_if! {
                _draw_calls += 2;
                _layers.blur_draws += 2;
              }
            }
            continue;
          }
          LayerStep::Composite(child) => {
            let child_layer = layer_plan.targets()[*child].layer;
            let child_slot = self.scratch_layer_slots.get(*child).copied().flatten();
            let (Some(layer), Some(slot), Some(compositor)) = (child_layer, child_slot, self.layer_compositor.as_ref())
            else {
              continue;
            };
            let layer = &list.layers[layer];
            if !set_scissor(&mut pass, space.layer_rect(layer.bounds), &space) {
              continue;
            }
            if active_vertex_buffer != Some(ActiveVertexBuffer::CommonQuad) {
              pass.set_vertex_buffer(0, vtx_buf.slice(..));
              active_vertex_buffer = Some(ActiveVertexBuffer::CommonQuad);
            }
            if active_index_buffer != Some(ActiveIndexBuffer::CommonU16) {
              pass.set_index_buffer(idx_buf.slice(..), wgpu::IndexFormat::Uint16);
              active_index_buffer = Some(ActiveIndexBuffer::CommonU16);
            }
            match self.scratch_blur_slots.get(*child).copied().flatten() {
              Some(blur_slots) => {
                compositor.composite_blurred(queue, &mut pass, &blur_slots, layer, &space);
                profile_if! { _layers.blur_draws += 1; }
              }
              None => compositor.composite(queue, &mut pass, slot, layer, &space),
            }
            profile_if! { _draw_calls += 1; }
            active_pipeline = Some(ActivePipeline::Layer);
            continue;
          }
        };
        let draws = &ordered_draws[range];
        let mut next_draw = 0;
        while let Some((_, draw)) = draws.get(next_draw) {
          next_draw += 1;
          match draw {
            OrderedDraw::Rect(index) => {
              let r = &list.rects[*index];
              if !set_scissor(&mut pass, space.clip(r.clip), &space) {
                continue;
              }
              let prepared = &self.scratch_rect_draws[*index];
              let start = (prepared.start * std::mem::size_of::<QuadInstance>()) as wgpu::BufferAddress;
              let end =
                ((prepared.start + prepared.count) * std::mem::size_of::<QuadInstance>()) as wgpu::BufferAddress;
              if active_pipeline != Some(ActivePipeline::Quad) {
                pass.set_pipeline(self.quad_pipeline.as_ref().unwrap());
                active_pipeline = Some(ActivePipeline::Quad);
              }
              if needs_clip_globals(r.clip, &space) {
                let (clip_key, clip_globals) = self.clip_globals.get(device, r.clip, &space);
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
                        resource: gradients.binding(),
                      },
                    ],
                  })
                });
                pass.set_bind_group(0, &*clip_bind_group, &[]);
              } else {
                pass.set_bind_group(0, self.quad_bind_group.as_ref().unwrap(), &[]);
              }
              if active_vertex_buffer != Some(ActiveVertexBuffer::CommonQuad) {
                pass.set_vertex_buffer(0, vtx_buf.slice(..));
                active_vertex_buffer = Some(ActiveVertexBuffer::CommonQuad);
              }
              pass.set_vertex_buffer(1, rect_instance_buf.unwrap().slice(start..end));
              if active_index_buffer != Some(ActiveIndexBuffer::CommonU16) {
                pass.set_index_buffer(idx_buf.slice(..), wgpu::IndexFormat::Uint16);
                active_index_buffer = Some(ActiveIndexBuffer::CommonU16);
              }
              pass.draw_indexed(0..6, 0, 0..prepared.count as u32);
              profile_if! { _draw_calls += 1; }
            }
            OrderedDraw::Glyph(run) => {
              // The texts painted next join this draw: those under its clip,
              // or any that no clip cuts when none cuts this one.
              let following = draws[next_draw..].iter().map(|(_, draw)| draw.glyph_run());
              let batch = batch_glyph_runs(&list.glyphs, *run, following, space.is_window());
              next_draw += batch.joined;
              let (start, count) = (run.start, batch.count);
              let glyph_slice = &list.glyphs[start..start + count];
              let clip = if batch.unclipped {
                crate::layout::quad::ClipRect::default()
              } else {
                glyph_slice[0].clip
              };
              if glyph_slice.is_empty() || !set_scissor(&mut pass, space.clip(clip), &space) {
                continue;
              }
              let start_byte = (start * std::mem::size_of::<GlyphInstance>()) as wgpu::BufferAddress;
              let end_byte = ((start + count) * std::mem::size_of::<GlyphInstance>()) as wgpu::BufferAddress;
              if active_pipeline != Some(ActivePipeline::Glyph) {
                pass.set_pipeline(self.glyph_pipeline.as_ref().unwrap());
                active_pipeline = Some(ActivePipeline::Glyph);
              }
              if needs_clip_globals(clip, &space) {
                let (clip_key, clip_globals) = self.clip_globals.get(device, clip, &space);
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
              if active_vertex_buffer != Some(ActiveVertexBuffer::CommonQuad) {
                pass.set_vertex_buffer(0, vtx_buf.slice(..));
                active_vertex_buffer = Some(ActiveVertexBuffer::CommonQuad);
              }
              pass.set_vertex_buffer(1, glyph_instance_buf.unwrap().slice(start_byte..end_byte));
              if active_index_buffer != Some(ActiveIndexBuffer::CommonU16) {
                pass.set_index_buffer(idx_buf.slice(..), wgpu::IndexFormat::Uint16);
                active_index_buffer = Some(ActiveIndexBuffer::CommonU16);
              }
              pass.draw_indexed(0..6, 0, 0..count as u32);
              profile_if! { _draw_calls += 1; }
            }
            #[cfg(feature = "raster")]
            OrderedDraw::Image(index) => {
              let img = &list.images[*index];
              let image_bgl = self.image_bgl.as_ref().unwrap().clone();
              let nv12_image_bgl = self.nv12_image_bgl.as_ref().unwrap().clone();
              let image_sampler = self.image_sampler.as_ref().unwrap().clone();
              let image_pipeline = self.image_pipeline.as_ref().unwrap().clone();
              let nv12_image_pipeline = self.nv12_image_pipeline.as_ref().unwrap().clone();

              let external_wgpu_image = img
                .native
                .as_ref()
                .filter(|native| native.backend() == crate::images::NativeImageBackend::WgpuExternalRgba);
              #[cfg(feature = "canvas")]
              let canvas_image = img
                .native
                .as_ref()
                .filter(|n| n.backend() == crate::images::NativeImageBackend::Canvas);
              #[cfg(not(feature = "canvas"))]
              let canvas_image: Option<&crate::images::NativeImageData> = None;
              #[cfg(feature = "canvas")]
              if let Some(native) = canvas_image {
                let snapshot = native
                  .payload::<crate::canvas::CanvasWeak>()
                  .and_then(|c| c.upgrade())
                  .and_then(|c| self.canvas_renderer.as_ref()?.snapshot(&c));
                let Some(snapshot) = snapshot else {
                  continue;
                };
                let current = self.image_texture_cache.get(&img.image_id).is_some_and(
                  |c| matches!(c,CachedImageTexture::ExternalRgba {version,..} if *version==snapshot.version),
                );
                if !current {
                  let Some(made) = create_external_rgba_cached_image_texture(
                    device,
                    &image_bgl,
                    &image_sampler,
                    globals_buffer,
                    snapshot,
                  ) else {
                    continue;
                  };
                  self.image_texture_cache.insert(img.image_id, made);
                  self.image_clip_bind_groups.clear();
                }
              }
              if let Some(native) = external_wgpu_image {
                let slot = native.payload::<crate::images::WgpuExternalImageState>();
                let Some(snapshot) = slot.and_then(crate::images::WgpuExternalImageState::snapshot) else {
                  self.image_texture_cache.remove(&img.image_id);
                  continue;
                };
                // A view the app gave before a loss is of the lost device, and of the instance of wgpu that
                // went with it: nothing is asked of wgpu with it, and nothing is drawn of it until the app
                // gives one of this device.
                if !snapshot.is_of(self.device_number) {
                  self.image_texture_cache.remove(&img.image_id);
                  continue;
                }
                let is_current = self.image_texture_cache.get(&img.image_id).is_some_and(|cached| {
                  matches!(
                    cached,
                    CachedImageTexture::ExternalRgba {
                      width,
                      height,
                      version,
                      ..
                    } if *width == snapshot.width && *height == snapshot.height && *version == snapshot.version
                  )
                });
                if !is_current {
                  // And one of another device all the same, given late by something of the app that was not
                  // told of the loss, is refused by this device, which ends nothing.
                  let Some(made) = create_external_rgba_cached_image_texture(
                    device,
                    &image_bgl,
                    &image_sampler,
                    globals_buffer,
                    snapshot,
                  ) else {
                    self.image_texture_cache.remove(&img.image_id);
                    continue;
                  };
                  self.image_texture_cache.insert(img.image_id, made);
                  self.image_clip_bind_groups.clear();
                  // Its view is this device's to let go of, should the device be lost.
                  if let Some(slot) = slot {
                    self.external_slots.retain(|known| known.upgrade().is_some());
                    if !self.external_slots.iter().any(|known| known.is(slot)) {
                      self.external_slots.push(slot.downgrade());
                    }
                  }
                }
              } else if canvas_image.is_none()
                && !self
                  .image_texture_cache
                  .get(&img.image_id)
                  .is_some_and(|cached| cached.is_compatible(img))
              {
                self.image_texture_cache.remove(&img.image_id);
                self.image_clip_bind_groups.clear();
              }
              // A frame of a video that stays on the card is another buffer with each frame: its textures are
              // made anew of the one shown now. Those of the frame before are kept until the next frame is
              // drawn, which the card may still be drawing them for.
              #[cfg(any(target_os = "macos", target_os = "ios"))]
              if self
                .image_texture_cache
                .get(&img.image_id)
                .is_some_and(|cached| cached.is_another_frame_of_the_card(img))
                && let Some(before) = self.image_texture_cache.remove(&img.image_id)
              {
                self.retired_card_frames.push(before);
                self.image_clip_bind_groups.clear();
              }

              if external_wgpu_image.is_none()
                && canvas_image.is_none()
                && !is_card_frames(img)
                && !self.image_texture_cache.contains_key(&img.image_id)
              {
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
                  crate::images::ImagePixelFormat::Nv12 => create_nv12_cached_image_texture(
                    device,
                    queue,
                    &nv12_image_bgl,
                    &image_sampler,
                    globals_buffer,
                    img,
                  ),
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
                CachedImageTexture::ExternalRgba { .. } => {}
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
                  if img.native.is_none() && (*frame_index != img.frame_index || *version != img.version) {
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
                // Nothing is sent to the card for a frame that is there already.
                #[cfg(windows)]
                CachedImageTexture::Card(_) => {}
              }

              if !set_scissor(&mut pass, space.clip(img.clip), &space) {
                continue;
              }

              // A card's frame of red, green and blue is drawn as a texture the app gave is: one view, bound.
              #[cfg(windows)]
              let card_rgba = match &*cached {
                CachedImageTexture::Card(opened) if opened.is_rgba() => match opened.drawn() {
                  Some((surface, slot)) => Some((&slot.bind_group, &slot.first, surface)),
                  None => continue,
                },
                _ => None,
              };
              #[cfg(not(windows))]
              let card_rgba: Option<(&wgpu::BindGroup, &wgpu::TextureView, usize)> = None;
              let one_view = match (&*cached, card_rgba) {
                (_, Some(found)) => Some(found),
                (CachedImageTexture::ExternalRgba { bind_group, view, .. }, None) => Some((bind_group, view, 0)),
                _ => None,
              };

              match &*cached {
                _ if one_view.is_some() => {
                  let (bind_group, view, frame) = one_view.expect("just found");
                  if active_pipeline != Some(ActivePipeline::Image) {
                    pass.set_pipeline(&image_pipeline);
                    active_pipeline = Some(ActivePipeline::Image);
                  }
                  if needs_clip_globals(img.clip, &space) {
                    let (clip_key, clip_globals) = self.clip_globals.get(device, img.clip, &space);
                    let bind_key = ImageClipBindGroupKey {
                      image_id: img.image_id,
                      frame_index: frame,
                      clip: clip_key,
                      format: ImageClipFormat::Rgba,
                    };
                    let clip_bind_group = self.image_clip_bind_groups.entry(bind_key).or_insert_with(|| {
                      device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("lurq_external_image_clip_bg"),
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
                  if active_pipeline != Some(ActivePipeline::Image) {
                    pass.set_pipeline(&image_pipeline);
                    active_pipeline = Some(ActivePipeline::Image);
                  }
                  if needs_clip_globals(img.clip, &space) {
                    let (clip_key, clip_globals) = self.clip_globals.get(device, img.clip, &space);
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
                    pass.set_bind_group(0, bind_group, &[]);
                  }
                }
                nv12 => {
                  // The two planes to draw, and which frame they are: of memory's one texture, or of the card's
                  // texture that is shown now.
                  let (bind_group, y_view, uv_view, frame) = match nv12 {
                    CachedImageTexture::Nv12 {
                      bind_group,
                      y_view,
                      uv_view,
                      ..
                    } => (&*bind_group, &*y_view, &*uv_view, img.frame_index),
                    #[cfg(windows)]
                    CachedImageTexture::Card(opened) => {
                      let Some((surface, slot)) = opened.drawn() else {
                        continue;
                      };
                      let Some(color) = slot.second.as_ref() else {
                        continue;
                      };
                      (&slot.bind_group, &slot.first, color, surface)
                    }
                    _ => continue,
                  };
                  if active_pipeline != Some(ActivePipeline::Nv12Image) {
                    pass.set_pipeline(&nv12_image_pipeline);
                    active_pipeline = Some(ActivePipeline::Nv12Image);
                  }
                  if needs_clip_globals(img.clip, &space) {
                    let (clip_key, clip_globals) = self.clip_globals.get(device, img.clip, &space);
                    let bind_key = ImageClipBindGroupKey {
                      image_id: img.image_id,
                      frame_index: frame,
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
              if active_vertex_buffer != Some(ActiveVertexBuffer::CommonQuad) {
                pass.set_vertex_buffer(0, vtx_buf.slice(..));
                active_vertex_buffer = Some(ActiveVertexBuffer::CommonQuad);
              }
              pass.set_vertex_buffer(
                1,
                image_instance_buf
                  .unwrap()
                  .slice(image_instance_start..image_instance_end),
              );
              if active_index_buffer != Some(ActiveIndexBuffer::CommonU16) {
                pass.set_index_buffer(idx_buf.slice(..), wgpu::IndexFormat::Uint16);
                active_index_buffer = Some(ActiveIndexBuffer::CommonU16);
              }
              pass.draw_indexed(0..6, 0, 0..1);
              profile_if! { _draw_calls += 1; }
            }
          }
        }
      }
      drop(pass);
      // A blurred layer is reduced and blurred along x once it is painted;
      // the pass compositing it blurs along y.
      if let (Some(slot), Some(blur_slots), Some(compositor)) = (
        layer_slot,
        self.scratch_blur_slots.get(target_index).copied().flatten(),
        self.layer_compositor.as_ref(),
      ) {
        let _blur_passes = compositor.prepare_blur(queue, &mut encoder, vtx_buf, idx_buf, slot, &blur_slots);
        profile_if! {
          _layers.render_passes += _blur_passes;
          _layers.blur_draws += _blur_passes;
        }
      }
    }
    if let Some(compositor) = self.layer_compositor.as_mut() {
      compositor.end_frame();
      profile_if! {
        (_layers.textures, _layers.texture_bytes) = compositor.texture_stats();
        (_, _layers.kept_textures, _layers.kept_texture_bytes) = compositor.backdrop_stats();
      }
    }
    if self.clip_globals.end_frame() {
      let clip_globals = &self.clip_globals;
      self.quad_clip_bind_groups.retain(|key, _| clip_globals.contains(key));
      self.glyph_clip_bind_groups.retain(|key, _| clip_globals.contains(key));
      #[cfg(feature = "raster")]
      self
        .image_clip_bind_groups
        .retain(|key, _| clip_globals.contains(&key.clip));
    }
    self.scratch_ordered_draws = ordered_draws;
    self.layer_plan = layer_plan;
    let _encode_dur = profile_elapsed!(_encode_start);
    #[cfg(feature = "perf_profile")]
    drop(_encode_phase);

    #[cfg(feature = "screenshot")]
    let frame_capture = self
      .pending_frame_capture
      .take()
      .and_then(|capture| encode_wgpu_frame_capture(device, &mut encoder, window_texture, config.format, capture));
    if let Some(proxy) = surface_proxy {
      proxy.put_onto(&mut encoder, &view);
    }

    #[cfg(feature = "perf_profile")]
    self.gpu_timer.resolve(&mut encoder);

    let _submit_start = profile_scope!();
    #[cfg(feature = "perf_profile")]
    let _submit_phase = self
      .profile_context
      .as_ref()
      .map(|context| context.phase(crate::app::profiler::Phase::RenderSubmit));
    queue.submit(std::iter::once(encoder.finish()));
    // The card's frames that were drawn before this one are let go of once the card has done this far.
    #[cfg(all(feature = "raster", windows))]
    self.card_leases.let_go_after(queue);
    // The textures the frame blurred backdrops into hold them from here on.
    if let Some(compositor) = self.layer_compositor.as_mut() {
      compositor.frame_submitted();
    }
    let _submit_dur = profile_elapsed!(_submit_start);
    #[cfg(feature = "perf_profile")]
    drop(_submit_phase);
    #[cfg(feature = "perf_profile")]
    self.gpu_timer.submitted();

    #[cfg(feature = "screenshot")]
    if let Some(frame_capture) = frame_capture {
      frame_capture.start(device.clone(), self.lost.clone());
    }

    let _present_start = profile_scope!();
    #[cfg(feature = "perf_profile")]
    let _present_phase = self
      .profile_context
      .as_ref()
      .map(|context| context.phase(crate::app::profiler::Phase::RenderPresent));
    // A device may be lost while its frame is drawn: what it was given to do tells. wgpu neither presents nor
    // discards a frame of a lost device, and keeps it; let go of with that frame out, the surface ends the
    // program under Vulkan (`Trying to destroy a SwapchainAcquireSemaphore that is still in use by a
    // SurfaceTexture`), and the engine that is to take this one's place never comes. The frame's texture is
    // destroyed first, which is what gives the frame back.
    if self.device_is_lost() {
      output.texture().destroy();
    }
    output.present();
    let _present_dur = profile_elapsed!(_present_start);
    #[cfg(feature = "perf_profile")]
    drop(_present_phase);

    profile_if! {
      self.last_profile = RenderProfile {
        init: _init_dur,
        acquire: _acquire_dur,
        canvas: _canvas_profile,
        globals_upload: _globals_dur,
        atlas_upload: _atlas_dur,
        glyph_atlas_upload_bytes: _atlas_upload_bytes,
        glyph_atlas_upload_rects: _atlas_upload_rects,
        glyph_atlas_full_uploads: _atlas_full_uploads,
        glyph_atlas_arena_uploads: 0,
        glyph_atlas_dedicated_uploads: 0,
        draw_calls: _draw_calls,
        buffer_upload: _buffer_upload_dur,
        image_upload: _image_texture_upload_dur,
        encode: _encode_dur,
        submit: _submit_dur,
        present: _present_dur,
        total: profile_elapsed!(_total_start),
      };
      self.last_layer_profile = _layers;
    }
    true
  }

  #[cfg(feature = "screenshot")]
  fn supports_frame_capture(&self) -> bool {
    true
  }

  #[cfg(feature = "screenshot")]
  fn render_with_capture(
    &mut self,
    list: &RenderList,
    window: WindowHandle<'_>,
    display: DisplayHandle<'_>,
    capture: Option<RenderFrameCapture>,
  ) -> bool {
    if let Some(previous) = std::mem::replace(&mut self.pending_frame_capture, capture) {
      previous.target.fail("superseded before a frame was rendered");
    }
    self.render(list, window, display)
  }

  /// Also where the device is whole and of another card than the one a device was lost on, once that card
  /// makes a device again: the engine that takes this one's place draws on it.
  /// Every engine on another card says so once one of them found that: a window that is not drawn now asks
  /// nothing, and goes back with the one that asked.
  fn lost(&self) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    if self.displaced && device_loss::the_card_gave() {
      return true;
    }
    self.device_is_lost() || self.card_is_back
  }

  fn release_window_surface(&mut self) {
    let _own = app_errors::OwnWork::begin();
    if let Some(device) = &self.device {
      device_loss::asked(device, Some(&self.lost), wgpu::PollType::Poll);
    }
    self.surface_config = None;
    self.surface = None;
  }

  fn wants_redraw(&self) -> bool {
    self.frame_extensions.iter().any(|entry| entry.extension.wants_redraw())
  }

  #[cfg(feature = "perf_profile")]
  fn last_profile(&self) -> Option<RenderProfile> {
    Some(self.last_profile)
  }

  #[cfg(feature = "perf_profile")]
  fn layer_profile(&self) -> Option<crate::app::profile_types::LayerProfile> {
    Some(self.last_layer_profile)
  }
}

/// Where a frame is drawn: the window surface's texture, or a texture of the surface's size and format that a capture
/// is taken from and that is never shown.
enum FrameTarget {
  Surface(wgpu::SurfaceTexture),
  Offscreen(wgpu::Texture),
}

impl FrameTarget {
  fn texture(&self) -> &wgpu::Texture {
    match self {
      Self::Surface(output) => &output.texture,
      Self::Offscreen(texture) => texture,
    }
  }

  fn present(self) {
    if let Self::Surface(output) = self {
      output.present();
    }
  }
}

#[cfg(feature = "screenshot")]
struct WgpuFrameCaptureReadback {
  buffer: wgpu::Buffer,
  format: wgpu::TextureFormat,
  padded_bytes_per_row: u32,
  capture: RenderFrameCapture,
}

#[cfg(feature = "screenshot")]
impl WgpuFrameCaptureReadback {
  fn start(self, device: wgpu::Device, lost: std::sync::Arc<std::sync::atomic::AtomicBool>) {
    std::thread::spawn(move || {
      let slice = self.buffer.slice(..);
      let (sender, receiver) = std::sync::mpsc::channel();
      slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
      });
      // A device that is lost gives no pixels, and asking it would end this thread with nothing said to
      // whoever waits for the screenshot.
      if !device_loss::asked(&device, Some(&lost), wgpu::PollType::wait_indefinitely()) {
        self.capture.target.fail("the graphics device was lost");
        return;
      }
      match receiver.recv() {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
          self
            .capture
            .target
            .fail(format!("failed to map screenshot readback buffer: {error:?}"));
          return;
        }
        Err(error) => {
          self
            .capture
            .target
            .fail(format!("failed to receive screenshot readback result: {error}"));
          return;
        }
      }

      let data = slice.get_mapped_range();
      let bgra = matches!(
        self.format,
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
      );
      let pixels = crate::app::frame_capture::capture_rows_to_rgba(
        &data,
        self.padded_bytes_per_row as usize,
        self.capture.width,
        self.capture.height,
        bgra,
      );
      drop(data);
      self.buffer.unmap();

      crate::app::frame_capture::finish_capture(pixels, &self.capture);
    });
  }
}

#[cfg(feature = "screenshot")]
fn encode_wgpu_frame_capture(
  device: &wgpu::Device,
  encoder: &mut wgpu::CommandEncoder,
  texture: &wgpu::Texture,
  format: wgpu::TextureFormat,
  capture: RenderFrameCapture,
) -> Option<WgpuFrameCaptureReadback> {
  if capture.width == 0 || capture.height == 0 {
    capture.target.fail("capture region is empty");
    return None;
  }

  let bytes_per_pixel = 4_u32;
  let unpadded_bytes_per_row = capture.width * bytes_per_pixel;
  let padded_bytes_per_row =
    crate::app::frame_capture::align_capture_row_pitch(unpadded_bytes_per_row, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
  let buffer_size = padded_bytes_per_row as u64 * capture.height as u64;
  let buffer = device.create_buffer(&wgpu::BufferDescriptor {
    label: Some("lurq_screenshot_frame_capture"),
    size: buffer_size,
    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
    mapped_at_creation: false,
  });

  encoder.copy_texture_to_buffer(
    wgpu::TexelCopyTextureInfo {
      texture,
      mip_level: 0,
      origin: wgpu::Origin3d {
        x: capture.x,
        y: capture.y,
        z: 0,
      },
      aspect: wgpu::TextureAspect::All,
    },
    wgpu::TexelCopyBufferInfo {
      buffer: &buffer,
      layout: wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(padded_bytes_per_row),
        rows_per_image: Some(capture.height),
      },
    },
    wgpu::Extent3d {
      width: capture.width,
      height: capture.height,
      depth_or_array_layers: 1,
    },
  );

  Some(WgpuFrameCaptureReadback {
    buffer,
    format,
    padded_bytes_per_row,
    capture,
  })
}

#[cfg(feature = "raster")]
fn create_external_rgba_cached_image_texture(
  device: &wgpu::Device,
  image_bgl: &wgpu::BindGroupLayout,
  image_sampler: &wgpu::Sampler,
  globals_buffer: &wgpu::Buffer,
  snapshot: crate::images::WgpuExternalImageSnapshot,
) -> Option<CachedImageTexture> {
  // The view may be of another device: one an app gave before a loss, which nothing of this device binds.
  let bind_group = device_loss::taken(device, || {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
      label: Some("lurq_external_img_bg"),
      layout: image_bgl,
      entries: &[
        wgpu::BindGroupEntry {
          binding: 0,
          resource: globals_buffer.as_entire_binding(),
        },
        wgpu::BindGroupEntry {
          binding: 1,
          resource: wgpu::BindingResource::TextureView(&snapshot.view),
        },
        wgpu::BindGroupEntry {
          binding: 2,
          resource: wgpu::BindingResource::Sampler(image_sampler),
        },
      ],
    })
  })?;
  Some(CachedImageTexture::ExternalRgba {
    bind_group,
    view: snapshot.view,
    width: snapshot.width,
    height: snapshot.height,
    version: snapshot.version,
  })
}

#[cfg(feature = "raster")]
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

#[cfg(feature = "raster")]
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

#[cfg(feature = "raster")]
fn create_nv12_cached_image_texture(
  device: &wgpu::Device,
  queue: &wgpu::Queue,
  nv12_image_bgl: &wgpu::BindGroupLayout,
  image_sampler: &wgpu::Sampler,
  globals_buffer: &wgpu::Buffer,
  image: &crate::images::ImageCmd,
) -> Option<CachedImageTexture> {
  if image.image_width == 0
    || image.image_height == 0
    || !image.image_width.is_multiple_of(2)
    || !image.image_height.is_multiple_of(2)
  {
    return None;
  }
  #[cfg(any(target_os = "macos", target_os = "ios"))]
  if matches!(
    image.native.as_ref().map(crate::images::NativeImageData::backend),
    Some(crate::images::NativeImageBackend::MacosCvPixelBufferNv12)
  ) {
    return create_macos_native_nv12_cached_image_texture(device, nv12_image_bgl, image_sampler, globals_buffer, image);
  }
  // The frames of the card are opened before the images are drawn, and are not made here.
  if is_card_frames(image) {
    return None;
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
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    _macos_native: None,
    width: image.image_width,
    height: image.image_height,
    frame_index: image.frame_index,
    version: image.version,
  })
}

#[cfg(all(feature = "raster", any(target_os = "macos", target_os = "ios")))]
fn create_macos_native_nv12_cached_image_texture(
  device: &wgpu::Device,
  nv12_image_bgl: &wgpu::BindGroupLayout,
  image_sampler: &wgpu::Sampler,
  globals_buffer: &wgpu::Buffer,
  image: &crate::images::ImageCmd,
) -> Option<CachedImageTexture> {
  let native = image.native.as_ref()?.clone();
  // One buffer the app gave, or the frame of a video that is shown now: none before its first.
  let buffer = match native.payload::<crate::images::CvPixelBufferFrames>() {
    Some(frames) => frames.shown()?,
    None => native
      .payload::<crate::images::MacosCvPixelBufferNv12Image>()?
      .pixel_buffer
      .clone(),
  };
  let (y_texture, y_cv_texture) = create_macos_native_plane_texture(
    device,
    buffer.as_ptr(),
    image.image_width,
    image.image_height,
    0,
    objc2_metal::MTLPixelFormat::R8Unorm,
    wgpu::TextureFormat::R8Unorm,
    "lurq_macos_nv12_y",
  )?;
  let (uv_texture, uv_cv_texture) = create_macos_native_plane_texture(
    device,
    buffer.as_ptr(),
    image.image_width / 2,
    image.image_height / 2,
    1,
    objc2_metal::MTLPixelFormat::RG8Unorm,
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
      _buffer: buffer,
      _y_cv_texture: y_cv_texture,
      _uv_cv_texture: uv_cv_texture,
    }),
    width: image.image_width,
    height: image.image_height,
    frame_index: image.frame_index,
    version: image.version,
  })
}

#[cfg(all(feature = "raster", any(target_os = "macos", target_os = "ios")))]
fn create_macos_native_plane_texture(
  device: &wgpu::Device,
  pixel_buffer: CVPixelBufferRef,
  width: u32,
  height: u32,
  plane_index: usize,
  metal_format: objc2_metal::MTLPixelFormat,
  wgpu_format: wgpu::TextureFormat,
  label: &'static str,
) -> Option<(wgpu::Texture, CvMetalTexture)> {
  // wgpu 29 returns a HAL guard and uses objc2 Metal protocol objects.
  // Clone the retained device before releasing the guard; the CoreVideo cache
  // and textures must originate from the same Metal device as wgpu.
  let raw_device = unsafe { device.as_hal::<wgpu_hal::api::Metal>() }?.raw_device().clone();
  // One cache of textures for a device, kept: a video asks it for two textures a frame.
  let device_ptr = Retained::as_ptr(&raw_device) as usize;
  let cache = CV_TEXTURE_CACHE.with(|kept| {
    let mut kept = kept.borrow_mut();
    if let Some((of, cache)) = kept.as_ref()
      && *of == device_ptr
    {
      return Some(cache.ptr);
    }
    let mut cache = std::ptr::null_mut();
    let status = unsafe {
      CVMetalTextureCacheCreate(
        kCFAllocatorDefault,
        std::ptr::null(),
        Retained::as_ptr(&raw_device).cast_mut().cast(),
        std::ptr::null(),
        &mut cache,
      )
    };
    if status != 0 || cache.is_null() {
      return None;
    }
    *kept = Some((device_ptr, CvMetalTextureCache { ptr: cache }));
    Some(cache)
  })?;

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
  // CVMetalTextureGetTexture returns a borrowed Objective-C texture. Retain it
  // for wgpu independently of the CVMetalTexture owner retained below.
  let metal_texture: Retained<ProtocolObject<dyn objc2_metal::MTLTexture>> =
    unsafe { Retained::retain(metal_texture_ptr.cast()).expect("non-null CoreVideo Metal texture") };
  let hal_texture = unsafe {
    wgpu_hal::metal::Device::texture_from_raw(
      metal_texture,
      wgpu_format,
      objc2_metal::MTLTextureType::Type2D,
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

#[cfg(feature = "raster")]
fn write_rgba_image_texture(queue: &wgpu::Queue, texture: &wgpu::Texture, image: &crate::images::ImageCmd) {
  write_rgba_texture_data(queue, texture, image.image_width, image.image_height, &image.data);
}

#[cfg(feature = "raster")]
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

#[cfg(feature = "raster")]
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

/// The format lurq renders the surface through: the surface's own format
/// without an sRGB suffix. Shaders write sRGB-encoded colour, so alpha blending
/// mixes encoded values like CSS instead of linear light.
fn render_target_format(surface_format: wgpu::TextureFormat) -> wgpu::TextureFormat {
  surface_format.remove_srgb_suffix()
}

/// The clear value for the non-sRGB render target: the colour's sRGB channels.
fn wgpu_clear_color(color: crate::node::color::Color) -> wgpu::Color {
  let [r, g, b, a] = color.to_f32_array();
  wgpu::Color {
    r: r as f64,
    g: g as f64,
    b: b as f64,
    a: a as f64,
  }
}

/// Globals for a draw clipped by `clip`, already in `space`'s pixels.
fn globals_for_clip(clip: crate::layout::quad::ClipRect, space: &TargetSpace) -> Globals {
  Globals::for_clip(space.viewport(), clip)
}

fn rounded_clip_needs_shader(clip: crate::layout::quad::ClipRect) -> bool {
  clip.is_rounded()
}

/// Whether a draw into `space` needs globals of its own: the shared ones
/// describe the whole window and no rounded clip.
fn needs_clip_globals(clip: crate::layout::quad::ClipRect, space: &TargetSpace) -> bool {
  !space.is_window() || rounded_clip_needs_shader(clip)
}

/// Frames a clip's globals may stay unused before their buffer is released.
/// Layers that move create globals of their own every frame.
const CLIP_GLOBALS_IDLE_FRAMES: u64 = 120;

/// Uniform buffers holding the globals of clipped draws and of draws into
/// layers, keyed by their contents.
#[derive(Default)]
struct ClipGlobalsCache {
  buffers: std::collections::HashMap<ClipGlobalsKey, (wgpu::Buffer, u64)>,
  frame: u64,
}

impl ClipGlobalsCache {
  /// The globals of a draw into `space` clipped by `clip` (window pixels).
  fn get(
    &mut self,
    device: &wgpu::Device,
    clip: crate::layout::quad::ClipRect,
    space: &TargetSpace,
  ) -> (ClipGlobalsKey, &wgpu::Buffer) {
    let globals = globals_for_clip(space.clip(clip), space);
    let key = clip_globals_key(&globals);
    let frame = self.frame;
    let (buffer, last_used) = self.buffers.entry(key).or_insert_with(|| {
      let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("lurq_clip_globals"),
        contents: bytemuck::bytes_of(&globals),
        usage: wgpu::BufferUsages::UNIFORM,
      });
      (buffer, frame)
    });
    *last_used = frame;
    (key, buffer)
  }

  fn contains(&self, key: &ClipGlobalsKey) -> bool {
    self.buffers.contains_key(key)
  }

  fn clear(&mut self) {
    self.buffers.clear();
  }

  /// Releases globals unused for a while; returns whether any were, so bind
  /// groups built on them can be dropped too.
  fn end_frame(&mut self) -> bool {
    let frame = self.frame;
    self.frame += 1;
    let before = self.buffers.len();
    self
      .buffers
      .retain(|_, (_, last_used)| frame - *last_used <= CLIP_GLOBALS_IDLE_FRAMES);
    self.buffers.len() != before
  }
}

fn clip_globals_key(globals: &Globals) -> ClipGlobalsKey {
  ClipGlobalsKey(bytemuck::cast(*globals))
}

/// Scissors to `clip`, already in `space`'s pixels.
fn set_scissor(pass: &mut wgpu::RenderPass<'_>, clip: crate::layout::quad::ClipRect, space: &TargetSpace) -> bool {
  let Some((x, y, width, height)) = scissor_rect(clip, space.width, space.height) else {
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

fn wgpu_staged_texture_bytes(row_bytes: u32, height: u32) -> usize {
  align_up_usize(row_bytes as usize, wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize) * height as usize
}

fn align_up_usize(value: usize, alignment: usize) -> usize {
  debug_assert!(alignment > 0);
  value.div_ceil(alignment) * alignment
}

#[cfg(test)]
mod tests {
  use crate::{app::render_engine::RenderEngine, layout::quad::ClipRect};

  #[test]
  fn backends_are_selected_without_initializing_excluded_loaders() {
    let engine = super::WgpuRenderEngine::new();
    assert!(engine.instance.is_none());
    #[cfg(windows)]
    assert_eq!(engine.backends, super::WgpuBackends::DX12);
    let engine = engine.with_backends(super::WgpuBackends::VULKAN);
    assert!(engine.instance.is_none());
    assert_eq!(engine.backends, super::WgpuBackends::VULKAN);
  }

  #[test]
  fn scissor_expands_fractional_clip_to_include_bottom_right_edge() {
    assert_eq!(
      super::scissor_rect(ClipRect::rect(10.6, 20.2, 30.1, 40.6), 100.0, 100.0,),
      Some((10, 20, 31, 41))
    );
  }

  #[test]
  fn resize_defers_surface_reconfiguration_to_the_next_frame() {
    let mut engine = super::WgpuRenderEngine::new();
    engine.surface_config = Some(wgpu::SurfaceConfiguration {
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      format: wgpu::TextureFormat::Bgra8UnormSrgb,
      width: 800,
      height: 600,
      present_mode: wgpu::PresentMode::Fifo,
      alpha_mode: wgpu::CompositeAlphaMode::Opaque,
      view_formats: vec![],
      desired_maximum_frame_latency: 1,
    });

    // A burst of size events (a live resize) only records the latest size.
    for width in (804..=1400).step_by(4) {
      engine.resize(width, 600);
    }

    let config = engine.surface_config.as_ref().unwrap();
    assert_eq!((config.width, config.height), (800, 600));
    assert_eq!((engine.width, engine.height), (1400, 600));
    assert!(engine.surface_size_pending());

    engine.resize(800, 600);
    assert!(!engine.surface_size_pending(), "back at the configured size");
  }

  #[test]
  fn resize_before_initialization_sets_initial_surface_size() {
    let mut engine = super::WgpuRenderEngine::new();

    engine.resize(1440, 900);

    assert_eq!(engine.width, 1440);
    assert_eq!(engine.height, 900);
  }

  #[test]
  fn staged_texture_bytes_uses_wgpu_row_alignment() {
    assert_eq!(super::wgpu_staged_texture_bytes(1, 3), 256 * 3);
    assert_eq!(super::wgpu_staged_texture_bytes(256, 2), 256 * 2);
    assert_eq!(super::wgpu_staged_texture_bytes(257, 2), 512 * 2);
  }

  #[test]
  fn requested_present_mode_falls_back_only_when_unsupported() {
    use super::WgpuPresentMode;

    assert_eq!(
      super::supported_present_mode(
        WgpuPresentMode::Mailbox,
        &[WgpuPresentMode::Fifo, WgpuPresentMode::Mailbox],
      ),
      WgpuPresentMode::Mailbox
    );
    assert_eq!(
      super::supported_present_mode(WgpuPresentMode::Mailbox, &[WgpuPresentMode::Fifo]),
      WgpuPresentMode::Fifo
    );
  }
}

#[cfg(feature = "raster")]
fn canvas_image_flag(native: &Option<crate::images::NativeImageData>) -> f32 {
  #[cfg(feature = "canvas")]
  if native
    .as_ref()
    .is_some_and(|n| n.backend() == crate::images::NativeImageBackend::Canvas)
  {
    return 1.;
  }
  // A card's frame of red, green and blue is in a texture whose format does not say that its bytes are encoded
  // as a screen's are, which they are, as a canvas's: taken for bytes of linear light, the frame was drawn
  // lighter than it is (a red of 255, 24, 0 as 255, 86, 0).
  #[cfg(windows)]
  if native
    .as_ref()
    .and_then(|n| n.payload::<crate::images::CardFrames>())
    .is_some_and(crate::images::CardFrames::is_rgba)
  {
    return 1.;
  }
  let _ = native;
  0.
}
