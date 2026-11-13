use std::sync::Arc;

use crate::{
  app::theme::{ThemeBorderSizes, ThemePalette, ThemeShadows},
  node::{
    box_shadow::ResolvedBoxShadow,
    color::Color,
    outline::ResolvedOutline,
    transform::{Decomposed, Transform2D, decompose},
  },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AnimatableProperty {
  BackgroundColor,
  BorderColor,
  BorderWidthTop,
  BorderWidthRight,
  BorderWidthBottom,
  BorderWidthLeft,
  BorderRadiusTopLeft,
  BorderRadiusTopRight,
  BorderRadiusBottomRight,
  BorderRadiusBottomLeft,
  OffsetX,
  OffsetY,
  Width,
  Height,
  Opacity,
  /// The node's blur radius (`Node::blur`), in logical pixels.
  Blur,
  /// The blur radius of the node's backdrop (`Node::backdrop_blur`), in
  /// logical pixels.
  BackdropBlur,
  Transform,
  /// The color of the text in the node and under it (`Node::foreground`).
  Foreground,
  /// The node's outline (`Node::outline`): its width, offset and color as
  /// one value, so a ring grows in from nothing and fades back to it.
  Outline,
  /// The node's box shadows (`Node::box_shadow`).
  BoxShadow,
  /// The dashes of the node's border (`Border::dash`): the length of a dash and of the gap after it, as one value.
  BorderDash,
}

macro_rules! property_accessors {
  ($target:ident { $($variant:ident => $method:ident),* $(,)? }) => {
    impl $target {
      $(
        pub fn $method() -> Self {
          Self::single(AnimatableProperty::$variant)
        }
      )*
    }
  };
}

pub(crate) use property_accessors;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AnimatableValue {
  Color(Color),
  Float(f32),
  Transform(Decomposed),
  /// An outline, or none.
  Outline(Option<ResolvedOutline>),
  /// A list of box shadows.
  Shadows(ShadowList),
  /// A height of `px` and `content` times the height the element's content
  /// gives it: what a height the content decides (no height, or `Auto`) is to
  /// a transition, as `Fit { px: 0.0, content: 1.0 }`. A transition between it
  /// and a height in pixels mixes the two, and the layout reads the content's
  /// height as it lays the element out, so an element opens to, or closes
  /// from, the height of what is in it.
  Fit {
    px: f32,
    content: f32,
  },
  /// A width or a height of a share of the parent's, in percent: what `Dimension::Pct` is to a transition or a
  /// keyframe. It moves to another share, as a bar filled to a value does; to and from pixels it changes at once.
  Percent(f32),
  /// The length of a border's dash and of the gap after it, in logical pixels: both 0 for a solid line.
  Dash {
    dash: f32,
    gap: f32,
  },
}

/// The box shadows of a node as one animated value: at most
/// [`ShadowList::MAX`] of them, with their colors resolved, first on top.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowList {
  len: u8,
  shadows: [ResolvedBoxShadow; ShadowList::MAX],
}

const NO_SHADOW: ResolvedBoxShadow = ResolvedBoxShadow {
  offset_x: 0.0,
  offset_y: 0.0,
  blur: 0.0,
  spread: 0.0,
  color: Color::new(0, 0, 0, 0),
  inset: false,
};

impl ShadowList {
  /// How many shadows a list that animates holds. A node whose shadows are more than these changes them at once.
  pub const MAX: usize = 4;

  pub fn new(shadows: &[ResolvedBoxShadow]) -> Self {
    let mut list = [NO_SHADOW; Self::MAX];
    let len = shadows.len().min(Self::MAX);
    list[..len].copy_from_slice(&shadows[..len]);
    Self {
      len: len as u8,
      shadows: list,
    }
  }

  pub fn as_slice(&self) -> &[ResolvedBoxShadow] {
    &self.shadows[..usize::from(self.len)]
  }

