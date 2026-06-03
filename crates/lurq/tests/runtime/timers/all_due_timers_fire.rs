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

struct TwoTimeouts {
  _first: Timeout,
  _second: Timeout,
}

impl Component for TwoTimeouts {
  type Props = SharedCounter;

  fn create(ctx: &mut Ctx) -> Self {
    let counter = ctx.props::<Self::Props>().0.clone();
    let first = ctx.create_timeout(std::time::Duration::ZERO, {
      let counter = counter.clone();
      move || {
        counter.fetch_add(1, Ordering::Relaxed);
      }
    });
    let second = ctx.create_timeout(std::time::Duration::ZERO, move || {
      counter.fetch_add(1, Ordering::Relaxed);
    });
    first.start();
    second.start();
    Self {
      _first: first,
      _second: second,
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<lurq::node::Element> {
    lurq::node::Element::new()
  }
}

#[test]
fn all_due_timers_fire_in_one_tick() {
  let counter = Arc::new(AtomicUsize::new(0));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<TwoTimeouts>(&mut app, SharedCounter(counter.clone()));

  tree.tick_timers();

  assert_eq!(counter.load(Ordering::Relaxed), 2);
}
