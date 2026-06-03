use std::sync::{Arc, Mutex};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Column, Text},
  core::Signal,
  node::Element,
};

use crate::support::run_pass;

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

struct Root;

impl Component for Root {
  type Props = Shared<Mutex<Option<Signal<bool>>>>;

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    Column::new().child(ctx.mount::<ModalChild>(ctx.props::<Self::Props>().clone()))
  }
}

struct ModalChild {
  open: Signal<bool>,
}

impl Component for ModalChild {
  type Props = Shared<Mutex<Option<Signal<bool>>>>;

  fn create(ctx: &mut Ctx) -> Self {
    let open = ctx.signal(false);
    *ctx.props::<Self::Props>().0.lock().unwrap() = Some(open.clone());
    Self { open }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    ctx.modal(self.open.clone(), |ctx| ctx.mount::<ModalPanel>(()));
    Text::new("child")
  }
}

struct ModalPanel;

impl Component for ModalPanel {
  type Props = ();

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    assert!(ctx.modal_context().expect("modal context should be set").is_open());
    Text::new("modal")
  }
}

#[test]
fn ctx_modal_renders_declared_modal_above_root() {
  let open = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<Root>(&mut app, Shared(open.clone()));

  open.lock().unwrap().as_ref().unwrap().set(true);
  run_pass(&mut tree);

  let root = tree.root().unwrap();
  assert_eq!(root.tag_name(), "__lurq_modal_host");
  assert_eq!(root.children().len(), 2);
  assert!(
    root
      .children()
      .iter()
      .any(|child| child.text_content() == Some("modal"))
  );
}

#[test]
fn ctx_modal_removes_modal_when_declaring_component_stops_rendering_it() {
  let open = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<Root>(&mut app, Shared(open.clone()));

  let signal = open.lock().unwrap().as_ref().unwrap().clone();
  signal.set(true);
  run_pass(&mut tree);
  assert_eq!(tree.root().unwrap().tag_name(), "__lurq_modal_host");

  signal.set(false);
  run_pass(&mut tree);

  let root = tree.root().unwrap();
  assert_ne!(root.tag_name(), "__lurq_modal_host");
  assert!(
    root
      .children()
      .iter()
      .all(|child| child.text_content() != Some("modal"))
  );
}
