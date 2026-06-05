use lurq::{
  app::Tree,
  layout::{Constraints, Size},
};

use super::super::PassLayoutExt;

#[test]
fn row_flex_child_max_width_caps_grow_allocation() {
  let mut runtime = Tree::new();
  runtime.set_root(lurq::components::Row::new().spacing(0.0).with_children(vec![
    lurq::components::Spacer::new().max_width(100.0).flex(1.0),
    lurq::components::Spacer::new().flex(1.0),
  ]));

  let result = runtime.pass_layout(Constraints::tight(Size::new(400.0, 80.0))).unwrap();

  assert_eq!(result.size.width, 400.0);
  assert_eq!(result.children[0].result.size.width, 100.0);
  assert_eq!(result.children[1].result.size.width, 200.0);
  assert_eq!(result.children[1].offset.x, 100.0);
}
