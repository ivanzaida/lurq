use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::app::{
  App, Tree,
  component::Component,
  ctx::{Ctx, Timeout},
};

#[derive(Clone, lurq::DevtoolsInspectable)]
struct SharedCounter(Arc<AtomicUsize>);

impl PartialEq for SharedCounter {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

struct TimeoutOnce {
  _timeout: Timeout,
}

impl Component for TimeoutOnce {
  type Props = SharedCounter;

  fn create(ctx: &mut Ctx) -> Self {
    let counter = ctx.props::<Self::Props>().0.clone();
    let timeout = ctx.create_timeout(std::time::Duration::ZERO, move || {
      counter.fetch_add(1, Ordering::Relaxed);
    });
    timeout.start();
    Self { _timeout: timeout }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<lurq::node::Element> {
    lurq::node::Element::new()
  }
}

#[test]
fn timeout_fires_once_when_due() {
  let counter = Arc::new(AtomicUsize::new(0));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<TimeoutOnce>(&mut app, SharedCounter(counter.clone()));

  assert_eq!(counter.load(Ordering::Relaxed), 0);

  tree.tick_timers();
  assert_eq!(counter.load(Ordering::Relaxed), 1);

  tree.tick_timers();
  assert_eq!(counter.load(Ordering::Relaxed), 1);
}
