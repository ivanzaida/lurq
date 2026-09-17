//! Persistent 2D drawing through an existing element ref.
//!
//! Enable `canvas`, attach a [`crate::core::ElementRef`] to a
//! [`crate::components::Canvas`], and call `as_canvas()` after layout. Contexts
//! are owned handles: clones share state and drawing does not rebuild the UI.
//! Drawing is queued into persistent GPU textures. Presentation is coalesced
//! through the host event-loop waker. Readbacks are explicit and asynchronous.

mod blend;
mod context;
mod effect;
pub(crate) mod gpu;
pub use blend::BlendMode;
pub use effect::{Filter, MAX_BLUR_RADIUS, MAX_EFFECT_PIXELS, MAX_SHADOW_BLUR, MAX_SHADOW_SPREAD, Shadow};
pub use gpu::{CanvasReadback, MAX_LAYER_DEPTH};
mod paint;
mod path;
mod text;
use std::{
  fmt,
  sync::{
    Arc, Weak,
    atomic::{AtomicU64, Ordering},
  },
};

pub use paint::{CanvasPaint, Gradient, GradientKind, MAX_GRADIENT_STOPS, MIN_GRADIENT_STOPS, Paint, RAMP_TEXELS};
use parking_lot::Mutex;
pub use path::{ArcDirection, Path2D};
pub(crate) use text::CanvasTextEngine;
pub use text::{CanvasFont, TextAlign, TextBaseline, TextMetrics};
pub use tiny_skia::{LineCap, LineJoin};
use tiny_skia::{Mask, Paint as SkiaPaint, Path, Pixmap, PixmapPaint, Point, Stroke, StrokeDash};

use crate::{
  app::window::Window,
  images::{ImageData, ImagePixelFormat, NativeImageBackend, NativeImageData, StreamingImage},
  layout::Size,
  node::{color::Color, transform::Transform2D},
};

const MAX_PIXELS: u64 = 16_777_216;
const MAX_SAVE_DEPTH: usize = 128;
const MAX_CLIP_BYTES: usize = 64 * 1024 * 1024;
static NEXT_CANVAS_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FillRule {
  #[default]
  NonZero,
  EvenOdd,
}
impl FillRule {
  fn skia(self) -> tiny_skia::FillRule {
    match self {
      Self::NonZero => tiny_skia::FillRule::Winding,
      Self::EvenOdd => tiny_skia::FillRule::EvenOdd,
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CanvasId(u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CanvasError {
  Detached,
  QueueFull,
  UnsupportedBackend,
  RendererLost,
  InvalidGeometry,
  InvalidImage,
  UnsupportedImage,
  SurfaceTooLarge,
  StateLimit,
  TextUnavailable,
  TextTooLarge,
  UnsupportedPaint,
  UnbalancedLayer,
}
impl fmt::Display for CanvasError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(match self {
      Self::Detached => "canvas is detached",
      Self::QueueFull => "canvas pending-work budget exceeded",
      Self::UnsupportedBackend => "renderer does not support GPU canvas; select software explicitly",
      Self::RendererLost => "canvas renderer or readback was released",
      Self::InvalidGeometry => "invalid canvas geometry",
      Self::InvalidImage => "invalid or empty image source",
      Self::UnsupportedImage => "canvas accepts immutable RGBA images only",
      Self::SurfaceTooLarge => "canvas backing storage exceeds its allocation limit",
      Self::StateLimit => "canvas path, clip, or saved-state limit exceeded",
      Self::TextUnavailable => "canvas text service is not attached yet",
      Self::TextTooLarge => "canvas text exceeds its rasterization limit",
      Self::UnsupportedPaint => "this canvas operation accepts a solid colour only",
      Self::UnbalancedLayer => "canvas layer was ended without being begun",
    })
  }
}
impl std::error::Error for CanvasError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasMetrics {
  pub size: Size,
  pub pixel_width: u32,
  pub pixel_height: u32,
  pub scale_factor: f32,
  /// Changes on first readiness, logical resize, or display-scale change.
  pub revision: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CanvasStatus {
  pub attached: bool,
  pub metrics: CanvasMetrics,
  pub content_revision: u64,
  pub error: Option<CanvasError>,
  /// Conservative memory charge for queued and encoding work, including retained sources.
  pub pending_bytes: usize,
  /// Persistent GPU backing bytes; excludes the renderer's shared tile scratch.
  pub gpu_bytes: usize,
  pub software: bool,
  pub gpu: CanvasGpuStats,
}

/// Cumulative work submitted for this surface. Source uploads exclude explicit
/// readbacks; drawing solid paths never uploads a canvas bitmap.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CanvasGpuStats {
  pub batches: u64,
  pub vertices: u64,
  pub tiles: u64,
  pub uploaded_bytes: u64,
  /// Cumulative CPU mesh-cache work while preparing this canvas, including
  /// attempted batches that later fail to submit.
  pub mesh_cache_hits: u64,
  pub mesh_cache_misses: u64,
  pub mesh_cache_evictions: u64,
  /// Shared renderer cache occupancy at this canvas's last preparation.
  /// Bytes include source geometry, triangle positions and estimated metadata.
  pub mesh_cache_entries: usize,
  pub mesh_cache_bytes: usize,
}

/// Straight-alpha sRGB RGBA8 pixels returned by an explicit readback.
#[derive(Clone, Debug)]
pub struct CanvasSnapshot {
  pub width: u32,
  pub height: u32,
  pub rgba: Vec<u8>,
  pub revision: u64,
}

type MetricsCallback = dyn Fn(CanvasMetrics) + Send + Sync;

/// Keep this value alive to receive metrics changes. Dropping it unsubscribes.
/// The surface holds only a weak callback, so capturing the canvas is safe.
pub struct CanvasObserver {
  _callback: Arc<MetricsCallback>,
}

#[derive(Clone)]
pub struct CanvasHandle {
  inner: Arc<Mutex<Surface>>,
}

#[derive(Clone)]
pub(crate) struct CanvasWeak {
  inner: Weak<Mutex<Surface>>,
}

impl CanvasWeak {
  pub(crate) fn upgrade(&self) -> Option<CanvasHandle> {
    self.inner.upgrade().map(|inner| CanvasHandle { inner })
  }
}

#[derive(Clone)]
pub struct Context2D {
  canvas: CanvasHandle,
}

#[derive(Clone)]
struct DrawingState {
  fill: Paint,
  stroke_paint: Paint,
  alpha: f32,
  blend: BlendMode,
  shadow: Option<Shadow>,
  filter: Filter,
  transform: Transform2D,
  stroke: Stroke,
  dash: Vec<f32>,
  dash_offset: f32,
  clip: Option<Arc<Mask>>,
  gpu_clip: Option<Arc<gpu::Clip>>,
  font: CanvasFont,
  align: TextAlign,
  baseline: TextBaseline,
  smoothing: bool,
}

impl Default for DrawingState {
  fn default() -> Self {
    Self {
      fill: Paint::Solid(Color::new(0, 0, 0, 255)),
      stroke_paint: Paint::Solid(Color::new(0, 0, 0, 255)),
      alpha: 1.0,
      blend: BlendMode::Normal,
      shadow: None,
      filter: Filter::None,
      transform: Transform2D::IDENTITY,
      stroke: Stroke {
        miter_limit: 10.0,
        ..Stroke::default()
      },
      dash: Vec::new(),
      dash_offset: 0.0,
      clip: None,
      gpu_clip: None,
      font: CanvasFont::default(),
      align: TextAlign::Left,
      baseline: TextBaseline::Alphabetic,
      smoothing: true,
    }
  }
}

struct Surface {
  id: CanvasId,
  metrics: CanvasMetrics,
  pixels: Option<Pixmap>,
  software: bool,
  commands: Vec<gpu::Command>,
  command_bytes: usize,
  /// Work held in open isolated layers, charged against the same budget.
  layers: Vec<gpu::LayerFrame>,
  layer_bytes: usize,
  layer_commands: usize,
  /// Software backend only: one pixmap per open layer, innermost last.
  software_layers: Vec<SoftwareLayer>,
  inflight_bytes: usize,
  gpu_bytes: usize,
  gpu: CanvasGpuStats,
  native: Option<NativeImageData>,
  readbacks: Arc<std::sync::atomic::AtomicUsize>,
  state: DrawingState,
  defaults: DrawingState,
  stack: Vec<DrawingState>,
  path: Path2D,
  revision: u64,
  exported_revision: u64,
  pending_paint: bool,
  image: Option<StreamingImage>,
  attached: bool,
  window: Option<Window>,
  to_window: Transform2D,
  observers: Vec<Weak<MetricsCallback>>,
  text: Option<Arc<Mutex<CanvasTextEngine>>>,
  error: Option<CanvasError>,
}

impl fmt::Debug for CanvasHandle {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("CanvasHandle")
      .field("id", &self.surface_id())
      .field("status", &self.status())
      .finish()
  }
}
impl fmt::Debug for Context2D {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_tuple("Context2D").field(&self.canvas).finish()
  }
}

