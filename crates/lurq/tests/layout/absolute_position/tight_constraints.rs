use lurq::{
  app::Runtime,
  layout::{Constraints, Size},
};

use super::super::PassLayoutExt;

#[test]
fn absolute_position_wrapper_keeps_child_size_under_tight_parent_constraints() {
  let mut runtime = Runtime::new();
  runtime.set_root(
    lurq::components::Stack::new()
      .size(200.0, 100.0)
      .child(lurq::components::Rect::new(50.0, 40.0).absolute_position(20.0, 10.0)),
  );

  let result = runtime
    .pass_layout(Constraints::tight(Size::new(200.0, 100.0)))
    .unwrap();
  let stack = &result.children[0].result;
  let absolute = &stack.children[0].result;

  assert_eq!(absolute.size.width, 50.0);
  assert_eq!(absolute.size.height, 40.0);
}
