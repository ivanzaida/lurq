//! Shadows and blurs: everything that needs a rasterised, blurred copy of what
//! is being drawn.
//!
//! Both are produced here on the CPU, in device pixels, and handed to the rest
//! of the canvas as an ordinary premultiplied RGBA image. That is what lets one
//! implementation serve the software backend and both native backends without a
//! third rendering path, and it is why a shadow's cost is stated in the same
//! units as every other command: the image it enqueues.
//!
//! Two bounds keep the work finite. A blurred copy is rasterised at a reduced
//! scale when its blur is wide (a wide blur has no detail to lose), and the
//! result is clipped to the surface, so nothing outside the visible canvas is
//! ever rasterised. What remains is capped at [`MAX_EFFECT_PIXELS`].

use std::{
  collections::HashMap,
  hash::{DefaultHasher, Hash, Hasher},
  sync::Arc,
};

use parking_lot::Mutex;
use tiny_skia::{Mask, Path, Pixmap, Stroke};

use super::{CanvasError, FillRule, path::Geometry, transform};
use crate::node::{color::Color, transform::Transform2D};

/// Blur radii are capped: an uncapped radius is an uncapped rasterisation.
/// In canvas-logical units, before the drawing transform scales them.
pub const MAX_SHADOW_BLUR: f32 = 512.0;
pub const MAX_BLUR_RADIUS: f32 = 512.0;
/// Spread is capped for the same reason; it grows the rasterised area directly.
pub const MAX_SHADOW_SPREAD: f32 = 512.0;
/// Device pixels one effect may rasterise, after reduction and surface clipping.
/// Four mebipixels is sixteen mebibytes of RGBA, the same order as one 2048²
/// image asset, and it is charged to the queue exactly like one.
pub const MAX_EFFECT_PIXELS: usize = 4 * 1024 * 1024;

const MAX_CACHE_BYTES: usize = 32 * 1024 * 1024;
const MAX_CACHE_ENTRIES: usize = 512;
const MAX_REDUCTION: u32 = 8;

/// A drop or inner shadow, in the user space in force when it is drawn: unlike
/// HTML Canvas, where shadows ignore the transform, offset, blur and spread
/// scale and rotate with the current transform. A design tool's shadows belong
/// to the node, so they have to follow it when the view zooms.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
  pub color: Color,
  pub offset: (f32, f32),
  /// 0..=[`MAX_SHADOW_BLUR`]. The visible falloff is about this wide.
  pub blur: f32,
  /// Grows (or, negative, shrinks) the shape the shadow is cast from.
  pub spread: f32,
  /// An inner shadow paints inside the shape instead of behind it.
  pub inset: bool,
}

impl Default for Shadow {
  fn default() -> Self {
    Self {
      color: Color::new(0, 0, 0, 128),
      offset: (0.0, 0.0),
      blur: 0.0,
      spread: 0.0,
      inset: false,
    }
  }
}

impl Shadow {
  pub fn new(color: Color) -> Self {
    Self {
      color,
      ..Self::default()
    }
  }
  pub fn offset(mut self, x: f32, y: f32) -> Self {
    self.offset = (x, y);
    self
  }
  pub fn blur(mut self, blur: f32) -> Self {
    self.blur = blur;
    self
  }
  pub fn spread(mut self, spread: f32) -> Self {
    self.spread = spread;
    self
  }
  pub fn inset(mut self, inset: bool) -> Self {
    self.inset = inset;
    self
  }
  pub(crate) fn is_valid(&self) -> bool {
    [self.offset.0, self.offset.1, self.blur, self.spread]
      .iter()
      .all(|v| v.is_finite())
      && (0.0..=MAX_SHADOW_BLUR).contains(&self.blur)
      && self.spread.abs() <= MAX_SHADOW_SPREAD
  }
}

/// A `filter`-like operation applied to what the following draws paint.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Filter {
  #[default]
  None,
  /// Layer blur: the drawn content is blurred by this radius, in user units.
  Blur(f32),
}

impl Filter {
  pub(crate) fn radius(self) -> f32 {
    match self {
      Self::None => 0.0,
      Self::Blur(radius) => radius,
    }
  }
  pub(crate) fn is_valid(self) -> bool {
    match self {
      Self::None => true,
      Self::Blur(radius) => radius.is_finite() && (0.0..=MAX_BLUR_RADIUS).contains(&radius),
    }
  }
}

