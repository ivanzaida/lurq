//! A context provided in `render` reaches the descendants that read it and
//! renders only those again: a component between that does not read it is
//! not rendered, and neither is one beside the reader. A value provided with
//! `provide_if_changed` that equals the previous render's renders nothing
//! again; a changed one renders the readers, which see it through the
//! component between even though that one did not render.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::Column,
  node::Element,
};

use crate::support::run_pass;

/// The value the root provides in its render.
#[derive(Clone, PartialEq)]
struct Shown(i32);

#[derive(Default)]
struct Probe {
  middle_renders: Mutex<usize>,
  bystander_renders: Mutex<usize>,
  /// What the reader saw, once per render.
  reader_seen: Mutex<Vec<Option<i32>>>,
}

#[derive(Clone)]
struct Shared(Arc<Probe>);

impl PartialEq for Shared {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for Shared {}

#[derive(Clone, PartialEq)]
struct RootProps {
  probe: Shared,
  /// Changes to render the root again without changing what it provides.
  tick: u32,
  shown: i32,
  if_changed: bool,
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for RootProps {}

/// Provides `Shown` in its render, above a component that does not read it.
struct Root;

impl Component for Root {
  type Props = RootProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    if props.if_changed {
      ctx.provide_if_changed(Shown(props.shown));
    } else {
      ctx.provide(Shown(props.shown));
    }
    Column::new().child(ctx.mount::<Middle>(props.probe))
  }
}

/// Reads no context; mounts the reader and a bystander.
struct Middle;

impl Component for Middle {
  type Props = Shared;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let probe = ctx.props::<Self::Props>().clone();
    *probe.0.middle_renders.lock().unwrap() += 1;
    Column::new()
      .child(ctx.mount::<Reader>(probe.clone()))
      .child(ctx.mount::<Bystander>(probe))
  }
}

/// Reads `Shown` in its render.
struct Reader;

impl Component for Reader {
  type Props = Shared;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let probe = ctx.props::<Self::Props>().clone();
    let shown = ctx.use_context::<Shown>().map(|shown| shown.0);
    probe.0.reader_seen.lock().unwrap().push(shown);
    Column::new()
  }
}

/// Reads no context.
struct Bystander;

impl Component for Bystander {
  type Props = Shared;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let probe = ctx.props::<Self::Props>().clone();
    *probe.0.bystander_renders.lock().unwrap() += 1;
    Column::new()
  }
}

struct Page {
  tree: Tree,
  probe: Arc<Probe>,
  tick: u32,
  if_changed: bool,
}

impl Page {
  fn mount(shown: i32, if_changed: bool) -> Self {
    let probe = Arc::new(Probe::default());
    let mut tree = Tree::new();
    tree.mount_root::<Root>(
      &mut App::new(),
      RootProps {
        probe: Shared(probe.clone()),
        tick: 0,
        shown,
        if_changed,
      },
    );
    run_pass(&mut tree);
    Self {
      tree,
      probe,
      tick: 0,
      if_changed,
    }
  }

  /// Renders the root again, providing `shown`.
  fn render_root(&mut self, shown: i32) {
    self.tick += 1;
    self.tree.update_root_props::<Root>(RootProps {
      probe: Shared(self.probe.clone()),
      tick: self.tick,
      shown,
      if_changed: self.if_changed,
    });
    run_pass(&mut self.tree);
  }

  fn middle_renders(&self) -> usize {
    *self.probe.middle_renders.lock().unwrap()
  }

  fn bystander_renders(&self) -> usize {
    *self.probe.bystander_renders.lock().unwrap()
  }

  fn reader_seen(&self) -> Vec<Option<i32>> {
    self.probe.reader_seen.lock().unwrap().clone()
  }
}

#[test]
fn a_context_provided_in_render_renders_only_the_descendants_that_read_it() {
  let mut page = Page::mount(1, false);
  assert_eq!(page.reader_seen(), vec![Some(1)]);
  assert_eq!((page.middle_renders(), page.bystander_renders()), (1, 1));

  // `provide` compares nothing, so the reader renders again; the component
  // between and the bystander, which do not read the value, do not.
  page.render_root(1);
  assert_eq!(page.reader_seen(), vec![Some(1), Some(1)], "the reader renders again");
  assert_eq!(
    (page.middle_renders(), page.bystander_renders()),
    (1, 1),
    "what does not read the value does not render"
  );

  page.render_root(2);
  assert_eq!(
    page.reader_seen(),
    vec![Some(1), Some(1), Some(2)],
    "the reader sees the new value through the component between"
  );
  assert_eq!((page.middle_renders(), page.bystander_renders()), (1, 1));
}

#[test]
fn a_value_provided_if_changed_renders_nothing_again_while_it_is_the_same() {
  let mut page = Page::mount(1, true);
  assert_eq!(page.reader_seen(), vec![Some(1)]);

  page.render_root(1);
  page.render_root(1);
  assert_eq!(page.reader_seen(), vec![Some(1)], "an equal value renders no reader");
  assert_eq!((page.middle_renders(), page.bystander_renders()), (1, 1));

  page.render_root(5);
  assert_eq!(
    page.reader_seen(),
    vec![Some(1), Some(5)],
    "a changed value renders the reader once, with the value"
  );
  assert_eq!((page.middle_renders(), page.bystander_renders()), (1, 1));

  page.render_root(5);
  assert_eq!(
    page.reader_seen(),
    vec![Some(1), Some(5)],
    "and the same value again, nothing"
  );
}
