//! Paints a fill or a stroke can use: a solid colour, or a gradient ramp.
//!
//! A gradient is described the way an authoring tool describes one — a centre, a
//! size and a rotation relative to a box, plus ordered stops — so a consumer that
//! stores paints that way maps them without inventing a second geometry. The ramp
//! is sampled into [`RAMP_TEXELS`] premultiplied texels once per distinct gradient
//! and is shared by every draw that uses it, on the GPU as an ordinary image asset
//! and on the software backend as a shader.

use std::{
  hash::{DefaultHasher, Hash, Hasher},
  sync::{Arc, OnceLock},
};

use tiny_skia::{
  FilterQuality, GradientStop, LinearGradient, Pattern, Pixmap, Point, RadialGradient, Shader, SpreadMode,
};

use super::{CanvasColor, transform};
use crate::node::{color::Color, transform::Transform2D};

/// Stops accepted per gradient. Fewer than two has no ramp to interpolate.
pub const MAX_GRADIENT_STOPS: usize = 16;
pub const MIN_GRADIENT_STOPS: usize = 2;
/// Texels in a sampled ramp. One kibibyte of premultiplied RGBA per gradient.
pub const RAMP_TEXELS: usize = 256;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum GradientKind {
  /// `t` runs along the box's own x axis, 0 at the left edge of the sized box.
  #[default]
  Linear,
  /// `t` is the distance from the centre, 1 on the sized box's ellipse.
  Radial,
  /// `t` is the angle about the centre, 0 along +x and increasing clockwise.
  Angular,
}

#[derive(Debug)]
pub(crate) struct Ramp {
  pub id: u64,
  pub texels: Arc<Vec<u8>>,
}

/// A gradient paint. Clones share one sampled ramp, so cloning a gradient into a
/// drawing state every frame does not resample it.
#[derive(Clone, Debug)]
pub struct Gradient {
  kind: GradientKind,
  center: [f32; 2],
  size: [f32; 2],
  rotation: f32,
  stops: Vec<(f32, Color)>,
  ramp: Arc<OnceLock<Ramp>>,
}

impl PartialEq for Gradient {
  fn eq(&self, other: &Self) -> bool {
    self.kind == other.kind
      && self.center == other.center
      && self.size == other.size
      && self.rotation == other.rotation
      && self.stops == other.stops
  }
}

impl Gradient {
  /// Centre and size are fractions of the box the paint is used in: the default
  /// is a gradient centred on the box and spanning it exactly.
  pub fn new(kind: GradientKind) -> Self {
    Self {
      kind,
      center: [0.5, 0.5],
      size: [1.0, 1.0],
      rotation: 0.0,
      stops: Vec::new(),
      ramp: Arc::new(OnceLock::new()),
    }
  }
  pub fn linear() -> Self {
    Self::new(GradientKind::Linear)
  }
  pub fn radial() -> Self {
    Self::new(GradientKind::Radial)
  }
  pub fn angular() -> Self {
    Self::new(GradientKind::Angular)
  }

  fn edited(mut self) -> Self {
    self.ramp = Arc::new(OnceLock::new());
    self
  }
  /// Fractions of the box, `(0.5, 0.5)` being its centre.
  pub fn center(mut self, x: f32, y: f32) -> Self {
    self.center = [x, y];
    self.edited()
  }
  /// Fractions of the box: `(1.0, 1.0)` spans it, `(0.5, 0.5)` spans its middle half.
  pub fn size(mut self, width: f32, height: f32) -> Self {
    self.size = [width, height];
    self.edited()
  }
  /// Clockwise, in radians, about the centre.
  pub fn rotation(mut self, radians: f32) -> Self {
    self.rotation = radians;
    self.edited()
  }
  /// Offsets run 0..=1 and must not decrease. Unparseable colours are ignored,
  /// which leaves the gradient short of stops and therefore refused at use.
  pub fn stop(mut self, offset: f32, color: impl CanvasColor) -> Self {
    if let Some(color) = color.canvas_color()
      && self.stops.len() < MAX_GRADIENT_STOPS
    {
      self.stops.push((offset, color));
    }
    self.edited()
  }

  pub fn kind(&self) -> GradientKind {
    self.kind
  }
  pub fn stops(&self) -> &[(f32, Color)] {
    &self.stops
  }

  /// Every reason a gradient cannot paint: too few or too many stops, offsets
  /// that are not ascending within 0..=1, and non-finite or degenerate geometry.
  pub fn is_valid(&self) -> bool {
    if !(MIN_GRADIENT_STOPS..=MAX_GRADIENT_STOPS).contains(&self.stops.len()) {
      return false;
    }
    let mut previous = f32::NEG_INFINITY;
    for (offset, _) in &self.stops {
      if !offset.is_finite() || !(0.0..=1.0).contains(offset) || *offset < previous {
        return false;
      }
      previous = *offset;
    }
    self
      .center
      .iter()
      .chain(self.size.iter())
      .chain(std::iter::once(&self.rotation))
      .all(|v| v.is_finite())
      && self.size[0] != 0.0
      && self.size[1] != 0.0
  }

