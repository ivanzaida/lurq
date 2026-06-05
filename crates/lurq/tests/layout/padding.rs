use lurq::{
  app::Tree,
  layout::{
    Constraints, Size,
    layout_result::{ChildLayout, LayoutResult},
  },
  node::{dimension::Dimension, padding::Padding},
};

use super::PassLayoutExt;

fn rt() -> Tree {
  Tree::new()
}

fn padded_child(result: &LayoutResult) -> &ChildLayout {
  &result.children[0]
}

fn padded_stack() -> lurq::components::Stack {
  lurq::components::Stack::new().size(100.0, 50.0).child(
    lurq::components::Spacer::new()
      .width(Dimension::Pct(100.0))
      .height(Dimension::Pct(100.0)),
  )
}

#[test]
fn padding_all_sides() {
  let mut rt = rt();
  let node = padded_stack().padding(Padding::all(Dimension::Px(10.0)));
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 100.0);
  assert_eq!(result.size.height, 50.0);
  let inner = padded_child(&result);
  assert_eq!(inner.offset.x, 10.0);
  assert_eq!(inner.offset.y, 10.0);
  assert_eq!(inner.result.size.width, 80.0);
  assert_eq!(inner.result.size.height, 30.0);
}

#[test]
fn padding_shorthand_all_sides() {
  let mut rt = rt();
  let node = padded_stack().padding(10.0);
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 100.0);
  assert_eq!(result.size.height, 50.0);
  let inner = padded_child(&result);
  assert_eq!(inner.offset.x, 10.0);
  assert_eq!(inner.offset.y, 10.0);
  assert_eq!(inner.result.size.width, 80.0);
  assert_eq!(inner.result.size.height, 30.0);
}

#[test]
fn padding_named_sides() {
  let mut rt = rt();
  let node = padded_stack()
    .padding_left(Dimension::Px(5.0))
    .padding_top(Dimension::Px(10.0));
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 100.0);
  assert_eq!(result.size.height, 50.0);
  let inner = padded_child(&result);
  assert_eq!(inner.offset.x, 5.0);
  assert_eq!(inner.offset.y, 10.0);
  assert_eq!(inner.result.size.width, 95.0);
  assert_eq!(inner.result.size.height, 40.0);
}

#[test]
fn chained_padding_overrides_named_sides() {
  let mut rt = rt();
  let node = padded_stack().padding(10.0).padding_left(Dimension::Px(5.0));
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 100.0);
  assert_eq!(result.size.height, 50.0);
  let inner = padded_child(&result);
  assert_eq!(inner.offset.x, 5.0);
  assert_eq!(inner.offset.y, 10.0);
  assert_eq!(inner.result.size.width, 85.0);
  assert_eq!(inner.result.size.height, 30.0);
}

#[test]
fn padding_asymmetric() {
  let mut rt = rt();
  let node = padded_stack().padding(
    Padding::new()
      .left(Dimension::Px(5.0))
      .top(Dimension::Px(10.0))
      .right(Dimension::Px(15.0))
      .bottom(Dimension::Px(20.0)),
  );
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 100.0);
  assert_eq!(result.size.height, 50.0);
  let inner = padded_child(&result);
  assert_eq!(inner.offset.x, 5.0);
  assert_eq!(inner.offset.y, 10.0);
  assert_eq!(inner.result.size.width, 80.0);
  assert_eq!(inner.result.size.height, 20.0);
}

#[test]
fn padding_horizontal_only() {
  let mut rt = rt();
  let node = padded_stack().padding(Padding::horizontal(Dimension::Px(20.0)));
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 100.0);
  assert_eq!(result.size.height, 50.0);
  let inner = padded_child(&result);
  assert_eq!(inner.offset.x, 20.0);
  assert_eq!(inner.offset.y, 0.0);
  assert_eq!(inner.result.size.width, 60.0);
  assert_eq!(inner.result.size.height, 50.0);
}

#[test]
fn padding_vertical_only() {
  let mut rt = rt();
  let node = padded_stack().padding(Padding::vertical(Dimension::Px(15.0)));
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 100.0);
  assert_eq!(result.size.height, 50.0);
  let inner = padded_child(&result);
  assert_eq!(inner.offset.x, 0.0);
  assert_eq!(inner.offset.y, 15.0);
  assert_eq!(inner.result.size.width, 100.0);
  assert_eq!(inner.result.size.height, 20.0);
}

#[test]
fn padding_symmetric() {
  let mut rt = rt();
  let node = padded_stack().padding(Padding::symmetric(Dimension::Px(10.0), Dimension::Px(20.0)));
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 100.0);
  assert_eq!(result.size.height, 50.0);
  let inner = padded_child(&result);
  assert_eq!(inner.offset.x, 10.0);
  assert_eq!(inner.offset.y, 20.0);
  assert_eq!(inner.result.size.width, 80.0);
  assert_eq!(inner.result.size.height, 10.0);
}

#[test]
fn padding_reduces_child_constraints() {
  let mut rt = rt();
  // Parent tight at 200x100, padding 20 all around -> child gets 160x60
  let node = lurq::components::Spacer::new()
    .flex(1.0)
    .padding(Padding::all(Dimension::Px(20.0)));
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::tight(Size::new(200.0, 100.0))).unwrap();
  assert_eq!(result.size.width, 200.0);
  assert_eq!(result.size.height, 100.0);
}

#[test]
fn padding_zero() {
  let mut rt = rt();
  let node = padded_stack().padding(Padding::all(Dimension::Px(0.0)));
  rt.set_root(node);
  let result = rt.pass_layout(Constraints::loose(Size::new(400.0, 400.0))).unwrap();
  assert_eq!(result.size.width, 100.0);
  assert_eq!(result.size.height, 50.0);
}
