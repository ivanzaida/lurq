use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{Tree, events::MouseButton},
  components::TextInput,
  core::{ElementRef, Signal},
  node::color::Color,
};

use crate::support::{pointer_click, run_pass};

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
          .background("#ef4444")
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
          .background("#22c55e")
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

  pointer_click(
    &mut runtime,
    first.x + 10.0,
    first.y + first.height / 2.0,
    MouseButton::Left,
  );
  pointer_click(
    &mut runtime,
    second.x + 10.0,
    second.y + second.height / 2.0,
    MouseButton::Left,
  );

  assert_eq!(first_focus.load(Ordering::SeqCst), 1);
  assert_eq!(first_blur.load(Ordering::SeqCst), 1);
  assert_eq!(second_focus.load(Ordering::SeqCst), 1);
}

#[test]
fn mouse_down_prevent_default_blocks_text_input_focus() {
  let input_ref = ElementRef::new();
  let focus = Arc::new(AtomicUsize::new(0));
  let mut runtime = Tree::new();

  runtime.set_root(
    TextInput::new(Signal::new(String::new()))
      .width(100.0)
      .ref_element(input_ref.clone())
      .on_mouse_down(|event| event.prevent_default())
      .on_focus({
        let focus = focus.clone();
        move || {
          focus.fetch_add(1, Ordering::SeqCst);
        }
      }),
  );
  run_pass(&mut runtime);
  let rect = input_ref.bounds();

  runtime.mouse_down(rect.x + 10.0, rect.y + rect.height / 2.0, MouseButton::Left);
  runtime.mouse_up(rect.x + 10.0, rect.y + rect.height / 2.0, MouseButton::Left);

  assert!(!input_ref.focused());
  assert_eq!(focus.load(Ordering::SeqCst), 0);
}

#[test]
fn escape_blurs_text_input_outside_form() {
  let blur = Arc::new(AtomicUsize::new(0));
  let input_ref = ElementRef::new();
  let mut runtime = Tree::new();

  runtime.set_root(
    TextInput::new(Signal::new(String::new()))
      .width(100.0)
      .ref_element(input_ref.clone())
      .on_blur({
        let blur = blur.clone();
        move || {
          blur.fetch_add(1, Ordering::SeqCst);
        }
      }),
  );
  run_pass(&mut runtime);
  let rect = input_ref.bounds();

  runtime.mouse_down(rect.x + 10.0, rect.y + rect.height / 2.0, MouseButton::Left);
  assert!(input_ref.active());
  assert!(input_ref.focused());

  runtime.key_down("Escape".to_owned(), "Escape".to_owned(), false, false, false);
  runtime.key_down("A".to_owned(), "KeyA".to_owned(), false, false, false);

  assert!(!input_ref.active());
  assert!(!input_ref.focused());
  assert_eq!(blur.load(Ordering::SeqCst), 1);
}

#[test]
fn enter_blurs_single_line_text_input_outside_form() {
  let value = Signal::new(String::new());
  let blur = Arc::new(AtomicUsize::new(0));
  let input_ref = ElementRef::new();
  let mut runtime = Tree::new();

  runtime.set_root(
    TextInput::new(value.clone())
      .single_line()
      .width(100.0)
      .ref_element(input_ref.clone())
      .on_blur({
        let blur = blur.clone();
        move || {
          blur.fetch_add(1, Ordering::SeqCst);
        }
      }),
  );
  run_pass(&mut runtime);
  let rect = input_ref.bounds();

  runtime.mouse_down(rect.x + 10.0, rect.y + rect.height / 2.0, MouseButton::Left);
  assert!(input_ref.active());
  assert!(input_ref.focused());

  runtime.key_down("Enter".to_owned(), "Enter".to_owned(), false, false, false);
  runtime.key_down("A".to_owned(), "KeyA".to_owned(), false, false, false);

  assert!(!input_ref.active());
  assert!(!input_ref.focused());
  assert_eq!(blur.load(Ordering::SeqCst), 1);
  assert_eq!(value.get(), "");
}
