use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  components::{Column, Form, FormHandle, FormOptions, FormProps, TextInput},
  core::Signal,
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
