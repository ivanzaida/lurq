//! Glass: a node bends, blurs and colours what was painted behind its box,
//! as a piece of glass lying on the page does, and lights its own rim.
//!
//! A glass is a backdrop blur (see [`crate::layout::backdrop`]) with one more
//! thing done as the blurred pixels are put back: each pixel of the shape is
//! read from where a ray through the glass comes out, not from under itself.
//! The glass is flat in the middle and curved along its edge over the width
//! of its `bezel`, so the middle shows the backdrop where it is and the edge
//! is a lens that pulls it in from further inside, the more the nearer the
//! edge ([`lens_pull`]): at the very edge from so far in that what is seen
//! there is the inside turned over, as in a drop. On the way the colour is
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
  /// How far inside itself the very edge of the glass shows the backdrop
  /// from: the strength of the lens. 0 bends nothing.
  pub bend: f32,
  /// The width of the curved edge: the lens. The glass is flat inside it. A
  /// shape narrower than two of them is curved all the way.
  pub bezel: f32,
  /// How far apart red and blue are bent, as a part of the bend: 0 for
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
  /// The colour of what is on the glass: the foreground of the node's
  /// words and symbols, where the node names none. `None` leaves them as
  /// they are.
  pub ink: Option<Color>,
  /// What the glass looks like over something dark: with it, the glass
  /// follows what is behind it, a light glass with dark words over a light
  /// backdrop and this look over a dark one, and goes from one to the other
  /// as the backdrop changes. `None` for a glass that looks the same over
  /// anything.
  pub over_dark: Option<GlassLook>,
  /// What decides between the glass's own look and its look
  /// [over dark](Self::over_dark).
  pub follows: GlassFollows,
  /// The glass answers a press: while the node is held down it grows a
  /// little and is lit from where the pointer or the finger is, and let go
  /// it springs back. The light follows a finger that moves.
  pub interactive: bool,
}

/// What a node hears when its glass turns to its other look (`Node::on_glass_look`): whether the glass is now
/// the dark one. Two are the same when they are the same function.
#[derive(Clone)]
pub struct GlassLookHandler(pub std::sync::Arc<dyn Fn(bool) + Send + Sync>);

impl PartialEq for GlassLookHandler {
  fn eq(&self, other: &Self) -> bool {
    std::sync::Arc::ptr_eq(&self.0, &other.0)
  }
}

impl std::fmt::Debug for GlassLookHandler {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("GlassLookHandler")
  }
}

/// What a glass with two looks takes its look from.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GlassFollows {
  /// What is painted behind it: a button over a list that scrolls. In the
  /// light appearance it turns dark over a backdrop darker than the middle;
  /// in the dark appearance it is dark over all but a very light one.
  #[default]
  Backdrop,
  /// The app's appearance alone: the light look in a light theme and the
  /// dark look in a dark one (one whose page is dark), whatever is behind.
  /// A large surface.
  Appearance,
}

/// The part of a glass that changes with what is behind it (see
/// [`Glass::over_dark`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlassLook {
  pub tint: Color,
  pub brightness: f32,
  pub saturation: f32,
  pub rim: f32,
  /// The colour of what is on the glass.
  pub ink: Color,
}

/// How dark a backdrop must get for a glass that follows it to turn dark,
/// then how light for it to turn light, as [`backdrop_lightness`](crate::layout::glass_tone::backdrop_lightness)
/// measures it. Between the two a glass stays as it is, so one over a
/// backdrop near the line does not flicker. The system's glass turns at a
/// grey of 128 of 255 (0.54) in the light appearance, and in the dark one
/// stays dark up to a grey of 225 (0.9).
pub const fn glass_turns(dark_appearance: bool) -> (f32, f32) {
  if dark_appearance { (0.86, 0.9) } else { (0.51, 0.56) }
}

