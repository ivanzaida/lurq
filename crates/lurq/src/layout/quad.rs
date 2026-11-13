use crate::{
  layout::{
    render_list::RenderGradient,
    text_style::{TextStyle, VerticalAlign},
  },
  node::{
    TextTransformMode,
    border::{BorderRadius, ResolvedBorders},
    color::Color,
    transform::Transform2D,
  },
};

/// How many rounded clips a [`ClipRect`] carries, which the shaders cut by.
pub const MAX_ROUNDED_CLIPS: usize = 2;

/// What content is cut by: every clip it is under, as nested
/// `overflow: hidden` boxes cut it in CSS.
///
/// The rectangle (`x`, `y`, `width`, `height`) is where all the clips above
/// the content overlap: what the scissor keeps and what culling and layer
/// bounds go by. A clip with rounded corners adds its shape to
/// [`Self::rounded`] with its own rectangle: a rail along the bottom edge of
/// a rounded panel is cut by the panel's round corners, which are not the
/// rail's. Only the shapes that cut something off the rectangle are kept.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ClipRect {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
  pub active: bool,
  /// The rounded clips the content is under that cut the rectangle,
  /// innermost first. Where more than [`MAX_ROUNDED_CLIPS`] would, the
  /// outermost are left out.
  pub rounded: [Option<RoundedClip>; MAX_ROUNDED_CLIPS],
}

/// A rectangle with rounded corners that cuts what is drawn under it: the
/// box of a clipping node with corner radii (see [`ClipRect::rounded`]).
/// The radii fit the rectangle (see [`BorderRadius::clamped_to_rect`]).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RoundedClip {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
  pub radius: BorderRadius,
}

/// How far, in pixels, a point may lie past a rounded clip and still count
/// as inside it when deciding whether the clip cuts anything: float error,
/// not a visible difference.
const ROUNDED_CLIP_SLACK: f32 = 0.01;

impl RoundedClip {
  /// Whether a corner is rounded at all.
  pub fn has_corners(&self) -> bool {
    let radius = self.radius;
    radius.top_left > 0.0 || radius.top_right > 0.0 || radius.bottom_right > 0.0 || radius.bottom_left > 0.0
  }

  /// Whether `(x, y)` is inside the rounded rectangle.
  pub fn contains(&self, x: f32, y: f32) -> bool {
    self.contains_within(x, y, 0.0)
  }

  fn contains_within(&self, x: f32, y: f32, slack: f32) -> bool {
    let right = self.x + self.width;
    let bottom = self.y + self.height;
    if x < self.x - slack || x > right + slack || y < self.y - slack || y > bottom + slack {
      return false;
    }
    let radius = self.radius;
    // Each corner: its radius, the centre of its arc, and whether the point is in the square the arc cuts.
    let top_left = (self.x + radius.top_left, self.y + radius.top_left);
    let top_right = (right - radius.top_right, self.y + radius.top_right);
    let bottom_right = (right - radius.bottom_right, bottom - radius.bottom_right);
    let bottom_left = (self.x + radius.bottom_left, bottom - radius.bottom_left);
    let corners = [
      (radius.top_left, top_left, x < top_left.0 && y < top_left.1),
      (radius.top_right, top_right, x > top_right.0 && y < top_right.1),
      (
        radius.bottom_right,
        bottom_right,
        x > bottom_right.0 && y > bottom_right.1,
      ),
      (radius.bottom_left, bottom_left, x < bottom_left.0 && y > bottom_left.1),
    ];
    for (radius, (cx, cy), in_square) in corners {
      if radius > 0.0 && in_square {
        let (dx, dy) = (x - cx, y - cy);
        return dx * dx + dy * dy <= (radius + slack) * (radius + slack);
      }
    }
    true
  }

  /// Whether the shape cuts anything off the rectangle from `(x1, y1)` to
  /// `(x2, y2)`. It is convex, so it does unless it holds all four corners.
  fn cuts(&self, x1: f32, y1: f32, x2: f32, y2: f32) -> bool {
    if x2 <= x1 || y2 <= y1 {
      return false;
    }
    ![(x1, y1), (x2, y1), (x2, y2), (x1, y2)]
      .into_iter()
      .all(|(x, y)| self.contains_within(x, y, ROUNDED_CLIP_SLACK))
  }

  #[cfg_attr(not(feature = "render"), allow(dead_code))]
  fn translated(self, dx: f32, dy: f32) -> Self {
    Self {
      x: self.x + dx,
      y: self.y + dy,
      ..self
    }
  }