impl CanvasHandle {
  pub(crate) fn new() -> Self {
    Self {
      inner: Arc::new(Mutex::new(Surface {
        id: CanvasId(NEXT_CANVAS_ID.fetch_add(1, Ordering::Relaxed)),
        metrics: CanvasMetrics {
          size: Size::new(0.0, 0.0),
          pixel_width: 0,
          pixel_height: 0,
          scale_factor: 1.0,
          revision: 0,
        },
        pixels: None,
        software: false,
        commands: Vec::new(),
        command_bytes: 0,
        layers: Vec::new(),
        layer_bytes: 0,
        layer_commands: 0,
        software_layers: Vec::new(),
        inflight_bytes: 0,
        gpu_bytes: 0,
        gpu: CanvasGpuStats::default(),
        native: None,
        readbacks: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        state: DrawingState::default(),
        defaults: DrawingState::default(),
        stack: Vec::new(),
        path: Path2D::new(),
        revision: 0,
        exported_revision: 0,
        pending_paint: false,
        image: None,
        attached: false,
        window: None,
        to_window: Transform2D::IDENTITY,
        observers: Vec::new(),
        text: None,
        error: None,
      })),
    }
  }

  pub fn surface_id(&self) -> CanvasId {
    self.inner.lock().id
  }
  pub fn context_2d(&self) -> Context2D {
    Context2D { canvas: self.clone() }
  }
  pub fn size(&self) -> Size {
    self.inner.lock().metrics.size
  }
  pub fn pixel_size(&self) -> (u32, u32) {
    let s = self.inner.lock();
    (s.metrics.pixel_width, s.metrics.pixel_height)
  }
  pub fn scale_factor(&self) -> f32 {
    self.inner.lock().metrics.scale_factor
  }
  pub fn metrics(&self) -> CanvasMetrics {
    self.inner.lock().metrics
  }
  pub fn is_attached(&self) -> bool {
    self.inner.lock().attached
  }
  pub fn status(&self) -> CanvasStatus {
    let s = self.inner.lock();
    CanvasStatus {
      attached: s.attached,
      metrics: s.metrics,
      content_revision: s.revision,
      error: s.error.clone(),
      pending_bytes: s.command_bytes + s.layer_bytes + s.inflight_bytes,
      gpu_bytes: s.gpu_bytes,
      software: s.software,
      gpu: s.gpu,
    }
  }

  /// Receives the current metrics immediately, then committed size/scale changes.
  pub fn observe_metrics(&self, callback: impl Fn(CanvasMetrics) + Send + Sync + 'static) -> CanvasObserver {
    let callback: Arc<MetricsCallback> = Arc::new(callback);
    let metrics = {
      let mut s = self.inner.lock();
      s.observers.retain(|o| o.strong_count() > 0);
      s.observers.push(Arc::downgrade(&callback));
      s.metrics
    };
    callback(metrics);
    CanvasObserver { _callback: callback }
  }