/// The shadow a glass casts: soft, straight down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlassShadow {
  /// How far it is blurred, as a box shadow's blur.
  pub radius: f32,
  /// How far down it lies.
  pub y: f32,
  /// How dark it is under the glass, 0 to 1; half of that is seen where
  /// the shadow's own edge is.
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
  /// The glass of buttons, bars and menus: it blurs what is behind enough
  /// for words on it to be read over anything, and follows it: light with
  /// dark words over a light backdrop, dark with light words over a dark
  /// one. The numbers are the ones measured on the system's regular glass.
  pub const fn regular() -> Self {
    Self {
      blur: 10.5,
      bend: 45.0,
      bezel: 19.5,
      dispersion: 0.0,
      saturation: 1.25,
      brightness: 0.1,
      tint: Color::new(255, 255, 255, 130),
      rim: 0.68,
      glow: 0.0,
      shadow: GlassShadow {
        radius: 32.0,
        y: 8.0,
        alpha: 0.12,
      },
      amount: 1.0,
      touch: None,
      ink: Some(Color::new(0, 0, 0, 217)),
      over_dark: Some(GlassLook {
        tint: Color::new(0, 0, 0, 97),
        brightness: 0.07,
        saturation: 1.1,
        rim: 0.18,
        ink: Color::new(255, 255, 255, 255),
      }),
      follows: GlassFollows::Backdrop,
      interactive: false,
    }
  }

  /// The regular glass of a large surface: a sheet, a panel, a sidebar. It
  /// is more of a wall than a button's glass is: whiter, with the colours
  /// behind it brought out, and it is light or dark by the app's appearance,
  /// not by what is behind it.
  pub const fn panel() -> Self {
    Self {
      saturation: 2.5,
      brightness: 0.0,
      tint: Color::new(255, 255, 255, 179),
      over_dark: Some(GlassLook {
        tint: Color::new(26, 26, 26, 181),
        brightness: 0.0,
        saturation: 2.5,
        rim: 0.18,
        ink: Color::new(255, 255, 255, 255),
      }),
      follows: GlassFollows::Appearance,
      ..Self::regular()
    }
  }

  /// A glass that leaves what is behind nearly sharp, for over pictures and
  /// video. It does not follow its backdrop and gives no colour to what is
  /// on it: words on it need a backdrop that is calm, or dimmed.
  pub const fn clear() -> Self {
    Self {
      blur: 3.8,
      saturation: 1.0,
      brightness: 0.085,
      tint: Color::new(255, 255, 255, 0),
      shadow: GlassShadow::NONE,
      ink: None,
      over_dark: None,
      ..Self::regular()
    }
  }

  /// The glass in this colour, laid over what is behind by its alpha: a
  /// coloured glass has that colour over anything, with white on it where
  /// the colour is dark and black where it is light.
  pub fn tint(mut self, tint: Color) -> Self {
    self.tint = tint;
    self.over_dark = None;
    let luma = 0.2126 * f32::from(tint.r()) + 0.7152 * f32::from(tint.g()) + 0.0722 * f32::from(tint.b());
    self.ink = Some(if luma < 150.0 {
      Color::new(255, 255, 255, 255)
    } else {
      Color::new(0, 0, 0, 217)
    });
    self
  }

  pub const fn blur(mut self, radius: f32) -> Self {
    self.blur = radius;
    self
  }

  /// How far inside itself the edge shows the backdrop from (see [`Self::bend`]).
  pub const fn bend(mut self, bend: f32) -> Self {
    self.bend = bend;
    self
  }

  pub const fn bezel(mut self, bezel: f32) -> Self {
    self.bezel = bezel;
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

  /// The colour of what is on the glass (see [`Self::ink`]).
  pub const fn ink(mut self, ink: Option<Color>) -> Self {
    self.ink = ink;
    self
  }

  /// What the glass looks like over a dark backdrop; `None` for the same
  /// look over anything (see [`Self::over_dark`]).
  pub const fn over_dark(mut self, look: Option<GlassLook>) -> Self {
    self.over_dark = look;
    self
  }

  /// Whether the glass answers a press (see [`Self::interactive`]).
  pub const fn interactive(mut self, interactive: bool) -> Self {
    self.interactive = interactive;
    self
  }

  /// What decides between the two looks (see [`GlassFollows`]).
  pub const fn follows(mut self, follows: GlassFollows) -> Self {
    self.follows = follows;
    self
  }

  /// The glass as it looks when what is behind it is `dark` of the way to
  /// dark (0 over a light backdrop, 1 over a dark one): its own look, its
  /// look [over dark](Self::over_dark), or between the two. The result
  /// follows nothing.
  pub fn toned(&self, dark: f32) -> Self {
    let Some(look) = self.over_dark else {
      return *self;
    };
    let dark = dark.clamp(0.0, 1.0);
    let mix = |a: f32, b: f32| a + (b - a) * dark;
    Self {
      tint: mix_color(self.tint, look.tint, dark),
      brightness: mix(self.brightness, look.brightness),
      saturation: mix(self.saturation, look.saturation),
      rim: mix(self.rim, look.rim),
      ink: Some(mix_color(self.ink.unwrap_or(look.ink), look.ink, dark)),
      over_dark: None,
      ..*self
    }
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
      bend: mix(self.bend, other.bend),
      bezel: mix(self.bezel, other.bezel),
      dispersion: mix(self.dispersion, other.dispersion),
      saturation: mix(self.saturation, other.saturation),
      brightness: mix(self.brightness, other.brightness),
      tint: mix_color(self.tint, other.tint, t),
      rim: mix(self.rim, other.rim),
      glow: mix(self.glow, other.glow),
      shadow: GlassShadow {
        radius: mix(self.shadow.radius, other.shadow.radius),
        y: mix(self.shadow.y, other.shadow.y),
        alpha: mix(self.shadow.alpha, other.shadow.alpha),
      },
      amount: mix(self.amount, other.amount),
      touch,
      ink: match (self.ink, other.ink) {
        (Some(a), Some(b)) => Some(mix_color(a, b, t)),
        (a, b) => {
          if t < 0.5 {
            a
          } else {
            b
          }
        }
      },
      over_dark: match (self.over_dark, other.over_dark) {
        (Some(a), Some(b)) => Some(GlassLook {
          tint: mix_color(a.tint, b.tint, t),
          brightness: mix(a.brightness, b.brightness),
          saturation: mix(a.saturation, b.saturation),
          rim: mix(a.rim, b.rim),
          ink: mix_color(a.ink, b.ink, t),
        }),
        (a, b) => {
          if t < 0.5 {
            a
          } else {
            b
          }
        }
      },
      follows: if t < 0.5 { self.follows } else { other.follows },
      interactive: if t < 0.5 { self.interactive } else { other.interactive },
    }
  }
}

