use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::app::{
  App, Tree,
  component::Component,
  ctx::{Ctx, Interval},
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

struct RepeatingInterval;

impl Component for RepeatingInterval {
  type Props = (Shared<AtomicUsize>, Shared<Mutex<Option<Interval>>>);

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>().clone();
    let counter = props.0.0.clone();
    let interval = ctx.create_interval(std::time::Duration::ZERO, move || {
      counter.fetch_add(1, Ordering::Relaxed);
    });
    interval.start();
    *props.1.0.lock().unwrap() = Some(interval);
    Self
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<lurq::node::Element> {
    lurq::node::Element::new()
  }
}

#[test]
fn interval_repeats_until_stopped() {
  let counter = Arc::new(AtomicUsize::new(0));
  let interval = Arc::new(Mutex::new(None));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<RepeatingInterval>(&mut app, (Shared(counter.clone()), Shared(interval.clone())));

  tree.tick_timers();
  assert_eq!(counter.load(Ordering::Relaxed), 1);

  tree.tick_timers();
  assert_eq!(counter.load(Ordering::Relaxed), 2);

  interval.lock().unwrap().as_ref().unwrap().stop();
  tree.tick_timers();
  assert_eq!(counter.load(Ordering::Relaxed), 2);
}