  /// Converts window-logical input to content coordinates, without undoing the drawing transform.
  pub fn point_from_window(&self, x: f32, y: f32) -> Option<(f32, f32)> {
    let s = self.inner.lock();
    if !s.attached || !x.is_finite() || !y.is_finite() {
      return None;
    }
    s.to_window.inverse_affine().map(|m| m.transform_point(x, y))
  }

  /// Queue an ordered GPU readback. Never wait for it on the rendering thread.
  pub fn snapshot(&self) -> CanvasReadback {
    let (ticket, mut done) = CanvasReadback::pair();
    let wake = {
      let mut s = self.inner.lock();
      if s.software {
        done.finish(Ok(CanvasSnapshot {
          width: s.metrics.pixel_width,
          height: s.metrics.pixel_height,
          rgba: s.straight_pixels(),
          revision: s.revision,
        }));
        return ticket;
      }
      if !s.attached {
        done.finish(Err(CanvasError::Detached));
        return ticket;
      }
      if !done.reserve(&s.readbacks) {
        done.finish(Err(CanvasError::QueueFull));
        return ticket;
      }
      let (metrics, revision) = (s.metrics, s.revision);
      if s.enqueue(gpu::Command::Readback(done, metrics, revision)) {
        s.pending_paint = true;
        s.window.clone()
      } else {
        None
      }
    };
    if let Some(window) = wake {
      window.wake();
    }
    ticket
  }

  pub(crate) fn update_placement(&self, matrix: Transform2D) {
    self.inner.lock().to_window = matrix;
  }

  pub(crate) fn clone_empty(&self) -> Self {
    let next = Self::new();
    next.inner.lock().software = self.inner.lock().software;
    next
  }
  pub(crate) fn downgrade(&self) -> CanvasWeak {
    CanvasWeak {
      inner: Arc::downgrade(&self.inner),
    }
  }
  pub(crate) fn dirty(&self) -> bool {
    let s = self.inner.lock();
    s.attached && s.pending_paint
  }
  pub(crate) fn consume_paint(&self) -> bool {
    let mut s = self.inner.lock();
    let pending = s.attached && s.pending_paint;
    s.pending_paint = false;
    pending
  }
  pub(crate) fn detach(&self) {
    let mut s = self.inner.lock();
    s.attached = false;
    s.window = None;
    s.commands.clear();
    s.command_bytes = 0;
    s.discard_layers();
    s.software_layers.clear();
  }

  pub(crate) fn bind_layout(
    &self,
    size: Size,
    scale: f32,
    to_window: Transform2D,
    window: Window,
    font: CanvasFont,
    text: Arc<Mutex<CanvasTextEngine>>,
  ) -> Option<(CanvasMetrics, Vec<Arc<MetricsCallback>>)> {
    let mut s = self.inner.lock();
    s.attached = true;
    s.window = Some(window);
    s.to_window = to_window;
    s.text = Some(text);
    let scale = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
    let size = Size::new(size.width.max(0.0), size.height.max(0.0));
    if s.metrics.revision != 0 && s.metrics.size == size && s.metrics.scale_factor == scale {
      return None;
    }
    let first = s.metrics.revision == 0;
    let resized = first || s.metrics.size != size;
    let old_scale = s.metrics.scale_factor;
    let dims = (
      f64::from(size.width) * f64::from(scale),
      f64::from(size.height) * f64::from(scale),
    );
    let valid = dims.0.is_finite()
      && dims.1.is_finite()
      && dims.0.ceil() <= 16384.0
      && dims.1.ceil() <= 16384.0
      && dims.0.ceil() * dims.1.ceil() <= MAX_PIXELS as f64;
    let (width, height) = if valid {
      (dims.0.ceil() as u32, dims.1.ceil() as u32)
    } else {
      (0, 0)
    };
    let mut next = if s.software && width > 0 && height > 0 {
      Pixmap::new(width, height)
    } else {
      None
    };
    s.error = if !valid || (s.software && width > 0 && height > 0 && next.is_none()) {
      Some(CanvasError::SurfaceTooLarge)
    } else {
      None
    };
    if !resized && s.error.is_some() {
      // A display-scale allocation failure must not discard the old bitmap.
      return None;
    }
    if !s.software
      && !resized
      && (s.commands.len() + s.layer_commands >= gpu::MAX_COMMANDS
        || s.command_bytes + s.layer_bytes + s.inflight_bytes + std::mem::size_of::<gpu::Command>()
          > gpu::MAX_QUEUE_BYTES)
    {
      s.error = Some(CanvasError::QueueFull);
      return None;
    }
    if resized {
      if first {
        s.defaults.font = font;
      }
      s.state = s.defaults.clone();
      s.stack.clear();
      s.path = Path2D::new();
      s.discard_layers();
      s.software_layers.clear();
    } else if s.software {
      if let (Some(old), Some(next)) = (s.pixels.as_ref(), next.as_mut()) {
        next.draw_pixmap(
          0,
          0,
          old.as_ref(),
          &PixmapPaint {
            quality: tiny_skia::FilterQuality::Bilinear,
            ..Default::default()
          },
          tiny_skia::Transform::from_scale(scale / old_scale, scale / old_scale),
          None,
        );
      }
      let factor = scale / old_scale;
      let mut masks = std::collections::HashMap::new();
      for state in s.stack.iter().chain(std::iter::once(&s.state)) {
        if let Some(mask) = &state.clip {
          masks.entry(Arc::as_ptr(mask)).or_insert_with(|| mask.clone());
        }
      }
      // Keep the previous backing scale if saved clips cannot fit at the new scale.
      // Shared clips remain shared, including after a display-scale transition.
      if u64::from(width) * u64::from(height) * masks.len() as u64 > MAX_CLIP_BYTES as u64 {
        s.error = Some(CanvasError::StateLimit);
        return None;
      }
      let masks: std::collections::HashMap<_, _> = masks
        .into_iter()
        .map(|(key, mask)| (key, rescale_clip(&mask, width, height, factor)))
        .collect();
      for state in &mut s.stack {
        if let Some(mask) = &state.clip {
          state.clip = masks[&Arc::as_ptr(mask)].clone();
        }
      }
      if let Some(mask) = &s.state.clip {
        s.state.clip = masks[&Arc::as_ptr(mask)].clone();
      }
    }
    if !s.software {
      if resized {
        s.commands.clear();
        s.command_bytes = 0;
      }
      s.enqueue(gpu::Command::Resize {
        width,
        height,
        preserve: !resized,
      });
      s.native = None;
      s.pending_paint = true;
    }
    s.pixels = next;
    s.image = None;
    s.revision += 1;
    s.metrics = CanvasMetrics {
      size,
      pixel_width: width,
      pixel_height: height,
      scale_factor: scale,
      revision: s.metrics.revision + 1,
    };
    let callbacks = s.observers.iter().filter_map(Weak::upgrade).collect();
    Some((s.metrics, callbacks))
  }