/// The colour `t` of the way from `a` to `b`, channel by channel.
fn mix_color(a: Color, b: Color, t: f32) -> Color {
  let channel = |a: u8, b: u8| {
    (f32::from(a) + (f32::from(b) - f32::from(a)) * t)
      .round()
      .clamp(0.0, 255.0) as u8
  };
  Color::new(
    channel(a.r(), b.r()),
    channel(a.g(), b.g()),
    channel(a.b(), b.b()),
    channel(a.a(), b.a()),
  )
}

impl Default for Glass {
  fn default() -> Self {
    Self::regular()
  }
}

/// Where the light on a glass's rim comes from when nothing says otherwise:
/// straight above the screen, so that the rim is lit the same all the way
/// round. The vector points toward the light across the screen, y down, and
/// is as long as the light is off to that side, up to 1: the edge that
/// faces it is lit more and the one that faces away less.
pub const DEFAULT_GLASS_LIGHT: [f32; 2] = [0.0, 0.0];

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
  /// How far inward the very edge is read from.
  pub bend: f32,
  pub bezel: f32,
  pub dispersion: f32,
  pub saturation: f32,
  pub brightness: f32,
  /// The tint, premultiplied, 0 to 1.
  pub tint: [f32; 4],
  pub rim: f32,
  pub glow: f32,
  /// The shadow's radius, how far down it lies, and its alpha.
  pub shadow: [f32; 3],
  /// Toward the light across the screen (see [`DEFAULT_GLASS_LIGHT`]).
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
      bend: (glass.bend * scale * amount).max(0.0),
      bezel: (glass.bezel * scale).max(1.0),
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

  /// How far outside the glass a pixel of it may be read from: what the
  /// copy of the backdrop is grown by. The edge reads [`Self::bend`] inward,
  /// which is past the far side of a shape narrower than that. Where shapes
  /// flow together the way in is not straight across one of them, and the
  /// whole bend is allowed for.
  pub fn reach(&self) -> f32 {
    if self.bend <= 0.0 {
      return 0.0;
    }
    let bend = self.bend * (1.0 + self.dispersion);
    let across = match self.shapes() {
      [only] => 2.0 * only.half[0].min(only.half[1]) * only.scale,
      _ => 0.0,
    };
    let past = (bend - across).max(0.0);
    if past > 0.0 { past.ceil() + 1.0 } else { 0.0 }
  }

  /// How far outside the shapes the glass paints: its shadow.
  pub fn spread(&self) -> f32 {
    if self.shadow[2] <= 0.0 {
      return 0.0;
    }
    // The shadow is under a hundredth of its darkness three quarters of its radius out.
    (self.shadow[0] * 0.75 + self.shadow[1].abs()).ceil() + 1.0
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

/// How much of its bend the glass pulls the backdrop in by `u` of the way
/// in from its edge over the bezel: all of it at the edge (0), none where
/// the glass is flat (1 and on). The curve is the one the lens of the
/// system's glass was measured to have, with a step passed behind it in a
/// simulator: it falls fast at first, so that just inside the edge the
/// picture runs backwards.
pub fn lens_pull(u: f32) -> f32 {
  let u = u.clamp(0.0, 1.0);
  (1.0 - u).powf(LENS_POWER) * (-LENS_DECAY * u).exp()
}

/// The numbers of [`lens_pull`]; the shaders have the same.
pub(crate) const LENS_POWER: f32 = 1.1;
pub(crate) const LENS_DECAY: f32 = 2.0;

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
  /// Bend, bezel, dispersion, then nothing.
  pub optics: [f32; 4],
  /// Saturation, brightness, rim light, inner glow.
  pub look: [f32; 4],
  pub tint: [f32; 4],
  /// The shadow's radius, distance down and alpha, then nothing.
  pub shadow: [f32; 4],
  /// The vector toward the light, the width of the rim's light, then 1.
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
      optics: [glass.bend, glass.bezel, glass.dispersion, 0.0],
      look: [glass.saturation, glass.brightness, glass.rim, glass.glow],
      tint: glass.tint,
      shadow: [glass.shadow[0], glass.shadow[1], glass.shadow[2], 0.0],
      light: [glass.light[0], glass.light[1], rim, 1.0],
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

  #[test]
  fn a_glass_that_follows_its_backdrop_is_between_its_two_looks() {
    let glass = Glass::regular();
    let light = glass.toned(0.0);
    assert_eq!((light.tint, light.ink), (glass.tint, glass.ink));
    assert_eq!(light.over_dark, None);
    let dark = glass.toned(1.0);
    assert_eq!(dark.tint, Color::new(0, 0, 0, 97));
    assert_eq!(dark.ink, Some(Color::new(255, 255, 255, 255)));
    assert_eq!(dark.rim, 0.18);
    let between = glass.toned(0.5);
    assert_eq!(between.tint, Color::new(128, 128, 128, 114));
    // A glass that follows nothing is itself over anything.
    assert_eq!(Glass::clear().toned(1.0), Glass::clear());
    assert_eq!(Glass::panel().toned(1.0).tint, Color::new(26, 26, 26, 181));
    // A coloured glass keeps its colour, with white on it where it is dark.
    let blue = Glass::regular().tint(Color::new(0, 122, 255, 200));
    assert_eq!(blue.toned(1.0), blue);
    assert_eq!(blue.ink, Some(Color::new(255, 255, 255, 255)));
  }

  fn pill(x: f32, y: f32, width: f32, height: f32) -> RoundedBox {
    RoundedBox::rect(x, y, width, height, [height * 0.5; 4])
  }

  fn cmd(shapes: &[RoundedBox], merge: f32) -> GlassCmd {
    GlassCmd::new(&Glass::regular(), shapes, merge, 1.0, DEFAULT_GLASS_LIGHT, [0.0, 0.0])
  }

  #[test]
  fn a_shape_is_as_far_as_its_rounded_edge() {
    let glass = cmd(&[pill(10.0, 10.0, 100.0, 40.0)], 0.0);
    assert!(
      (glass.distance(60.0, 30.0) + 20.0).abs() < 1e-4,
      "the middle is half the height in"
    );
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
    assert!(
      (far.distance(41.0, 20.0) - 1.0).abs() < 1e-4,
      "and the edge is where it was"
    );
  }

  #[test]
  fn the_edge_pulls_in_and_the_middle_does_not() {
    assert_eq!(lens_pull(0.0), 1.0);
    assert_eq!(lens_pull(1.0), 0.0);
    assert_eq!(lens_pull(3.0), 0.0);
    // As measured on the system's glass: half the bend is gone a fifth of the way in.
    assert!((lens_pull(0.2) - 0.524).abs() < 0.01, "{}", lens_pull(0.2));
    assert!(lens_pull(0.5) < 0.18);
  }

  #[test]
  fn a_narrow_shape_is_read_past_its_far_side() {
    // 45 inward from the edge of a pill 40 high comes out 5 past it.
    assert_eq!(cmd(&[pill(0.0, 0.0, 100.0, 40.0)], 0.0).reach(), 6.0);
    assert_eq!(cmd(&[pill(0.0, 0.0, 100.0, 60.0)], 0.0).reach(), 0.0);
    // Shapes that flow together are read from anywhere within the bend.
    let two = cmd(&[pill(0.0, 0.0, 100.0, 60.0), pill(0.0, 70.0, 100.0, 60.0)], 20.0);
    assert_eq!(two.reach(), 46.0);
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
    assert_eq!(
      (glass.saturation, glass.brightness, glass.rim, glass.glow),
      (1.0, 0.0, 0.0, 0.0)
    );
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
    assert_eq!((glass.bend, glass.bezel), (135.0, 58.5));
    assert_eq!(glass.touch, [130.0, 215.0, 90.0, 0.5]);
    assert_eq!(glass.shadow, [96.0, 24.0, 0.12]);
  }

  #[test]
  fn two_glasses_meet_half_way() {
    let half = Glass::regular().lerp(&Glass::clear(), 0.5);
    assert_eq!(half.blur, 7.15);
    assert_eq!(half.tint, Color::new(255, 255, 255, 65));
    assert_eq!(half.amount, 1.0);
  }
}
