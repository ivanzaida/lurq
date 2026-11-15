//! Persistent 2D drawing through an existing element ref.
//!
//! Enable `canvas`, attach a [`crate::core::ElementRef`] to a
//! [`crate::components::Canvas`], and call `as_canvas()` after layout. Contexts
//! are owned handles: clones share state and drawing does not rebuild the UI.
//! Drawing is queued into persistent GPU textures. Presentation is coalesced
//! through the host event-loop waker. Readbacks are explicit and asynchronous.

mod handle;
use handle::{ResolvedPaint, SoftwareLayer};
mod presentation;
mod presentation_budget;
pub use presentation::MAX_PRESENTATION_BYTES;
pub(crate) use presentation::PresentationEvent;
pub(crate) use presentation_budget::{TargetCharge, replacement_admitted};
mod software;
pub use software::CanvasColor;
use software::rescale_clip;
pub(crate) use software::transform;
mod asset_cache;
mod blend;
mod context;
mod effect;
#[cfg(any(test, feature = "wgpu", all(feature = "dx12", target_os = "windows")))]
mod frame_boundary;
mod frame_cache;
pub(crate) mod gpu;
mod items;
#[cfg(any(feature = "wgpu", all(feature = "dx12", target_os = "windows")))]
pub(crate) use asset_cache::AssetCache;
pub use asset_cache::CanvasAssetBudget;
pub use blend::BlendMode;
pub use effect::{Filter, MAX_BLUR_RADIUS, MAX_EFFECT_PIXELS, MAX_SHADOW_BLUR, MAX_SHADOW_SPREAD, Shadow};
#[cfg(any(feature = "wgpu", all(feature = "dx12", target_os = "windows")))]
pub(crate) use frame_boundary::FrameBoundary;
pub use gpu::{CanvasReadback, MAX_LAYER_DEPTH};
pub use items::{CanvasItem, CanvasItemShape};

mod paint;
mod path;
#[cfg(all(test, any(feature = "wgpu", all(feature = "dx12", target_os = "windows"))))]
pub(crate) mod test_pages;
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
  PresentationBusy,
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
      Self::PresentationBusy => "canvas presentation targets are still in flight",
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

/// What a painter draws from, as the engine keeps it between drawings: the value a moving drawing goes to, and
/// the one it comes from.
pub(crate) type MotionValue = std::sync::Arc<dyn std::any::Any + Send + Sync>;

/// Where a drawing is as a painter draws it: what it moves from (none when it enters or is at rest) and to, and how
/// far along the way, eased, 1 at rest.
pub(crate) struct PaintFrame<'a> {
  pub(crate) from: Option<&'a MotionValue>,
  pub(crate) to: Option<&'a MotionValue>,
  pub(crate) progress: f32,
}

pub(crate) type PaintFn = std::sync::Arc<dyn Fn(&CanvasHandle, &PaintFrame<'_>) + Send + Sync>;

/// What draws a canvas from what it is given: called with the canvas when it is first laid out, when its size or
/// its scale changes, and when its key changes, and not otherwise; and, for a moving one, at each frame while it
/// moves. See `Canvas::paint` and `Canvas::paint_moving`.
#[derive(Clone)]
pub struct CanvasPainter {
  pub(crate) key: u64,
  pub(crate) paint: PaintFn,
  pub(crate) motion: Option<PainterMotion>,
}

/// What a moving painter draws toward: `value`, which `of` (its hash) tells apart from what it drew before.
#[derive(Clone)]
pub(crate) struct PainterMotion {
  pub(crate) of: u64,
  pub(crate) value: MotionValue,
  pub(crate) motion: CanvasMotion,
}

impl CanvasPainter {
  pub fn new(key: u64, paint: impl Fn(&CanvasHandle) + Send + Sync + 'static) -> Self {
    Self {
      key,
      paint: std::sync::Arc::new(move |canvas, _| paint(canvas)),
      motion: None,
    }
  }

  /// A painter whose drawing moves to `value` when `of` changes, and enters when it is first drawn: see
  /// `Canvas::paint_moving`.
  pub fn moving<T: Send + Sync + 'static>(
    key: u64,
    motion: CanvasMotion,
    of: u64,
    value: std::sync::Arc<T>,
    paint: impl Fn(&CanvasHandle, CanvasMove<'_, T>) + Send + Sync + 'static,
  ) -> Self {
    let fallback = value.clone();
    let paint = move |canvas: &CanvasHandle, frame: &PaintFrame<'_>| {
      let to = frame
        .to
        .and_then(|value| value.downcast_ref::<T>())
        .unwrap_or(&*fallback);
      let from = frame.from.and_then(|value| value.downcast_ref::<T>());
      paint(
        canvas,
        CanvasMove {
          from,
          to,
          progress: frame.progress,
        },
      )
    };
    Self {
      key,
      paint: std::sync::Arc::new(paint),
      motion: Some(PainterMotion { of, value, motion }),
    }
  }
}

