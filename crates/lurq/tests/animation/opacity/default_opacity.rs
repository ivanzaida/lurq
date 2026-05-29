use lurq::{
  app::Tree,
  layout::{Constraints, Size},
};

use crate::support::run_pass;

#[test]
fn default_opacity_is_one() {
  let mut rt = Tree::new();
  let node = lurq::components::Rect::new(100.0, 50.0).fill("#ff0000");
  rt.set_root(node);

  rt.set_layout_constraints_override(Some(Constraints::loose(Size::new(400.0, 400.0))));
  run_pass(&mut rt);

  let result = rt.last_layout().unwrap();
  let quads = rt.resolve_quads(result);
  assert_eq!(quads[0].opacity, 1.0);
}
