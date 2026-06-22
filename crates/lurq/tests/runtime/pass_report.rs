use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::{Ctx, Timeout},
    events::MouseButton,
    render_engine::RenderEngine,
  },
  components::{Modal, Rect, Root as ModalRoot, Stack},
  core::Signal,
  layout::render_list::RenderList,
  node::Element,
};
use raw_window_handle::{DisplayHandle, WindowHandle};

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

#[derive(Clone)]
struct SharedModalState {
  open: Arc<Signal<bool>>,
  render_count: Arc<AtomicUsize>,
  target: ModalRegressionTarget,
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for SharedModalState {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

impl PartialEq for SharedModalState {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.open, &other.open)
      && Arc::ptr_eq(&self.render_count, &other.render_count)
      && self.target == other.target
  }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ModalRegressionTarget {
  Parent,
  Root,
}

struct ModalRedrawRoot {
  open: Signal<bool>,
  render_count: Arc<AtomicUsize>,
  target: ModalRegressionTarget,
}

impl Component for ModalRedrawRoot {
  type Props = SharedModalState;

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>();
    Self {
      open: (*props.open).clone(),
      render_count: props.render_count.clone(),
      target: props.target,
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    self.render_count.fetch_add(1, Ordering::Relaxed);
    let modal = Modal::new(Rect::new(60.0, 40.0).background("#ef4444")).open(self.open.clone());
    let modal = match self.target {
      ModalRegressionTarget::Parent => modal,
      ModalRegressionTarget::Root => modal.target(ModalRoot),
    };

    Stack::new()
      .size(240.0, 160.0)
      .child(Rect::new(240.0, 160.0).background("#22c55e"))
      .child(modal)
  }
}

struct EventModalRedrawRoot {
  open: Signal<bool>,
  render_count: Arc<AtomicUsize>,
  target: ModalRegressionTarget,
}

impl Component for EventModalRedrawRoot {
  type Props = SharedModalState;

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>();
    let render_count = props.render_count.clone();
    let target = props.target;
    Self {
      open: ctx.signal(false),
      render_count,
      target,
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    self.render_count.fetch_add(1, Ordering::Relaxed);
    let open = self.open.clone();
    let modal = Modal::new(Rect::new(60.0, 40.0).background("#ef4444")).open(self.open.clone());
    let modal = match self.target {
      ModalRegressionTarget::Parent => modal,
      ModalRegressionTarget::Root => modal.target(ModalRoot),
    };

    Stack::new()
      .size(240.0, 160.0)
      .child(
        Rect::new(80.0, 40.0)
          .background("#22c55e")
          .on_click(move |_| open.set(true)),
      )
      .child(modal)
  }
}

struct CountingRenderEngine {
  render_count: Arc<AtomicUsize>,
}

impl RenderEngine for CountingRenderEngine {
  fn resize(&mut self, _width: u32, _height: u32) {}

  fn render(&mut self, _list: &RenderList, _window: WindowHandle<'_>, _display: DisplayHandle<'_>) -> bool {
    self.render_count.fetch_add(1, Ordering::Relaxed);
    true
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

#[test]
fn signal_opened_parent_modal_requires_and_presents_next_pass_without_explicit_redraw() {
  signal_opened_modal_requires_and_presents_next_pass_without_explicit_redraw(ModalRegressionTarget::Parent);
}

#[test]
fn signal_opened_root_modal_requires_and_presents_next_pass_without_explicit_redraw() {
  signal_opened_modal_requires_and_presents_next_pass_without_explicit_redraw(ModalRegressionTarget::Root);
}

#[test]
fn click_opened_parent_modal_presents_next_pass_after_event_flush() {
  click_opened_modal_presents_next_pass_after_event_flush(ModalRegressionTarget::Parent);
}

#[test]
fn click_opened_root_modal_presents_next_pass_after_event_flush() {
  click_opened_modal_presents_next_pass_after_event_flush(ModalRegressionTarget::Root);
}

fn signal_opened_modal_requires_and_presents_next_pass_without_explicit_redraw(target: ModalRegressionTarget) {
  let open = Arc::new(Signal::new(false));
  let component_renders = Arc::new(AtomicUsize::new(0));
  let presented_frames = Arc::new(AtomicUsize::new(0));
  let mut app = App::new();
  let mut tree = Tree::new();
  let presented_frames_for_engine = presented_frames.clone();
  tree.set_render_engine_factory(move || {
    Box::new(CountingRenderEngine {
      render_count: presented_frames_for_engine.clone(),
    })
  });
  tree.mount_root::<ModalRedrawRoot>(
    &mut app,
    SharedModalState {
      open: open.clone(),
      render_count: component_renders.clone(),
      target,
    },
  );

  let initial = tree.pass(&mut app, &TestSurface);
  assert!(initial.required);
  assert!(initial.rendered);
  assert_eq!(component_renders.load(Ordering::Relaxed), 1);
  assert_eq!(presented_frames.load(Ordering::Relaxed), 1);

  open.set(true);
  assert!(tree.needs_redraw());

  let report = tree.pass(&mut app, &TestSurface);

  assert!(report.required);
  assert!(report.rendered);
  assert!(report.reasons.component_dirty);
  assert_eq!(component_renders.load(Ordering::Relaxed), 2);
  assert_eq!(presented_frames.load(Ordering::Relaxed), 2);
  assert_eq!(tree.root().unwrap().tag_name(), "OverlayHost");
}

fn click_opened_modal_presents_next_pass_after_event_flush(target: ModalRegressionTarget) {
  let open = Arc::new(Signal::new(false));
  let component_renders = Arc::new(AtomicUsize::new(0));
  let presented_frames = Arc::new(AtomicUsize::new(0));
  let mut app = App::new();
  let mut tree = Tree::new();
  let presented_frames_for_engine = presented_frames.clone();
  tree.set_render_engine_factory(move || {
    Box::new(CountingRenderEngine {
      render_count: presented_frames_for_engine.clone(),
    })
  });
  tree.mount_root::<EventModalRedrawRoot>(
    &mut app,
    SharedModalState {
      open,
      render_count: component_renders.clone(),
      target,
    },
  );

  let initial = tree.pass(&mut app, &TestSurface);
  assert!(initial.rendered);

  tree.mouse_down(10.0, 10.0, MouseButton::Left);
  tree.mouse_up(10.0, 10.0, MouseButton::Left);
  assert!(tree.needs_redraw());

  let report = tree.pass(&mut app, &TestSurface);

  assert!(report.required);
  assert!(report.rendered);
  assert_eq!(component_renders.load(Ordering::Relaxed), 2);
  assert_eq!(presented_frames.load(Ordering::Relaxed), 2);
  assert_eq!(tree.root().unwrap().tag_name(), "OverlayHost");
}
