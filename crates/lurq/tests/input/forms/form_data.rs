use std::sync::{Arc, Mutex};

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Form, FormHandle, FormOptions, FormProps, FormValues, TextInput},
  core::Signal,
  node::Element,
};

use crate::support::run_pass;

#[test]
fn submit_collects_named_text_input_values() {
  let submitted = Arc::new(Mutex::new(None::<FormValues>));
  let mut runtime = lurq::app::Tree::new();

  runtime.set_root(Form::element(
    FormProps::new(FormHandle::new(FormOptions::new()).on_submit({
      let submitted = submitted.clone();
      move |values| {
        *submitted.lock().unwrap() = Some(values);
      }
    })),
    Column::new()
      .child(TextInput::new(Signal::new("Ada".to_owned())).name("user"))
      .child(TextInput::new(Signal::new("ada@example.com".to_owned())).name("email"))
      .child(TextInput::new(Signal::new("ignored".to_owned()))),
  ));
  run_pass(&mut runtime);

  runtime.key_down("Tab".to_owned(), "Tab".to_owned(), false, false, false);
  runtime.key_down("Enter".to_owned(), "Enter".to_owned(), false, false, false);

  let values = submitted.lock().unwrap().clone().expect("form should submit data");
  assert_eq!(values.get_string("user"), Some("Ada"));
  assert_eq!(values.get_string("email"), Some("ada@example.com"));
  assert_eq!(values.get("ignored"), None);
  assert_eq!(values.len(), 2);
}

#[derive(Clone)]
struct Submitted(Arc<Mutex<Option<FormValues>>>);

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for Submitted {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

impl PartialEq for Submitted {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

struct MountedFormRoot {
  form: FormHandle,
}

impl Component for MountedFormRoot {
  type Props = Submitted;

  fn create(ctx: &mut Ctx) -> Self {
    let submitted = ctx.props::<Self::Props>().clone();
    let form = ctx.form(FormOptions::new()).on_submit(move |values| {
      *submitted.0.lock().unwrap() = Some(values);
    });
    Self { form }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    Form::mount(
      ctx,
      FormProps::new(self.form.clone()),
      Column::new().child(TextInput::new(Signal::new("Ada".to_owned())).name("user")),
    )
  }
}

#[test]
fn mounted_form_component_submits_form_data() {
  let submitted = Arc::new(Mutex::new(None::<FormValues>));
  let mut runtime = lurq::app::Tree::new();

  runtime.mount_root::<MountedFormRoot>(&mut lurq::app::App::new(), Submitted(submitted.clone()));
  run_pass(&mut runtime);

  runtime.key_down("Tab".to_owned(), "Tab".to_owned(), false, false, false);
  runtime.key_down("Enter".to_owned(), "Enter".to_owned(), false, false, false);

  let values = submitted.lock().unwrap().clone().expect("form should submit data");
  assert_eq!(values.get_string("user"), Some("Ada"));
}
