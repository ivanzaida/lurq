use crate::{
  layout::{
    layout_kind::{LayoutKind, Overflow},
    layout_result::LayoutResult,
  },
  node::{HitTestBehavior, node::Node, transform::Transform2D},
};

pub struct HitRect {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
  pub local_x: f32,
  pub local_y: f32,
  pub transform: Transform2D,
}

impl HitRect {
  pub fn contains(&self, px: f32, py: f32) -> bool {
    px >= self.x && px <= self.x + self.width && py >= self.y && py <= self.y + self.height
  }
}

pub(crate) fn hit_test_tree<'a>(
  node: &'a Node,
  result: &LayoutResult,
  abs_x: f32,
  abs_y: f32,
  px: f32,
  py: f32,
  hits: &mut Vec<(&'a Node, HitRect)>,
) {
  hit_test_tree_with_transform(node, result, abs_x, abs_y, px, py, Transform2D::IDENTITY, true, hits);
}

pub(crate) fn hit_test_tree_all<'a>(
  node: &'a Node,
  result: &LayoutResult,
  abs_x: f32,
  abs_y: f32,
  px: f32,
  py: f32,
  hits: &mut Vec<(&'a Node, HitRect)>,
) {
  hit_test_tree_with_transform(node, result, abs_x, abs_y, px, py, Transform2D::IDENTITY, false, hits);
}

fn hit_test_tree_with_transform<'a>(
  node: &'a Node,
  result: &LayoutResult,
  abs_x: f32,
  abs_y: f32,
  px: f32,
  py: f32,
  inherited_transform: Transform2D,
  occlude_stack_siblings: bool,
  hits: &mut Vec<(&'a Node, HitRect)>,
) {
  let behavior = node.hit_test_behavior();
  if behavior == HitTestBehavior::None {
    return;
  }

  let rect = HitRect {
    x: abs_x,
    y: abs_y,
    width: result.size.width,
    height: result.size.height,
    local_x: px,
    local_y: py,
    transform: Transform2D::IDENTITY,
  };

  let local_transform = node.effective_transform();
  let local_transform_origin = [rect.x + rect.width * 0.5, rect.y + rect.height * 0.5];
  let local_affine = if local_transform.is_identity() {
    Transform2D::IDENTITY
  } else {
    local_transform.around_origin(local_transform_origin)
  };
  let transform = if inherited_transform.is_identity() {
    local_affine
  } else if local_affine.is_identity() {
    inherited_transform
  } else if local_transform.is_identity() {
    inherited_transform
  } else {
    inherited_transform.then(&local_affine)
  };
  let (local_x, local_y) = inverse_transform_point(px, py, transform);
  let rect = HitRect {
    local_x,
    local_y,
    transform,
    ..rect
  };
  let inside = rect.contains(local_x, local_y);
  let can_have_visible_children = inside || node.overflow == Overflow::Visible;

  if can_have_visible_children {
    if occlude_stack_siblings && matches!(node.layout_kind(), LayoutKind::Stack { .. }) {
      for (child_layout, child_node) in result.children.iter().zip(node.children().iter()).rev() {
        let hit_count = hits.len();
        hit_test_tree_with_transform(
          child_node,
          &child_layout.result,
          abs_x + child_layout.offset.x,
          abs_y + child_layout.offset.y,
          px,
          py,
          transform,
          occlude_stack_siblings,
          hits,
        );
        if hits.len() > hit_count {
          break;
        }
      }
    } else {
      for (child_layout, child_node) in result.children.iter().zip(node.children().iter()) {
        hit_test_tree_with_transform(
          child_node,
          &child_layout.result,
          abs_x + child_layout.offset.x,
          abs_y + child_layout.offset.y,
          px,
          py,
          transform,
          occlude_stack_siblings,
          hits,
        );
      }
    }
  }

  if inside && behavior == HitTestBehavior::Auto {
    hits.push((node, rect));
  }
}

fn inverse_transform_point(px: f32, py: f32, transform: Transform2D) -> (f32, f32) {
  if transform.is_identity() {
    return (px, py);
  }

  let Some(inverse) = transform.inverse_affine() else {
    return (px, py);
  };
  inverse.transform_point(px, py)
}