  fn scaled(self, scale: f32) -> Self {
    let radius = self.radius;
    Self {
      x: self.x * scale,
      y: self.y * scale,
      width: self.width * scale,
      height: self.height * scale,
      radius: BorderRadius {
        top_left: radius.top_left * scale,
        top_right: radius.top_right * scale,
        bottom_right: radius.bottom_right * scale,
        bottom_left: radius.bottom_left * scale,
      },
    }
  }

  fn outset(self, left: f32, top: f32, right: f32, bottom: f32) -> Self {
    Self {
      x: self.x - left,
      y: self.y - top,
      width: self.width + left + right,
      height: self.height + top + bottom,
      radius: self.radius,
    }
  }
}

impl ClipRect {
  /// A rectangular clip.
  pub const fn rect(x: f32, y: f32, width: f32, height: f32) -> Self {
    Self {
      x,
      y,
      width,
      height,
      active: true,
      rounded: [None; MAX_ROUNDED_CLIPS],
    }
  }

  /// The clip of a box with corner radii, which must fit it (see
  /// [`BorderRadius::clamped_to_rect`]). Without a rounded corner it is
  /// [`Self::rect`].
  pub fn rounded_rect(x: f32, y: f32, width: f32, height: f32, radius: Option<BorderRadius>) -> Self {
    let mut clip = Self::rect(x, y, width, height);
    clip.rounded[0] = radius
      .map(|radius| RoundedClip {
        x,
        y,
        width,
        height,
        radius,
      })
      .filter(RoundedClip::has_corners);
    clip
  }

  /// The rounded clips that cut the clip's rectangle, innermost first.
  pub fn rounded_clips(&self) -> impl Iterator<Item = &RoundedClip> {
    self.rounded.iter().flatten()
  }

  /// Whether a rounded clip cuts the content, which the shaders then cut,
  /// not only the scissor.
  pub fn is_rounded(&self) -> bool {
    self.active && self.rounded[0].is_some()
  }

  /// Whether `(x, y)` is inside the clip, its rounded corners included.
  pub fn contains(&self, x: f32, y: f32) -> bool {
    !self.active
      || (x >= self.x
        && y >= self.y
        && x <= self.x + self.width
        && y <= self.y + self.height
        && self.rounded_clips().all(|rounded| rounded.contains(x, y)))
  }

  /// The clip of content under both `self` and the `inner` clip inside it:
  /// the rectangles intersected, and the rounded clips of both that cut the
  /// result, `inner`'s first.
  pub fn intersect(self, inner: ClipRect) -> ClipRect {
    if !self.active {
      return inner;
    }
    if !inner.active {
      return self;
    }
    let x1 = self.x.max(inner.x);
    let y1 = self.y.max(inner.y);
    let x2 = (self.x + self.width).min(inner.x + inner.width);
    let y2 = (self.y + self.height).min(inner.y + inner.height);
    let mut clip = ClipRect::rect(x1, y1, (x2 - x1).max(0.0), (y2 - y1).max(0.0));
    // A rectangle under rectangles, the common case, ends here.
    if self.rounded[0].is_none() && inner.rounded[0].is_none() {
      return clip;
    }
    let mut kept = 0;
    for rounded in inner.rounded_clips().chain(self.rounded_clips()) {
      if kept == MAX_ROUNDED_CLIPS {
        break;
      }
      if rounded.cuts(x1, y1, x2, y2) && !clip.rounded[..kept].contains(&Some(*rounded)) {
        clip.rounded[kept] = Some(*rounded);
        kept += 1;
      }
    }
    clip
  }

  /// Moved by `(dx, dy)`, its rounded clips with it.
  #[cfg_attr(not(feature = "render"), allow(dead_code))]
  pub(crate) fn translated(self, dx: f32, dy: f32) -> Self {
    Self {
      x: self.x + dx,
      y: self.y + dy,
      rounded: self
        .rounded
        .map(|rounded| rounded.map(|rounded| rounded.translated(dx, dy))),
      ..self
    }
  }

  /// In pixels `scale` times as small, its rounded clips with it.
  pub(crate) fn scaled(self, scale: f32) -> Self {
    if !self.active {
      return self;
    }
    Self {
      x: self.x * scale,
      y: self.y * scale,
      width: self.width * scale,
      height: self.height * scale,
      active: true,
      rounded: self.rounded.map(|rounded| rounded.map(|rounded| rounded.scaled(scale))),
    }
  }

  /// Let out by the given amounts on each side, its rounded clips by the
  /// same with their corners kept: slop around the rectangle is not cut by
  /// the straight edges of a rounded clip either.
  pub(crate) fn outset(self, left: f32, top: f32, right: f32, bottom: f32) -> Self {
    if !self.active {
      return self;
    }
    Self {
      x: self.x - left,
      y: self.y - top,
      width: self.width + left + right,
      height: self.height + top + bottom,
      active: true,
      rounded: self
        .rounded
        .map(|rounded| rounded.map(|rounded| rounded.outset(left, top, right, bottom))),
    }
  }

