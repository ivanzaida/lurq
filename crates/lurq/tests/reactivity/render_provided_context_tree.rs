//! What the tree shows after a context provided in `render` changes, when
//! the components between the provider and its readers do not render again:
//! a `for_each` item that read the value is rendered by its owner, since
//! only the owner's render holds its closure; a lookup between passes sees
//! the readers rendered; and a `provide_if_changed` below a changed context
//! still keeps an equal value's providing.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::Column,
  core::Signal,
  node::Element,
};

use crate::support::run_pass;

#[derive(Clone, PartialEq)]
struct Shown(i32);

/// Another context, provided above the `provide_if_changed` provider.
#[derive(Clone, PartialEq)]
struct Other(i32);

/// What the reader saw, once per render, and the signal a provider shows.
#[derive(Clone, Default)]
struct Probe {
  seen: Arc<Mutex<Vec<Option<i32>>>>,
  signal: Arc<Mutex<Option<Signal<i32>>>>,
}

impl Probe {
  fn seen(&self) -> Vec<Option<i32>> {
    self.seen.lock().unwrap().clone()
  }
}

impl PartialEq for Probe {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.seen, &other.seen)
  }
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for Probe {}

#[derive(Clone, PartialEq)]
struct ShownProps {
  shown: i32,
  items_provide: bool,
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for ShownProps {}

/// Provides `Shown` in its render, above the owner of the items.
struct ShownRoot;

impl Component for ShownRoot {
  type Props = ShownProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    ctx.provide(Shown(props.shown));
    Column::new().child(ctx.mount::<Owner>(props.items_provide))
  }
}

/// Reads no context; its `for_each` items read `Shown` and show it in their
/// ids (and provide a value of their own when asked).
struct Owner;

impl Component for Owner {
  type Props = bool;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let items_provide = *ctx.props::<Self::Props>();
    let items = ctx.for_each(
      ["a", "b"],
      |item| *item,
      move |ctx, item| {
        if items_provide {
          ctx.provide(Other(0));
        }
        let shown = ctx.use_context::<Shown>().map_or(0, |shown| shown.0);
        Column::new().id(format!("item-{item}-{shown}")).into()
      },
    );
    let mut column = Column::new();
    for item in items {
      column = column.child(item);
    }
    column
  }
}

fn mount_items(items_provide: bool) -> Tree {
  let mut tree = Tree::new();
  tree.mount_root::<ShownRoot>(
    &mut App::new(),
    ShownProps {
      shown: 1,
      items_provide,
    },
  );
  run_pass(&mut tree);
  tree
}

fn has(tree: &mut Tree, id: &str) -> bool {
  tree.get_element_by_id(id).is_some()
}

#[test]
fn for_each_items_that_read_a_changed_context_are_rendered_by_their_owner() {
  for items_provide in [false, true] {
    let mut tree = mount_items(items_provide);
    assert!(has(&mut tree, "item-a-1") && has(&mut tree, "item-b-1"));

    tree.update_root_props::<ShownRoot>(ShownProps {
      shown: 2,
      items_provide,
    });
    for _ in 0..4 {
      run_pass(&mut tree);
    }
    assert!(
      has(&mut tree, "item-a-2") && has(&mut tree, "item-b-2"),
      "the items show the new value (items provide: {items_provide})"
    );
    assert!(!has(&mut tree, "item-a-1"), "and not the old one");
  }
}

// --- A lookup between passes ---

/// Provides `Shown` from a signal; a child between does not read it.
struct SignalRoot;

impl Component for SignalRoot {
  type Props = Probe;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let probe = ctx.props::<Self::Props>().clone();
    Column::new().child(ctx.mount::<SignalProvider>(probe))
  }
}

struct SignalProvider {
  shown: Signal<i32>,
}

impl Component for SignalProvider {
  type Props = Probe;

  fn create(ctx: &mut Ctx) -> Self {
    let shown = ctx.signal(1);
    *ctx.props::<Self::Props>().signal.lock().unwrap() = Some(shown.clone());
    Self { shown }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let probe = ctx.props::<Self::Props>().clone();
    ctx.provide(Shown(self.shown.get()));
    Column::new().child(ctx.mount::<Between>(probe))
  }
}

/// Reads no context.
struct Between;

impl Component for Between {
  type Props = Probe;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let probe = ctx.props::<Self::Props>().clone();
    Column::new().child(ctx.mount::<Reader>(probe))
  }
}

/// Shows `Shown` in its id and records what it saw.
struct Reader;

impl Component for Reader {
  type Props = Probe;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let probe = ctx.props::<Self::Props>().clone();
    let shown = ctx.use_context::<Shown>().map(|shown| shown.0);
    probe.seen.lock().unwrap().push(shown);
    Column::new().id(format!("reader-{}", shown.unwrap_or(0)))
  }
}

#[test]
fn a_lookup_between_passes_sees_the_readers_of_a_changed_context() {
  let probe = Probe::default();
  let mut tree = Tree::new();
  tree.mount_root::<SignalRoot>(&mut App::new(), probe.clone());
  run_pass(&mut tree);
  assert!(has(&mut tree, "reader-1"));

  let signal = probe
    .signal
    .lock()
    .unwrap()
    .clone()
    .expect("the provider made its signal");
  signal.set(2);
  // No pass: the lookup itself flushes the provider's render, and with it
  // the reader it marked.
  assert!(
    has(&mut tree, "reader-2"),
    "the reader rendered for the lookup: {:?}",
    probe.seen()
  );
  assert!(!has(&mut tree, "reader-1"));
}

// --- provide_if_changed below a changed context ---

#[derive(Clone, PartialEq)]
struct OtherProps {
  other: i32,
  probe: Probe,
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for OtherProps {}

/// Provides `Other` in its render, above a provider of `Shown`.
struct OtherRoot;

impl Component for OtherRoot {
  type Props = OtherProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    ctx.provide(Other(props.other));
    Column::new().child(ctx.mount::<ShownIfChanged>(props.probe))
  }
}

/// Reads `Other` and provides the same `Shown` on every render.
struct ShownIfChanged;

impl Component for ShownIfChanged {
  type Props = Probe;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let probe = ctx.props::<Self::Props>().clone();
    let _other = ctx.use_context::<Other>();
    ctx.provide_if_changed(Shown(7));
    Column::new().child(ctx.mount::<Between>(probe))
  }
}

#[test]
fn provide_if_changed_keeps_its_providing_when_an_inherited_context_changed() {
  let probe = Probe::default();
  let mut tree = Tree::new();
  tree.mount_root::<OtherRoot>(
    &mut App::new(),
    OtherProps {
      other: 1,
      probe: probe.clone(),
    },
  );
  run_pass(&mut tree);
  assert_eq!(probe.seen(), vec![Some(7)]);

  for other in [2, 3] {
    tree.update_root_props::<OtherRoot>(OtherProps {
      other,
      probe: probe.clone(),
    });
    run_pass(&mut tree);
  }
  assert_eq!(
    probe.seen(),
    vec![Some(7)],
    "the reader of an equal value does not render again"
  );
}