  /// Shadow by shadow, as CSS does: one with no partner comes from, or goes
  /// to, nothing where it is. A shadow inside the box cannot turn into one
  /// outside it by degrees, so such a list changes at once, halfway.
  fn lerp(&self, to: &Self, t: f32) -> Self {
    let (from, to_list) = (self.as_slice(), to.as_slice());
    let len = from.len().max(to_list.len());
    let turns =
      (0..len).any(|index| matches!((from.get(index), to_list.get(index)), (Some(a), Some(b)) if a.inset != b.inset));
    if turns {
      return if t >= 0.5 { *to } else { *self };
    }
    let nothing_of = |shadow: &ResolvedBoxShadow| ResolvedBoxShadow {
      color: Color::new(shadow.color.r(), shadow.color.g(), shadow.color.b(), 0),
      inset: shadow.inset,
      ..NO_SHADOW
    };
    let mut shadows = [NO_SHADOW; Self::MAX];
    for (index, slot) in shadows.iter_mut().enumerate().take(len) {
      let (a, b) = match (from.get(index), to_list.get(index)) {
        (Some(a), Some(b)) => (*a, *b),
        (Some(a), None) => (*a, nothing_of(a)),
        (None, Some(b)) => (nothing_of(b), *b),
        (None, None) => continue,
      };
      let mix = |a: f32, b: f32| a + (b - a) * t;
      *slot = ResolvedBoxShadow {
        offset_x: mix(a.offset_x, b.offset_x),
        offset_y: mix(a.offset_y, b.offset_y),
        blur: mix(a.blur, b.blur).max(0.0),
        spread: mix(a.spread, b.spread),
        color: lerp_color(a.color, b.color, t),
        inset: b.inset,
      };
    }
    Self {
      len: len as u8,
      shadows,
    }
  }
}

/// Between two outlines every value moves; to and from no outline the line
/// keeps its offset and color and its width and opacity go to nothing.
fn lerp_outline(from: Option<ResolvedOutline>, to: Option<ResolvedOutline>, t: f32) -> Option<ResolvedOutline> {
  let nothing_of = |outline: ResolvedOutline| ResolvedOutline {
    width: 0.0,
    color: Color::new(outline.color.r(), outline.color.g(), outline.color.b(), 0),
    offset: outline.offset,
  };
  let (a, b) = match (from, to) {
    (Some(a), Some(b)) => (a, b),
    (Some(a), None) => (a, nothing_of(a)),
    (None, Some(b)) => (nothing_of(b), b),
    (None, None) => return None,
  };
  Some(ResolvedOutline {
    width: (a.width + (b.width - a.width) * t).max(0.0),
    color: lerp_color(a.color, b.color, t),
    offset: a.offset + (b.offset - a.offset) * t,
  })
}

/// What reading the value a node is heading to needs of the theme: the roles
/// its colors, widths and shadows may be given as.
#[derive(Clone, Default)]
pub(crate) struct TargetTheme {
  pub(crate) palette: ThemePalette,
  pub(crate) border_sizes: ThemeBorderSizes,
  pub(crate) shadows: Arc<ThemeShadows>,
}

impl AnimatableValue {
  pub fn lerp(&self, to: &AnimatableValue, t: f32) -> AnimatableValue {
    match (self, to) {
      (AnimatableValue::Color(a), AnimatableValue::Color(b)) => AnimatableValue::Color(lerp_color(*a, *b, t)),
      (AnimatableValue::Float(a), AnimatableValue::Float(b)) => AnimatableValue::Float(a + (b - a) * t),
      (AnimatableValue::Transform(a), AnimatableValue::Transform(b)) => AnimatableValue::Transform(a.lerp(b, t)),
      (AnimatableValue::Outline(a), AnimatableValue::Outline(b)) => AnimatableValue::Outline(lerp_outline(*a, *b, t)),
      (AnimatableValue::Shadows(a), AnimatableValue::Shadows(b)) => AnimatableValue::Shadows(a.lerp(b, t)),
      (AnimatableValue::Percent(a), AnimatableValue::Percent(b)) => AnimatableValue::Percent(a + (b - a) * t),
      (AnimatableValue::Dash { dash: a, gap: a_gap }, AnimatableValue::Dash { dash: b, gap: b_gap }) => {
        AnimatableValue::Dash {
          dash: a + (b - a) * t,
          gap: a_gap + (b_gap - a_gap) * t,
        }
      }
      (
        AnimatableValue::Float(_) | AnimatableValue::Fit { .. },
        AnimatableValue::Float(_) | AnimatableValue::Fit { .. },
      ) => {
        let ((a_px, a_content), (b_px, b_content)) = (self.as_fit(), to.as_fit());
        AnimatableValue::Fit {
          px: a_px + (b_px - a_px) * t,
          content: a_content + (b_content - a_content) * t,
        }
      }
      _ => {
        if t >= 0.5 {
          *to
        } else {
          *self
        }
      }
    }
  }
}

