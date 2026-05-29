use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{Tree, events::MouseButton},
  core::Signal,
  node::color::Color,
};

use crate::support::run_pass;

#[test]
fn clicking_text_inputs_moves_focus_and_fires_focus_and_blur() {
  let first_focus = Arc::new(AtomicUsize::new(0));
  let first_blur = Arc::new(AtomicUsize::new(0));
  let second_focus = Arc::new(AtomicUsize::new(0));

  let mut runtime = Tree::new();
  runtime.set_root(
    lurq::components::Row::new()
      .spacing(8.0)
      .child(
        lurq::components::TextInput::new(Signal::new(String::new()))
          .width(100.0)
          .fill("#ef4444")
          .on_focus({
            let first_focus = first_focus.clone();
            move || {
              first_focus.fetch_add(1, Ordering::SeqCst);
            }
          })
          .on_blur({
            let first_blur = first_blur.clone();
            move || {
              first_blur.fetch_add(1, Ordering::SeqCst);
            }
          }),
      )
      .child(
        lurq::components::TextInput::new(Signal::new(String::new()))
          .width(100.0)
          .fill("#22c55e")
          .on_focus({
            let second_focus = second_focus.clone();
            move || {
              second_focus.fetch_add(1, Ordering::SeqCst);
            }
          }),
      ),
  );
  run_pass(&mut runtime);

  let first = runtime
    .find_element(|el| el.color() == Some(Color::from_hex("#ef4444")))
    .unwrap()
    .bounds();
  let second = runtime
    .find_element(|el| el.color() == Some(Color::from_hex("#22c55e")))
    .unwrap()
    .bounds();

  runtime.click(first.x + 10.0, first.y + first.height / 2.0, MouseButton::Left);
  runtime.click(second.x + 10.0, second.y + second.height / 2.0, MouseButton::Left);

  assert_eq!(first_focus.load(Ordering::SeqCst), 1);
  assert_eq!(first_blur.load(Ordering::SeqCst), 1);
  assert_eq!(second_focus.load(Ordering::SeqCst), 1);
}