  pub(crate) fn image_data(&self) -> Option<ImageData> {
    let mut s = self.inner.lock();
    s.pending_paint = false;
    if !s.software {
      if s.metrics.pixel_width == 0 || s.metrics.pixel_height == 0 {
        return None;
      }
      if s.native.is_none() {
        s.native = Some(NativeImageData::new(
          s.metrics.pixel_width,
          s.metrics.pixel_height,
          ImagePixelFormat::Rgba8,
          NativeImageBackend::Canvas,
          self.downgrade(),
        ));
      }
      return s.native.as_ref().map(NativeImageData::image_data);
    }
    s.pixels.as_ref()?;
    if s.image.is_none() || s.revision != s.exported_revision {
      let data = s.straight_pixels();
      if let Some(image) = &s.image {
        image.set_rgba(data);
      } else {
        s.image = Some(StreamingImage::new_rgba_manual_redraw(
          data,
          s.metrics.pixel_width,
          s.metrics.pixel_height,
        ));
      }
      s.exported_revision = s.revision;
    }
    s.image.as_ref().map(StreamingImage::image_data)
  }
}

/// One open isolated layer on the software backend. The GPU backends keep their
/// layers as recorded commands instead; see `gpu::LayerFrame`.
struct SoftwareLayer {
  /// `None` only for a zero-sized surface, where the layer still has to exist so
  /// that `end_layer` stays balanced with `begin_layer`.
  pixels: Option<Pixmap>,
  alpha: f32,
  blend: BlendMode,
}

/// A paint with its geometry resolved against the box it was given and the
/// transform in force: what both backends draw from.
enum ResolvedPaint {
  Solid(Color),
  Gradient {
    gradient: Gradient,
    /// Model coordinates to the gradient's own frame, for GPU vertices.
    frame_from_model: Transform2D,
    /// The gradient's own frame to user coordinates, for a software shader.
    user_from_frame: Transform2D,
  },
}

impl ResolvedPaint {
  fn identity(&self) -> u64 {
    match self {
      Self::Solid(color) => {
        (u64::from(color.r()) << 24) | (u64::from(color.g()) << 16) | (u64::from(color.b()) << 8) | u64::from(color.a())
      }
      Self::Gradient {
        gradient,
        frame_from_model,
        ..
      } => {
        gradient.ramp().id
          ^ (u64::from(frame_from_model.a.to_bits()) << 8)
          ^ (u64::from(frame_from_model.d.to_bits()) << 16)
          ^ (u64::from(frame_from_model.tx.to_bits()) << 24)
          ^ (u64::from(frame_from_model.ty.to_bits()) << 32)
      }
    }
  }
}

