use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{Tree, events::MouseButton},
  components::{Column, Slider},
  core::Signal,
};

use crate::support::run_pass;

#[test]
fn dragging_slider_does_not_dispatch_click_to_parent() {
  let value = Signal::new(0);
  let parent_clicks = Arc::new(AtomicUsize::new(0));
  let click_counter = parent_clicks.clone();
  let mut runtime = Tree::new();

  runtime.set_root(
    Column::new()
      .width(120.0)
      .height(40.0)
      .on_click(move |_| {
        click_counter.fetch_add(1, Ordering::Relaxed);
      })
      .child(Slider::new(value.clone()).range(0, 10).width(100.0)),
  );
  run_pass(&mut runtime);
  let rect = runtime
    .find_element(|element| element.tag_name() == "Slider")
    .expect("slider should be layoutable")
    .bounds();
  let y = rect.y + rect.height / 2.0;

  runtime.mouse_down(rect.x, y, MouseButton::Left);
  runtime.mouse_move(rect.x + rect.width, y);
  runtime.mouse_up(rect.x + rect.width, y, MouseButton::Left);

  assert_eq!(value.get(), 10);
  assert_eq!(parent_clicks.load(Ordering::Relaxed), 0);
}
