//! Opacity groups. A node with `opacity < 1` fades its subtree as one layer
//! (see [`crate::layout::opacity_layer`]): its quads, its children's and its
//! overlay quads are emitted at full opacity and recorded as a group. A group
//! that paints one primitive folds its opacity into that quad instead, which
//! paints the same pixels without a layer.

use super::LayoutEngine;
use crate::layout::{
  opacity_layer::OpacityGroup,
  quad::{Quad, QuadContent},
};

impl LayoutEngine {
  /// Closes the group a node with `opacity < 1` opened when its first quad
  /// would have been pushed at `start`.
  #[inline(never)]
  pub(super) fn close_opacity_group(&self, opacity: f32, start: usize, quads: &mut [Quad]) {
    let opacity = opacity.max(0.0);
    let end = quads.len();
    if start >= end {
      return;
    }
    if end - start == 1 && paints_one_primitive(&quads[start]) {
      quads[start].opacity *= opacity;
      return;
    }
    let mut groups = self.opacity_groups.borrow_mut();
    // A layer composited into an otherwise empty layer is that layer at the
    // product of both opacities.
    if let Some(inner) = groups.last_mut()
      && inner.start == start
      && inner.end == end
    {
      inner.opacity *= opacity;
      return;
    }
    groups.push(OpacityGroup { start, end, opacity });
  }

  /// Moves the groups recorded by the last quad resolution into `groups`.
  pub(crate) fn take_opacity_groups(&self, groups: &mut Vec<OpacityGroup>) {
    groups.clear();
    std::mem::swap(groups, &mut self.opacity_groups.borrow_mut());
  }
}

/// Whether a quad paints a single primitive, so fading it and fading a layer
/// holding it give the same pixels. A rect with a border paints its fill and
/// then the border over the fill's edge, and a text shadow paints under its
/// text, so those overlap.
///
/// Glyphs of one text run are treated as one primitive, as browsers do for a
/// text run under `opacity`: they overlap only where outlines touch.
fn paints_one_primitive(quad: &Quad) -> bool {
  match &quad.content {
    QuadContent::Rect { .. } => quad.border.is_none(),
    QuadContent::Text { style, .. } => style.shadow.is_none(),
    QuadContent::RichText { spans, .. } => spans.first().is_none_or(|span| span.style.shadow.is_none()),
    #[cfg(feature = "raster")]
    QuadContent::Image { .. } | QuadContent::Video { .. } => true,
    // Rasterized, an SVG is one image; tessellated, its paths overlap.
    #[cfg(all(feature = "svg", feature = "raster"))]
    QuadContent::Svg { .. } => true,
    #[cfg(all(feature = "svg", not(feature = "raster")))]
    QuadContent::Svg { .. } => false,
    QuadContent::BoxShadow(_) | QuadContent::None => true,
  }
}