impl AnimatableValue {
  /// A height in pixels or one the content decides as its pixels and its share of the content's height.
  fn as_fit(&self) -> (f32, f32) {
    match *self {
      AnimatableValue::Float(px) => (px, 0.0),
      AnimatableValue::Fit { px, content } => (px, content),
      _ => (0.0, 0.0),
    }
  }
}

impl From<Color> for AnimatableValue {
  fn from(c: Color) -> Self {
    Self::Color(c)
  }
}

impl From<f32> for AnimatableValue {
  fn from(v: f32) -> Self {
    Self::Float(v)
  }
}

impl From<Transform2D> for AnimatableValue {
  fn from(t: Transform2D) -> Self {
    Self::Transform(decompose(&t).unwrap_or(Decomposed {
      translate_x: 0.0,
      translate_y: 0.0,
      scale_x: 1.0,
      scale_y: 1.0,
      rotate: 0.0,
      skew_x: 0.0,
    }))
  }
}

impl From<Decomposed> for AnimatableValue {
  fn from(d: Decomposed) -> Self {
    Self::Transform(d)
  }
}

fn lerp_color(a: Color, b: Color, t: f32) -> Color {
  let ar = a.r() as f32;
  let ag = a.g() as f32;
  let ab = a.b() as f32;
  let aa = a.a() as f32;
  let br = b.r() as f32;
  let bg = b.g() as f32;
  let bb = b.b() as f32;
  let ba = b.a() as f32;
  Color::new(
    (ar + (br - ar) * t).round() as u8,
    (ag + (bg - ag) * t).round() as u8,
    (ab + (bb - ab) * t).round() as u8,
    (aa + (ba - aa) * t).round() as u8,
  )
}

