use lurq::{app::Tree, node::CursorIcon};

use crate::support::run_pass;

#[test]
fn cursor_follows_hovered_element() {
  let mut runtime = Tree::new();
  runtime.set_root(lurq::components::Rect::new(100.0, 40.0).cursor(CursorIcon::Pointer));
  run_pass(&mut runtime);

  assert_eq!(runtime.cursor(), CursorIcon::Default);

  runtime.mouse_move(10.0, 10.0);
  assert_eq!(runtime.cursor(), CursorIcon::Pointer);

  runtime.mouse_move(150.0, 150.0);
  assert_eq!(runtime.cursor(), CursorIcon::Default);
}

#[test]
fn hovered_style_can_override_cursor() {
  let mut runtime = Tree::new();
  runtime.set_root(lurq::components::Rect::new(100.0, 40.0).hovered(|style| style.cursor(CursorIcon::Text)));
  run_pass(&mut runtime);

  runtime.mouse_move(10.0, 10.0);
  assert_eq!(runtime.cursor(), CursorIcon::Text);

  runtime.mouse_move(150.0, 150.0);
  assert_eq!(runtime.cursor(), CursorIcon::Default);
}
