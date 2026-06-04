use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{Tree, events::MouseButton},
  components::{Button, Column, Form, FormHandle, FormOptions, FormProps, TextInput},
  core::Signal,
};

use crate::support::{pointer_click, run_pass};

#[test]
fn enter_on_submit_button_submits_form() {
  let submits = Arc::new(AtomicUsize::new(0));
  let mut runtime = Tree::new();

  runtime.set_root(Form::element(
    FormProps::new(FormHandle::new(FormOptions::new()).on_submit({
      let submits = submits.clone();
      move |_| {
        submits.fetch_add(1, Ordering::SeqCst);
      }
    })),
    Column::new()
      .child(TextInput::new(Signal::new(String::new())))
      .child(Button::new("Save").submit()),
  ));
  run_pass(&mut runtime);

  runtime.key_down("Tab".to_owned(), "Tab".to_owned(), false, false, false);
  runtime.key_down("Tab".to_owned(), "Tab".to_owned(), false, false, false);
  runtime.key_down("Enter".to_owned(), "Enter".to_owned(), false, false, false);

  assert_eq!(submits.load(Ordering::SeqCst), 1);
}

#[test]
fn clicking_submit_button_submits_form() {
  let submits = Arc::new(AtomicUsize::new(0));
  let mut runtime = Tree::new();

  runtime.set_root(Form::element(
    FormProps::new(FormHandle::new(FormOptions::new()).on_submit({
      let submits = submits.clone();
      move |_| {
        submits.fetch_add(1, Ordering::SeqCst);
      }
    })),
    Column::new().child(Button::new("Save").submit()),
  ));
  run_pass(&mut runtime);

  let button = runtime
    .find_element(|el| el.text_content() == Some("Save"))
    .expect("submit button should render text")
    .bounds();
  pointer_click(
    &mut runtime,
    button.x + button.width / 2.0,
    button.y + button.height / 2.0,
    MouseButton::Left,
  );

  assert_eq!(submits.load(Ordering::SeqCst), 1);
}
