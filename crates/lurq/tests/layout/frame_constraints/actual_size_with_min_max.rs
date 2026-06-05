use lurq::{
  app::Tree,
  layout::{Constraints, Size},
};

use super::super::PassLayoutExt;

#[test]
fn explicit_width_is_raised_to_min_width() {
  let mut runtime = Tree::new();
  runtime.set_root(lurq::components::Spacer::new().width(80.0).min_width(120.0));

  let result = runtime
    .pass_layout(Constraints::loose(Size::new(400.0, 400.0)))
    .unwrap();

  assert_eq!(result.size.width, 120.0);
}

#[test]
fn explicit_width_is_lowered_to_max_width() {
  let mut runtime = Tree::new();
  runtime.set_root(lurq::components::Spacer::new().width(300.0).max_width(180.0));

  let result = runtime
    .pass_layout(Constraints::loose(Size::new(400.0, 400.0)))
    .unwrap();

  assert_eq!(result.size.width, 180.0);
}

#[test]
fn explicit_size_between_min_and_max_is_preserved() {
  let mut runtime = Tree::new();
  runtime.set_root(
    lurq::components::Spacer::new()
      .size(160.0, 90.0)
      .min_size(120.0, 80.0)
      .max_size(200.0, 120.0),
  );

  let result = runtime
    .pass_layout(Constraints::loose(Size::new(400.0, 400.0)))
    .unwrap();

  assert_eq!(result.size.width, 160.0);
  assert_eq!(result.size.height, 90.0);
}

#[test]
fn max_size_wins_when_min_and_max_conflict() {
  let mut runtime = Tree::new();
  runtime.set_root(
    lurq::components::Spacer::new()
      .size(160.0, 90.0)
      .min_size(200.0, 140.0)
      .max_size(150.0, 100.0),
  );

  let result = runtime
    .pass_layout(Constraints::loose(Size::new(400.0, 400.0)))
    .unwrap();

  assert_eq!(result.size.width, 150.0);
  assert_eq!(result.size.height, 100.0);
}