/// A rasterised effect, in premultiplied sRGB RGBA at `1 / reduction` of device
/// scale, to be drawn with its top-left corner at `origin` device pixels.
#[derive(Clone)]
pub(crate) struct EffectImage {
  pub id: u64,
  pub data: Arc<Vec<u8>>,
  pub width: u32,
  pub height: u32,
  pub origin: (i32, i32),
  pub reduction: u32,
}

impl EffectImage {
  /// Places the image in device pixels. The canvas draws it with no drawing
  /// transform of its own: the transform is already in the rasterisation.
  pub fn matrix(&self) -> Transform2D {
    Transform2D::translate(self.origin.0 as f32, self.origin.1 as f32)
      .then(&Transform2D::scale_uniform(self.reduction as f32))
  }
  pub fn bytes(&self) -> usize {
    self.data.len() + 128
  }
}

#[derive(Default)]
struct EffectCache {
  entries: HashMap<u64, EffectImage>,
  keys: Vec<u64>,
  bytes: usize,
  random: u64,
}

impl EffectCache {
  fn evict_one(&mut self) {
    // Same reasoning as the mesh cache: a page is rasterised in the same order
    // every frame, so an ordered policy evicts exactly the next entry needed.
    self.random = self.random.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut value = self.random;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    let index = ((value ^ (value >> 31)) % self.keys.len() as u64) as usize;
    let victim = self.keys.swap_remove(index);
    if let Some(entry) = self.entries.remove(&victim) {
      self.bytes -= entry.bytes();
    }
  }
  fn insert(&mut self, key: u64, image: EffectImage) {
    let bytes = image.bytes();
    if bytes > MAX_CACHE_BYTES {
      return;
    }
    while !self.keys.is_empty() && (self.bytes + bytes > MAX_CACHE_BYTES || self.entries.len() >= MAX_CACHE_ENTRIES) {
      self.evict_one();
    }
    self.bytes += bytes;
    self.keys.push(key);
    self.entries.insert(key, image);
  }
}

/// Shared by every canvas in the process, because an effect is keyed by its own
/// content and a surface has no claim on it. Bounded in bytes and entries.
static EFFECTS: Mutex<Option<EffectCache>> = Mutex::new(None);

pub(crate) fn lookup(key: u64) -> Option<EffectImage> {
  EFFECTS.lock().as_ref()?.entries.get(&key).cloned()
}
pub(crate) fn store(key: u64, image: &EffectImage) {
  let mut guard = EFFECTS.lock();
  guard
    .get_or_insert_with(EffectCache::default)
    .insert(key, image.clone());
}

/// Standard deviation for a blur radius, following CSS's `blur()`.
fn sigma(radius: f32) -> f32 {
  radius * 0.5
}

/// Box radius whose three successive passes have the variance of `sigma`.
fn box_radius(sigma: f32) -> u32 {
  if sigma <= 0.0 {
    return 0;
  }
  (((1.0 + 4.0 * sigma * sigma).sqrt() - 1.0) * 0.5).round().max(0.0) as u32
}

fn box_blur_axis(source: &[u8], target: &mut [u8], width: usize, height: usize, radius: usize, horizontal: bool) {
  let window = (radius * 2 + 1) as u32;
  let (outer, inner) = if horizontal { (height, width) } else { (width, height) };
  let (step, stride) = if horizontal { (1usize, width) } else { (width, 1usize) };
  for o in 0..outer {
    let base = o * stride;
    let at = |i: usize| base + i * step;
    let mut sum: u32 = 0;
    for i in 0..=radius.min(inner.saturating_sub(1)) {
      sum += u32::from(source[at(i)]);
    }
    for i in 0..inner {
      target[at(i)] = (sum / window) as u8;
      let leaving = i as isize - radius as isize;
      let entering = i + radius + 1;
      if leaving >= 0 {
        sum -= u32::from(source[at(leaving as usize)]);
      }
      if entering < inner {
        sum += u32::from(source[at(entering)]);
      }
    }
  }
}

