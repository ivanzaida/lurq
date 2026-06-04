use std::sync::{Arc, Mutex};

use lurq::{
  components::{Column, Form, FormHandle, FormOptions, FormProps, FormValue, FormValues, TextInput},
};

use crate::support::run_pass;

#[test]
fn field_defaults_prefill_registered_field_signals() {
  let form = FormHandle::new(
    FormOptions::new()
      .field("name", "Ada")
      .field("active", true)
      .field("score", 42.0),
  );

  assert_eq!(form.string("name").get(), "Ada");
  assert!(form.bool("active").get());
  assert_eq!(form.number("score").get(), 42.0);
}

#[test]
fn default_values_collection_prefills_registered_field_signals() {
  let defaults = FormValues::new()
    .with("name", "Ada")
    .with("active", true)
    .with("score", 42.0);
  let form = FormHandle::new(FormOptions::new().default(defaults));

  assert_eq!(form.string("name").get(), "Ada");
  assert!(form.bool("active").get());
  assert_eq!(form.number("score").get(), 42.0);
}

#[test]
fn prefilled_text_input_renders_default_value() {
  let form = FormHandle::new(FormOptions::new().field("name", "Ada"));
  let mut runtime = lurq::app::Tree::new();

  runtime.set_root(Form::element(
    FormProps::new(form.clone()),
    Column::new().child(TextInput::new(form.string("name")).name("name")),
  ));
  run_pass(&mut runtime);

  assert!(
    runtime
      .find_element(|element| element.text_content() == Some("Ada"))
      .is_some()
  );
}

#[test]
fn submit_includes_default_values_before_user_edits() {
  let submitted = Arc::new(Mutex::new(None::<FormValues>));
  let form = FormHandle::new(
    FormOptions::new()
      .field("name", "Ada")
      .field("role", FormValue::from("admin")),
  )
  .on_submit({
    let submitted = submitted.clone();
    move |values| {
      *submitted.lock().unwrap() = Some(values);
    }
  });
  let mut runtime = lurq::app::Tree::new();

  runtime.set_root(Form::element(
    FormProps::new(form.clone()),
    Column::new().child(TextInput::new(form.string("name")).name("name").single_line()),
  ));
  run_pass(&mut runtime);

  runtime.key_down("Tab".to_owned(), "Tab".to_owned(), false, false, false);
  runtime.key_down("Enter".to_owned(), "Enter".to_owned(), false, false, false);

  let values = submitted.lock().unwrap().clone().expect("form should submit values");
  assert_eq!(values.get_string("name"), Some("Ada"));
  assert_eq!(values.get_string("role"), Some("admin"));
}

#[test]
fn submit_uses_current_field_signal_over_original_default() {
  let submitted = Arc::new(Mutex::new(None::<FormValues>));
  let form = FormHandle::new(FormOptions::new().field("name", "Ada")).on_submit({
    let submitted = submitted.clone();
    move |values| {
      *submitted.lock().unwrap() = Some(values);
    }
  });
  let name = form.string("name");
  name.set("Grace".to_owned());
  let mut runtime = lurq::app::Tree::new();

  runtime.set_root(Form::element(
    FormProps::new(form),
    Column::new().child(TextInput::new(name).name("name").single_line()),
  ));
  run_pass(&mut runtime);

  runtime.key_down("Tab".to_owned(), "Tab".to_owned(), false, false, false);
  runtime.key_down("Enter".to_owned(), "Enter".to_owned(), false, false, false);

  let values = submitted.lock().unwrap().clone().expect("form should submit values");
  assert_eq!(values.get_string("name"), Some("Grace"));
}
