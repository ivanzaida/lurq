//! A lookup between passes (`get_element_by_id`) flushes the dirty child
//! subtrees also while the root component itself is dirty: the root's own
//! render waits for the pass, the children's do not.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::Column,
  core::Signal,
  node::Element,
};

use crate::support::run_pass;

/// Where the components hand their signals to the test.
#[derive(Clone, Default)]
struct Signals {
  root: Arc<Mutex<Option<Signal<i32>>>>,
  child: Arc<Mutex<Option<Signal<i32>>>>,
}

impl PartialEq for Signals {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.root, &other.root)
  }
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for Signals {}

/// Reads its own signal in render and mounts the child.
struct Root {
  tick: Signal<i32>,
}

impl Component for Root {
  type Props = Signals;

  fn create(ctx: &mut Ctx) -> Self {
    let tick = ctx.signal(1);
    *ctx.props::<Self::Props>().root.lock().unwrap() = Some(tick.clone());
    Self { tick }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let signals = ctx.props::<Self::Props>().clone();
    let _tick = self.tick.get();
    Column::new().child(ctx.mount::<Child>(signals))
  }
}

/// Shows its own signal in its id.
struct Child {
  shown: Signal<i32>,
}

impl Component for Child {
  type Props = Signals;

  fn create(ctx: &mut Ctx) -> Self {
    let shown = ctx.signal(1);
    *ctx.props::<Self::Props>().child.lock().unwrap() = Some(shown.clone());
    Self { shown }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Column::new().id(format!("child-{}", self.shown.get()))
  }
}

#[test]
fn a_lookup_refreshes_dirty_children_while_the_root_is_dirty_too() {
  let signals = Signals::default();
  let mut tree = Tree::new();
  tree.mount_root::<Root>(&mut App::new(), signals.clone());
  run_pass(&mut tree);
  assert!(tree.get_element_by_id("child-1").is_some());

  let child = signals
    .child
    .lock()
    .unwrap()
    .clone()
    .expect("the child made its signal");
  let root = signals.root.lock().unwrap().clone().expect("the root made its signal");
  child.set(2);
  root.set(2);
  assert!(
    tree.get_element_by_id("child-2").is_some(),
    "the child's subtree is refreshed for the lookup"
  );
  assert!(tree.get_element_by_id("child-1").is_none());
}