/// Three box passes, which is the usual Gaussian approximation and what CSS and
/// the design tools this matches are specified against.
fn blur_alpha(buffer: &mut [u8], width: usize, height: usize, radius: u32) {
  if radius == 0 || width == 0 || height == 0 {
    return;
  }
  let radius = radius as usize;
  let mut scratch = vec![0u8; buffer.len()];
  for _ in 0..3 {
    box_blur_axis(buffer, &mut scratch, width, height, radius, true);
    box_blur_axis(&scratch, buffer, width, height, radius, false);
  }
}

/// Device-space bounds of `path` under `matrix`, as an inclusive rectangle.
fn device_bounds(path: &Path, matrix: Transform2D) -> Option<[f32; 4]> {
  let b = path.bounds();
  let corners = [
    (b.left(), b.top()),
    (b.right(), b.top()),
    (b.right(), b.bottom()),
    (b.left(), b.bottom()),
  ]
  .map(|(x, y)| matrix.transform_point(x, y));
  let mut result = [f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY];
  for (x, y) in corners {
    if !x.is_finite() || !y.is_finite() {
      return None;
    }
    result[0] = result[0].min(x);
    result[1] = result[1].min(y);
    result[2] = result[2].max(x);
    result[3] = result[3].max(y);
  }
  Some(result)
}

/// The largest singular value of a linear map: how much one user unit can become
/// in device pixels. Blur, spread and offset are scaled by it.
pub(crate) fn device_scale(matrix: Transform2D) -> f32 {
  let [a, b, c, d] = [matrix.a, matrix.b, matrix.c, matrix.d].map(f64::from);
  (((a + d).hypot(b - c) + (a - d).hypot(b + c)) * 0.5) as f32
}

struct Plan {
  origin: (i32, i32),
  width: usize,
  height: usize,
  reduction: u32,
  /// Maps model coordinates to the reduced raster's own pixels.
  raster: Transform2D,
}

/// Chooses the raster the effect is drawn into: reduced for a wide blur, clipped
/// to the surface, and refused outright when what is left is still too large.
fn plan(
  bounds: [f32; 4],
  grow: f32,
  offset: (f32, f32),
  surface: (u32, u32),
  matrix: Transform2D,
  sigma_device: f32,
) -> Result<Option<Plan>, CanvasError> {
  let mut reduction = 1u32;
  loop {
    let pad = (grow + 3.0 * sigma_device) / reduction as f32 + 2.0;
    let left = ((bounds[0] + offset.0) / reduction as f32 - pad).floor();
    let top = ((bounds[1] + offset.1) / reduction as f32 - pad).floor();
    let right = ((bounds[2] + offset.0) / reduction as f32 + pad).ceil();
    let bottom = ((bounds[3] + offset.1) / reduction as f32 + pad).ceil();
    // Nothing outside the surface can be seen, so nothing outside it is drawn.
    let limit = (
      surface.0 as f32 / reduction as f32 + 1.0,
      surface.1 as f32 / reduction as f32 + 1.0,
    );
    let (left, top) = (left.max(-1.0), top.max(-1.0));
    let (right, bottom) = (right.min(limit.0), bottom.min(limit.1));
    if !(left.is_finite() && top.is_finite() && right.is_finite() && bottom.is_finite()) {
      return Ok(None);
    }
    if right <= left || bottom <= top {
      return Ok(None);
    }
    let (width, height) = ((right - left) as usize, (bottom - top) as usize);
    if width.saturating_mul(height) <= MAX_EFFECT_PIXELS {
      return Ok(Some(Plan {
        origin: ((left * reduction as f32) as i32, (top * reduction as f32) as i32),
        width,
        height,
        reduction,
        raster: Transform2D::translate(-left, -top)
          .then(&Transform2D::scale_uniform(1.0 / reduction as f32))
          .then(&matrix),
      }));
    }
    if reduction >= MAX_REDUCTION {
      return Err(CanvasError::StateLimit);
    }
    reduction *= 2;
  }
}

