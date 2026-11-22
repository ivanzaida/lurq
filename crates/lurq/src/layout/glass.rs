//! Glass: a node bends, blurs and colours what was painted behind its box,
//! as a piece of glass lying on the page does, and lights its own rim.
//!
//! A glass is a backdrop blur (see [`crate::layout::backdrop`]) with one more
//! thing done as the blurred pixels are put back: each pixel of the shape is
//! read from where a ray through the glass comes out, not from under itself.
//! The shape is thick in the middle and falls to nothing along its edge over
//! the width of its `bezel`, so the middle shows the backdrop where it is
//! and the edge is a lens that pulls it in. On the way the colour is
//! saturated, brightened and tinted; a light runs along the rim, a glow lies
//! inside it and a shadow under it.
//!
//! Several shapes are one glass: where two of them come within `merge` of
//! each other their edges flow together, as two drops do. A shape is a
//! rounded box, and the surface is the smooth union of the boxes' distance
//! fields ([`GlassCmd::distance`]), which is what the shaders measure too.
//!
//! [`Glass`] is what a node is given (`Node::glass`), in logical pixels;
//! [`GlassCmd`] is what a render engine draws, in physical ones.

use crate::{layout::opacity_layer::RoundedBox, node::color::Color};

/// The most shapes one glass has. A container with more children draws the
/// rest as glasses of their own.
pub const MAX_GLASS_SHAPES: usize = 8;

/// The material of a glass node. Lengths are logical pixels.
///
/// [`Glass::regular`] and [`Glass::clear`] are the two a phone's system has;
/// every number can be changed from there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Glass {
  /// The blur of what is behind, as [`Node::backdrop_blur`](crate::node::Node::backdrop_blur)'s
  /// radius.
  pub blur: f32,
  /// How thick the glass is in its middle. A thicker glass bends further.
  pub thickness: f32,
  /// The width of the curved edge: the lens. The glass is flat inside it. A
  /// shape narrower than two of them is curved all the way.
  pub bezel: f32,
  /// The index of refraction: 1 bends nothing, 1.5 is window glass.
  pub refraction: f32,
  /// How far apart red and blue are bent, as a part of the index: 0 for
  /// none. It colours the edges of what the lens shows.
  pub dispersion: f32,
  /// The saturation of what is behind: 1 leaves it, 0 is grey.
  pub saturation: f32,
  /// Light added to what is behind, -1 to 1.
  pub brightness: f32,
  /// The colour of the glass, laid over what is behind by its alpha.
  pub tint: Color,
  /// The light along the rim, 0 to 1.
  pub rim: f32,
  /// The glow just inside the rim, 0 to 1.
  pub glow: f32,
  /// The shadow under the glass.
  pub shadow: GlassShadow,
  /// How much of the material is there, 0 to 1: at 0 the node shows what is
  /// behind it as it is, and between the bending, the blur and the light
  /// grow in together. A glass comes and goes by this, not by opacity; it
  /// animates with the glass, as [`AnimatableProperty::Glass`](crate::animation::AnimatableProperty::Glass).
  pub amount: f32,
  /// Where a finger lights the glass, relative to the node's box, and how
  /// strongly (0 to 1). `None` for no touch.
  pub touch: Option<GlassTouch>,
}

/// The shadow a glass casts: soft, straight down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlassShadow {
  /// How far it spreads.
  pub radius: f32,
  /// How far down it lies.
  pub y: f32,
  /// How dark it is at the edge of the glass, 0 to 1.
  pub alpha: f32,
}

impl GlassShadow {
  /// No shadow.
  pub const NONE: Self = Self {
    radius: 0.0,
    y: 0.0,
    alpha: 0.0,
  };
}

/// The light a touch makes in a glass.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlassTouch {
  /// Where, from the node's top left corner.
  pub x: f32,
  pub y: f32,
  /// How far the light reaches.
  pub radius: f32,
  /// How bright, 0 to 1.
  pub strength: f32,
}

impl Glass {
  /// The glass of bars, buttons and menus: it blurs what is behind enough
  /// for words on it to be read over anything.
  pub const fn regular() -> Self {
    Self {
      blur: 14.0,
      thickness: 18.0,
      bezel: 14.0,
      refraction: 1.5,
      dispersion: 0.02,
      saturation: 1.6,
      brightness: 0.02,
      tint: Color::new(255, 255, 255, 46),
      rim: 0.55,
      glow: 0.18,
      shadow: GlassShadow {
        radius: 18.0,
        y: 6.0,
        alpha: 0.16,
      },
      amount: 1.0,
      touch: None,
    }
  }

