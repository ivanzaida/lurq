//! Paint of a `Select` trigger, drawn from its part style for the current
//! hover/focus/open state, and the open-state scope its chevrons read.

use super::{DEFAULT_CONTROL_SURFACE_COLOR, LayoutEngine, transformed_quad_frame};
use crate::{
  layout::{
    layout_result::LayoutResult,
    quad::{ClipRect, Quad, QuadContent},
  },
  node::{
    color::Color,
    node::Node,
    node_kind::{NodeKind, SelectState},
    transform::Transform2D,
  },
};

impl LayoutEngine {
  /// Pushes the trigger's outer shadows, fill, inset shadows and border.
  #[inline(never)]
  #[allow(clippy::too_many_arguments)]
  pub(super) fn push_select_trigger_quads(
    &self,
    node: &Node,
    state: &SelectState,
    result: &LayoutResult,
    (abs_x, abs_y): (f32, f32),
    opacity: f32,
    transform: Transform2D,
    clip: ClipRect,
    quads: &mut Vec<Quad>,
  ) {
    let style = state.style();
    let hovered = node.style_state.is_hovered();
    let focused = node.style_state.is_focused();
    let trigger = style.resolved_trigger(hovered, focused, state.is_open());

    let background = {
      let palette = self.palette.borrow();
      trigger.background.as_ref().and_then(|color| color.resolve(&palette))
    };
    let radius = trigger
      .border_radius
      .map(|radius| radius.resolve(&self.radii.borrow()))
      .or_else(|| node.get_border_radius(&self.radii.borrow()));
    let border = trigger
      .border
      .as_ref()
      .and_then(|border| border.resolve_with_sizes(&self.palette.borrow(), &self.border_sizes.borrow()))
      .or_else(|| node.get_resolved_border(&self.palette.borrow(), &self.border_sizes.borrow()));
    let frame = (abs_x, abs_y, result.size.width, result.size.height);
    let shadow = trigger.box_shadow.as_ref();
    let has_inset = shadow.is_some_and(|value| value.shadows(&self.shadows.borrow()).iter().any(|s| s.inset));

    if let Some(value) = shadow {
      self.push_part_box_shadow_quads(value, frame, radius, None, false, opacity, transform, clip, quads);
    }
    let (x, y, quad_transform, transform_origin) = transformed_quad_frame(abs_x, abs_y, transform);
    let quad = |color: Color, border| Quad {
      x,
      y,
      width: result.size.width,
      height: result.size.height,
      opacity,
      transform: quad_transform,
      transform_origin,
      content: QuadContent::Rect { color, gradient: None },
      border_radius: radius,
      border,
      clip,
    };
    let fill = background.unwrap_or(DEFAULT_CONTROL_SURFACE_COLOR);
    if !has_inset {
      quads.push(quad(fill, border));
      return;
    }
    // Inset shadows paint between the fill and the border.
    quads.push(quad(fill, None));
    if let Some(value) = shadow {
      self.push_part_box_shadow_quads(value, frame, radius, border, true, opacity, transform, clip, quads);
    }
    if border.is_some() {
      quads.push(quad(Color::new(0, 0, 0, 0), border));
    }
  }

  /// Enters `node` for painting its children: inside a `Select`, chevrons
  /// read its open state. Returns the scope to restore afterwards.
  pub(super) fn enter_select_scope(&self, node: &Node) -> Option<bool> {
    match node.node_kind() {
      NodeKind::Select { state } => self.select_open.replace(Some(state.is_open())),
      _ => self.select_open.get(),
    }
  }

  pub(super) fn leave_select_scope(&self, previous: Option<bool>) {
    self.select_open.set(previous);
  }

  /// Whether `node` is a `Select` chevron for the other open state.
  pub(super) fn is_hidden_select_chevron(&self, node: &Node) -> bool {
    node
      .select_chevron_open()
      .is_some_and(|open| self.select_open.get() != Some(open))
  }
}