  /// Premultiplied sRGB RGBA, `RAMP_TEXELS` wide and one texel tall, with a
  /// content-derived identity so one ramp is uploaded once however many draws
  /// use it. Colour and alpha are interpolated separately and premultiplied
  /// afterwards, which is what the software backend's own gradients do; the two
  /// have to agree, because a pixel test compares them.
  pub(crate) fn ramp(&self) -> &Ramp {
    self.ramp.get_or_init(|| {
      let mut texels = vec![0u8; RAMP_TEXELS * 4];
      for (index, texel) in texels.chunks_exact_mut(4).enumerate() {
        let t = (index as f32 + 0.5) / RAMP_TEXELS as f32;
        texel.copy_from_slice(&self.sample(t));
      }
      let mut hash = DefaultHasher::new();
      texels.hash(&mut hash);
      Ramp {
        // Bit 62 keeps ramp identities out of the image and shaped-text asset
        // spaces, which are a counter and bit 63 respectively.
        id: (hash.finish() & ((1 << 62) - 1)) | (1 << 62),
        texels: Arc::new(texels),
      }
    })
  }

  fn sample(&self, t: f32) -> [u8; 4] {
    let channels = |c: &Color| [c.r(), c.g(), c.b(), c.a()].map(|v| f32::from(v) / 255.0);
    let first = &self.stops[0];
    let last = &self.stops[self.stops.len() - 1];
    let straight = if t <= first.0 {
      channels(&first.1)
    } else if t >= last.0 {
      channels(&last.1)
    } else {
      let index = self
        .stops
        .windows(2)
        .position(|w| t >= w[0].0 && t <= w[1].0)
        .unwrap_or(0);
      let (a, b) = (&self.stops[index], &self.stops[index + 1]);
      let span = b.0 - a.0;
      let k = if span > 0.0 { (t - a.0) / span } else { 0.0 };
      let (ca, cb) = (channels(&a.1), channels(&b.1));
      [0, 1, 2, 3].map(|i| ca[i] + (cb[i] - ca[i]) * k)
    };
    let alpha = straight[3].clamp(0.0, 1.0);
    [straight[0] * alpha, straight[1] * alpha, straight[2] * alpha, alpha]
      .map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
  }

  /// The parameter this gradient reads at a point of its own frame. The two
  /// native backends compute the same three expressions in their shaders.
  pub(crate) fn parameter(kind: GradientKind, x: f32, y: f32) -> f32 {
    match kind {
      GradientKind::Linear => x * 0.5 + 0.5,
      GradientKind::Radial => x.hypot(y),
      GradientKind::Angular => {
        let turn = y.atan2(x) / std::f32::consts::TAU;
        turn - turn.floor()
      }
    }
  }

  /// Maps a point in the space `bounds` is written in onto the gradient's own
  /// frame, where linear runs -1..1 along x and radial reaches 1 on the box's
  /// ellipse. The box is normalised first, then rotated, then divided by `size`,
  /// so a rotation means the same thing in a wide box as in a square one — which
  /// is what an authoring tool's normalised gradient handles mean. `None` when
  /// the box or the gradient collapses.
  pub(crate) fn frame_from_box(&self, bounds: [f32; 4]) -> Option<Transform2D> {
    let [x, y, width, height] = bounds;
    if !bounds.iter().all(|v| v.is_finite()) || width == 0.0 || height == 0.0 {
      return None;
    }
    let (cx, cy) = (x + self.center[0] * width, y + self.center[1] * height);
    let (sx, sy) = (self.size[0], self.size[1]);
    if sx == 0.0 || sy == 0.0 {
      return None;
    }
    let frame = Transform2D::scale(1.0 / sx, 1.0 / sy)
      .then(&Transform2D::rotate(-self.rotation))
      .then(&Transform2D::scale(2.0 / width, 2.0 / height))
      .then(&Transform2D::translate(-cx, -cy));
    [frame.a, frame.b, frame.c, frame.d, frame.tx, frame.ty]
      .iter()
      .all(|v| v.is_finite())
      .then_some(frame)
  }
}

/// What a fill or a stroke paints with.
#[derive(Clone, Debug, PartialEq)]
pub enum Paint {
  Solid(Color),
  /// `bounds` is `[x, y, width, height]` in the user space in force when the
  /// paint is used, exactly as HTML Canvas gradient coordinates are.
  Gradient {
    gradient: Gradient,
    bounds: [f32; 4],
  },
}