fn fill_mask(plan: &Plan, path: &Path, rule: FillRule, grow: f32) -> Option<Mask> {
  let mut mask = Mask::new(plan.width as u32, plan.height as u32)?;
  mask.fill_path(path, rule.skia(), true, transform(plan.raster));
  if grow != 0.0 {
    let stroke = Stroke {
      width: 2.0 * grow.abs(),
      ..Stroke::default()
    };
    if let Some(outline) = path.stroke(&stroke, 1.0) {
      let mut edge = Mask::new(plan.width as u32, plan.height as u32)?;
      edge.fill_path(&outline, tiny_skia::FillRule::Winding, true, transform(plan.raster));
      if grow > 0.0 {
        for (a, b) in mask.data_mut().iter_mut().zip(edge.data()) {
          *a = (*a).max(*b);
        }
      } else {
        for (a, b) in mask.data_mut().iter_mut().zip(edge.data()) {
          *a = ((u16::from(*a) * u16::from(255 - *b) + 127) / 255) as u8;
        }
      }
    }
  }
  Some(mask)
}

fn tint(alpha: &[u8], color: Color) -> Vec<u8> {
  let premultiplied = [color.r(), color.g(), color.b()].map(|c| u16::from(c) * u16::from(color.a()) / 255);
  let mut data = Vec::with_capacity(alpha.len() * 4);
  for coverage in alpha {
    let coverage = u16::from(*coverage);
    for channel in premultiplied {
      data.push(((channel * coverage + 127) / 255) as u8);
    }
    data.push(((u16::from(color.a()) * coverage + 127) / 255) as u8);
  }
  data
}

fn identity(kind: u8, parts: &[u64]) -> u64 {
  let mut hash = DefaultHasher::new();
  kind.hash(&mut hash);
  parts.hash(&mut hash);
  // Bit 61 keeps effect identities out of the image, ramp and text asset spaces.
  (hash.finish() & ((1 << 61) - 1)) | (1 << 61)
}

fn quantised(matrix: Transform2D) -> [u64; 4] {
  [matrix.a, matrix.b, matrix.c, matrix.d].map(|v| u64::from(v.to_bits()))
}

/// Rasterises a shadow cast by `path` under `matrix`. `user` is the linear map
/// from user units to device pixels, which is what scales offset, blur and
/// spread. Returns `None` when nothing would be visible.
pub(crate) fn shadow_image(
  geometry: &Geometry,
  rule: FillRule,
  matrix: Transform2D,
  user: Transform2D,
  surface: (u32, u32),
  shadow: &Shadow,
) -> Result<Option<EffectImage>, CanvasError> {
  if !shadow.is_valid() || surface.0 == 0 || surface.1 == 0 {
    return Ok(None);
  }
  let scale = device_scale(user);
  let sigma_device = sigma(shadow.blur) * scale;
  let spread_device = shadow.spread * scale;
  let (ox, oy) = user.transform_point(shadow.offset.0, shadow.offset.1);
  let Some(bounds) = device_bounds(geometry, matrix) else {
    return Ok(None);
  };
  let grow = spread_device.abs() + 1.0;
  // An inner shadow only ever paints inside the shape, so it is planned on the
  // shape's own bounds rather than on the offset, grown ones.
  let (plan_offset, plan_grow) = if shadow.inset {
    ((0.0, 0.0), 1.0)
  } else {
    ((ox, oy), grow)
  };
  let Some(plan) = plan(bounds, plan_grow, plan_offset, surface, matrix, sigma_device)? else {
    return Ok(None);
  };
  let key = identity(
    if shadow.inset { 1 } else { 0 },
    &[
      {
        let mut hash = DefaultHasher::new();
        geometry.hash(&mut hash);
        hash.finish()
      },
      u64::from(rule == FillRule::EvenOdd),
      u64::from(quantised(matrix)[0] as u32) ^ (quantised(matrix)[1] << 32),
      u64::from(quantised(matrix)[2] as u32) ^ (quantised(matrix)[3] << 32),
      ((plan.origin.0 as i64) as u64) ^ (((plan.origin.1 as i64) as u64) << 32),
      (plan.width as u64) ^ ((plan.height as u64) << 32),
      u64::from(sigma_device.to_bits()) ^ (u64::from(spread_device.to_bits()) << 32),
      u64::from(ox.to_bits()) ^ (u64::from(oy.to_bits()) << 32),
      u64::from(u32::from(shadow.color.r())) << 24
        | u64::from(u32::from(shadow.color.g())) << 16
        | u64::from(u32::from(shadow.color.b())) << 8
        | u64::from(u32::from(shadow.color.a())),
    ],
  );
  if let Some(image) = lookup(key) {
    return Ok(Some(image));
  }
  let radius = box_radius(sigma_device / plan.reduction as f32);
  let shifted = Transform2D::translate(ox / plan.reduction as f32, oy / plan.reduction as f32).then(&plan.raster);
  let shifted = Plan {
    origin: plan.origin,
    width: plan.width,
    height: plan.height,
    reduction: plan.reduction,
    raster: shifted,
  };
  let alpha = if shadow.inset {
    // What falls outside the offset, eroded shape, kept inside the shape itself.
    let (Some(shape), Some(cast)) = (
      fill_mask(&plan, geometry, rule, 0.0),
      fill_mask(&shifted, geometry, rule, -spread_device),
    ) else {
      return Err(CanvasError::SurfaceTooLarge);
    };
    let mut hole: Vec<u8> = cast.data().iter().map(|v| 255 - *v).collect();
    blur_alpha(&mut hole, plan.width, plan.height, radius);
    hole
      .iter()
      .zip(shape.data())
      .map(|(a, b)| ((u16::from(*a) * u16::from(*b) + 127) / 255) as u8)
      .collect::<Vec<u8>>()
  } else {
    let Some(cast) = fill_mask(&shifted, geometry, rule, spread_device) else {
      return Err(CanvasError::SurfaceTooLarge);
    };
    let mut alpha = cast.data().to_vec();
    blur_alpha(&mut alpha, plan.width, plan.height, radius);
    alpha
  };
  let image = EffectImage {
    id: key,
    data: Arc::new(tint(&alpha, shadow.color)),
    width: plan.width as u32,
    height: plan.height as u32,
    origin: plan.origin,
    reduction: plan.reduction,
  };
  store(key, &image);
  Ok(Some(image))
}