impl std::fmt::Debug for CanvasPainter {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("CanvasPainter")
      .field("key", &self.key)
      .field("moving", &self.motion.is_some())
      .finish()
  }
}

/// How a drawing that follows from a key moves to what it is given next: how long it takes, and how it is eased.
/// See `Canvas::paint_moving`. A short ease out by default, of 300 ms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasMotion {
  pub duration: std::time::Duration,
  pub easing: crate::animation::Easing,
}

impl CanvasMotion {
  pub const fn new(duration: std::time::Duration) -> Self {
    Self {
      duration,
      easing: crate::animation::Easing::EASE_OUT,
    }
  }

  pub const fn easing(mut self, easing: crate::animation::Easing) -> Self {
    self.easing = easing;
    self
  }
}

impl Default for CanvasMotion {
  fn default() -> Self {
    Self::new(std::time::Duration::from_millis(300))
  }
}

/// Where a moving drawing is, as its painter is given it: `to` is what it is drawn from now, `from` what it was
/// drawn from before (none as it enters, and at rest), and `progress` how far along the way it is, eased, from 0
/// to 1. At rest `progress` is 1 and `from` none: the painter draws `to` as it is.
#[derive(Clone, Copy, Debug)]
pub struct CanvasMove<'a, T> {
  pub from: Option<&'a T>,
  pub to: &'a T,
  pub progress: f32,
}

impl<T> CanvasMove<'_, T> {
  /// Whether the drawing is at rest.
  pub fn at_rest(&self) -> bool {
    self.progress >= 1.0
  }

  /// Whether the drawing is entering: first shown, and still on its way in.
  pub fn entering(&self) -> bool {
    self.from.is_none() && !self.at_rest()
  }
}

/// A drawing on its way to what it was given last: what it comes from, when it started, and how it moves.
pub(crate) struct Run {
  pub(crate) from: Option<MotionValue>,
  pub(crate) started: crate::time::Instant,
  pub(crate) motion: CanvasMotion,
  pub(crate) paint: PaintFn,
}

impl Run {
  /// How far along the way it is at `now`, eased: 1 at its end.
  pub(crate) fn progress(&self, now: crate::time::Instant) -> f32 {
    let duration = self.motion.duration.as_secs_f64();
    if duration <= 0.0 {
      return 1.0;
    }
    let t = (now.saturating_duration_since(self.started).as_secs_f64() / duration).clamp(0.0, 1.0);
    if t >= 1.0 {
      return 1.0;
    }
    (self.motion.easing.evaluate(t) as f32).clamp(0.0, 1.0)
  }
}

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
  /// Backend submission evidence, not physical scanout acknowledgement.
  pub presentation_workspace_bytes: usize,
  pub presentation_commits: u64,
  pub presentation_token: u64,
  pub fallback_quads: u64,
  pub presentation_allocations: u64,
  pub presentation_reuses: u64,
  pub presentation_refusals: u64,

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
  presentation_pixels: Option<Pixmap>,
  artwork_pixels: Option<Pixmap>,
  next_artwork_pixels: Option<Pixmap>,
  presentation_revision: u64,
  visible_revision: u64,
  presentation_serial: u64,
  presentation_refused: bool,
  presentation: Option<u64>,
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
  /// Semantic items registered by the app, replaced as a whole.
  items: Arc<[CanvasItem]>,
  /// Fingerprint of the duplicated ids last warned about, so a redraw that
  /// repeats the same duplicates every frame warns once.
  warned_duplicates: Option<u64>,
  /// The key a painter last drew this surface for.
  painted: Option<u64>,
  /// What a moving painter last drew toward, and its hash: what the next motion comes from.
  shown: Option<(u64, MotionValue)>,
  /// The motion on its way, while the drawing moves.
  run: Option<Run>,
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
/// software suite, the wgpu suite and the `canvas_capture_check` example that
/// covers dx12, so a difference between them is a difference in the backend
/// rather than in the fixture. Not part of the drawing API.
#[doc(hidden)]
pub fn effects_scene(d: &Context2D) {
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

/// Surfaces for the GPU renderers' canvas tests, which use them without a tree.
#[cfg(all(test, any(feature = "wgpu", all(feature = "dx12", target_os = "windows"))))]
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
