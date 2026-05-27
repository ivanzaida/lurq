use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{Runtime, component::Component, ctx::Ctx},
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

struct ChildProps {
  value: i32,
  renders: Shared<AtomicUsize>,
}

impl PartialEq for ChildProps {
  fn eq(&self, other: &Self) -> bool {
    self.value == other.value && self.renders == other.renders
  }
}

struct Child {
  renders: Arc<AtomicUsize>,
}

impl Component for Child {
  type Props = ChildProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      renders: ctx.props::<Self::Props>().renders.0.clone(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    self.renders.fetch_add(1, Ordering::Relaxed);
    lurq::components::Text::new(&format!("{}", ctx.props::<Self::Props>().value))
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
    ctx.mount::<Child>(ChildProps {
      value: self.count.get(),
      renders: Shared(self.child_renders.clone()),
    })
  }
}

#[test]
fn changed_child_props_rerender_child() {
  let signal_out = Arc::new(Mutex::new(None));
  let child_renders = Arc::new(AtomicUsize::new(0));
  let mut runtime = Runtime::new();
  runtime.mount_root::<Parent>((Shared(signal_out.clone()), Shared(child_renders.clone())));

  assert_eq!(child_renders.load(Ordering::Relaxed), 1);

  signal_out.lock().unwrap().as_ref().unwrap().set(1);
  run_pass(&mut runtime);

  assert_eq!(child_renders.load(Ordering::Relaxed), 2);
  assert_eq!(runtime.root().unwrap().text_content(), Some("1"));
}
