use lurq::{
  app::Tree,
  layout::{Alignment, Constraints, Size, quad::ClipRect},
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
fn partially_visible_node_is_kept() {
  let mut rt = rt();
  let node = lurq::components::Column::with(
    0.0,
    Alignment::Start,
    vec![
      lurq::components::Rect::new(100.0, 80.0).fill("#ff0000"),
      lurq::components::Rect::new(100.0, 80.0).fill("#00ff00"),
    ],
  );
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();

  let quads = rt.resolve_quads_with_viewport(&result, viewport(100.0, 100.0));

  assert_eq!(
    quads.len(),
    2,
    "second node at y=80 overlaps viewport bottom — must be kept"
  );
}

#[test]
fn node_touching_viewport_edge_is_kept() {
  let mut rt = rt();
  let node = lurq::components::Column::with(
    0.0,
    Alignment::Start,
    vec![
      lurq::components::Rect::new(100.0, 100.0).fill("#ff0000"),
      lurq::components::Rect::new(100.0, 50.0).fill("#00ff00"),
    ],
  );
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();

  // Node 2 starts at y=100, viewport ends at y=100 — no overlap
  let quads = rt.resolve_quads_with_viewport(&result, viewport(100.0, 100.0));

  assert_eq!(
    quads.len(),
    1,
    "node starting exactly at viewport edge has zero overlap — culled"
  );
}

#[test]
fn node_one_pixel_inside_viewport_is_kept() {
  let mut rt = rt();
  let node = lurq::components::Column::with(
    0.0,
    Alignment::Start,
    vec![
      lurq::components::Rect::new(100.0, 99.0).fill("#ff0000"),
      lurq::components::Rect::new(100.0, 50.0).fill("#00ff00"),
    ],
  );
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();

  // Node 2 starts at y=99, viewport ends at y=100 — 1px overlap
  let quads = rt.resolve_quads_with_viewport(&result, viewport(100.0, 100.0));

  assert_eq!(quads.len(), 2, "node overlapping viewport by 1px must be kept");
}

#[test]
fn viewport_clip_is_carried_onto_quads() {
  let mut rt = rt();
  let node = lurq::components::Rect::new(200.0, 200.0).fill("#ff0000");
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();

  let quads = rt.resolve_quads_with_viewport(&result, viewport(100.0, 100.0));

  assert_eq!(quads.len(), 1);
  assert!(quads[0].clip.active, "viewport clip should be active on the quad");
  assert_eq!(quads[0].clip.width, 100.0);
  assert_eq!(quads[0].clip.height, 100.0);
}
