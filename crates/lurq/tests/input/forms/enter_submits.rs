use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::events::MouseButton,
  components::{Column, Form, FormHandle, FormOptions, FormProps, TextInput},
  core::{ElementRef, Signal},
};

use crate::support::run_pass;

#[test]
fn enter_submits_nearest_form_from_single_line_text_input() {
  let submits = Arc::new(AtomicUsize::new(0));
  let mut runtime = lurq::app::Tree::new();

  runtime.set_root(Form::element(
    FormProps::new(FormHandle::new(FormOptions::new()).on_submit({
      let submits = submits.clone();
      move |_| {
        submits.fetch_add(1, Ordering::SeqCst);
      }
    })),
    Column::new().child(TextInput::new(Signal::new(String::new())).single_line()),
  ));
  run_pass(&mut runtime);

  runtime.key_down("Tab".to_owned(), "Tab".to_owned(), false, false, false);
  runtime.key_down("Enter".to_owned(), "Enter".to_owned(), false, false, false);

  assert_eq!(submits.load(Ordering::SeqCst), 1);
}

#[test]
fn enter_in_multiline_text_input_does_not_submit_form() {
  let submits = Arc::new(AtomicUsize::new(0));
  let value = Signal::new(String::new());
  let mut runtime = lurq::app::Tree::new();

  runtime.set_root(Form::element(
    FormProps::new(FormHandle::new(FormOptions::new()).on_submit({
      let submits = submits.clone();
      move |_| {
        submits.fetch_add(1, Ordering::SeqCst);
      }
    })),
    Column::new().child(TextInput::new(value.clone()).multiline()),
  ));
  run_pass(&mut runtime);

  runtime.key_down("Tab".to_owned(), "Tab".to_owned(), false, false, false);
  runtime.key_down("Enter".to_owned(), "Enter".to_owned(), false, false, false);

  assert_eq!(submits.load(Ordering::SeqCst), 0);
  assert_eq!(value.get(), "\n");
}

#[test]
fn escape_blurs_text_input_inside_form() {
  let blur = Arc::new(AtomicUsize::new(0));
  let input_ref = ElementRef::new();
  let mut runtime = lurq::app::Tree::new();

  runtime.set_root(Form::element(
    FormProps::new(FormHandle::new(FormOptions::new())),
    Column::new().child(
      TextInput::new(Signal::new(String::new()))
        .single_line()
        .ref_element(input_ref.clone())
        .on_blur({
          let blur = blur.clone();
          move || {
            blur.fetch_add(1, Ordering::SeqCst);
          }
        }),
    ),
  ));
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
