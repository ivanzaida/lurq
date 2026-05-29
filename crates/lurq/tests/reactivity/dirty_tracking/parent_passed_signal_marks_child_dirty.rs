use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{Tree, component::Component, ctx::Ctx, theme::Theme},
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

#[derive(Clone)]
struct SignalProp(Signal<i32>);

impl PartialEq for SignalProp {
  fn eq(&self, _other: &Self) -> bool {
    true
  }
}

struct ChildProps {
  signal: SignalProp,
  renders: Shared<AtomicUsize>,
}

impl PartialEq for ChildProps {
  fn eq(&self, other: &Self) -> bool {
    self.signal == other.signal && self.renders == other.renders
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
    let value = ctx.props::<Self::Props>().signal.0.get();
    lurq::components::Text::new(&format!("{value}"))
  }
}

struct Parent {
  signal: Signal<i32>,
  parent_renders: Arc<AtomicUsize>,
  child_renders: Arc<AtomicUsize>,
}

impl Component for Parent {
  type Props = (SignalProp, Shared<AtomicUsize>, Shared<AtomicUsize>);

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>().clone();
    Self {
      signal: props.0.0,
      parent_renders: props.1.0,
      child_renders: props.2.0,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    self.parent_renders.fetch_add(1, Ordering::Relaxed);
    ctx.mount::<Child>(ChildProps {
      signal: SignalProp(self.signal.clone()),
      renders: Shared(self.child_renders.clone()),
    })
  }
}

#[test]
fn signal_passed_from_parent_marks_child_dirty_when_child_reads_it() {
  let signal = Signal::new(0);
  let parent_renders = Arc::new(AtomicUsize::new(0));
  let child_renders = Arc::new(AtomicUsize::new(0));
  let mut runtime = Tree::new();
  runtime.mount_root::<Parent>(
    Theme::default(),
    (
      SignalProp(signal.clone()),
      Shared(parent_renders.clone()),
      Shared(child_renders.clone()),
    ),
  );

  assert_eq!(parent_renders.load(Ordering::Relaxed), 1);
  assert_eq!(child_renders.load(Ordering::Relaxed), 1);

  signal.set(1);
  run_pass(&mut runtime);

  assert_eq!(parent_renders.load(Ordering::Relaxed), 1);
  assert_eq!(child_renders.load(Ordering::Relaxed), 2);
  assert_eq!(runtime.root().unwrap().text_content(), Some("1"));
}
