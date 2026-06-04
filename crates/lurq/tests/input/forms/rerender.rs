use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{FormData, FormHandle, FormOptions, Text, validators},
  core::Signal,
  node::Element,
};

use crate::support::run_pass;

#[derive(Debug, lurq::DevtoolsInspectable)]
struct Shared<T>(Arc<T>);

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
struct FieldRenderProps {
  #[devtools_ignore]
  value_out: Shared<Mutex<Option<Signal<String>>>>,
  #[devtools_ignore]
  renders: Shared<AtomicUsize>,
}

struct FieldRenderCounter {
  form: FormHandle,
  value: Signal<String>,
  renders: Arc<AtomicUsize>,
}

impl Component for FieldRenderCounter {
  type Props = FieldRenderProps;

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>().clone();
    let form = ctx.form(FormOptions::new().field("name", "Ada"));
    let name = form.string_field("name");
    let value = name.value();
    *props.value_out.0.lock().unwrap() = Some(value.clone());
    Self {
      form,
      value,
      renders: props.renders.0,
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    self.renders.fetch_add(1, Ordering::Relaxed);
    let dirty = self.form.dirty("name").get();
    Text::new(&format!("{} dirty={dirty}", self.value.get()))
  }
}

#[test]
fn value_change_with_derived_dirty_state_rerenders_once() {
  let value_out = Arc::new(Mutex::new(None));
  let renders = Arc::new(AtomicUsize::new(0));
  let mut tree = Tree::new();
  tree.mount_root::<FieldRenderCounter>(
    &mut App::new(),
    FieldRenderProps {
      value_out: Shared(value_out.clone()),
      renders: Shared(renders.clone()),
    },
  );

  assert_eq!(renders.load(Ordering::Relaxed), 1);

  value_out.lock().unwrap().as_ref().unwrap().set("Grace".to_owned());
  run_pass(&mut tree);

  assert_eq!(renders.load(Ordering::Relaxed), 2);
  assert_eq!(tree.root().unwrap().text_content(), Some("Grace dirty=true"));

  run_pass(&mut tree);

  assert_eq!(renders.load(Ordering::Relaxed), 2);
}

struct UnsubscribedFieldRenderCounter {
  renders: Arc<AtomicUsize>,
}

impl Component for UnsubscribedFieldRenderCounter {
  type Props = FieldRenderProps;

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>().clone();
    let form = ctx.form(FormOptions::new().field("name", "Ada"));
    let value = form.string("name");
    let _dirty = form.dirty("name");
    *props.value_out.0.lock().unwrap() = Some(value.clone());
    Self {
      renders: props.renders.0,
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    self.renders.fetch_add(1, Ordering::Relaxed);
    Text::new("static")
  }
}

#[test]
fn unsubscribed_field_and_dirty_updates_do_not_rerender() {
  let value_out = Arc::new(Mutex::new(None));
  let renders = Arc::new(AtomicUsize::new(0));
  let mut tree = Tree::new();
  tree.mount_root::<UnsubscribedFieldRenderCounter>(
    &mut App::new(),
    FieldRenderProps {
      value_out: Shared(value_out.clone()),
      renders: Shared(renders.clone()),
    },
  );

  assert_eq!(renders.load(Ordering::Relaxed), 1);

  value_out.lock().unwrap().as_ref().unwrap().set("Grace".to_owned());
  run_pass(&mut tree);

  assert_eq!(renders.load(Ordering::Relaxed), 1);
  assert_eq!(tree.root().unwrap().text_content(), Some("static"));
}

#[derive(Clone, Debug, PartialEq, lurq::DevtoolsInspectable)]
struct SubmitRenderProps {
  #[devtools_ignore]
  form_out: Shared<Mutex<Option<FormHandle>>>,
  #[devtools_ignore]
  renders: Shared<AtomicUsize>,
}

struct SubmitRenderCounter {
  form: FormHandle,
  renders: Arc<AtomicUsize>,
}

impl Component for SubmitRenderCounter {
  type Props = SubmitRenderProps;

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>().clone();
    let form = ctx.form(
      FormOptions::new()
        .field("email", "")
        .validate_string("email", validators::required("Email is required")),
    );
    *props.form_out.0.lock().unwrap() = Some(form.clone());
    Self {
      form,
      renders: props.renders.0,
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    self.renders.fetch_add(1, Ordering::Relaxed);
    let attempted = self.form.submit_attempted().get();
    let touched = self.form.touched("email").get();
    let error = self.form.error("email").get().unwrap_or_else(|| Arc::from(""));
    Text::new(&format!("attempted={attempted} touched={touched} error={error}"))
  }
}

#[test]
fn submit_state_updates_rerender_once() {
  let form_out = Arc::new(Mutex::new(None));
  let renders = Arc::new(AtomicUsize::new(0));
  let mut tree = Tree::new();
  tree.mount_root::<SubmitRenderCounter>(
    &mut App::new(),
    SubmitRenderProps {
      form_out: Shared(form_out.clone()),
      renders: Shared(renders.clone()),
    },
  );

  assert_eq!(renders.load(Ordering::Relaxed), 1);

  let mut data = FormData::new();
  data.append("email", "");
  form_out.lock().unwrap().as_ref().unwrap().submit(data);
  run_pass(&mut tree);

  assert_eq!(renders.load(Ordering::Relaxed), 2);
  assert_eq!(
    tree.root().unwrap().text_content(),
    Some("attempted=true touched=true error=Email is required")
  );

  run_pass(&mut tree);

  assert_eq!(renders.load(Ordering::Relaxed), 2);
}

struct UnsubscribedFormStateRenderCounter {
  renders: Arc<AtomicUsize>,
}

impl Component for UnsubscribedFormStateRenderCounter {
  type Props = SubmitRenderProps;

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>().clone();
    let form = ctx.form(
      FormOptions::new()
        .field("email", "")
        .validate_string("email", validators::required("Email is required")),
    );
    let _error = form.error("email");
    let _touched = form.touched("email");
    let _attempted = form.submit_attempted();
    *props.form_out.0.lock().unwrap() = Some(form.clone());
    Self {
      renders: props.renders.0,
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    self.renders.fetch_add(1, Ordering::Relaxed);
    Text::new("static")
  }
}

#[test]
fn unsubscribed_error_touched_and_submit_state_do_not_rerender() {
  let form_out = Arc::new(Mutex::new(None));
  let renders = Arc::new(AtomicUsize::new(0));
  let mut tree = Tree::new();
  tree.mount_root::<UnsubscribedFormStateRenderCounter>(
    &mut App::new(),
    SubmitRenderProps {
      form_out: Shared(form_out.clone()),
      renders: Shared(renders.clone()),
    },
  );

  assert_eq!(renders.load(Ordering::Relaxed), 1);

  let form = form_out.lock().unwrap().as_ref().unwrap().clone();
  form.set_error("email", "Server error");
  form.mark_touched("email");
  let mut data = FormData::new();
  data.append("email", "");
  form.submit(data);
  run_pass(&mut tree);

  assert_eq!(renders.load(Ordering::Relaxed), 1);
  assert_eq!(tree.root().unwrap().text_content(), Some("static"));
}
