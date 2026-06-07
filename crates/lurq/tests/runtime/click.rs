use lurq::{
  app::{Tree, events::MouseButton},
  components::{Column, Rect, Stack},
  core::Signal,
};

use crate::support::run_pass;

#[test]
fn release_over_click_target_does_not_click_when_press_started_elsewhere() {
  let clicks = Signal::new(0);
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::Rect::new(100.0, 40.0)
      .background("#22c55e")
      .on_click({
        let clicks = clicks.clone();
        move |_| clicks.update(|count| *count += 1)
      }),
  );
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.mouse_down(x + rect.width + 20.0, y, MouseButton::Left);
  runtime.mouse_up(x, y, MouseButton::Left);

  assert_eq!(clicks.get(), 0);
}

#[test]
fn release_far_from_press_clicks_same_target() {
  let clicks = Signal::new(0);
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::Rect::new(100.0, 40.0)
      .background("#22c55e")
      .on_click({
        let clicks = clicks.clone();
        move |_| clicks.update(|count| *count += 1)
      }),
  );
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let y = rect.y + rect.height / 2.0;

  runtime.mouse_down(rect.x + 10.0, y, MouseButton::Left);
  runtime.mouse_up(rect.x + 90.0, y, MouseButton::Left);

  assert_eq!(clicks.get(), 1);
}

#[test]
fn release_near_press_clicks_same_target() {
  let clicks = Signal::new(0);
  let mut runtime = Tree::new();

  runtime.set_root(
    lurq::components::Rect::new(100.0, 40.0)
      .background("#22c55e")
      .on_click({
        let clicks = clicks.clone();
        move |_| clicks.update(|count| *count += 1)
      }),
  );
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.mouse_down(x, y, MouseButton::Left);
  runtime.mouse_up(x + 2.0, y, MouseButton::Left);

  assert_eq!(clicks.get(), 1);
}

#[test]
fn child_press_parent_release_clicks_parent() {
  let parent_clicks = Signal::new(0);
  let child_clicks = Signal::new(0);
  let mut runtime = Tree::new();

  runtime.set_root(
    Column::new()
      .size(100.0, 100.0)
      .on_click({
        let parent_clicks = parent_clicks.clone();
        move |_| parent_clicks.update(|count| *count += 1)
      })
      .child(Rect::new(40.0, 40.0).background("#22c55e").on_click({
        let child_clicks = child_clicks.clone();
        move |_| child_clicks.update(|count| *count += 1)
      })),
  );
  run_pass(&mut runtime);

  runtime.mouse_down(20.0, 20.0, MouseButton::Left);
  runtime.mouse_up(80.0, 80.0, MouseButton::Left);

  assert_eq!(parent_clicks.get(), 1);
  assert_eq!(child_clicks.get(), 0);
}

#[test]
fn parent_press_child_release_clicks_parent_not_child() {
  let parent_clicks = Signal::new(0);
  let child_clicks = Signal::new(0);
  let mut runtime = Tree::new();

  runtime.set_root(
    Column::new()
      .size(100.0, 100.0)
      .on_click({
        let parent_clicks = parent_clicks.clone();
        move |_| parent_clicks.update(|count| *count += 1)
      })
      .child(Rect::new(40.0, 40.0).background("#22c55e").on_click({
        let child_clicks = child_clicks.clone();
        move |_| child_clicks.update(|count| *count += 1)
      })),
  );
  run_pass(&mut runtime);

  runtime.mouse_down(80.0, 80.0, MouseButton::Left);
  runtime.mouse_up(20.0, 20.0, MouseButton::Left);

  assert_eq!(parent_clicks.get(), 1);
  assert_eq!(child_clicks.get(), 0);
}

#[test]
fn stack_top_child_occludes_lower_sibling_clicks() {
  let lower_clicks = Signal::new(0);
  let mut runtime = Tree::new();

  runtime.set_root(
    Stack::new()
      .child(Rect::new(100.0, 100.0).background("#111827").on_click({
        let lower_clicks = lower_clicks.clone();
        move |_| lower_clicks.update(|count| *count += 1)
      }))
      .child(Rect::new(50.0, 50.0).background("#ffffff")),
  );
  run_pass(&mut runtime);

  runtime.mouse_down(25.0, 25.0, MouseButton::Left);
  runtime.mouse_up(25.0, 25.0, MouseButton::Left);

  assert_eq!(lower_clicks.get(), 0);

  runtime.mouse_down(75.0, 75.0, MouseButton::Left);
  runtime.mouse_up(75.0, 75.0, MouseButton::Left);

  assert_eq!(lower_clicks.get(), 1);
}

#[test]
fn rebuilt_descendant_at_same_position_receives_click() {
  let clicks = Signal::new(0);
  let mut runtime = Tree::new();

  runtime.set_root(Column::new().child(Rect::new(100.0, 40.0).background("#22c55e")));
  run_pass(&mut runtime);
  let rect = runtime
    .find_element(|element| element.color().is_some())
    .unwrap()
    .bounds();
  let (x, y) = rect.center();

  runtime.mouse_down(x, y, MouseButton::Left);
  runtime.set_root(Column::new().child(Stack::new().size(100.0, 40.0).on_click({
    let clicks = clicks.clone();
    move |_| clicks.update(|count| *count += 1)
  })));
  run_pass(&mut runtime);
  runtime.mouse_up(x, y, MouseButton::Left);

  assert_eq!(clicks.get(), 1);
}
