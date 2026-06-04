use std::{
  fmt, future,
  sync::{Arc, Mutex},
};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::{Ctx, FutureAction},
    events::MouseButton,
  },
  components::{Button, Column, Form, FormData, FormErrors, FormHandle, FormOptions, FormProps, Text, validators},
  node::Element,
};

use crate::support::{pointer_click, run_pass};

struct Shared<T>(Arc<T>);

impl<T> fmt::Debug for Shared<T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_tuple("Shared").field(&Arc::as_ptr(&self.0)).finish()
  }
}

impl<T> lurq::app::component::DevtoolsInspectable for Shared<T> {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

impl<T> Clone for Shared<T> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<T> PartialEq for Shared<T> {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

#[derive(Clone, Debug, PartialEq, lurq::DevtoolsInspectable)]
struct AsyncSubmitProps {
  #[devtools_ignore]
  form_out: Shared<Mutex<Option<FormHandle>>>,
  #[devtools_ignore]
  action_out: Shared<Mutex<Option<FutureAction<lurq::components::FormValues, (), FormErrors>>>>,
}

struct RejectingSubmitForm {
  form: FormHandle,
}

impl Component for RejectingSubmitForm {
  type Props = AsyncSubmitProps;

  fn create(ctx: &mut Ctx) -> Self {
    let form = ctx.form(
      FormOptions::new()
        .field("email", "")
        .validate_string("email", validators::required("Email is required")),
    );
    *ctx.props::<Self::Props>().form_out.0.lock().unwrap() = Some(form.clone());
    Self { form }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let action = ctx.future_action(|_values: lurq::components::FormValues| async move {
      Err::<(), _>(FormErrors::new().with("email", "Email already exists"))
    });
    *ctx.props::<Self::Props>().action_out.0.lock().unwrap() = Some(action);

    let submitting = self.form.submitting().get();
    let error = self.form.error("email").get().unwrap_or_else(|| Arc::from(""));
    Text::new(&format!("submitting={submitting} error={error}"))
  }
}

#[test]
fn submit_action_sets_submitting_and_maps_rejected_form_errors() {
  let form_out = Arc::new(Mutex::new(None));
  let action_out = Arc::new(Mutex::new(None));
  let mut tree = Tree::new();
  tree.mount_root::<RejectingSubmitForm>(
    &mut App::new(),
    AsyncSubmitProps {
      form_out: Shared(form_out.clone()),
      action_out: Shared(action_out.clone()),
    },
  );

  let form = form_out.lock().unwrap().clone().unwrap();
  let action = action_out.lock().unwrap().clone().unwrap();
  let mut data = FormData::new();
  data.append("email", "taken@example.com");

  assert!(form.submit_action(data, &action));
  assert!(form.is_submitting());

  run_pass(&mut tree);
  assert_eq!(tree.root().unwrap().text_content(), Some("submitting=true error="));

  tree.tick_futures();
  run_pass(&mut tree);

  assert!(!form.is_submitting());
  assert_eq!(form.errors().first("email"), Some("Email already exists"));
  assert_eq!(
    tree.root().unwrap().text_content(),
    Some("submitting=false error=Email already exists")
  );
}

#[test]
fn submit_action_rejects_client_invalid_and_does_not_start_submitting() {
  let form_out = Arc::new(Mutex::new(None));
  let action_out = Arc::new(Mutex::new(None));
  let mut tree = Tree::new();
  tree.mount_root::<RejectingSubmitForm>(
    &mut App::new(),
    AsyncSubmitProps {
      form_out: Shared(form_out.clone()),
      action_out: Shared(action_out.clone()),
    },
  );

  let form = form_out.lock().unwrap().clone().unwrap();
  let action = action_out.lock().unwrap().clone().unwrap();

  assert!(!form.submit_action(FormData::new(), &action));
  assert!(!form.is_submitting());
  assert_eq!(form.errors().first("email"), Some("Email is required"));
}

struct PendingSubmitForm {
  form: FormHandle,
}

impl Component for PendingSubmitForm {
  type Props = AsyncSubmitProps;

  fn create(ctx: &mut Ctx) -> Self {
    let form = ctx.form(FormOptions::new().field("email", "ada@example.com"));
    *ctx.props::<Self::Props>().form_out.0.lock().unwrap() = Some(form.clone());
    Self { form }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let action = ctx.future_action(|_values: lurq::components::FormValues| async move {
      future::pending::<Result<(), FormErrors>>().await
    });
    *ctx.props::<Self::Props>().action_out.0.lock().unwrap() = Some(action);

    Text::new(if self.form.submitting().get() {
      "submitting"
    } else {
      "idle"
    })
  }
}

#[test]
fn submit_action_blocks_duplicate_submits_while_pending() {
  let form_out = Arc::new(Mutex::new(None));
  let action_out = Arc::new(Mutex::new(None));
  let mut tree = Tree::new();
  tree.mount_root::<PendingSubmitForm>(
    &mut App::new(),
    AsyncSubmitProps {
      form_out: Shared(form_out.clone()),
      action_out: Shared(action_out.clone()),
    },
  );

  let form = form_out.lock().unwrap().clone().unwrap();
  let action = action_out.lock().unwrap().clone().unwrap();

  assert!(form.submit_action(FormData::new(), &action));
  assert!(!form.submit_action(FormData::new(), &action));
  assert!(form.is_submitting());
}

struct MountedActionForm {
  form: FormHandle,
}

impl Component for MountedActionForm {
  type Props = AsyncSubmitProps;

  fn create(ctx: &mut Ctx) -> Self {
    let form = ctx.form(FormOptions::new().field("email", "taken@example.com"));
    *ctx.props::<Self::Props>().form_out.0.lock().unwrap() = Some(form.clone());
    Self { form }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let action = ctx.future_action(|_values: lurq::components::FormValues| async move {
      Err::<(), _>(FormErrors::new().with("email", "Email already exists"))
    });
    *ctx.props::<Self::Props>().action_out.0.lock().unwrap() = Some(action.clone());

    Form::mount(
      ctx,
      FormProps::new(self.form.clone()).submit_action(action),
      Column::new()
        .child(Text::new("status"))
        .child(Button::new("Save").submit()),
    )
  }
}

#[test]
fn form_props_submit_action_wires_submit_buttons_to_action() {
  let form_out = Arc::new(Mutex::new(None));
  let action_out = Arc::new(Mutex::new(None));
  let mut tree = Tree::new();
  tree.mount_root::<MountedActionForm>(
    &mut App::new(),
    AsyncSubmitProps {
      form_out: Shared(form_out.clone()),
      action_out: Shared(action_out),
    },
  );
  run_pass(&mut tree);

  let button = tree
    .find_element(|el| el.text_content() == Some("Save"))
    .expect("submit button should render text")
    .bounds();
  pointer_click(
    &mut tree,
    button.x + button.width / 2.0,
    button.y + button.height / 2.0,
    MouseButton::Left,
  );

  let form = form_out.lock().unwrap().clone().unwrap();
  assert!(form.is_submitting());

  tree.tick_futures();

  assert!(!form.is_submitting());
  assert_eq!(form.errors().first("email"), Some("Email already exists"));
}
