use lurq::{
  app::{Tree, events::MouseButton},
  components::Rect,
  core::Signal,
};

use crate::support::run_pass;

const DOUBLE_CLICK_SETTLE_MS: u64 = 550;

fn runtime_with_click_log() -> (Tree, Signal<Vec<&'static str>>) {
  let events = Signal::new(Vec::new());
  let mut runtime = Tree::new();

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
  let (mut runtime, events) = runtime_with_click_log();
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);
  assert_eq!(events.get(), Vec::<&'static str>::new());

  std::thread::sleep(std::time::Duration::from_millis(DOUBLE_CLICK_SETTLE_MS));
  run_pass(&mut runtime);
  assert_eq!(events.get(), vec!["click"]);
}

#[test]
fn second_nearby_click_dispatches_only_dblclick_handler() {
  let (mut runtime, events) = runtime_with_click_log();
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);
  runtime.click(x, y, MouseButton::Left);

  assert_eq!(events.get(), vec!["dblclick"]);
}

#[test]
fn distant_click_does_not_dispatch_dblclick_handler() {
  let (mut runtime, events) = runtime_with_click_log();
  let rect = runtime.find_element(|_| true).unwrap().bounds();
  let (x, y) = rect.center();

  runtime.click(x, y, MouseButton::Left);
  runtime.click(x + 10.0, y, MouseButton::Left);

  assert_eq!(events.get(), vec!["click"]);

  std::thread::sleep(std::time::Duration::from_millis(DOUBLE_CLICK_SETTLE_MS));
  run_pass(&mut runtime);
  assert_eq!(events.get(), vec!["click", "click"]);
}