impl Paint {
  /// The colour a solid paint paints with. A gradient has no single colour, and
  /// operations that need one (shaped text today) refuse rather than pick one.
  pub fn color(&self) -> Option<Color> {
    match self {
      Self::Solid(color) => Some(*color),
      Self::Gradient { .. } => None,
    }
  }
  pub(crate) fn gradient(&self) -> Option<(&Gradient, [f32; 4])> {
    match self {
      Self::Solid(_) => None,
      Self::Gradient { gradient, bounds } => Some((gradient, *bounds)),
    }
  }
}

impl Gradient {
  /// Uses this gradient over `[x, y, width, height]` of the current user space.
  pub fn in_box(self, x: f32, y: f32, width: f32, height: f32) -> Paint {
    Paint::Gradient {
      gradient: self,
      bounds: [x, y, width, height],
    }
  }
}

impl From<Color> for Paint {
  fn from(color: Color) -> Self {
    Self::Solid(color)
  }
}

/// Accepted paints. A colour, a parseable colour string, or a built [`Paint`].
/// A value that cannot be resolved leaves the current paint in place, which is
/// what the colour setters have always done with an unparseable string.
pub trait CanvasPaint {
  fn canvas_paint(&self) -> Option<Paint>;
}
impl CanvasPaint for Paint {
  fn canvas_paint(&self) -> Option<Paint> {
    Some(self.clone())
  }
}
impl CanvasPaint for Color {
  fn canvas_paint(&self) -> Option<Paint> {
    Some(Paint::Solid(*self))
  }
}
impl CanvasPaint for &str {
  fn canvas_paint(&self) -> Option<Paint> {
    self.canvas_color().map(Paint::Solid)
  }
}
impl CanvasPaint for String {
  fn canvas_paint(&self) -> Option<Paint> {
    self.canvas_color().map(Paint::Solid)
  }
}

/// The tiny-skia shader for a gradient. `frame_to_target` maps the gradient's
/// own frame onto the raster being painted. An angular gradient has no
/// tiny-skia shader, so it is handed a pattern this module evaluated itself.
pub(crate) fn skia_shader<'a>(
  gradient: &Gradient,
  frame_to_target: Transform2D,
  conic: Option<(&'a Pixmap, (i32, i32))>,
) -> Option<Shader<'a>> {
  if !gradient.is_valid() {
    return None;
  }
  if gradient.kind == GradientKind::Angular {
    let (pixmap, origin) = conic?;
    return Some(Pattern::new(
      pixmap.as_ref(),
      SpreadMode::Pad,
      FilterQuality::Nearest,
      1.0,
      tiny_skia::Transform::from_translate(origin.0 as f32, origin.1 as f32),
    ));
  }
  let stops: Vec<GradientStop> = gradient
    .stops
    .iter()
    .map(|(offset, color)| {
      GradientStop::new(
        *offset,
        tiny_skia::Color::from_rgba8(color.r(), color.g(), color.b(), color.a()),
      )
    })
    .collect();
  let matrix = transform(frame_to_target);
  match gradient.kind {
    GradientKind::Linear => LinearGradient::new(
      Point::from_xy(-1.0, 0.0),
      Point::from_xy(1.0, 0.0),
      stops,
      SpreadMode::Pad,
      matrix,
    ),
    _ => RadialGradient::new(
      Point::from_xy(0.0, 0.0),
      Point::from_xy(0.0, 0.0),
      1.0,
      stops,
      SpreadMode::Pad,
      matrix,
    ),
  }
}

/// Evaluates an angular gradient into a premultiplied pattern the size of the
/// raster it will paint. `frame_from_target` maps a raster pixel centre, offset
/// by `origin`, onto the gradient's frame.
pub(crate) fn angular_pixmap(
  gradient: &Gradient,
  frame_from_target: Transform2D,
  origin: (i32, i32),
  width: u32,
  height: u32,
) -> Option<Pixmap> {
  if !gradient.is_valid() || width == 0 || height == 0 {
    return None;
  }
  let mut pixmap = Pixmap::new(width, height)?;
  let ramp = gradient.ramp();
  let texels = ramp.texels.as_ref();
  let data = pixmap.data_mut();
  for y in 0..height {
    for x in 0..width {
      let (u, v) =
        frame_from_target.transform_point(x as f32 + origin.0 as f32 + 0.5, y as f32 + origin.1 as f32 + 0.5);
      let t = Gradient::parameter(GradientKind::Angular, u, v);
      let index = ((t * RAMP_TEXELS as f32) as usize).min(RAMP_TEXELS - 1) * 4;
      let out = ((y * width + x) * 4) as usize;
      data[out..out + 4].copy_from_slice(&texels[index..index + 4]);
    }
  }
  Some(pixmap)
}
