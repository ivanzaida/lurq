use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::Text,
  core::Signal,
  node::Element,
};

use crate::support::run_pass;

#[derive(Clone)]
struct ProbeProps {
  creates: Arc<AtomicUsize>,
  renders: Arc<AtomicUsize>,
  mounted: Arc<AtomicUsize>,
  unmounted: Arc<AtomicUsize>,
  value: Arc<Mutex<Option<Signal<usize>>>>,
}

impl PartialEq for ProbeProps {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.value, &other.value)
  }
}

impl lurq::app::component::DevtoolsInspectable for ProbeProps {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

struct OffstageProps {
  active: Signal<bool>,
  show: Signal<bool>,
  probe: ProbeProps,
}

impl Clone for OffstageProps {
  fn clone(&self) -> Self {
    Self {
      active: self.active.clone(),
      show: self.show.clone(),
      probe: self.probe.clone(),
    }
  }
}

impl PartialEq for OffstageProps {
  fn eq(&self, other: &Self) -> bool {
    self.active.id() == other.active.id() && self.show.id() == other.show.id() && self.probe == other.probe
  }
}

impl lurq::app::component::DevtoolsInspectable for OffstageProps {
  fn write_info(&self, _buffer: &mut Vec<lurq::app::component::ComponentInfo>) {}
}

struct Probe {
  value: Signal<usize>,
  props: ProbeProps,
}

impl Component for Probe {
  type Props = ProbeProps;

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<ProbeProps>().clone();
    props.creates.fetch_add(1, Ordering::Relaxed);
    let value = ctx.signal(1);
    *props.value.lock().unwrap() = Some(value.clone());
    Self { value, props }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    self.props.renders.fetch_add(1, Ordering::Relaxed);
    Text::new(&self.value.get().to_string())
  }

  fn on_mounted(&self) {
    self.props.mounted.fetch_add(1, Ordering::Relaxed);
  }

  fn on_unmounted(&self) {
    self.props.unmounted.fetch_add(1, Ordering::Relaxed);
  }
}

struct OffstageHost;

impl Component for OffstageHost {
  type Props = OffstageProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<OffstageProps>().clone();
    if !props.show.get() {
      return Element::new();
    }
    ctx.mount_keyed_offstage::<Probe>("probe", props.probe, props.active.get())
  }
}

#[test]
fn offstage_component_retains_state_without_rendering_or_unmounting() {
  let active = Signal::new(true);
  let show = Signal::new(true);
  let probe = ProbeProps {
    creates: Arc::new(AtomicUsize::new(0)),
    renders: Arc::new(AtomicUsize::new(0)),
    mounted: Arc::new(AtomicUsize::new(0)),
    unmounted: Arc::new(AtomicUsize::new(0)),
    value: Arc::new(Mutex::new(None)),
  };
  let props = OffstageProps {
    active: active.clone(),
    show: show.clone(),
    probe: probe.clone(),
  };
  let mut tree = Tree::new();
  tree.mount_root::<OffstageHost>(&mut App::new(), props);

  assert_eq!(tree.root().and_then(|root| root.text_content()), Some("1"));
  assert_eq!(probe.creates.load(Ordering::Relaxed), 1);
  assert_eq!(probe.renders.load(Ordering::Relaxed), 1);
  assert_eq!(probe.mounted.load(Ordering::Relaxed), 1);

  active.set(false);
  run_pass(&mut tree);
  assert_eq!(tree.root().and_then(|root| root.text_content()), None);
  assert_eq!(probe.renders.load(Ordering::Relaxed), 1);
  assert_eq!(probe.unmounted.load(Ordering::Relaxed), 0);

  probe.value.lock().unwrap().as_ref().unwrap().set(7);
  run_pass(&mut tree);
  assert_eq!(probe.renders.load(Ordering::Relaxed), 1);

  active.set(true);
  run_pass(&mut tree);
  assert_eq!(tree.root().and_then(|root| root.text_content()), Some("7"));
  assert_eq!(probe.creates.load(Ordering::Relaxed), 1);
  assert_eq!(probe.renders.load(Ordering::Relaxed), 2);
  assert_eq!(probe.mounted.load(Ordering::Relaxed), 1);
  assert_eq!(probe.unmounted.load(Ordering::Relaxed), 0);

  show.set(false);
  run_pass(&mut tree);
  assert_eq!(probe.unmounted.load(Ordering::Relaxed), 1);
}
