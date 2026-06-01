use lurq::{
  app::{Tree, events::MouseButton},
  core::Signal,
};

use crate::support::run_pass;

#[test]
fn release_over_click_target_does_not_click_when_press_started_elsewhere() {
  let clicks = Signal::new(0);
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::Rect::new(100.0, 40.0).fill("#22c55e").on_click({
    let clicks = clicks.clone();
    move |_| clicks.update(|count| *count += 1)
  }));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.mouse_down(x + rect.width + 20.0, y, MouseButton::Left);
  runtime.mouse_up(x, y, MouseButton::Left);
  runtime.click(x, y, MouseButton::Left);

  assert_eq!(clicks.get(), 0);
}

#[test]
fn release_far_from_press_does_not_click_even_on_same_target() {
  let clicks = Signal::new(0);
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::Rect::new(100.0, 40.0).fill("#22c55e").on_click({
    let clicks = clicks.clone();
    move |_| clicks.update(|count| *count += 1)
  }));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let y = rect.y + rect.height / 2.0;

  runtime.mouse_down(rect.x + 10.0, y, MouseButton::Left);
  runtime.mouse_up(rect.x + 90.0, y, MouseButton::Left);
  runtime.click(rect.x + 90.0, y, MouseButton::Left);

  assert_eq!(clicks.get(), 0);
}

#[test]
fn release_near_press_clicks_same_target() {
  let clicks = Signal::new(0);
  let mut runtime = Tree::new();

  runtime.set_root(lurq::components::Rect::new(100.0, 40.0).fill("#22c55e").on_click({
    let clicks = clicks.clone();
    move |_| clicks.update(|count| *count += 1)
  }));
  run_pass(&mut runtime);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.mouse_down(x, y, MouseButton::Left);
  runtime.mouse_up(x + 2.0, y, MouseButton::Left);
  runtime.click(x + 2.0, y, MouseButton::Left);

  assert_eq!(clicks.get(), 1);
}
