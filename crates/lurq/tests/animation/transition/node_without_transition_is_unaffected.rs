use lurq::{
  app::Tree,
  layout::{Constraints, Size, quad::QuadContent},
  node::color::Color,
};

use crate::support::run_pass;

#[test]
fn transition_engine_does_not_affect_nodes_without_transition_spec() {
  let mut rt = Tree::new();

  let node = lurq::components::Column::new()
    .child(lurq::components::Rect::new(100.0, 50.0).background("#ff0000"))
    .child(lurq::components::Rect::new(100.0, 50.0).background("#00ff00"));
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);
  run_pass(&mut rt);
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  let colors: Vec<_> = quads
    .iter()
    .filter_map(|q| match &q.content {
      QuadContent::Rect { color } => Some(*color),
      _ => None,
    })
    .collect();

  assert_eq!(colors[0], Color::from_hex("#ff0000"));
  assert_eq!(colors[1], Color::from_hex("#00ff00"));
}
