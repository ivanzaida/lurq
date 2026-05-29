use lurq::{
  app::Tree,
  layout::{Constraints, Size, quad::QuadContent},
  node::color::Color,
};

use crate::support::run_pass;

#[test]
fn node_without_transition_changes_color_immediately() {
  let mut rt = Tree::new();
  let node = lurq::components::Rect::new(100.0, 50.0).fill("#ff0000");
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  assert_eq!(quads.len(), 1);
  if let QuadContent::Rect { color } = &quads[0].content {
    assert_eq!(*color, Color::from_hex("#ff0000"));
  } else {
    panic!("expected rect quad");
  }
}
