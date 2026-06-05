use lurq::{
  app::Tree,
  layout::{Constraints, Size},
};

use super::super::PassLayoutExt;

#[test]
fn row_flex_child_min_width_limits_shrink() {
  let mut runtime = Tree::new();
  runtime.set_root(lurq::components::Row::new().spacing(0.0).with_children(vec![
      lurq::components::Rect::new(200.0, 20.0)
        .min_width(180.0)
        .flex_full(0.0, 1.0, None),
      lurq::components::Rect::new(200.0, 20.0).flex_full(0.0, 1.0, None),
    ]));

  let result = runtime.pass_layout(Constraints::tight(Size::new(250.0, 80.0))).unwrap();

  assert_eq!(result.size.width, 250.0);
  assert_eq!(result.children[0].result.size.width, 180.0);
  assert_eq!(result.children[1].result.size.width, 70.0);
  assert_eq!(result.children[1].offset.x, 180.0);
}
