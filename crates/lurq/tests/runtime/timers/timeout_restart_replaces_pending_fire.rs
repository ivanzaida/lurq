use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::app::{
  App, Tree,
  component::Component,
  ctx::{Ctx, Timeout},
};

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

struct RestartableTimeout;

impl Component for RestartableTimeout {
  type Props = (Shared<AtomicUsize>, Shared<Mutex<Option<Timeout>>>);

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>().clone();
    let counter = props.0.0.clone();
    let timeout = ctx.create_timeout(std::time::Duration::ZERO, move || {
      counter.fetch_add(1, Ordering::Relaxed);
    });
    *props.1.0.lock().unwrap() = Some(timeout);
    Self
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<lurq::node::Element> {
    lurq::node::Element::new()
  }
}

#[test]
fn timeout_restart_replaces_pending_fire() {
  let counter = Arc::new(AtomicUsize::new(0));
  let timeout = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<RestartableTimeout>(&mut app, (Shared(counter.clone()), Shared(timeout.clone())));

  let timeout = timeout.lock().unwrap().clone().unwrap();
  timeout.start();
  timeout.restart();

  tree.tick_timers();
  assert_eq!(counter.load(Ordering::Relaxed), 1);

  tree.tick_timers();
  assert_eq!(counter.load(Ordering::Relaxed), 1);
}