/// Blurs an already rasterised, premultiplied RGBA pixmap in place.
pub(crate) fn blur_pixmap(pixmap: &mut Pixmap, radius: u32) {
  if radius == 0 {
    return;
  }
  let (width, height) = (pixmap.width() as usize, pixmap.height() as usize);
  let mut channel = vec![0u8; width * height];
  for offset in 0..4 {
    for (index, value) in channel.iter_mut().enumerate() {
      *value = pixmap.data()[index * 4 + offset];
    }
    blur_alpha(&mut channel, width, height, radius);
    for (index, value) in channel.iter().enumerate() {
      pixmap.data_mut()[index * 4 + offset] = *value;
    }
  }
}

/// What a layer blur needs before its content is drawn: the raster to draw into,
/// the image that raster will become, the box radius to blur it by, and the map
/// from model coordinates into the raster.
pub(crate) struct BlurPlan {
  pub pixmap: Pixmap,
  pub image: EffectImage,
  pub radius: u32,
  pub raster: Transform2D,
}

/// Plans a layer blur of `radius` for `geometry` under `matrix`.
pub(crate) fn blur_plan(
  geometry: &Geometry,
  matrix: Transform2D,
  user: Transform2D,
  surface: (u32, u32),
  radius: f32,
) -> Result<Option<BlurPlan>, CanvasError> {
  if surface.0 == 0 || surface.1 == 0 || !radius.is_finite() || !(0.0..=MAX_BLUR_RADIUS).contains(&radius) {
    return Ok(None);
  }
  let sigma_device = sigma(radius) * device_scale(user);
  let Some(bounds) = device_bounds(geometry, matrix) else {
    return Ok(None);
  };
  let Some(plan) = plan(bounds, 1.0, (0.0, 0.0), surface, matrix, sigma_device)? else {
    return Ok(None);
  };
  let Some(pixmap) = Pixmap::new(plan.width as u32, plan.height as u32) else {
    return Err(CanvasError::SurfaceTooLarge);
  };
  Ok(Some(BlurPlan {
    pixmap,
    image: EffectImage {
      id: 0,
      data: Arc::new(Vec::new()),
      width: plan.width as u32,
      height: plan.height as u32,
      origin: plan.origin,
      reduction: plan.reduction,
    },
    radius: box_radius(sigma_device / plan.reduction as f32),
    raster: plan.raster,
  }))
}

