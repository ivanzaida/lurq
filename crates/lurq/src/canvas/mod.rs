//! Persistent 2D drawing through an existing element ref.
//!
//! Enable `canvas`, attach a [`crate::core::ElementRef`] to a
//! [`crate::components::Canvas`], and call `as_canvas()` after layout. Contexts
//! are owned handles: clones share state and drawing does not rebuild the UI.
//! Drawing is queued into persistent GPU textures. Presentation is coalesced
//! through the host event-loop waker. Readbacks are explicit and asynchronous.

mod context;
pub(crate) mod gpu;
pub use gpu::CanvasReadback;
mod path;
mod text;

use std::{
  fmt,
  sync::{
    Arc, Weak,
    atomic::{AtomicU64, Ordering},
  },
};

use parking_lot::Mutex;
pub use path::{ArcDirection, Path2D};
pub(crate) use text::CanvasTextEngine;
pub use text::{CanvasFont, TextAlign, TextBaseline, TextMetrics};
pub use tiny_skia::{LineCap, LineJoin};
use tiny_skia::{Mask, Paint, Path, Pixmap, PixmapPaint, Point, Stroke, StrokeDash};

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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
  fill: Color,
  stroke_color: Color,
  alpha: f32,
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
      fill: Color::new(0, 0, 0, 255),
      stroke_color: Color::new(0, 0, 0, 255),
      alpha: 1.0,
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
      pending_bytes: s.command_bytes + s.inflight_bytes,
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
      && (s.commands.len() >= 8192
        || s.command_bytes + s.inflight_bytes + std::mem::size_of::<gpu::Command>() > gpu::MAX_QUEUE_BYTES)
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

  fn paint_path(&mut self, path: &Path, matrix: Transform2D, stroke: bool, clear: bool, rule: FillRule) -> bool {
    let color = if stroke {
      self.state.stroke_color
    } else {
      self.state.fill
    };
    if !self.software {
      let path = if stroke {
        context::stroke_outline(path, &self.state.stroke)
      } else {
        Some(path.clone())
      };
      let matrix = transform(Transform2D::scale_uniform(self.metrics.scale_factor).then(&matrix));
      let Some(path) = path.and_then(|p| p.transform(matrix)) else {
        return false;
      };
      let alpha = f32::from(color.a()) / 255.0 * self.state.alpha;
      return self.enqueue(gpu::Command::Path {
        path,
        rule,
        color: [
          f32::from(color.r()) / 255.0 * alpha,
          f32::from(color.g()) / 255.0 * alpha,
          f32::from(color.b()) / 255.0 * alpha,
          alpha,
        ],
        erase: clear,
        clip: self.state.gpu_clip.clone(),
        scale: self.metrics.scale_factor,
      });
    }
    let mut paint = Paint::default();
    paint.set_color_rgba8(
      color.r(),
      color.g(),
      color.b(),
      (f32::from(color.a()) * self.state.alpha).round() as u8,
    );
    if clear {
      paint.blend_mode = tiny_skia::BlendMode::Clear;
    }
    let matrix = transform(Transform2D::scale_uniform(self.metrics.scale_factor).then(&matrix));
    let Some(pixels) = &mut self.pixels else {
      return false;
    };
    if stroke {
      pixels.stroke_path(path, &paint, &self.state.stroke, matrix, self.state.clip.as_deref());
    } else {
      pixels.fill_path(path, &paint, rule.skia(), matrix, self.state.clip.as_deref());
    }
    true
  }
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
