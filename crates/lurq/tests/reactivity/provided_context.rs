//! A context a component provides in `create` stays visible to its descendants
//! when an ancestor re-renders and the inherited contexts are refreshed.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  core::{ReactiveContext, Signal},
  node::Element,
};

use crate::support::run_pass;

#[derive(Debug, Clone, PartialEq)]
struct Seen {
  local: Option<i32>,
  inherited: Option<String>,
  reactive: Option<u32>,
}

#[derive(Default)]
struct Probe {
  root_tick: Mutex<Option<Signal<i32>>>,
  reader_tick: Mutex<Option<Signal<i32>>>,
  reactive: Mutex<Option<ReactiveContext<u32>>>,
  provider_renders: Mutex<usize>,
  seen: Mutex<Vec<Seen>>,
}

impl Probe {
  fn last_seen(&self) -> Seen {
    self.seen.lock().unwrap().last().cloned().expect("the reader rendered")
  }

  fn reader_renders(&self) -> usize {
    self.seen.lock().unwrap().len()
  }
}

#[derive(Clone, lurq::DevtoolsInspectable)]
struct Props {
  #[devtools_ignore]
  probe: Shared,
  root_provides: bool,
}

impl PartialEq for Props {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.probe.0, &other.probe.0) && self.root_provides == other.root_provides
  }
}

impl std::fmt::Debug for Props {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Props")
      .field("root_provides", &self.root_provides)
      .finish()
  }
}

#[derive(Clone)]
struct Shared(Arc<Probe>);

struct Root {
  tick: Signal<i32>,
}

impl Component for Root {
  type Props = Props;

  fn create(ctx: &mut Ctx) -> Self {
    let tick = ctx.signal(0);
    *ctx.props::<Props>().probe.0.root_tick.lock().unwrap() = Some(tick.clone());
    Self { tick }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let tick = self.tick.get();
    let props = ctx.props::<Props>().clone();
    // Stops providing from tick 2 on, which removes the value.
    if props.root_provides && tick < 2 {
      ctx.provide(format!("tick-{tick}"));
    }
    ctx.mount::<Provider>(props)
  }
}

struct Provider;

impl Component for Provider {
  type Props = Props;

  fn create(ctx: &mut Ctx) -> Self {
    ctx.provide(7_i32);
    let reactive = ctx.create_context(100_u32);
    *ctx.props::<Props>().probe.0.reactive.lock().unwrap() = Some(reactive);
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Props>().clone();
    *props.probe.0.provider_renders.lock().unwrap() += 1;
    ctx.mount::<Reader>(props)
  }
}

struct Reader {
  tick: Signal<i32>,
}

impl Component for Reader {
  type Props = Props;

  fn create(ctx: &mut Ctx) -> Self {
    let tick = ctx.signal(0);
    *ctx.props::<Props>().probe.0.reader_tick.lock().unwrap() = Some(tick.clone());
    Self { tick }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    self.tick.get();
    let seen = Seen {
      local: ctx.use_context::<i32>(),
      inherited: ctx.use_context::<String>(),
      reactive: ctx.consume_context::<u32>().map(|value| value.get()),
    };
    ctx.props::<Props>().probe.0.seen.lock().unwrap().push(seen);
    Element::new()
  }
}

fn mount(root_provides: bool) -> (Tree, Arc<Probe>) {
  let probe = Arc::new(Probe::default());
  let mut tree = Tree::new();
  tree.mount_root::<Root>(
    &mut App::new(),
    Props {
      probe: Shared(probe.clone()),
      root_provides,
    },
  );
  (tree, probe)
}

fn set(signal: &Mutex<Option<Signal<i32>>>, value: i32) {
  signal
    .lock()
    .unwrap()
    .as_ref()
    .expect("signal published in create")
    .set(value);
}

#[test]
fn context_provided_in_create_survives_parent_rerender() {
  let (mut tree, probe) = mount(false);
  set(&probe.root_tick, 1);
  run_pass(&mut tree);
  set(&probe.reader_tick, 1);
  run_pass(&mut tree);

  let expected = Seen {
    local: Some(7),
    inherited: None,
    reactive: Some(100),
  };
  assert_eq!(probe.last_seen(), expected);
}

#[test]
fn parent_rerender_without_context_change_does_not_rerender_provider() {
  let (mut tree, probe) = mount(false);
  set(&probe.root_tick, 1);
  run_pass(&mut tree);

  assert_eq!(*probe.provider_renders.lock().unwrap(), 1);
  assert_eq!(probe.reader_renders(), 1);
}

#[test]
fn changed_ancestor_context_reaches_descendants_next_to_provided_one() {
  let (mut tree, probe) = mount(true);
  assert_eq!(probe.last_seen().inherited.as_deref(), Some("tick-0"));

  set(&probe.root_tick, 1);
  run_pass(&mut tree);

  let expected = Seen {
    local: Some(7),
    inherited: Some("tick-1".to_owned()),
    reactive: Some(100),
  };
  assert_eq!(probe.last_seen(), expected);
}

#[test]
fn reactive_context_provided_in_create_still_notifies_after_parent_rerender() {
  let (mut tree, probe) = mount(true);
  set(&probe.root_tick, 1);
  run_pass(&mut tree);

  probe.reactive.lock().unwrap().as_ref().expect("created").set(5);
  run_pass(&mut tree);

  assert_eq!(probe.last_seen().reactive, Some(5));
  assert_eq!(probe.last_seen().local, Some(7));
}

#[test]
fn value_provided_in_render_is_removed_when_render_stops_providing_it() {
  let (mut tree, probe) = mount(true);
  set(&probe.root_tick, 2);
  run_pass(&mut tree);

  let expected = Seen {
    local: Some(7),
    inherited: None,
    reactive: Some(100),
  };
  assert_eq!(probe.last_seen(), expected);
}
