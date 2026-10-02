use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::Text,
  core::Signal,
  node::Element,
};
use tokio::runtime::{Builder, Runtime};

use super::{Harness, Probe, Trigger, drive, label};

struct DepProps {
  dep: Signal<usize>,
  rerender: Signal<usize>,
  probe: Probe,
}

impl PartialEq for DepProps {
  fn eq(&self, other: &Self) -> bool {
    self.dep.id() == other.dep.id() && self.rerender.id() == other.rerender.id() && self.probe == other.probe
  }
}

impl lurq::app::component::DevtoolsInspectable for DepProps {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

/// A future whose deps are `dep`; reading `rerender` re-renders without changing them.
struct DepFuture;

impl Component for DepFuture {
  type Props = DepProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<DepProps>();
    let (dep, _, probe) = (props.dep.get(), props.rerender.get(), props.probe.clone());
    let state = ctx.future(dep, move |dep| probe.pending_work(dep)).state().get();
    Text::new(&label(state.status, state.data))
  }
}

struct DepHarness {
  runtime: Runtime,
  tree: Tree,
  dep: Signal<usize>,
  rerender: Signal<usize>,
  probe: Probe,
}

impl DepHarness {
  fn mount() -> Self {
    let runtime = Builder::new_current_thread().build().unwrap();
    let mut app = App::new().with_tokio_handle(runtime.handle().clone());
    let dep = Signal::new(1);
    let rerender = Signal::new(0);
    let probe = Probe::default();
    let mut tree = Tree::new();
    tree.mount_root::<DepFuture>(
      &mut app,
      DepProps {
        dep: dep.clone(),
        rerender: rerender.clone(),
        probe: probe.clone(),
      },
    );
    Self {
      runtime,
      tree,
      dep,
      rerender,
      probe,
    }
  }

  fn text(&self) -> Option<&str> {
    self.tree.root().and_then(|root| root.text_content())
  }
}

#[test]
fn deps_change_still_cancels_the_previous_tokio_future() {
  let mut harness = DepHarness::mount();
  drive(&harness.runtime);

  harness.dep.set(10);
  harness.tree.rebuild();
  harness.probe.release();
  drive(&harness.runtime);
  harness.tree.tick_futures();

  assert_eq!(harness.probe.starts(), 2);
  assert_eq!(harness.probe.effects(), 10);
  assert_eq!(harness.probe.dropped(), 2);
  assert_eq!(harness.text(), Some("done:10"));
}

#[test]
fn rerender_keeps_a_completed_tokio_future_without_restarting_it() {
  let mut harness = DepHarness::mount();
  harness.probe.release();
  drive(&harness.runtime);
  harness.tree.tick_futures();
  assert_eq!(harness.text(), Some("done:1"));

  harness.rerender.set(1);
  harness.tree.rebuild();
  drive(&harness.runtime);
  harness.tree.tick_futures();

  assert_eq!(harness.text(), Some("done:1"));
  assert_eq!(harness.probe.starts(), 1);
  assert_eq!(harness.probe.effects(), 1);
}

#[test]
fn future_action_still_runs_when_triggered() {
  let mut harness = Harness::mount::<Trigger>();
  assert_eq!(harness.text(), Some("idle"));
  let action = harness.probe.action.lock().unwrap().clone().unwrap();

  action.run(());
  harness.probe.release();
  harness.drive();
  harness.tree.tick_futures();

  assert_eq!(harness.text(), Some("done:1"));
  assert_eq!(harness.probe.starts(), 1);
}
