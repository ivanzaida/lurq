use lurq::{
  app::Tree,
  layout::{
    Alignment, Constraints, Size,
    quad::{ClipRect, QuadContent},
  },
  node::Element,
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
fn fully_offscreen_node_below_viewport_is_culled() {
  let mut rt = rt();
  let node = lurq::components::Column::with(
    0.0,
    Alignment::Start,
    vec![
      lurq::components::Rect::new(100.0, 100.0).background("#ff0000"),
      lurq::components::Rect::new(100.0, 100.0).background("#00ff00"),
    ],
  );
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();

  let quads = rt.resolve_quads_with_viewport(&result, viewport(100.0, 100.0));

  assert_eq!(
    quads.len(),
    1,
    "only the node inside the viewport should produce a quad"
  );
  assert!(matches!(quads[0].content, QuadContent::Rect { .. }));
  assert_eq!(quads[0].y, 0.0);
}

#[test]
fn fully_offscreen_node_right_of_viewport_is_culled() {
  let mut rt = rt();
  let node = lurq::components::Row::with(
    0.0,
    Alignment::Start,
    vec![
      lurq::components::Rect::new(100.0, 50.0).background("#ff0000"),
      lurq::components::Rect::new(100.0, 50.0).background("#00ff00"),
    ],
  );
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();

  let quads = rt.resolve_quads_with_viewport(&result, viewport(100.0, 100.0));

  assert_eq!(quads.len(), 1, "node at x=100 should be outside viewport width=100");
  assert_eq!(quads[0].x, 0.0);
}

#[test]
fn all_visible_nodes_are_kept() {
  let mut rt = rt();
  let node = lurq::components::Column::with(
    0.0,
    Alignment::Start,
    vec![
      lurq::components::Rect::new(50.0, 30.0).background("#ff0000"),
      lurq::components::Rect::new(50.0, 30.0).background("#00ff00"),
      lurq::components::Rect::new(50.0, 30.0).background("#0000ff"),
    ],
  );
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();

  let quads = rt.resolve_quads_with_viewport(&result, viewport(200.0, 200.0));

  assert_eq!(quads.len(), 3, "all three nodes fit inside the 200x200 viewport");
}

#[test]
fn many_offscreen_nodes_in_column_are_culled() {
  let mut rt = rt();
  let children: Vec<Element> = (0..20)
    .map(|_| lurq::components::Rect::new(50.0, 50.0).background("#334155").into())
    .collect();
  let node = lurq::components::Column::with(0.0, Alignment::Start, children);
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 2000.0))).unwrap();

  let quads = rt.resolve_quads_with_viewport(&result, viewport(400.0, 200.0));

  assert_eq!(quads.len(), 4, "only nodes at y=0..199 (4 x 50px) should be visible");
}

#[test]
fn without_viewport_all_nodes_are_emitted() {
  let mut rt = rt();
  let children: Vec<Element> = (0..20)
    .map(|_| lurq::components::Rect::new(50.0, 50.0).background("#334155").into())
    .collect();
  let node = lurq::components::Column::with(0.0, Alignment::Start, children);
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 2000.0))).unwrap();

  let quads = rt.resolve_quads(&result);

  assert_eq!(quads.len(), 20, "without viewport clip, all nodes should be emitted");
}

#[test]
fn offscreen_subtree_is_culled_entirely() {
  let mut rt = rt();
  let node = lurq::components::Column::with(
    0.0,
    Alignment::Start,
    vec![
      Element::from(lurq::components::Rect::new(100.0, 50.0).background("#ff0000")),
      Element::from(
        lurq::components::Column::with(
          0.0,
          Alignment::Start,
          vec![
            lurq::components::Rect::new(100.0, 50.0).background("#00ff00"),
            lurq::components::Rect::new(100.0, 50.0).background("#0000ff"),
          ],
        )
        .background("#aaaaaa"),
      ),
    ],
  );
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();

  let quads = rt.resolve_quads_with_viewport(&result, viewport(100.0, 50.0));

  assert_eq!(
    quads.len(),
    1,
    "the entire offscreen subtree (parent + 2 children) should be culled"
  );
  assert_eq!(quads[0].y, 0.0);
}