/// Finishes a layer blur: the rasterised content becomes an addressable image.
pub(crate) fn blur_finish(mut pixmap: Pixmap, mut image: EffectImage, radius: u32, key: u64) -> EffectImage {
  blur_pixmap(&mut pixmap, radius);
  image.id = key;
  image.data = Arc::new(pixmap.data().to_vec());
  image
}

/// The device-pixel box an already rasterised source covers under `matrix`.
fn pixmap_bounds(source: &Pixmap, matrix: Transform2D) -> Option<[f32; 4]> {
  let mut path = tiny_skia::PathBuilder::new();
  path.push_rect(tiny_skia::Rect::from_xywh(
    0.0,
    0.0,
    source.width() as f32,
    source.height() as f32,
  )?);
  device_bounds(&path.finish()?, matrix)
}

fn draw_into(target: &mut Pixmap, source: &Pixmap, matrix: Transform2D) {
  target.draw_pixmap(
    0,
    0,
    source.as_ref(),
    &tiny_skia::PixmapPaint {
      quality: tiny_skia::FilterQuality::Bilinear,
      ..Default::default()
    },
    transform(matrix),
    None,
  );
}

/// A shadow cast by an already rasterised premultiplied source — shaped text is
/// the one that matters — placed by `matrix`, which maps its pixels to device
/// pixels. Spread has no meaning for a raster and is not applied; everything
/// else behaves as it does for a path.
pub(crate) fn pixmap_shadow(
  source: &Pixmap,
  identity_of: u64,
  matrix: Transform2D,
  user: Transform2D,
  surface: (u32, u32),
  shadow: &Shadow,
) -> Result<Option<EffectImage>, CanvasError> {
  if !shadow.is_valid() || surface.0 == 0 || surface.1 == 0 {
    return Ok(None);
  }
  let scale = device_scale(user);
  let sigma_device = sigma(shadow.blur) * scale;
  let (ox, oy) = user.transform_point(shadow.offset.0, shadow.offset.1);
  let Some(bounds) = pixmap_bounds(source, matrix) else {
    return Ok(None);
  };
  let (plan_offset, plan_grow) = if shadow.inset {
    ((0.0, 0.0), 1.0)
  } else {
    ((ox, oy), 1.0)
  };
  let Some(plan) = plan(bounds, plan_grow, plan_offset, surface, matrix, sigma_device)? else {
    return Ok(None);
  };
  let key = identity(
    if shadow.inset { 4 } else { 3 },
    &[
      identity_of,
      ((plan.origin.0 as i64) as u64) ^ (((plan.origin.1 as i64) as u64) << 32),
      (plan.width as u64) ^ ((plan.height as u64) << 32),
      u64::from(sigma_device.to_bits()) ^ (u64::from(plan.reduction) << 32),
      u64::from(ox.to_bits()) ^ (u64::from(oy.to_bits()) << 32),
      (u64::from(shadow.color.r()) << 24)
        | (u64::from(shadow.color.g()) << 16)
        | (u64::from(shadow.color.b()) << 8)
        | u64::from(shadow.color.a()),
    ],
  );
  if let Some(image) = lookup(key) {
    return Ok(Some(image));
  }
  let radius = box_radius(sigma_device / plan.reduction as f32);
  let Some(mut raster) = Pixmap::new(plan.width as u32, plan.height as u32) else {
    return Err(CanvasError::SurfaceTooLarge);
  };
  let shifted = Transform2D::translate(ox / plan.reduction as f32, oy / plan.reduction as f32).then(&plan.raster);
  draw_into(&mut raster, source, if shadow.inset { plan.raster } else { shifted });
  let mut alpha: Vec<u8> = raster.data().chunks_exact(4).map(|p| p[3]).collect();
  if shadow.inset {
    let Some(mut shape) = Pixmap::new(plan.width as u32, plan.height as u32) else {
      return Err(CanvasError::SurfaceTooLarge);
    };
    draw_into(&mut shape, source, plan.raster);
    let mut hole: Vec<u8> = raster.data().chunks_exact(4).map(|p| 255 - p[3]).collect();
    // The offset copy is what the hole is punched from.
    let mut offset = Pixmap::new(plan.width as u32, plan.height as u32).ok_or(CanvasError::SurfaceTooLarge)?;
    draw_into(&mut offset, source, shifted);
    for (h, p) in hole.iter_mut().zip(offset.data().chunks_exact(4)) {
      *h = 255 - p[3];
    }
    blur_alpha(&mut hole, plan.width, plan.height, radius);
    alpha = hole
      .iter()
      .zip(shape.data().chunks_exact(4))
      .map(|(a, p)| ((u16::from(*a) * u16::from(p[3]) + 127) / 255) as u8)
      .collect();
  } else {
    blur_alpha(&mut alpha, plan.width, plan.height, radius);
  }
  let image = EffectImage {
    id: key,
    data: Arc::new(tint(&alpha, shadow.color)),
    width: plan.width as u32,
    height: plan.height as u32,
    origin: plan.origin,
    reduction: plan.reduction,
  };
  store(key, &image);
  Ok(Some(image))
}

