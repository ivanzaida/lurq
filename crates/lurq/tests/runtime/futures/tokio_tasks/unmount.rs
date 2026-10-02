use std::{
  sync::{Arc, Mutex, mpsc},
  thread,
  time::{Duration, Instant},
};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::{Ctx, FutureAction},
  },
  components::Text,
  core::Signal,
  node::Element,
};
use tokio::runtime::Builder;

use super::{Harness, Probe, Ticker, Trigger, Waiter, drive, label};

#[test]
fn stream_task_stops_when_its_component_unmounts() {
  let mut harness = Harness::mount::<Ticker>();
  harness.drive();
  assert!(harness.probe.iterations() > 0);

  harness.set_show(false);
  let after_unmount = harness.probe.iterations();
  harness.drive();

  assert_eq!(harness.probe.iterations(), after_unmount);
  assert_eq!(harness.probe.dropped(), 1);
}

#[test]
fn pending_future_is_cancelled_when_its_component_unmounts() {
  let mut harness = Harness::mount::<Waiter>();
  harness.drive();
  assert_eq!(harness.text(), Some("pending"));

  harness.set_show(false);
  harness.probe.release();
  harness.drive();

  assert_eq!(harness.probe.effects(), 0);
  assert_eq!(harness.probe.dropped(), 1);
}

#[test]
fn tasks_stop_when_the_tree_is_dropped() {
  let harness = Harness::mount::<Ticker>();
  harness.drive();
  let Harness {
    runtime, tree, probe, ..
  } = harness;

  drop(tree);
  let after_drop = probe.iterations();
  drive(&runtime);

  assert_eq!(probe.iterations(), after_drop);
  assert_eq!(probe.dropped(), 1);
}

#[test]
fn stream_task_stops_on_a_multi_thread_runtime() {
  let runtime = Builder::new_multi_thread().worker_threads(2).build().unwrap();
  let mut harness = Harness::mount_on::<Ticker>(runtime);
  wait_until(|| harness.probe.iterations() > 0);

  harness.set_show(false);
  wait_until(|| harness.probe.dropped() == 1);
  let after_abort = harness.probe.iterations();
  thread::sleep(Duration::from_millis(20));

  assert_eq!(harness.probe.iterations(), after_abort);
}

#[test]
fn unmounting_after_the_runtime_shut_down_does_not_panic() {
  let harness = Harness::mount::<Ticker>();
  harness.drive();
  let Harness {
    runtime,
    mut tree,
    show,
    probe,
    ..
  } = harness;

  drop(runtime);
  assert_eq!(probe.dropped(), 1);
  show.set(false);
  tree.rebuild();
  drop(tree);

  assert_eq!(probe.dropped(), 1);
}

/// Touches the action that owns the future it lives in when that future is dropped.
struct TouchesOwnHandle(Arc<Mutex<Option<FutureAction<(), usize, String>>>>);

impl Drop for TouchesOwnHandle {
  fn drop(&mut self) {
    let action = self.0.lock().unwrap().clone();
    if let Some(action) = action {
      action.is_active();
      action.cancel();
    }
  }
}

/// Renders a `ctx.future_action` whose future touches the action itself when dropped.
struct SelfTouching;

impl Component for SelfTouching {
  type Props = Probe;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let probe = ctx.props::<Probe>().clone();
    let slot = probe.action.clone();
    let action = ctx.future_action(move |()| {
      let toucher = TouchesOwnHandle(slot.clone());
      async move {
        let _toucher = toucher;
        Ok::<usize, String>(1)
      }
    });
    *probe.action.lock().unwrap() = Some(action.clone());
    let state = action.state().get();
    Text::new(&label(state.status, state.data))
  }
}

#[test]
fn future_dropped_by_spawn_on_a_shut_down_runtime_may_touch_its_own_handle() {
  let harness = Harness::mount::<SelfTouching>();
  let action = harness.probe.action.lock().unwrap().clone().unwrap();
  let Harness { runtime, mut tree, .. } = harness;
  // Spawning on a runtime that is gone drops the future right away, on the calling thread.
  drop(runtime);

  let (done, finished) = mpsc::channel();
  let runner = action.clone();
  thread::spawn(move || {
    runner.run(());
    done.send(()).unwrap();
  });

  if finished.recv_timeout(Duration::from_secs(5)).is_err() {
    // The deadlocked thread holds the task lock, and dropping the tree would wait for it.
    std::mem::forget(tree);
    panic!("run deadlocked while the dropped future touched its own handle");
  }
  // The dropped task never reports back; the next poll sees that and retires it.
  tree.tick_futures();
  assert!(!action.is_active());
}

#[test]
fn future_action_run_after_its_component_unmounts_starts_nothing() {
  let mut harness = Harness::mount::<Trigger>();
  let action = harness.probe.action.lock().unwrap().clone().unwrap();

  harness.set_show(false);
  action.run(());
  harness.probe.release();
  harness.drive();

  assert!(!action.is_active());
  assert_eq!(harness.probe.starts(), 0);
  assert_eq!(harness.probe.effects(), 0);
}

struct SeveralProps {
  count: Signal<usize>,
  probe: Probe,
}

impl PartialEq for SeveralProps {
  fn eq(&self, other: &Self) -> bool {
    self.count.id() == other.count.id() && self.probe == other.probe
  }
}

impl lurq::app::component::DevtoolsInspectable for SeveralProps {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

/// Renders `count` futures; future `i` records an effect worth `10^i`.
struct Several;

impl Component for Several {
  type Props = SeveralProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<SeveralProps>();
    let (count, probe) = (props.count.get(), props.probe.clone());
    let mut first = None;
    for index in 0..count {
      let probe = probe.clone();
      let state = ctx.future(index, move |index| probe.pending_work(10_usize.pow(index as u32)));
      first.get_or_insert(state.state().get());
    }
    let first = first.expect("Several renders at least one future");
    Text::new(&label(first.status, first.data))
  }
}

#[test]
fn future_slot_a_rerender_no_longer_renders_is_cancelled() {
  let runtime = Builder::new_current_thread().build().unwrap();
  let mut app = App::new().with_tokio_handle(runtime.handle().clone());
  let count = Signal::new(2);
  let probe = Probe::default();
  let mut tree = Tree::new();
  tree.mount_root::<Several>(
    &mut app,
    SeveralProps {
      count: count.clone(),
      probe: probe.clone(),
    },
  );
  drive(&runtime);

  count.set(1);
  tree.rebuild();
  probe.release();
  drive(&runtime);
  tree.tick_futures();

  assert_eq!(probe.starts(), 2);
  assert_eq!(probe.effects(), 1);
  assert_eq!(probe.dropped(), 2);
  assert_eq!(tree.root().and_then(|root| root.text_content()), Some("done:1"));
}

fn wait_until(condition: impl Fn() -> bool) {
  let deadline = Instant::now() + Duration::from_secs(5);
  while !condition() {
    assert!(Instant::now() < deadline, "condition not reached within 5 s");
    thread::sleep(Duration::from_millis(1));
  }
}