/// The value of `prop` the node is heading to, with colors, widths and
/// shadows given as theme roles resolved against `theme`.
pub(crate) fn read_target(
  node: &crate::node::Node,
  prop: AnimatableProperty,
  theme: &TargetTheme,
) -> Option<AnimatableValue> {
  // These read one value of the winning state style and build no merged style.
  match prop {
    AnimatableProperty::Foreground => {
      return node
        .effective_foreground()
        .and_then(|color| color.resolve(&theme.palette))
        .map(AnimatableValue::Color);
    }
    AnimatableProperty::Outline => {
      return Some(AnimatableValue::Outline(
        node
          .effective_outline()
          .and_then(|outline| outline.resolve(&theme.palette, &theme.border_sizes)),
      ));
    }
    AnimatableProperty::BoxShadow => {
      let shadows: Vec<_> = node.effective_box_shadow().map_or_else(Vec::new, |value| {
        value
          .shadows(&theme.shadows)
          .iter()
          .filter_map(|shadow| shadow.resolve(&theme.palette))
          .collect()
      });
      // A list too long to animate whole changes at once, rather than lose shadows on the way.
      return (shadows.len() <= ShadowList::MAX).then(|| AnimatableValue::Shadows(ShadowList::new(&shadows)));
    }
    _ => {}
  }
  let style = node.target_style();
  match prop {
    AnimatableProperty::Foreground | AnimatableProperty::Outline | AnimatableProperty::BoxShadow => None,
    AnimatableProperty::BackgroundColor => style
      .color
      .or_else(|| <Option<_> as Clone>::clone(&node.color))
      .and_then(|color| color.resolve(&theme.palette))
      .map(AnimatableValue::Color),
    AnimatableProperty::BorderColor => style
      .border
      .or_else(|| node.border.as_deref().cloned())
      .and_then(|b| b.color())
      .and_then(|color| color.resolve(&theme.palette))
      .map(AnimatableValue::Color),
    // The dashes of the first side that has a border: a border dashed on every side is dashed alike.
    AnimatableProperty::BorderDash => style
      .border
      .or_else(|| node.border.as_deref().cloned())
      .and_then(|borders| borders.top.or(borders.right).or(borders.bottom).or(borders.left))
      .map(|border| {
        let [dash, gap] = match border.dash {
          // As given, a gap of nothing too: a dash as long as the outline is a solid line that moves to a shorter
          // dash as one dash, where its resolved lengths (none, for a solid line) would cut it into many.
          Some(crate::node::border::BorderDash::Lengths { dash, gap }) => [dash, gap],
          Some(dash) => dash.resolve(border.width.resolve(&theme.border_sizes)),
          None => [0.0; 2],
        };
        AnimatableValue::Dash { dash, gap }
      }),
    AnimatableProperty::BorderWidthTop => style
      .border
      .or_else(|| node.border.as_deref().cloned())
      .and_then(|b| b.top_width())
      .and_then(|width| width.as_px())
      .map(AnimatableValue::Float),
    AnimatableProperty::BorderWidthRight => style
      .border
      .or_else(|| node.border.as_deref().cloned())
      .and_then(|b| b.right_width())
      .and_then(|width| width.as_px())
      .map(AnimatableValue::Float),
    AnimatableProperty::BorderWidthBottom => style
      .border
      .or_else(|| node.border.as_deref().cloned())
      .and_then(|b| b.bottom_width())
      .and_then(|width| width.as_px())
      .map(AnimatableValue::Float),
    AnimatableProperty::BorderWidthLeft => style
      .border
      .or_else(|| node.border.as_deref().cloned())
      .and_then(|b| b.left_width())
      .and_then(|width| width.as_px())
      .map(AnimatableValue::Float),
    AnimatableProperty::BorderRadiusTopLeft => style
      .border_radius
      .or(*node.border_radius)
      .and_then(|r| r.top_left.as_px())
      .map(AnimatableValue::Float),
    AnimatableProperty::BorderRadiusTopRight => style
      .border_radius
      .or(*node.border_radius)
      .and_then(|r| r.top_right.as_px())
      .map(AnimatableValue::Float),
    AnimatableProperty::BorderRadiusBottomRight => style
      .border_radius
      .or(*node.border_radius)
      .and_then(|r| r.bottom_right.as_px())
      .map(AnimatableValue::Float),
    AnimatableProperty::BorderRadiusBottomLeft => style
      .border_radius
      .or(*node.border_radius)
      .and_then(|r| r.bottom_left.as_px())
      .map(AnimatableValue::Float),
    AnimatableProperty::OffsetX => read_offset_x(node).map(AnimatableValue::Float),
    AnimatableProperty::OffsetY => read_offset_y(node).map(AnimatableValue::Float),
    AnimatableProperty::Width => read_target_frame_dim(node, &style, true),
    // A height the content decides is one too, so that an element opens to it and closes from it.
    AnimatableProperty::Height => Some(read_target_height(node, &style)),
    AnimatableProperty::Opacity => Some(AnimatableValue::Float(style.opacity.unwrap_or(node.opacity))),
    AnimatableProperty::Blur => Some(AnimatableValue::Float(node.blur)),
    AnimatableProperty::BackdropBlur => Some(AnimatableValue::Float(node.backdrop_blur)),
    AnimatableProperty::Transform => decompose(&node.transform).map(AnimatableValue::Transform),
  }
}

pub(crate) fn write_property(node: &mut crate::node::Node, prop: AnimatableProperty, value: &AnimatableValue) -> bool {
  let affects_layout = matches!(
    prop,
    AnimatableProperty::OffsetX | AnimatableProperty::OffsetY | AnimatableProperty::Width | AnimatableProperty::Height
  );
  // Every property goes to the overrides, which each tick clears, and never into the node's own fields: those
  // hold what the render set, which `read_target` must keep seeing.
  if let Some(pos) = node.animation_overrides.iter().position(|(p, _)| *p == prop) {
    node.animation_overrides[pos].1 = *value;
  } else {
    node.animation_overrides.push((prop, *value));
  }
  affects_layout
}