impl Surface {
  fn straight_pixels(&self) -> Vec<u8> {
    let Some(pixels) = &self.pixels else {
      return Vec::new();
    };
    let mut rgba = Vec::with_capacity(pixels.data().len());
    for pixel in pixels.pixels() {
      let c = pixel.demultiply();
      rgba.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    rgba
  }

  /// Where a software draw lands: the innermost open layer, or the surface.
  pub(super) fn target_pixels(&mut self) -> Option<&mut Pixmap> {
    match self.software_layers.last_mut() {
      Some(layer) => layer.pixels.as_mut(),
      None => self.pixels.as_mut(),
    }
  }

  fn surface_size(&self) -> (u32, u32) {
    (self.metrics.pixel_width, self.metrics.pixel_height)
  }

  /// One user unit as device pixels, including rotation and skew. Shadow offsets,
  /// blur radii and spreads are written in user units and scaled by this.
  fn user_to_device(&self) -> Transform2D {
    Transform2D::scale_uniform(self.metrics.scale_factor).then(&self.state.transform.linear_part())
  }

  pub(super) fn begin_layer(&mut self, alpha: f32, blend: BlendMode) -> Result<(), CanvasError> {
    if !self.software {
      return self.open_layer(alpha, blend);
    }
    if !self.attached {
      self.error = Some(CanvasError::Detached);
      return Err(CanvasError::Detached);
    }
    if self.software_layers.len() >= MAX_LAYER_DEPTH {
      self.error = Some(CanvasError::StateLimit);
      return Err(CanvasError::StateLimit);
    }
    let (width, height) = self.surface_size();
    self.software_layers.push(SoftwareLayer {
      pixels: Pixmap::new(width, height),
      alpha,
      blend,
    });
    Ok(())
  }

  /// Composites the innermost layer onto its parent. `false` when there was
  /// nothing to composite, which is what tells the caller no pixels changed.
  pub(super) fn end_layer(&mut self) -> bool {
    if !self.software {
      return self.close_layer();
    }
    let Some(layer) = self.software_layers.pop() else {
      return false;
    };
    let (alpha, blend) = (layer.alpha, layer.blend);
    let Some(source) = layer.pixels else {
      return false;
    };
    let Some(target) = self.target_pixels() else {
      return false;
    };
    blend::composite_premultiplied(target.data_mut(), source.data(), alpha, blend);
    true
  }

  /// One already rasterised premultiplied source — shaped text — with the blend
  /// mode, shadow and filter in force. `matrix` maps its own pixels into user
  /// space. Spread has no meaning for a raster and is not applied.
  fn paint_pixmap(&mut self, pixels: &Pixmap, data: &Arc<Vec<u8>>, id: u64, matrix: Transform2D) -> bool {
    let blend = self.state.blend;
    let isolated = !blend.is_normal() && self.begin_layer(1.0, blend).is_ok();
    let device = Transform2D::scale_uniform(self.metrics.scale_factor)
      .then(&self.state.transform)
      .then(&matrix);
    let user = self.user_to_device();
    let shadow = self.state.shadow.filter(Shadow::is_valid);
    let mut drew = false;
    if let Some(shadow) = shadow.filter(|s| !s.inset) {
      drew |= self.pixmap_shadow(pixels, id, device, user, &shadow);
    }
    let radius = self.state.filter.radius();
    drew |= if self.state.filter.is_valid() && radius > 0.0 {
      match effect::pixmap_blur(pixels, id, device, user, self.surface_size(), radius) {
        Ok(Some(image)) => self.draw_effect(&image),
        Ok(None) => false,
        Err(error) => {
          self.error = Some(error);
          false
        }
      }
    } else {
      self.draw_source(pixels, data, id, matrix)
    };
    if let Some(shadow) = shadow.filter(|s| s.inset) {
      drew |= self.pixmap_shadow(pixels, id, device, user, &shadow);
    }
    if isolated {
      drew |= self.end_layer();
    }
    drew
  }

  fn pixmap_shadow(
    &mut self,
    pixels: &Pixmap,
    id: u64,
    device: Transform2D,
    user: Transform2D,
    shadow: &Shadow,
  ) -> bool {
    match effect::pixmap_shadow(pixels, id, device, user, self.surface_size(), shadow) {
      Ok(Some(image)) => self.draw_effect(&image),
      Ok(None) => false,
      Err(error) => {
        self.error = Some(error);
        false
      }
    }
  }

  fn draw_source(&mut self, pixels: &Pixmap, data: &Arc<Vec<u8>>, id: u64, matrix: Transform2D) -> bool {
    if !self.software {
      let device = Transform2D::scale_uniform(self.metrics.scale_factor)
        .then(&self.state.transform)
        .then(&matrix);
      return self.enqueue(gpu::Command::Image {
        asset: gpu::Asset {
          id,
          width: pixels.width(),
          height: pixels.height(),
          data: data.clone(),
          premultiplied: true,
        },
        matrix: device,
        source: [0., 0., pixels.width() as f32, pixels.height() as f32],
        alpha: self.state.alpha,
        smooth: self.state.smoothing,
        clip: self.state.gpu_clip.clone(),
        scale: self.metrics.scale_factor,
      });
    }
    context::blit(self, pixels, matrix, None)
  }

  /// A fill or a stroke with the paint, blend mode, shadow and filter in force.
  /// The order is the one a design tool draws in: the drop shadow, the shape,
  /// then the inner shadow, all inside one isolation when a blend mode is set.
  fn paint_path(
    &mut self,
    path: &path::Geometry,
    matrix: Transform2D,
    stroke: bool,
    clear: bool,
    rule: FillRule,
  ) -> bool {
    if clear {
      return self.paint_direct(path, matrix, stroke, true, rule);
    }
    let blend = self.state.blend;
    let isolated = !blend.is_normal() && self.begin_layer(1.0, blend).is_ok();
    let shadow = self.state.shadow.filter(Shadow::is_valid);
    let mut drew = false;
    if let Some(shadow) = shadow.filter(|s| !s.inset) {
      drew |= self.paint_shadow(path, matrix, stroke, rule, &shadow);
    }
    drew |= if self.state.filter.is_valid() && self.state.filter.radius() > 0.0 {
      self.paint_filtered(path, matrix, stroke, rule)
    } else {
      self.paint_direct(path, matrix, stroke, false, rule)
    };
    if let Some(shadow) = shadow.filter(|s| s.inset) {
      drew |= self.paint_shadow(path, matrix, stroke, rule, &shadow);
    }
    if isolated {
      drew |= self.end_layer();
    }
    drew
  }

  /// The geometry an effect is cast from: a stroke's own outline, or the fill.
  fn effect_geometry(&self, path: &path::Geometry, stroke: bool, rule: FillRule) -> Option<(path::Geometry, FillRule)> {
    if !stroke {
      return Some((path.clone(), rule));
    }
    context::stroke_outline(path, &self.state.stroke).map(|outline| (path::Geometry::new(outline), FillRule::NonZero))
  }

  fn paint_shadow(
    &mut self,
    path: &path::Geometry,
    matrix: Transform2D,
    stroke: bool,
    rule: FillRule,
    shadow: &Shadow,
  ) -> bool {
    let Some((geometry, rule)) = self.effect_geometry(path, stroke, rule) else {
      return false;
    };
    let device = Transform2D::scale_uniform(self.metrics.scale_factor).then(&matrix);
    let user = self.user_to_device();
    match effect::shadow_image(&geometry, rule, device, user, self.surface_size(), shadow) {
      Ok(Some(image)) => self.draw_effect(&image),
      Ok(None) => false,
      Err(error) => {
        self.error = Some(error);
        false
      }
    }
  }

  /// A layer blur: the shape is rasterised with its own paint into a reduced
  /// raster, blurred there, and drawn as one image.
  fn paint_filtered(&mut self, path: &path::Geometry, matrix: Transform2D, stroke: bool, rule: FillRule) -> bool {
    let Some((geometry, rule)) = self.effect_geometry(path, stroke, rule) else {
      return false;
    };
    let radius = self.state.filter.radius();
    let device = Transform2D::scale_uniform(self.metrics.scale_factor).then(&matrix);
    let user = self.user_to_device();
    let Some(paint) = self.resolved_paint(stroke, matrix) else {
      return false;
    };
    let planned = match effect::blur_plan(&geometry, device, user, self.surface_size(), radius) {
      Ok(Some(planned)) => planned,
      Ok(None) => return false,
      Err(error) => {
        self.error = Some(error);
        return false;
      }
    };
    let key = effect::blur_identity(&[
      geometry.content_hash(),
      u64::from(rule == FillRule::EvenOdd),
      u64::from(device.a.to_bits()) ^ (u64::from(device.b.to_bits()) << 32),
      u64::from(device.c.to_bits()) ^ (u64::from(device.d.to_bits()) << 32),
      ((planned.image.origin.0 as i64) as u64) ^ (((planned.image.origin.1 as i64) as u64) << 32),
      (u64::from(planned.image.width) << 32) | u64::from(planned.image.height),
      u64::from(planned.radius) ^ (u64::from(planned.image.reduction) << 32),
      paint.identity(),
    ]);
    let image = match effect::lookup(key) {
      Some(image) => image,
      None => {
        let effect::BlurPlan {
          mut pixmap,
          image,
          radius,
          raster,
        } = planned;
        let raster_from_device = Transform2D::scale_uniform(1.0 / image.reduction as f32)
          .then(&Transform2D::translate(-image.origin.0 as f32, -image.origin.1 as f32));
        let frame_to_raster = raster_from_device.then(&self.device_from_frame(&paint));
        let conic = self.angular_pattern(&paint, frame_to_raster, (0, 0), image.width, image.height);
        let skia = paint_of(&paint, frame_to_raster, conic.as_ref().map(|p| (p, (0, 0))), 1.0);
        pixmap.fill_path(&geometry, &skia, rule.skia(), transform(raster), None);
        drop(skia);
        drop(conic);
        let image = effect::blur_finish(pixmap, image, radius, key);
        effect::store(key, &image);
        image
      }
    };
    self.draw_effect(&image)
  }

  /// Places a rasterised effect in device pixels, under the current clip and
  /// global alpha but under no drawing transform: it is already rasterised.
  fn draw_effect(&mut self, image: &effect::EffectImage) -> bool {
    if image.width == 0 || image.height == 0 {
      return false;
    }
    let alpha = self.state.alpha;
    let smooth = image.reduction > 1;
    if !self.software {
      return self.enqueue(gpu::Command::Image {
        asset: gpu::Asset {
          id: image.id,
          width: image.width,
          height: image.height,
          data: image.data.clone(),
          premultiplied: true,
        },
        matrix: image.matrix(),
        source: [0., 0., image.width as f32, image.height as f32],
        alpha,
        smooth,
        clip: self.state.gpu_clip.clone(),
        scale: self.metrics.scale_factor,
      });
    }
    let Some(size) = tiny_skia::IntSize::from_wh(image.width, image.height) else {
      return false;
    };
    let Some(source) = Pixmap::from_vec(image.data.as_ref().clone(), size) else {
      return false;
    };
    let matrix = transform(image.matrix());
    let clip = self.state.clip.clone();
    let Some(target) = self.target_pixels() else {
      return false;
    };
    target.draw_pixmap(
      0,
      0,
      source.as_ref(),
      &PixmapPaint {
        opacity: alpha,
        quality: if smooth {
          tiny_skia::FilterQuality::Bilinear
        } else {
          tiny_skia::FilterQuality::Nearest
        },
        ..Default::default()
      },
      matrix,
      clip.as_deref(),
    );
    true
  }

  /// The paint a fill or stroke uses. `None` when a gradient cannot be placed,
  /// in which case nothing is drawn rather than something else in its place.
  fn resolved_paint(&self, stroke: bool, matrix: Transform2D) -> Option<ResolvedPaint> {
    let paint = if stroke {
      &self.state.stroke_paint
    } else {
      &self.state.fill
    };
    let Some((gradient, bounds)) = paint.gradient() else {
      return Some(ResolvedPaint::Solid(paint.color().unwrap_or(Color::new(0, 0, 0, 255))));
    };
    if !gradient.is_valid() {
      return None;
    }
    let frame_from_user = gradient.frame_from_box(bounds)?;
    let user_from_model = self.state.transform.inverse_affine()?.then(&matrix);
    Some(ResolvedPaint::Gradient {
      gradient: gradient.clone(),
      frame_from_model: frame_from_user.then(&user_from_model),
      user_from_frame: frame_from_user.inverse_affine()?,
    })
  }

  /// Maps the gradient's own frame onto device pixels.
  fn device_from_frame(&self, paint: &ResolvedPaint) -> Transform2D {
    match paint {
      ResolvedPaint::Solid(_) => Transform2D::IDENTITY,
      ResolvedPaint::Gradient { user_from_frame, .. } => Transform2D::scale_uniform(self.metrics.scale_factor)
        .then(&self.state.transform)
        .then(user_from_frame),
    }
  }

  /// An angular gradient has no tiny-skia shader, so the software backend
  /// evaluates one into a pattern bounded by the raster it paints into.
  fn angular_pattern(
    &self,
    paint: &ResolvedPaint,
    frame_to_target: Transform2D,
    origin: (i32, i32),
    width: u32,
    height: u32,
  ) -> Option<Pixmap> {
    let ResolvedPaint::Gradient { gradient, .. } = paint else {
      return None;
    };
    if gradient.kind() != GradientKind::Angular || width == 0 || height == 0 {
      return None;
    }
    paint::angular_pixmap(gradient, frame_to_target.inverse_affine()?, origin, width, height)
  }

  fn paint_direct(
    &mut self,
    path: &path::Geometry,
    matrix: Transform2D,
    stroke: bool,
    clear: bool,
    rule: FillRule,
  ) -> bool {
    let Some(paint) = self.resolved_paint(stroke, matrix) else {
      return false;
    };
    let alpha = self.state.alpha;
    let device = Transform2D::scale_uniform(self.metrics.scale_factor).then(&matrix);
    if !self.software {
      let path = if stroke {
        context::stroke_outline(path, &self.state.stroke).map(path::Geometry::new)
      } else {
        Some(path.clone())
      };
      let Some(path) = path else {
        return false;
      };
      let (color, gradient) = match &paint {
        ResolvedPaint::Solid(color) => {
          let a = f32::from(color.a()) / 255.0 * alpha;
          (
            [
              f32::from(color.r()) / 255.0 * a,
              f32::from(color.g()) / 255.0 * a,
              f32::from(color.b()) / 255.0 * a,
              a,
            ],
            None,
          )
        }
        ResolvedPaint::Gradient {
          gradient,
          frame_from_model,
          ..
        } => {
          let ramp = gradient.ramp();
          (
            [alpha; 4],
            Some(gpu::GradientPaint {
              kind: gradient.kind(),
              ramp: gpu::Asset {
                id: ramp.id,
                width: RAMP_TEXELS as u32,
                height: 1,
                data: ramp.texels.clone(),
                premultiplied: true,
              },
              frame: *frame_from_model,
            }),
          )
        }
      };
      return self.enqueue(gpu::Command::Path {
        path,
        matrix: device,
        rule,
        color,
        gradient,
        erase: clear,
        clip: self.state.gpu_clip.clone(),
        scale: self.metrics.scale_factor,
      });
    }
    let frame_to_device = self.device_from_frame(&paint);
    let (origin, width, height) = device_window(path, device, self.surface_size());
    let conic = self.angular_pattern(&paint, frame_to_device, origin, width, height);
    let mut skia = paint_of(
      &paint,
      frame_to_device,
      conic.as_ref().map(|pixmap| (pixmap, origin)),
      alpha,
    );
    if clear {
      skia.blend_mode = tiny_skia::BlendMode::Clear;
    }
    let stroke_params = self.state.stroke.clone();
    let clip = self.state.clip.clone();
    let matrix = transform(device);
    let Some(pixels) = self.target_pixels() else {
      return false;
    };
    if stroke {
      pixels.stroke_path(path, &skia, &stroke_params, matrix, clip.as_deref());
    } else {
      pixels.fill_path(path, &skia, rule.skia(), matrix, clip.as_deref());
    }
    true
  }
}

/// The tiny-skia paint for a resolved paint, with the global alpha applied once.
fn paint_of<'a>(
  paint: &ResolvedPaint,
  frame_to_target: Transform2D,
  conic: Option<(&'a Pixmap, (i32, i32))>,
  alpha: f32,
) -> SkiaPaint<'a> {
  let mut skia = SkiaPaint {
    anti_alias: true,
    ..SkiaPaint::default()
  };
  match paint {
    ResolvedPaint::Solid(color) => skia.set_color_rgba8(
      color.r(),
      color.g(),
      color.b(),
      (f32::from(color.a()) * alpha).round() as u8,
    ),
    ResolvedPaint::Gradient { gradient, .. } => {
      if let Some(mut shader) = paint::skia_shader(gradient, frame_to_target, conic) {
        shader.apply_opacity(alpha);
        skia.shader = shader;
      }
    }
  }
  skia
}

