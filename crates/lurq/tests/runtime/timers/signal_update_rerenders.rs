use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::{Ctx, Timeout},
  },
  core::Signal,
  node::Element,
};

#[derive(Clone, lurq::DevtoolsInspectable)]
struct SharedCounter(Arc<AtomicUsize>);

impl PartialEq for SharedCounter {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

struct TimerSignalUpdate {
  done: Signal<bool>,
  renders: Arc<AtomicUsize>,
  _timeout: Timeout,
}

impl Component for TimerSignalUpdate {
  type Props = SharedCounter;

  fn create(ctx: &mut Ctx) -> Self {
    let done = ctx.signal(false);
    let renders = ctx.props::<Self::Props>().0.clone();
    let timeout = ctx.create_timeout(std::time::Duration::ZERO, {
      let done = done.clone();
      move || done.set(true)
    });
    timeout.start();
    Self {
      done,
      renders,
      _timeout: timeout,
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    self.renders.fetch_add(1, Ordering::Relaxed);
    lurq::components::Text::new(if self.done.get() { "done" } else { "idle" })
  }
}

#[test]
fn timer_signal_update_rerenders_component() {
  let renders = Arc::new(AtomicUsize::new(0));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<TimerSignalUpdate>(&mut app, SharedCounter(renders.clone()));

  assert_eq!(renders.load(Ordering::Relaxed), 1);
  assert_eq!(tree.root().unwrap().text_content(), Some("idle"));

  tree.tick_timers();

  assert_eq!(renders.load(Ordering::Relaxed), 2);
  assert_eq!(tree.root().unwrap().text_content(), Some("done"));
  assert!(tree.needs_redraw());
}