  /// A glass that leaves what is behind sharp, for over pictures and video.
  /// Words on it need a backdrop that is calm, or dimmed.
  pub const fn clear() -> Self {
    Self {
      blur: 2.0,
      thickness: 22.0,
      bezel: 16.0,
      saturation: 1.2,
      brightness: 0.0,
      tint: Color::new(255, 255, 255, 14),
      rim: 0.7,
      glow: 0.1,
      ..Self::regular()
    }
  }

  /// The glass with this colour laid over what is behind, by its alpha.
  pub const fn tint(mut self, tint: Color) -> Self {
    self.tint = tint;
    self
  }

  pub const fn blur(mut self, radius: f32) -> Self {
    self.blur = radius;
    self
  }

  pub const fn thickness(mut self, thickness: f32) -> Self {
    self.thickness = thickness;
    self
  }

  pub const fn bezel(mut self, bezel: f32) -> Self {
    self.bezel = bezel;
    self
  }

  pub const fn refraction(mut self, index: f32) -> Self {
    self.refraction = index;
    self
  }

  pub const fn dispersion(mut self, dispersion: f32) -> Self {
    self.dispersion = dispersion;
    self
  }

  pub const fn saturation(mut self, saturation: f32) -> Self {
    self.saturation = saturation;
    self
  }

  pub const fn brightness(mut self, brightness: f32) -> Self {
    self.brightness = brightness;
    self
  }

  pub const fn rim(mut self, rim: f32) -> Self {
    self.rim = rim;
    self
  }

  pub const fn glow(mut self, glow: f32) -> Self {
    self.glow = glow;
    self
  }

  pub const fn shadow(mut self, shadow: GlassShadow) -> Self {
    self.shadow = shadow;
    self
  }

  /// How much of the material is there (see [`Self::amount`]).
  pub const fn amount(mut self, amount: f32) -> Self {
    self.amount = amount;
    self
  }

  /// Lit by a touch (see [`GlassTouch`]).
  pub const fn touch(mut self, touch: Option<GlassTouch>) -> Self {
    self.touch = touch;
    self
  }

  /// The glass between this one and `other`, `t` of the way: every number
  /// and the tint go straight across.
  pub fn lerp(&self, other: &Self, t: f32) -> Self {
    let mix = |a: f32, b: f32| a + (b - a) * t;
    let channel = |a: u8, b: u8| mix(f32::from(a), f32::from(b)).round().clamp(0.0, 255.0) as u8;
    let touch = match (self.touch, other.touch) {
      (Some(a), Some(b)) => Some(GlassTouch {
        x: mix(a.x, b.x),
        y: mix(a.y, b.y),
        radius: mix(a.radius, b.radius),
        strength: mix(a.strength, b.strength),
      }),
      (Some(a), None) => Some(GlassTouch {
        strength: a.strength * (1.0 - t),
        ..a
      }),
      (None, Some(b)) => Some(GlassTouch {
        strength: b.strength * t,
        ..b
      }),
      (None, None) => None,
    };
    Self {
      blur: mix(self.blur, other.blur),
      thickness: mix(self.thickness, other.thickness),
      bezel: mix(self.bezel, other.bezel),
      refraction: mix(self.refraction, other.refraction),
      dispersion: mix(self.dispersion, other.dispersion),
      saturation: mix(self.saturation, other.saturation),
      brightness: mix(self.brightness, other.brightness),
      tint: Color::new(
        channel(self.tint.r(), other.tint.r()),
        channel(self.tint.g(), other.tint.g()),
        channel(self.tint.b(), other.tint.b()),
        channel(self.tint.a(), other.tint.a()),
      ),
      rim: mix(self.rim, other.rim),
      glow: mix(self.glow, other.glow),
      shadow: GlassShadow {
        radius: mix(self.shadow.radius, other.shadow.radius),
        y: mix(self.shadow.y, other.shadow.y),
        alpha: mix(self.shadow.alpha, other.shadow.alpha),
      },
      amount: mix(self.amount, other.amount),
      touch,
    }
  }
}

impl Default for Glass {
  fn default() -> Self {
    Self::regular()
  }
}

/// Where the light on a glass's rim comes from when nothing says otherwise:
/// above and to the left, as a unit vector toward the light with y down.
pub const DEFAULT_GLASS_LIGHT: [f32; 2] = [-0.6, -0.8];

