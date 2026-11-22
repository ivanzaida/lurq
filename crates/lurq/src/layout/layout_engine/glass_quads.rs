//! Paint of glass: a node with a glass gives a
//! [`QuadContent::Backdrop`] quad that holds its box as the glass's shape,
//! before everything else it paints.
//!
//! The glass nodes under a glass container (`Node::glass_container`) are one
//! surface. The container gives the quad, before anything in it is painted,
//! and each glass node under it adds its box to that quad instead of giving
//! one of its own, so the whole surface is drawn at once, under everything
//! the container holds. The first shape's node gives the material. A quad
//! holds [`MAX_GLASS_SHAPES`] shapes; a glass node that finds it full is a
//! glass of its own.

use super::{LayoutEngine, transformed_quad_frame};
use crate::{
  layout::{
    glass::MAX_GLASS_SHAPES,
    layout_result::LayoutResult,
    quad::{ClipRect, GlassQuad, GlassQuadShape, Quad, QuadContent},
  },
  node::{node::Node, transform::Transform2D},
};

impl LayoutEngine {
  /// Gives `node`'s glass: its shape goes to the container's quad when the
  /// node is under one with room, and to a quad of its own otherwise. A
  /// glass container gives the quad its glass nodes fill, with its own
  /// shape first when it is a glass itself.
  #[allow(clippy::too_many_arguments)]
  pub(super) fn push_glass_quad(
    &self,
    node: &Node,
    result: &LayoutResult,
    abs_x: f32,
    abs_y: f32,
    transform: Transform2D,
    clip: ClipRect,
    quads: &mut Vec<Quad>,
  ) {
    let glass = node.effective_glass();
    let container = node.glass_merge();
    if glass.is_none() && container.is_none() {
      return;
    }
    let (x, y, quad_transform, transform_origin) = transformed_quad_frame(abs_x, abs_y, transform);
    let shape = glass.map(|_| GlassQuadShape {
      x,
      y,
      width: result.size.width,
      height: result.size.height,
      border_radius: node.get_border_radius(&self.radii.borrow()),
      transform: quad_transform,
      transform_origin,
    });
    if container.is_none()
      && let (Some(glass), Some(shape), Some(scope)) = (glass, shape, self.glass_scope.get())
      && let Some(QuadContent::Backdrop {
        glass: Some(surface), ..
      }) = quads.get_mut(scope).map(|quad| &mut quad.content)
      && surface.shapes.len() < MAX_GLASS_SHAPES
    {
      if surface.shapes.is_empty() {
        surface.glass = glass;
      }
      surface.shapes.push(shape);
      return;
    }
    quads.push(Quad {
      x,
      y,
      width: result.size.width,
      height: result.size.height,
      opacity: node.effective_opacity().clamp(0.0, 1.0),
      transform: quad_transform,
      transform_origin,
      content: QuadContent::Backdrop {
        blur: 0.0,
        glass: Some(Box::new(GlassQuad {
          glass: glass.unwrap_or_default(),
          merge: container.unwrap_or(0.0).max(0.0),
          shapes: shape.into_iter().collect(),
        })),
      },
      border_radius: None,
      border: None,
      clip,
    });
  }

  /// Makes `node`, a glass container, the one the glass nodes painted from
  /// here on add their shapes to: its quad is among the ones the node gave,
  /// `quads[start..]`. Returns the scope to give back to
  /// [`Self::leave_glass_scope`]; `None` for a node that is no container.
  pub(super) fn enter_glass_scope(&self, node: &Node, start: usize, quads: &[Quad]) -> Option<Option<usize>> {
    node.glass_merge()?;
    let index = quads[start..]
      .iter()
      .position(|quad| matches!(quad.content, QuadContent::Backdrop { glass: Some(_), .. }))?;
    Some(self.glass_scope.replace(Some(start + index)))
  }

  pub(super) fn leave_glass_scope(&self, scope: Option<Option<usize>>) {
    if let Some(outer) = scope {
      self.glass_scope.set(outer);
    }
  }
}
