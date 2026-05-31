use lurq::{
  app::Tree,
  layout::{Constraints, Size, quad::ClipRect},
};

use crate::layout::PassLayoutExt;

fn rt() -> Tree {
  Tree::new()
}

fn viewport(w: f32, h: f32) -> ClipRect {
  ClipRect {
    x: 0.0,
    y: 0.0,
    width: w,
    height: h,
    active: true,
  }
}

#[test]
fn overflow_visible_node_inside_viewport_is_kept() {
  let mut rt = rt();
  let node = lurq::components::Row::new()
    .child(lurq::components::Rect::new(200.0, 50.0).fill("#ff0000"))
    .width(100.0)
    .height(50.0)
    .fill("#000000")
    .overflow_visible();
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::tight(Size::new(200.0, 200.0))).unwrap();

  let quads = rt.resolve_quads_with_viewport(&result, viewport(200.0, 200.0));

  assert!(
    quads.len() >= 2,
    "overflow-visible parent and overflowing child should both be kept"
  );
}

#[test]
fn overflow_visible_offscreen_child_not_culled_when_no_active_clip() {
  let mut rt = rt();
  // Parent is overflow-visible so child_clip remains the viewport clip.
  // The child at x=0 width=200 extends beyond the parent (width=100)
  // but falls within the viewport (width=300).
  // With Overflow::Visible, the child's own clip.active is false,
  // so clipped_subtree_is_hidden returns false (safe — not culled).
  let node = lurq::components::Row::new()
    .child(lurq::components::Rect::new(200.0, 50.0).fill("#ff0000"))
    .width(100.0)
    .height(50.0)
    .fill("#000000")
    .overflow_visible();
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::tight(Size::new(300.0, 200.0))).unwrap();

  let quads = rt.resolve_quads_with_viewport(&result, viewport(300.0, 200.0));

  let child_quads: Vec<_> = quads.iter().filter(|q| q.width == 200.0).collect();
  assert_eq!(child_quads.len(), 1, "overflowing child within viewport should be kept");
}