/// A glass as a render engine draws it: the shapes and the material, in
/// physical pixels of the window. Its [`BackdropCmd`](crate::layout::backdrop::BackdropCmd)
/// (the one whose `glass` is this command's index in
/// [`RenderList::glasses`](crate::layout::render_list::RenderList::glasses))
/// says when it is drawn, the blur, the clip and the opacity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlassCmd {
  /// The boxes that are the glass; the first `count` are used.
  pub shapes: [RoundedBox; MAX_GLASS_SHAPES],
  pub count: usize,
  /// How near two shapes flow together.
  pub merge: f32,
  pub thickness: f32,
  pub bezel: f32,
  pub refraction: f32,
  pub dispersion: f32,
  pub saturation: f32,
  pub brightness: f32,
  /// The tint, premultiplied, 0 to 1.
  pub tint: [f32; 4],
  pub rim: f32,
  pub glow: f32,
  /// The shadow's radius, how far down it lies, and its alpha.
  pub shadow: [f32; 3],
  /// The unit vector toward the light, y down.
  pub light: [f32; 2],
  /// A touch's light: its centre in the window, its radius, its strength.
  pub touch: [f32; 4],
}

/// A box no pixel is in: what the unused shapes of a [`GlassCmd`] hold.
pub const NO_GLASS_SHAPE: RoundedBox = RoundedBox {
  center: [0.0, 0.0],
  half: [0.0, 0.0],
  radii: [0.0; 4],
  inverse: [1.0, 0.0, 0.0, 1.0],
  scale: 1.0,
};

/// The steepest the glass's surface is taken to be, as rise over run: its
/// profile is upright at the very edge, where a ray would not get in.
pub(crate) const MAX_GLASS_SLOPE: f32 = 6.0;

impl GlassCmd {
  /// The command of `glass` over `shapes` (already in physical pixels), with
  /// the material's lengths multiplied by `scale`. `merge` is in physical
  /// pixels, and `touch_origin` is the window pixel a [`GlassTouch`] is
  /// measured from.
  pub fn new(
    glass: &Glass,
    shapes: &[RoundedBox],
    merge: f32,
    scale: f32,
    light: [f32; 2],
    touch_origin: [f32; 2],
  ) -> Self {
    let amount = glass.amount.clamp(0.0, 1.0);
    let mut boxes = [NO_GLASS_SHAPE; MAX_GLASS_SHAPES];
    let count = shapes.len().min(MAX_GLASS_SHAPES);
    boxes[..count].copy_from_slice(&shapes[..count]);
    let alpha = f32::from(glass.tint.a()) / 255.0 * amount;
    let tint = [
      f32::from(glass.tint.r()) / 255.0 * alpha,
      f32::from(glass.tint.g()) / 255.0 * alpha,
      f32::from(glass.tint.b()) / 255.0 * alpha,
      alpha,
    ];
    let touch = glass.touch.map_or([0.0; 4], |touch| {
      [
        touch_origin[0] + touch.x * scale,
        touch_origin[1] + touch.y * scale,
        (touch.radius * scale).max(1.0),
        touch.strength.clamp(0.0, 1.0) * amount,
      ]
    });
    Self {
      shapes: boxes,
      count,
      merge: merge.max(0.0),
      thickness: (glass.thickness * scale * amount).max(0.0),
      bezel: (glass.bezel * scale).max(1.0),
      refraction: glass.refraction.clamp(1.0, 3.0),
      dispersion: glass.dispersion.clamp(0.0, 0.5),
      saturation: 1.0 + (glass.saturation.max(0.0) - 1.0) * amount,
      brightness: glass.brightness.clamp(-1.0, 1.0) * amount,
      tint,
      rim: glass.rim.clamp(0.0, 1.0) * amount,
      glow: glass.glow.clamp(0.0, 1.0) * amount,
      shadow: [
        (glass.shadow.radius * scale).max(0.0),
        glass.shadow.y * scale,
        glass.shadow.alpha.clamp(0.0, 1.0) * amount,
      ],
      light,
      touch,
    }
  }

  /// The shapes in use.
  pub fn shapes(&self) -> &[RoundedBox] {
    &self.shapes[..self.count.min(MAX_GLASS_SHAPES)]
  }

