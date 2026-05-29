use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{Tree, component::Component, ctx::Ctx},
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

struct DirtyChild {
  count: Signal<i32>,
  renders: Arc<AtomicUsize>,
}

impl Component for DirtyChild {
  type Props = (Shared<Mutex<Option<Signal<i32>>>>, Shared<AtomicUsize>);

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>().clone();
    let count = ctx.signal(0);
    *props.0.0.lock().unwrap() = Some(count.clone());
    Self {
      count,
      renders: props.1.0,
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    self.renders.fetch_add(1, Ordering::Relaxed);
    lurq::components::Text::new(&format!("{}", self.count.get()))
  }
}

struct Parent {
  child_signal: Arc<Mutex<Option<Signal<i32>>>>,
  child_renders: Arc<AtomicUsize>,
  parent_renders: Arc<AtomicUsize>,
}

impl Component for Parent {
  type Props = (
    Shared<Mutex<Option<Signal<i32>>>>,
    Shared<AtomicUsize>,
    Shared<AtomicUsize>,
  );

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>().clone();
    Self {
      child_signal: props.0.0,
      child_renders: props.1.0,
      parent_renders: props.2.0,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    self.parent_renders.fetch_add(1, Ordering::Relaxed);
    ctx.mount::<DirtyChild>((Shared(self.child_signal.clone()), Shared(self.child_renders.clone())))
  }
}

#[test]
fn child_signal_update_refreshes_child_without_rerendering_parent() {
  let child_signal = Arc::new(Mutex::new(None));
  let child_renders = Arc::new(AtomicUsize::new(0));
  let parent_renders = Arc::new(AtomicUsize::new(0));
  let mut runtime = Tree::new();
  runtime.mount_root::<Parent>((
    Shared(child_signal.clone()),
    Shared(child_renders.clone()),
    Shared(parent_renders.clone()),
  ));

  assert_eq!(parent_renders.load(Ordering::Relaxed), 1);
  assert_eq!(child_renders.load(Ordering::Relaxed), 1);

  child_signal.lock().unwrap().as_ref().unwrap().set(1);
  run_pass(&mut runtime);

  assert_eq!(parent_renders.load(Ordering::Relaxed), 1);
  assert_eq!(child_renders.load(Ordering::Relaxed), 2);
  assert_eq!(runtime.root().unwrap().text_content(), Some("1"));
}