/// The device-pixel window a path can touch, clipped to the surface.
fn device_window(path: &path::Geometry, matrix: Transform2D, surface: (u32, u32)) -> ((i32, i32), u32, u32) {
  let b = path.bounds();
  let mut box_ = [f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY];
  for (x, y) in [
    (b.left(), b.top()),
    (b.right(), b.top()),
    (b.right(), b.bottom()),
    (b.left(), b.bottom()),
  ] {
    let (x, y) = matrix.transform_point(x, y);
    if !x.is_finite() || !y.is_finite() {
      return ((0, 0), 0, 0);
    }
    box_[0] = box_[0].min(x);
    box_[1] = box_[1].min(y);
    box_[2] = box_[2].max(x);
    box_[3] = box_[3].max(y);
  }
  let left = box_[0].floor().max(0.0).min(surface.0 as f32);
  let top = box_[1].floor().max(0.0).min(surface.1 as f32);
  let right = (box_[2].ceil() + 1.0).max(0.0).min(surface.0 as f32);
  let bottom = (box_[3].ceil() + 1.0).max(0.0).min(surface.1 as f32);
  (
    (left as i32, top as i32),
    (right - left).max(0.0) as u32,
    (bottom - top).max(0.0) as u32,
  )
}

fn rescale_clip(mask: &Mask, width: u32, height: u32, factor: f32) -> Option<Arc<Mask>> {
  let mut pixels = Pixmap::new(mask.width(), mask.height())?;
  for (pixel, alpha) in pixels.data_mut().chunks_exact_mut(4).zip(mask.data()) {
    pixel.fill(*alpha);
  }
  let mut next = Pixmap::new(width, height)?;
  next.draw_pixmap(
    0,
    0,
    pixels.as_ref(),
    &PixmapPaint {
      quality: tiny_skia::FilterQuality::Bilinear,
      ..Default::default()
    },
    tiny_skia::Transform::from_scale(factor, factor),
    None,
  );
  Some(Arc::new(Mask::from_pixmap(next.as_ref(), tiny_skia::MaskType::Alpha)))
}