  /// How far from under itself a pixel of the glass may be read: what the
  /// copy of the backdrop is grown by. A ray that meets the surface at a
  /// grazing angle leaves it within 42 degrees of the surface's normal (for
  /// an index of 1.5), so it lands within about the glass's thickness of
  /// where it went in; less for a surface no steeper than
  /// [`MAX_GLASS_SLOPE`].
  pub fn reach(&self) -> f32 {
    if self.thickness <= 0.0 {
      return 0.0;
    }
    (glass_offset(MAX_GLASS_SLOPE, 1.0, self.thickness, (self.refraction - self.dispersion).max(1.0))
      .max(glass_offset(MAX_GLASS_SLOPE, 1.0, self.thickness, self.refraction + self.dispersion)))
    .ceil()
      + 1.0
  }

  /// How far outside the shapes the glass paints: its shadow.
  pub fn spread(&self) -> f32 {
    if self.shadow[2] <= 0.0 {
      return 0.0;
    }
    (self.shadow[0] + self.shadow[1].abs()).ceil() + 1.0
  }

  /// The bounds of the shapes in the window: x0, y0, x1, y1. `None` without
  /// a shape.
  pub fn extent(&self) -> Option<[f32; 4]> {
    self
      .shapes()
      .iter()
      .filter(|shape| shape.half[0] > 0.0 && shape.half[1] > 0.0)
      .map(RoundedBox::extent)
      .reduce(|a, b| [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])])
  }

  /// The distance from the window pixel (`x`, `y`) to the glass's edge, in
  /// physical pixels: negative inside. The smooth union of the shapes'
  /// distances, as the shaders take it.
  pub fn distance(&self, x: f32, y: f32) -> f32 {
    let mut distance = f32::MAX;
    for shape in self.shapes() {
      distance = smooth_min(distance, box_distance(shape, x, y), self.merge);
    }
    distance
  }
}

/// The distance from (`x`, `y`) to the rounded edge of `shape`, in window
/// pixels; negative inside. A box without a size is infinitely far.
pub(crate) fn box_distance(shape: &RoundedBox, x: f32, y: f32) -> f32 {
  if !(shape.half[0] > 0.0 && shape.half[1] > 0.0) {
    return f32::MAX;
  }
  let (dx, dy) = (x - shape.center[0], y - shape.center[1]);
  let [a, b, c, d] = shape.inverse;
  let p = [a * dx + c * dy, b * dx + d * dy];
  let radius = match (p[0] >= 0.0, p[1] >= 0.0) {
    (false, false) => shape.radii[0],
    (true, false) => shape.radii[1],
    (true, true) => shape.radii[2],
    (false, true) => shape.radii[3],
  };
  let q = [p[0].abs() - shape.half[0] + radius, p[1].abs() - shape.half[1] + radius];
  (q[0].max(0.0).hypot(q[1].max(0.0)) + q[0].max(q[1]).min(0.0) - radius) * shape.scale
}

/// The lower of two distances, rounded where they are within `k` of each
/// other: the union of two shapes with the corner between them filled in.
pub(crate) fn smooth_min(a: f32, b: f32, k: f32) -> f32 {
  if k <= 0.0 || a == f32::MAX || b == f32::MAX {
    return a.min(b);
  }
  let h = (k - (a - b).abs()).max(0.0) / k;
  a.min(b) - h * h * k * 0.25
}

/// The height of the glass's surface `u` of the way in from its edge over
/// the bezel (0 at the edge, 1 where the glass is flat), as a part of its
/// thickness, and how fast it rises there (per unit of `u`). The profile is
/// a squircle's quarter: upright at the edge, flat at the top.
pub(crate) fn glass_profile(u: f32) -> (f32, f32) {
  let u = u.clamp(0.0, 1.0);
  let inside = 1.0 - u;
  let base = (1.0 - inside.powi(4)).max(1.0e-6);
  (base.powf(0.25), inside.powi(3) * base.powf(-0.75))
}

/// How far, and inward, a pixel of a glass of `thickness` is read from under
/// itself where its surface is `height` of the thickness up and rises by
/// `slope` (rise over run): Snell's law for a ray that comes straight down,
/// followed to the bottom of the glass.
pub(crate) fn glass_offset(slope: f32, height: f32, thickness: f32, index: f32) -> f32 {
  let slope = slope.clamp(0.0, MAX_GLASS_SLOPE);
  // The surface's normal leans outward by the slope.
  let cos_in = 1.0 / (1.0 + slope * slope).sqrt();
  let sin_in = slope * cos_in;
  let eta = 1.0 / index.max(1.0);
  let sin_out = eta * sin_in;
  let cos_out = (1.0 - sin_out * sin_out).max(0.0).sqrt();
  // The refracted ray, in the plane of the normal: its sideways and its
  // downward part.
  let across = (eta * cos_in - cos_out) * sin_in;
  let down = eta - (eta * cos_in - cos_out) * cos_in;
  if down <= 1.0e-4 {
    return 0.0;
  }
  -across / down * height * thickness
}