pub(crate) fn clear_overrides(node: &mut crate::node::Node) {
  if !node.prepare_layout_tracking().timeline {
    return;
  }
  #[cfg(test)]
  crate::core::layout_dirty::visit();
  // Writing through the lazy box would allocate it for a node holding none.
  if !node.animation_overrides.is_empty() {
    node.animation_overrides.clear();
  }
  for child in &mut node.children {
    clear_overrides(child);
  }
}

fn read_offset_x(node: &crate::node::Node) -> Option<f32> {
  node.offset.map(|offset| offset.x)
}

fn read_offset_y(node: &crate::node::Node) -> Option<f32> {
  node.offset.map(|offset| offset.y)
}

/// The height `node` is heading to: in pixels where it is given one, else the height of its content.
fn read_target_height(node: &crate::node::Node, style: &crate::node::style::Style) -> AnimatableValue {
  let height = match style.frame {
    Some(overlay) => crate::node::node::merge_frame(node.frame, overlay).height,
    None => node.frame.height,
  };
  match height {
    None | Some(crate::node::dimension::Dimension::Auto) => AnimatableValue::Fit { px: 0.0, content: 1.0 },
    Some(crate::node::dimension::Dimension::Pct(share)) => AnimatableValue::Percent(share),
    Some(height) => AnimatableValue::Float(height.to_px()),
  }
}

/// The width (or height) `node` is heading to: in pixels, or a share of its parent's.
fn read_target_frame_dim(
  node: &crate::node::Node,
  style: &crate::node::style::Style,
  is_width: bool,
) -> Option<AnimatableValue> {
  let base = node.frame;
  if base == crate::layout::layout_kind::FrameConstraints::default() && style.frame.is_none() {
    return None;
  }
  let effective = match style.frame {
    Some(overlay) => crate::node::node::merge_frame(base, overlay),
    None => base,
  };
  let dimension = if is_width { effective.width } else { effective.height };
  dimension.map(|dimension| match dimension {
    crate::node::dimension::Dimension::Pct(share) => AnimatableValue::Percent(share),
    dimension => AnimatableValue::Float(dimension.to_px()),
  })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn lerp_color_midpoint() {
    let a = Color::new(0, 0, 0, 255);
    let b = Color::new(255, 255, 255, 255);
    let mid = lerp_color(a, b, 0.5);
    assert_eq!(mid.r(), 128);
    assert_eq!(mid.g(), 128);
    assert_eq!(mid.b(), 128);
  }

  #[test]
  fn lerp_float() {
    let a = AnimatableValue::Float(0.0);
    let b = AnimatableValue::Float(100.0);
    assert_eq!(a.lerp(&b, 0.75), AnimatableValue::Float(75.0));
  }

  #[test]
  fn lerp_percent() {
    let a = AnimatableValue::Percent(20.0);
    let b = AnimatableValue::Percent(60.0);
    assert_eq!(a.lerp(&b, 0.5), AnimatableValue::Percent(40.0));
    // A share and pixels are not mixed: the value changes at once, halfway.
    assert_eq!(a.lerp(&AnimatableValue::Float(10.0), 0.3), a);
  }

  #[test]
  fn lerp_dash() {
    let a = AnimatableValue::Dash { dash: 10.0, gap: 90.0 };
    let b = AnimatableValue::Dash { dash: 70.0, gap: 30.0 };
    assert_eq!(a.lerp(&b, 0.5), AnimatableValue::Dash { dash: 40.0, gap: 60.0 });
    assert_eq!(a.lerp(&b, 0.0), a);
    assert_eq!(a.lerp(&b, 1.0), b);
    // The sum stays: a dash cut to fit an outline stays one dash of it on the way.
    let AnimatableValue::Dash { dash, gap } = a.lerp(&b, 0.3) else {
      panic!("a dash")
    };
    assert!((dash + gap - 100.0).abs() < 1e-4);
  }

  #[test]
  fn lerp_mismatched_types_discrete() {
    let a = AnimatableValue::Float(10.0);
    let b = AnimatableValue::Color(Color::new(255, 0, 0, 255));
    assert_eq!(a.lerp(&b, 0.3), a);
    assert_eq!(a.lerp(&b, 0.7), b);
  }
}