pub(crate) fn transform(m: Transform2D) -> tiny_skia::Transform {
  tiny_skia::Transform::from_row(m.a, m.b, m.c, m.d, m.tx, m.ty)
}

/// Accepted solid paints. String parsing is checked and never panics.
pub trait CanvasColor {
  fn canvas_color(&self) -> Option<Color>;
}
impl CanvasColor for Color {
  fn canvas_color(&self) -> Option<Color> {
    Some(*self)
  }
}
impl CanvasColor for &str {
  fn canvas_color(&self) -> Option<Color> {
    parse_color(self)
  }
}
impl CanvasColor for String {
  fn canvas_color(&self) -> Option<Color> {
    parse_color(self)
  }
}

fn parse_color(value: &str) -> Option<Color> {
  let value = value.trim();
  let hex = value.strip_prefix('#')?;
  if !hex.is_ascii() {
    return None;
  }
  let byte = |a: usize, b: usize| u8::from_str_radix(hex.get(a..b)?, 16).ok();
  match hex.len() {
    3 | 4 => Some(Color::new(
      byte(0, 1)? * 17,
      byte(1, 2)? * 17,
      byte(2, 3)? * 17,
      if hex.len() == 4 { byte(3, 4)? * 17 } else { 255 },
    )),
    6 | 8 => Some(Color::new(
      byte(0, 2)?,
      byte(2, 4)?,
      byte(4, 6)?,
      if hex.len() == 8 { byte(6, 8)? } else { 255 },
    )),
    _ => None,
  }
}