/// Uniforms of the pass that puts a blurred backdrop back as glass
/// (`glass.wgsl`): where the finished blur is read, the clips, the material
/// and the shapes, in the pixels of the target the pass draws into.
#[cfg(feature = "render")]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct GlassPass {
  /// As a [`LayerBlurPass`](crate::layout::opacity_layer::LayerBlurPass)'s
  /// that composites a finished blur.
  pub rect: [f32; 4],
  pub target: [f32; 4],
  pub source: [f32; 4],
  pub limit: [f32; 4],
  pub clip: [f32; 4],
  pub clip_inverse: [f32; 4],
  pub clip_radii: [f32; 4],
  pub outer_clip: [f32; 4],
  pub outer_clip_radii: [f32; 4],
  /// The clips' scale, the opacity, the number of shapes, the merge
  /// distance.
  pub cut: [f32; 4],
  /// Thickness, bezel, index of refraction, dispersion.
  pub optics: [f32; 4],
  /// Saturation, brightness, rim light, inner glow.
  pub look: [f32; 4],
  pub tint: [f32; 4],
  /// The shadow's radius, distance down and alpha, then
  /// [`MAX_GLASS_SLOPE`].
  pub shadow: [f32; 4],
  /// The unit vector toward the light, then the width of the rim's light.
  pub light: [f32; 4],
  /// A touch's centre in the target's pixels, its radius and strength.
  pub touch: [f32; 4],
  /// Each shape: its centre in the target's pixels and half size, its
  /// inverse matrix, its radii, and its scale.
  pub shapes: [[f32; 16]; MAX_GLASS_SHAPES],
}