  /// Maps a clip laid out in a subtree's layout coordinates into the screen
  /// space its transformed quads are painted in (clips are tested against
  /// fragment positions).
  ///
  /// Axis-aligned transforms (scale, flip, translation) map exactly; corner
  /// radii follow flips and scale by the smaller axis scale. Other transforms
  /// clip to the bounding box of the transformed rect without radii, which never
  /// cuts content inside the transformed box.
  pub(crate) fn transformed(self, transform: Transform2D) -> Self {
    if !self.active || transform.is_identity() {
      return self;
    }
    let [x, y, width, height] = transformed_bounds(self.x, self.y, self.width, self.height, transform);
    let rounded = if transform.is_axis_aligned() {
      self.rounded.map(|rounded| {
        rounded.map(|rounded| {
          let [x, y, width, height] =
            transformed_bounds(rounded.x, rounded.y, rounded.width, rounded.height, transform);
          RoundedClip {
            x,
            y,
            width,
            height,
            radius: axis_aligned_radius(rounded.radius, transform),
          }
        })
      })
    } else {
      [None; MAX_ROUNDED_CLIPS]
    };
    Self {
      x,
      y,
      width,
      height,
      active: true,
      rounded,
    }
  }
}

/// The bounding box of a rect mapped through `transform`: x, y, width, height.
fn transformed_bounds(x: f32, y: f32, width: f32, height: f32, transform: Transform2D) -> [f32; 4] {
  let corners =
    [(x, y), (x + width, y), (x, y + height), (x + width, y + height)].map(|(x, y)| transform.transform_point(x, y));
  let min_x = corners.iter().map(|corner| corner.0).fold(f32::INFINITY, f32::min);
  let min_y = corners.iter().map(|corner| corner.1).fold(f32::INFINITY, f32::min);
  let max_x = corners.iter().map(|corner| corner.0).fold(f32::NEG_INFINITY, f32::max);
  let max_y = corners.iter().map(|corner| corner.1).fold(f32::NEG_INFINITY, f32::max);
  [min_x, min_y, max_x - min_x, max_y - min_y]
}

fn axis_aligned_radius(radius: BorderRadius, transform: Transform2D) -> BorderRadius {
  let scale = transform.a.abs().min(transform.d.abs());
  let (flip_x, flip_y) = (transform.a < 0.0, transform.d < 0.0);
  let corner = |top: bool, left: bool| {
    let value = match (top != flip_y, left != flip_x) {
      (true, true) => radius.top_left,
      (true, false) => radius.top_right,
      (false, false) => radius.bottom_right,
      (false, true) => radius.bottom_left,
    };
    value * scale
  };
  BorderRadius {
    top_left: corner(true, true),
    top_right: corner(true, false),
    bottom_right: corner(false, false),
    bottom_left: corner(false, true),
  }
}

pub struct Quad {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
  /// Opacity within the quad's opacity layer. A node with `opacity < 1`
  /// fades its subtree as one layer (`RenderList::layers`), so its quads
  /// keep `1.0` here, unless the node paints a single primitive, which takes
  /// the node's opacity directly (see [`crate::layout::opacity_layer`]).
  pub opacity: f32,
  pub transform: Transform2D,
  pub transform_origin: Option<[f32; 2]>,
  pub content: QuadContent,
  pub border_radius: Option<BorderRadius>,
  pub border: Option<ResolvedBorders>,
  pub clip: ClipRect,
}

#[doc(hidden)]
pub use crate::app::glyph_engine::input_tokens::DecoratedInputLayout;

pub enum QuadContent {
  #[doc(hidden)]
  DecoratedInput {
    layout: std::sync::Arc<DecoratedInputLayout>,
  },
  Rect {
    color: Color,
    gradient: Option<RenderGradient>,
  },
  /// Blurs what is painted behind the quad's box by this radius, in logical
  /// pixels, before the quads after it are painted. Paints nothing itself;
  /// the quad's opacity is how much of the blurred backdrop shows. See
  /// [`crate::layout::backdrop`].
  Backdrop {
    blur: f32,
  },
  Text {
    text: String,
    style: TextStyle,
    wrap: bool,
    vertical_align: VerticalAlign,
    transform_mode: TextTransformMode,
  },
  RichText {
    spans: Vec<RichTextSpan>,
    wrap: bool,
    vertical_align: VerticalAlign,
    transform_mode: TextTransformMode,
  },
  #[cfg(feature = "raster")]
  Image {
    data: crate::images::ImageData,
    uv_min: [f32; 2],
    uv_max: [f32; 2],
  },
  #[cfg(feature = "raster")]
  Video {
    data: crate::images::ImageData,
    uv_min: [f32; 2],
    uv_max: [f32; 2],
  },
  #[cfg(feature = "svg")]
  Svg {
    data: crate::svg::SvgData,
  },
  /// A box shadow cast by the quad's box (its `border_radius` included). For
  /// an inset shadow the box is the element's padding box.
  BoxShadow(crate::node::box_shadow::ResolvedBoxShadow),
  None,
}

