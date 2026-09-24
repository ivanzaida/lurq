//! Box-shadow quads. A shadow is a quad of its own in paint order: outer
//! shadows right before the element's background, inset shadows right after
//! it (before the border and the content), each list back to front so the
//! first shadow ends on top, as in CSS. The quads inherit the element's clip,
//! transform and opacity, and never take part in layout or hit testing.

use super::{LayoutEngine, transformed_quad_frame};
use crate::{
  app::theme::ThemeShadows,
  layout::{
    layout_result::LayoutResult,
    quad::{ClipRect, Quad, QuadContent},
  },
  node::{
    border::{BorderPlacement, BorderRadius, ResolvedBorder},
    node::Node,
    transform::Transform2D,
  },
};

impl LayoutEngine {
  /// Replaces the shadow table [`ShadowStyle`](crate::app::theme::ShadowStyle)
  /// roles resolve against. Shared, so a pass does not copy it.
  pub(crate) fn set_shadows(&self, shadows: std::sync::Arc<ThemeShadows>) {
    *self.shadows.borrow_mut() = shadows;
  }

  pub(super) fn has_inset_box_shadow(&self, node: &Node) -> bool {
    node
      .effective_box_shadow()
      .is_some_and(|value| value.shadows(&self.shadows.borrow()).iter().any(|shadow| shadow.inset))
  }

  /// How far the node's outer shadows can paint outside its layout box, so
  /// culling keeps a node whose shadow is visible although its box is not.
  #[inline(never)]
  pub(super) fn box_shadow_outset(&self, node: &Node) -> f32 {
    let Some(value) = node.effective_box_shadow() else {
      return 0.0;
    };
    let palette = self.palette.borrow();
    value
      .shadows(&self.shadows.borrow())
      .iter()
      .filter_map(|shadow| shadow.resolve(&palette))
      .map(|shadow| shadow.outset())
      .fold(0.0, f32::max)
  }

  /// Pushes the node's outer (`inset == false`) or inset shadows. Outer
  /// shadows are cast by the layout box; inset shadows by the padding box,
  /// inside any inside or centred border.
  #[inline(never)]
  #[allow(clippy::too_many_arguments)]
  pub(super) fn push_box_shadow_quads(
    &self,
    node: &Node,
    result: &LayoutResult,
    abs_x: f32,
    abs_y: f32,
    inset: bool,
    opacity: f32,
    transform: Transform2D,
    clip: ClipRect,
    quads: &mut Vec<Quad>,
  ) {
    let Some(value) = node.effective_box_shadow() else {
      return;
    };
    let shadows = self.shadows.borrow();
    let list = value.shadows(&shadows);
    if !list.iter().any(|shadow| shadow.inset == inset) {
      return;
    }
    let radius = node.get_border_radius(&self.radii.borrow());
    let mut rect = ShadowRect {
      x: abs_x,
      y: abs_y,
      width: result.size.width,
      height: result.size.height,
      radius,
    };
    if inset {
      let border = node.get_resolved_border(&self.palette.borrow(), &self.border_sizes.borrow());
      rect = rect.inside_border(border.map(|border| [border.top, border.right, border.bottom, border.left]));
    }
    let (x, y, quad_transform, transform_origin) = transformed_quad_frame(rect.x, rect.y, transform);
    let palette = self.palette.borrow();
    for shadow in list.iter().rev().filter(|shadow| shadow.inset == inset) {
      let Some(shadow) = shadow.resolve(&palette) else {
        continue;
      };
      quads.push(Quad {
        x,
        y,
        width: rect.width,
        height: rect.height,
        opacity,
        transform: quad_transform,
        transform_origin,
        content: QuadContent::BoxShadow(shadow),
        border_radius: rect.radius,
        border: None,
        clip,
      });
    }
  }
}

struct ShadowRect {
  x: f32,
  y: f32,
  width: f32,
  height: f32,
  radius: Option<BorderRadius>,
}

impl ShadowRect {
  /// The padding box: the rect less the part of each border drawn inside it.
  /// Corners shrink by the larger adjacent border, as the inner border edge does.
  fn inside_border(self, sides: Option<[Option<ResolvedBorder>; 4]>) -> Self {
    let Some(sides) = sides else {
      return self;
    };
    let [top, right, bottom, left] = sides.map(|side| {
      side.map_or(0.0, |border| match border.placement {
        BorderPlacement::Inside => border.width,
        BorderPlacement::Center => border.width * 0.5,
        BorderPlacement::Outside => 0.0,
      })
    });
    let radius = self.radius.map(|radius| BorderRadius {
      top_left: (radius.top_left - top.max(left)).max(0.0),
      top_right: (radius.top_right - top.max(right)).max(0.0),
      bottom_right: (radius.bottom_right - bottom.max(right)).max(0.0),
      bottom_left: (radius.bottom_left - bottom.max(left)).max(0.0),
    });
    Self {
      x: self.x + left,
      y: self.y + top,
      width: (self.width - left - right).max(0.0),
      height: (self.height - top - bottom).max(0.0),
      radius,
    }
  }
}