#[cfg(feature = "render")]
impl GlassPass {
  /// The pass that puts `glass` back over the finished blur `composite`
  /// reads (a [`LayerBlurPass::backdrop`](crate::layout::opacity_layer::LayerBlurPass::backdrop)
  /// of the glass's region), into a target whose pixel (0, 0) is the window
  /// pixel `origin`. `rim` is the width of the rim's light in pixels.
  pub(crate) fn new(
    glass: &GlassCmd,
    composite: &crate::layout::opacity_layer::LayerBlurPass,
    origin: [f32; 2],
    rim: f32,
  ) -> Self {
    let mut shapes = [[0.0; 16]; MAX_GLASS_SHAPES];
    for (slot, shape) in shapes.iter_mut().zip(glass.shapes()) {
      slot[..4].copy_from_slice(&[
        shape.center[0] - origin[0],
        shape.center[1] - origin[1],
        shape.half[0],
        shape.half[1],
      ]);
      slot[4..8].copy_from_slice(&shape.inverse);
      slot[8..12].copy_from_slice(&shape.radii);
      slot[12] = shape.scale;
    }
    Self {
      rect: composite.rect,
      target: composite.target,
      source: composite.source,
      limit: composite.limit,
      clip: composite.clip,
      clip_inverse: composite.clip_inverse,
      clip_radii: composite.clip_radii,
      outer_clip: composite.outer_clip,
      outer_clip_radii: composite.outer_clip_radii,
      cut: [composite.mode[2], composite.kernel[3], glass.count as f32, glass.merge],
      optics: [glass.thickness, glass.bezel, glass.refraction, glass.dispersion],
      look: [glass.saturation, glass.brightness, glass.rim, glass.glow],
      tint: glass.tint,
      shadow: [glass.shadow[0], glass.shadow[1], glass.shadow[2], MAX_GLASS_SLOPE],
      light: [glass.light[0], glass.light[1], rim, 0.0],
      touch: [
        glass.touch[0] - origin[0],
        glass.touch[1] - origin[1],
        glass.touch[2],
        glass.touch[3],
      ],
      shapes,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn pill(x: f32, y: f32, width: f32, height: f32) -> RoundedBox {
    RoundedBox::rect(x, y, width, height, [height * 0.5; 4])
  }

  fn cmd(shapes: &[RoundedBox], merge: f32) -> GlassCmd {
    GlassCmd::new(&Glass::regular(), shapes, merge, 1.0, DEFAULT_GLASS_LIGHT, [0.0, 0.0])
  }

  #[test]
  fn a_shape_is_as_far_as_its_rounded_edge() {
    let glass = cmd(&[pill(10.0, 10.0, 100.0, 40.0)], 0.0);
    assert!((glass.distance(60.0, 30.0) + 20.0).abs() < 1e-4, "the middle is half the height in");
    assert!((glass.distance(60.0, 10.0)).abs() < 1e-4, "the top edge");
    assert!((glass.distance(60.0, 0.0) - 10.0).abs() < 1e-4);
    // The corner of the box is outside the pill's round end.
    assert!(glass.distance(11.0, 11.0) > 0.0);
    assert_eq!(glass.extent(), Some([10.0, 10.0, 110.0, 50.0]));
  }

  #[test]
  fn shapes_near_each_other_flow_together() {
    let (left, right) = (pill(0.0, 0.0, 40.0, 40.0), pill(48.0, 0.0, 40.0, 40.0));
    // Between the two, 4 pixels from either.
    let apart = cmd(&[left, right], 0.0);
    assert!((apart.distance(44.0, 20.0) - 4.0).abs() < 1e-4);
    let joined = cmd(&[left, right], 24.0);
    assert!(joined.distance(44.0, 20.0) < 0.0, "the gap is glass");
    // Far from the other shape nothing changes.
    assert_eq!(joined.distance(-10.0, 20.0), apart.distance(-10.0, 20.0));
    // Moved apart by more than the merge, they are two again.
    let far = cmd(&[left, pill(80.0, 0.0, 40.0, 40.0)], 24.0);
    assert!(far.distance(60.0, 20.0) > 10.0, "the gap is no glass");
    assert!((far.distance(41.0, 20.0) - 1.0).abs() < 1e-4, "and the edge is where it was");
  }

  #[test]
  fn the_edge_bends_inward_and_the_middle_does_not() {
    let (height, rise) = glass_profile(1.0);
    assert!((height - 1.0).abs() < 1e-5 && rise.abs() < 1e-5, "flat inside the bezel");
    assert_eq!(glass_offset(0.0, 1.0, 20.0, 1.5), 0.0);
    let (height, rise) = glass_profile(0.3);
    let offset = glass_offset(rise * 20.0 / 14.0, height, 20.0, 1.5);
    assert!(offset > 1.0, "{offset}");
    // No index, no bending.
    assert!(glass_offset(2.0, 0.8, 20.0, 1.0).abs() < 1e-4);
    // Nothing is read from further than the reach.
    let glass = cmd(&[pill(0.0, 0.0, 100.0, 40.0)], 0.0);
    for step in 0..=100 {
      let (height, rise) = glass_profile(step as f32 / 100.0);
      let slope = rise * glass.thickness / glass.bezel;
      for index in [glass.refraction - glass.dispersion, glass.refraction + glass.dispersion] {
        assert!(glass_offset(slope, height, glass.thickness, index) <= glass.reach());
      }
    }
  }

  #[test]
  fn a_glass_that_is_not_there_changes_nothing() {
    let glass = GlassCmd::new(
      &Glass::regular().amount(0.0),
      &[pill(0.0, 0.0, 100.0, 40.0)],
      0.0,
      2.0,
      DEFAULT_GLASS_LIGHT,
      [0.0, 0.0],
    );
    assert_eq!((glass.reach(), glass.spread()), (0.0, 0.0));
    assert_eq!((glass.saturation, glass.brightness, glass.rim, glass.glow), (1.0, 0.0, 0.0, 0.0));
    assert_eq!(glass.tint, [0.0; 4]);
  }

  #[test]
  fn the_material_is_scaled_to_the_window() {
    let glass = GlassCmd::new(
      &Glass::regular().touch(Some(GlassTouch {
        x: 10.0,
        y: 5.0,
        radius: 30.0,
        strength: 0.5,
      })),
      &[pill(0.0, 0.0, 100.0, 40.0)],
      0.0,
      3.0,
      DEFAULT_GLASS_LIGHT,
      [100.0, 200.0],
    );
    assert_eq!((glass.thickness, glass.bezel), (54.0, 42.0));
    assert_eq!(glass.touch, [130.0, 215.0, 90.0, 0.5]);
    assert_eq!(glass.shadow, [54.0, 18.0, 0.16]);
  }

  #[test]
  fn two_glasses_meet_half_way() {
    let half = Glass::regular().lerp(&Glass::clear(), 0.5);
    assert_eq!(half.blur, 8.0);
    assert_eq!(half.tint, Color::new(255, 255, 255, 30));
    assert_eq!(half.amount, 1.0);
  }
}