/// A layer blur of an already rasterised premultiplied source.
pub(crate) fn pixmap_blur(
  source: &Pixmap,
  identity_of: u64,
  matrix: Transform2D,
  user: Transform2D,
  surface: (u32, u32),
  blur: f32,
) -> Result<Option<EffectImage>, CanvasError> {
  if surface.0 == 0 || surface.1 == 0 || !blur.is_finite() || !(0.0..=MAX_BLUR_RADIUS).contains(&blur) {
    return Ok(None);
  }
  let sigma_device = sigma(blur) * device_scale(user);
  let Some(bounds) = pixmap_bounds(source, matrix) else {
    return Ok(None);
  };
  let Some(plan) = plan(bounds, 1.0, (0.0, 0.0), surface, matrix, sigma_device)? else {
    return Ok(None);
  };
  let key = identity(
    5,
    &[
      identity_of,
      ((plan.origin.0 as i64) as u64) ^ (((plan.origin.1 as i64) as u64) << 32),
      (plan.width as u64) ^ ((plan.height as u64) << 32),
      u64::from(sigma_device.to_bits()) ^ (u64::from(plan.reduction) << 32),
    ],
  );
  if let Some(image) = lookup(key) {
    return Ok(Some(image));
  }
  let Some(mut raster) = Pixmap::new(plan.width as u32, plan.height as u32) else {
    return Err(CanvasError::SurfaceTooLarge);
  };
  draw_into(&mut raster, source, plan.raster);
  let image = blur_finish(
    raster,
    EffectImage {
      id: key,
      data: Arc::new(Vec::new()),
      width: plan.width as u32,
      height: plan.height as u32,
      origin: plan.origin,
      reduction: plan.reduction,
    },
    box_radius(sigma_device / plan.reduction as f32),
    key,
  );
  store(key, &image);
  Ok(Some(image))
}

/// Identity for a layer blur, over everything that changes its pixels.
pub(crate) fn blur_identity(parts: &[u64]) -> u64 {
  identity(2, parts)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn box_radius_tracks_sigma_and_a_blur_conserves_alpha() {
    assert_eq!(box_radius(0.0), 0);
    assert!(box_radius(4.0) >= 3);
    let (width, height) = (64usize, 64usize);
    let mut buffer = vec![0u8; width * height];
    for y in 24..40 {
      for x in 24..40 {
        buffer[y * width + x] = 255;
      }
    }
    let before: u64 = buffer.iter().map(|v| u64::from(*v)).sum();
    blur_alpha(&mut buffer, width, height, box_radius(3.0));
    let after: u64 = buffer.iter().map(|v| u64::from(*v)).sum();
    assert!(buffer[32 * width + 32] > 200, "the centre stays nearly opaque");
    assert!(buffer[32 * width + 20] > 0, "coverage spreads outward");
    // A box blur of a padded region is mass-preserving to within rounding.
    assert!(after * 100 > before * 90 && after < before * 2, "{before} -> {after}");
  }

  #[test]
  fn an_effect_larger_than_the_cap_is_refused_rather_than_rasterised() {
    let bounds = [0., 0., 40_000., 40_000.];
    let planned = plan(bounds, 0.0, (0.0, 0.0), (16_384, 16_384), Transform2D::IDENTITY, 0.0);
    assert!(matches!(planned, Err(CanvasError::StateLimit)));
  }
}