#[derive(Clone, PartialEq)]
pub struct RichTextSpan {
  pub text: String,
  pub style: TextStyle,
}

#[cfg(test)]
mod tests {
  use super::{ClipRect, RoundedClip};
  use crate::node::border::BorderRadius;

  fn rounded(x: f32, y: f32, width: f32, height: f32, radius: f32) -> ClipRect {
    ClipRect::rounded_rect(x, y, width, height, Some(BorderRadius::all(radius)))
  }

  fn shape(clip: ClipRect) -> RoundedClip {
    clip.rounded[0].unwrap()
  }

  #[test]
  fn a_rectangle_inside_a_rounded_clip_keeps_the_corners_only_where_they_cut_it() {
    let panel = rounded(0.0, 0.0, 200.0, 100.0, 8.0);
    let rail = panel.intersect(ClipRect::rect(0.0, 98.0, 200.0, 2.0));
    assert_eq!([rail.x, rail.y, rail.width, rail.height], [0.0, 98.0, 200.0, 2.0]);
    assert_eq!(rail.rounded, [Some(shape(panel)), None]);

    let middle = panel.intersect(ClipRect::rect(0.0, 40.0, 200.0, 2.0));
    assert!(!middle.is_rounded());
    // Ending exactly where a corner's arc starts cuts nothing either.
    assert!(!panel.intersect(ClipRect::rect(8.0, 0.0, 184.0, 100.0)).is_rounded());
    assert!(panel.intersect(ClipRect::rect(7.0, 0.0, 186.0, 100.0)).is_rounded());
  }

  #[test]
  fn nested_rounded_clips_keep_the_innermost_two_that_cut() {
    let outer = rounded(0.0, 0.0, 300.0, 300.0, 40.0);
    let middle = rounded(0.0, 0.0, 200.0, 200.0, 20.0);
    let inner = rounded(0.0, 0.0, 100.0, 100.0, 10.0);
    let clip = outer.intersect(middle).intersect(inner);
    assert_eq!(clip.rounded, [Some(shape(inner)), Some(shape(middle))]);

    // The same clip twice is kept once.
    let twice = middle.intersect(middle);
    assert_eq!(twice.rounded, [Some(shape(middle)), None]);

    // A clip without rounded corners lets the ones around it through.
    let rect = middle.intersect(ClipRect::rect(0.0, 0.0, 50.0, 50.0));
    assert_eq!(rect.rounded, [Some(shape(middle)), None]);
    assert!(ClipRect::rect(0.0, 0.0, 50.0, 50.0).intersect(middle).is_rounded());
  }

  #[test]
  fn a_point_is_in_a_clip_when_it_is_in_every_clip() {
    let clip = rounded(0.0, 0.0, 100.0, 100.0, 20.0).intersect(rounded(0.0, 0.0, 50.0, 50.0, 4.0));
    assert!(clip.contains(25.0, 25.0));
    // In the inner clip's square corner, outside the outer's round one.
    assert!(!clip.contains(3.0, 3.0));
    // Outside the inner clip's corner, inside the outer.
    assert!(!clip.contains(49.5, 49.5));
    assert!(!clip.contains(60.0, 10.0));
    assert!(ClipRect::default().contains(-1000.0, 1000.0));
  }

  #[test]
  fn moving_scaling_and_letting_out_take_the_rounded_clips_along() {
    let clip = rounded(0.0, 0.0, 100.0, 50.0, 10.0).intersect(ClipRect::rect(0.0, 40.0, 100.0, 10.0));
    let moved = clip.translated(5.0, -5.0);
    assert_eq!((moved.y, shape(moved).y, shape(moved).x), (35.0, -5.0, 5.0));
    let scaled = clip.scaled(2.0);
    assert_eq!(
      (scaled.height, shape(scaled).height, shape(scaled).radius),
      (20.0, 100.0, BorderRadius::all(20.0))
    );
    let let_out = clip.outset(1.0, 1.0, 1.0, 1.0);
    assert_eq!(
      (let_out.height, shape(let_out).width, shape(let_out).radius),
      (12.0, 102.0, BorderRadius::all(10.0))
    );
  }
}
