use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{Tree, component::Component, ctx::Ctx, theme::Theme},
  core::Signal,
  node::Element,
};

use crate::support::run_pass;

#[derive(lurq::DevtoolsInspectable)]
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

impl<T> std::fmt::Debug for Shared<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("Shared").field(&(Arc::as_ptr(&self.0) as usize)).finish()
  }
}

struct CleanChild {
  renders: Arc<AtomicUsize>,
}

impl Component for CleanChild {
  type Props = Shared<AtomicUsize>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      renders: ctx.props::<Self::Props>().0.clone(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    self.renders.fetch_add(1, Ordering::Relaxed);
    Element::new()
  }
}

struct Parent {
  count: Signal<i32>,
  child_renders: Arc<AtomicUsize>,
}

impl Component for Parent {
  type Props = (Shared<Mutex<Option<Signal<i32>>>>, Shared<AtomicUsize>);

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>().clone();
    let count = ctx.signal(0);
    *props.0.0.lock().unwrap() = Some(count.clone());
    Self {
      count,
      child_renders: props.1.0,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Column::new()
      .child(lurq::components::Text::new(&format!("{}", self.count.get())))
      .child(ctx.mount::<CleanChild>(Shared(self.child_renders.clone())))
  }
}

#[test]
fn parent_signal_update_does_not_rerender_clean_child_with_same_props() {
  let signal_out = Arc::new(Mutex::new(None));
  let child_renders = Arc::new(AtomicUsize::new(0));
  let mut runtime = Tree::new();
  runtime.mount_root::<Parent>(
    Theme::default(),
    (Shared(signal_out.clone()), Shared(child_renders.clone())),
  );

  assert_eq!(child_renders.load(Ordering::Relaxed), 1);

  signal_out.lock().unwrap().as_ref().unwrap().set(1);
  run_pass(&mut runtime);

  assert_eq!(child_renders.load(Ordering::Relaxed), 1);
  let root = runtime.root().unwrap();
  assert_eq!(root.children().iter().next().unwrap().text_content(), Some("1"));
}
