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

use crate::support::TestSurface;

struct StaticRoot;

impl Component for StaticRoot {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Rect::new(120.0, 40.0)
  }
}

#[derive(Clone, lurq::DevtoolsInspectable)]
struct SharedCounter(Arc<AtomicUsize>);

impl PartialEq for SharedCounter {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

struct TimerRoot {
  done: Signal<bool>,
  renders: Arc<AtomicUsize>,
  _timeout: Timeout,
}

impl Component for TimerRoot {
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
fn pass_report_marks_initial_layout_and_noop_pass() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<StaticRoot>(&mut app, ());

  let first = tree.pass(&mut app, &TestSurface);
  assert!(first.required);
  assert!(first.layout_updated);
  assert!(first.layout_recalculated);
  assert!(first.reasons.redraw_requested);
  assert!(first.reasons.theme_changed);

  let second = tree.pass(&mut app, &TestSurface);
  assert!(!second.required);
  assert!(!second.rendered);
  assert!(!second.layout_updated);
  assert!(!second.layout_recalculated);
  assert!(!second.reasons.any());
}

#[test]
fn pass_report_marks_explicit_redraw_without_layout_recalc() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<StaticRoot>(&mut app, ());
  let _ = tree.pass(&mut app, &TestSurface);

  tree.request_redraw();
  let report = tree.pass(&mut app, &TestSurface);

  assert!(report.required);
  assert!(report.reasons.redraw_requested);
  assert!(!report.layout_updated);
  assert!(!report.layout_recalculated);
}

#[test]
fn pass_report_marks_timer_run_and_dirty_component() {
  let renders = Arc::new(AtomicUsize::new(0));
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.mount_root::<TimerRoot>(&mut app, SharedCounter(renders.clone()));
  let _ = tree.pass(&mut app, &TestSurface);

  tree.tick_timers();
  let report = tree.pass(&mut app, &TestSurface);

  assert_eq!(renders.load(Ordering::Relaxed), 2);
  assert!(report.required);
  assert!(report.reasons.timer_run);
  assert!(!report.reasons.component_dirty);
  assert!(report.layout_updated);
}
