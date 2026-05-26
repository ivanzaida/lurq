use crate::{layout::layout_result::LayoutResult, node::node::Node};

pub struct HitRect {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
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
  let rect = HitRect {
    x: abs_x,
    y: abs_y,
    width: result.size.width,
    height: result.size.height,
  };

  for (child_layout, child_node) in result.children.iter().zip(node.children().iter()) {
    hit_test_tree(
      child_node,
      &child_layout.result,
      abs_x + child_layout.offset.x,
      abs_y + child_layout.offset.y,
      px,
      py,
      hits,
    );
  }

  if rect.contains(px, py) {
    hits.push((node, rect));
  }
}
