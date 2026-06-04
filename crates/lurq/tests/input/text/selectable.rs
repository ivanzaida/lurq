use lurq::{
  app::{Tree, events::MouseButton},
  core::element_ref::ElementRect,
  node::{color::Color, transform::Transform2D},
};

use crate::support::{render_pass, run_pass};

fn pointer_click(runtime: &mut Tree, x: f32, y: f32) {
  runtime.mouse_down(x, y, MouseButton::Left);
  runtime.mouse_up(x, y, MouseButton::Left);
}

fn selection_rect_count(runtime: &mut Tree) -> usize {
  render_pass(runtime)
    .rects
    .iter()
    .filter(|rect| rect.color == Color::from_hex("#bfdbfe") && rect.width > 1.0 && rect.height > 0.0)
    .count()
}

fn transform_point_around_rect(x: f32, y: f32, rect: ElementRect, transform: Transform2D) -> (f32, f32) {
  let (origin_x, origin_y) = rect.center();
  let dx = x - origin_x;
  let dy = y - origin_y;
  (
    origin_x + transform.a * dx + transform.c * dy,
    origin_y + transform.b * dx + transform.d * dy,
  )
}

#[test]
fn selectable_text_drag_renders_selection() {
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::Text::new("Hello world").selectable(true));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let y = rect.y + rect.height / 2.0;

  runtime.mouse_down(rect.x, y, MouseButton::Left);
  runtime.mouse_move(rect.x + rect.width, y);
  runtime.mouse_up(rect.x + rect.width, y, MouseButton::Left);

  assert!(selection_rect_count(&mut runtime) > 0);
}

#[test]
fn transformed_selectable_text_drag_uses_visual_coordinates() {
  let mut runtime = Tree::new();
  let transform = Transform2D::rotate_deg(-8.0).then(&Transform2D::scale(1.1, 1.1));

  runtime.set_root(
    lurq::components::Text::new("Hello transformed world")
      .selectable(true)
      .transform(transform),
  );
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let local_y = rect.y + rect.height / 2.0;
  let start = transform_point_around_rect(rect.x, local_y, rect, transform);
  let end = transform_point_around_rect(rect.x + rect.width, local_y, rect, transform);

  runtime.mouse_down(start.0, start.1, MouseButton::Left);
  runtime.mouse_move(end.0, end.1);
  runtime.mouse_up(end.0, end.1, MouseButton::Left);

  assert!(selection_rect_count(&mut runtime) > 0);
}

#[test]
fn non_selectable_text_drag_does_not_render_selection() {
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::Text::new("Hello world").selectable(false));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let y = rect.y + rect.height / 2.0;

  runtime.mouse_down(rect.x, y, MouseButton::Left);
  runtime.mouse_move(rect.x + rect.width, y);
  runtime.mouse_up(rect.x + rect.width, y, MouseButton::Left);

  assert_eq!(selection_rect_count(&mut runtime), 0);
}

#[test]
fn double_click_selectable_text_selects_clicked_word() {
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::Text::new("one two").selectable(true));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let y = rect.y + rect.height / 2.0;
  let x = rect.x + 46.0;

  pointer_click(&mut runtime, x, y);
  pointer_click(&mut runtime, x, y);

  let snapshot = render_pass(&mut runtime);
  let selection = snapshot
    .rects
    .iter()
    .find(|rect| rect.color == Color::from_hex("#bfdbfe") && rect.width > 1.0 && rect.height > 0.0)
    .expect("double-clicking selectable text should render a word selection");

  assert!(selection.width < rect.width);
}

#[test]
fn triple_click_selectable_text_selects_clicked_line() {
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::Text::new("one\ntwo words\nthree").selectable(true));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let line_height = 19.2;
  let y = rect.y + line_height + 1.0;
  let x = rect.x + 46.0;

  pointer_click(&mut runtime, x, y);
  pointer_click(&mut runtime, x, y);
  pointer_click(&mut runtime, x, y);

  assert_eq!(selection_rect_count(&mut runtime), 1);
}

#[test]
fn transformed_multiline_selectable_text_click_uses_visual_line() {
  let mut runtime = Tree::new();
  let transform = Transform2D::rotate_deg(28.0);

  runtime.set_root(
    lurq::components::Text::new("one\nvery very long second line\nthree")
      .selectable(true)
      .transform(transform),
  );
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let line_height = 19.2;
  let local_x = rect.x + rect.width - 4.0;
  let local_y = rect.y + line_height * 0.5;
  let point = transform_point_around_rect(local_x, local_y, rect, transform);

  pointer_click(&mut runtime, point.0, point.1);
  pointer_click(&mut runtime, point.0, point.1);
  pointer_click(&mut runtime, point.0, point.1);

  let snapshot = render_pass(&mut runtime);
  let selection = snapshot
    .rects
    .iter()
    .find(|rect| rect.color == Color::from_hex("#bfdbfe") && rect.width > 1.0 && rect.height > 0.0)
    .expect("triple-clicking transformed selectable text should select a line");

  assert!(
    selection.width < rect.width * 0.35,
    "visual click on short first line should not select the long second line: selection_width={}, text_width={}",
    selection.width,
    rect.width
  );
}

#[test]
fn transformed_parent_selectable_text_click_uses_visual_line() {
  let mut runtime = Tree::new();
  let transform = Transform2D::rotate_deg(28.0);

  runtime.set_root(
    lurq::components::Column::new()
      .child(
        lurq::components::Text::new("one\nvery very long second line\nthree")
          .selectable(true)
          .width(260.0),
      )
      .padding(14.0)
      .width(300.0)
      .transform(transform)
      .overflow_visible(),
  );
  run_pass(&mut runtime);
  let text_rect = runtime.find_element(|node| node.tag_name() == "Text").unwrap().bounds();
  let parent_rect = runtime
    .find_element(|node| node.tag_name() == "Column")
    .unwrap()
    .bounds();
  let line_height = 19.2;
  let local_x = text_rect.x + text_rect.width - 4.0;
  let local_y = text_rect.y + line_height * 0.5;
  let point = transform_point_around_rect(local_x, local_y, parent_rect, transform);

  pointer_click(&mut runtime, point.0, point.1);
  pointer_click(&mut runtime, point.0, point.1);
  pointer_click(&mut runtime, point.0, point.1);

  let snapshot = render_pass(&mut runtime);
  let selection = snapshot
    .rects
    .iter()
    .find(|rect| rect.color == Color::from_hex("#bfdbfe") && rect.width > 1.0 && rect.height > 0.0)
    .expect("triple-clicking selectable text in transformed parent should select a line");

  assert!(
    selection.width < text_rect.width * 0.35,
    "visual click on short first line should not select the long second line: selection_width={}, text_width={}",
    selection.width,
    text_rect.width
  );
}