/// One scene exercising every paint and effect this module adds, shared by the
/// software suite and both native backends so a difference is a difference in
/// the backend rather than in the fixture.
#[cfg(test)]
pub(crate) fn effects_scene(d: &Context2D) {
  d.reset();
  d.set_fill_style("#101820");
  d.fill_rect(0., 0., 512., 512.);
  // Gradient fills: linear, radial and angular, each over its own box.
  d.set_fill_style(
    Gradient::linear()
      .stop(0., "#2563eb")
      .stop(1., "#f43f5e")
      .rotation(0.6)
      .in_box(20., 20., 130., 100.),
  );
  d.fill_rect(20., 20., 130., 100.);
  d.set_fill_style(
    Gradient::radial()
      .stop(0., "#fde68a")
      .stop(1., "#7c3aed")
      .in_box(170., 20., 130., 100.),
  );
  d.fill_rect(170., 20., 130., 100.);
  d.set_fill_style(
    Gradient::angular()
      .stop(0., "#22d3ee")
      .stop(0.5, "#0f172a")
      .stop(1., "#22d3ee")
      .in_box(320., 20., 130., 100.),
  );
  d.fill_rect(320., 20., 130., 100.);
  // A gradient stroke, and a shadow cast by a rounded rectangle.
  d.set_line_width(6.);
  d.set_stroke_style(
    Gradient::linear()
      .stop(0., "#34d399")
      .stop(1., "#f59e0b")
      .in_box(20., 150., 200., 80.),
  );
  d.stroke_rect(20., 150., 200., 80.);
  d.set_shadow(Some(
    Shadow::new(Color::new(0, 0, 0, 200))
      .offset(8., 10.)
      .blur(14.)
      .spread(2.),
  ));
  d.set_fill_style("#e2e8f0");
  d.round_rect(260., 150., 180., 80., 16.).unwrap();
  d.fill();
  d.set_shadow(None);
  // An inner shadow, and a layer blur.
  d.set_shadow(Some(
    Shadow::new(Color::new(0, 0, 0, 220))
      .offset(6., 6.)
      .blur(10.)
      .inset(true),
  ));
  d.set_fill_style("#94a3b8");
  d.fill_rect(20., 260., 140., 110.);
  d.set_shadow(None);
  d.set_filter(Filter::Blur(10.));
  d.set_fill_style("#f97316");
  d.fill_rect(190., 270., 90., 90.);
  d.set_filter(Filter::None);
  // Every blend mode over the same backdrop, as a row of swatches.
  d.set_fill_style("#334155");
  d.fill_rect(20., 400., 468., 40.);
  for (index, mode) in BlendMode::ALL.iter().enumerate() {
    d.set_global_composite_operation(*mode);
    d.set_fill_style("#9ae6b4");
    d.fill_rect(20. + index as f32 * 26., 400., 26., 40.);
  }
  d.set_global_composite_operation(BlendMode::Normal);
  // An isolated group: two overlapping shapes fade together, and a nested
  // layer composites with a blend mode of its own.
  d.begin_layer(0.5, BlendMode::Normal).unwrap();
  d.set_fill_style("#ef4444");
  d.fill_rect(280., 280., 80., 80.);
  d.begin_layer(1., BlendMode::Multiply).unwrap();
  d.set_fill_style("#60a5fa");
  d.fill_rect(320., 320., 80., 80.);
  d.end_layer().unwrap();
  d.end_layer().unwrap();
}

#[cfg(test)]
impl CanvasHandle {
  pub(crate) fn test_surface(width: u32, height: u32, scale: f32, software: bool) -> Self {
    let canvas = Self::new();
    {
      let mut s = canvas.inner.lock();
      s.attached = true;
      s.software = software;
      s.metrics = CanvasMetrics {
        size: Size::new(width as f32 / scale, height as f32 / scale),
        pixel_width: width,
        pixel_height: height,
        scale_factor: scale,
        revision: 1,
      };
      s.text = Some(Arc::new(Mutex::new(CanvasTextEngine::new(
        cosmic_text::FontSystem::new(),
        Default::default(),
      ))));
      if software {
        s.pixels = Pixmap::new(width, height);
      } else {
        s.enqueue(gpu::Command::Resize {
          width,
          height,
          preserve: false,
        });
      }
    }
    canvas
  }
  pub(crate) fn test_resize(&self, width: u32, height: u32, preserve: bool) {
    let mut s = self.inner.lock();
    s.metrics.pixel_width = width;
    s.metrics.pixel_height = height;
    s.enqueue(gpu::Command::Resize {
      width,
      height,
      preserve,
    });
  }
}
