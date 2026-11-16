use std::time::{Duration, Instant};

use lurq::{
  app::{Tree, events::MouseButton},
  components::{Column, Rect},
  core::Signal,
};

use crate::support::{pointer_click, run_pass};

/// Longer than the double-click interval, after which a held click is delivered.
const DOUBLE_CLICK_SETTLE: Duration = Duration::from_millis(550);

/// A tree whose presses are given at `at`, on its clock of presses and not the wall clock's: two presses in a row
/// are at one time however slow the machine, and the test moves the clock on where a person would wait.
fn pressed_at(at: Instant) -> Tree {
  let mut runtime = Tree::new();
  runtime.set_press_clock(Some(at));
  runtime
}

fn runtime_with_click_log(at: Instant) -> (Tree, Signal<Vec<&'static str>>) {
  let events = Signal::new(Vec::new());
  let mut runtime = pressed_at(at);

  runtime.set_root(
    Rect::new(100.0, 40.0)
      .on_click({
        let events = events.clone();
        move |_| events.update(|events| events.push("click"))
      })
      .on_dblclick({
        let events = events.clone();
        move |_| events.update(|events| events.push("dblclick"))
      }),
  );
  run_pass(&mut runtime);

  (runtime, events)
}

#[test]
fn single_click_dispatches_click_after_dblclick_threshold() {
  let start = Instant::now();
  let (mut runtime, events) = runtime_with_click_log(start);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  pointer_click(&mut runtime, x, y, MouseButton::Left);
  assert_eq!(events.get(), Vec::<&'static str>::new());

  runtime.set_press_clock(Some(start + DOUBLE_CLICK_SETTLE));
  run_pass(&mut runtime);
  assert_eq!(events.get(), vec!["click"]);
}

#[test]
fn second_nearby_click_dispatches_only_dblclick_handler() {
  let start = Instant::now();
  let (mut runtime, events) = runtime_with_click_log(start);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  pointer_click(&mut runtime, x, y, MouseButton::Left);
  pointer_click(&mut runtime, x, y, MouseButton::Left);

  assert_eq!(events.get(), vec!["dblclick"]);
}

#[test]
fn child_click_uses_ancestor_dblclick_handler() {
  let events = Signal::new(Vec::new());
  let mut runtime = pressed_at(Instant::now());

  runtime.set_root(
    Column::new()
      .on_dblclick({
        let events = events.clone();
        move |_| events.update(|events| events.push("ancestor-dblclick"))
      })
      .child(Rect::new(100.0, 40.0).background("#22c55e")),
  );
  run_pass(&mut runtime);

  let rect = runtime
    .find_element(|element| element.color().is_some())
    .unwrap()
    .bounds();
  let (x, y) = rect.center();

  pointer_click(&mut runtime, x, y, MouseButton::Left);
  pointer_click(&mut runtime, x, y, MouseButton::Left);

  assert_eq!(events.get(), vec!["ancestor-dblclick"]);
}

#[test]
fn distant_click_does_not_dispatch_dblclick_handler() {
  let start = Instant::now();
  let (mut runtime, events) = runtime_with_click_log(start);
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  pointer_click(&mut runtime, x, y, MouseButton::Left);
  pointer_click(&mut runtime, x + 10.0, y, MouseButton::Left);

  assert_eq!(events.get(), vec!["click"]);

  runtime.set_press_clock(Some(start + DOUBLE_CLICK_SETTLE));
  run_pass(&mut runtime);
  assert_eq!(events.get(), vec!["click", "click"]);
}
