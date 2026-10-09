//! Row state in a `VirtualizedList`: a row scrolled out of the rendered window
//! is unmounted, so state it created itself starts over when it comes back.
//! State kept above the list, keyed by row key, survives — the documented
//! pattern.

use std::{
  collections::HashMap,
  sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
  },
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::ScrollPhase},
  components::{Rect, VirtualizedList},
  core::Signal,
  node::Element,
};

use crate::support::run_pass;

const COLLAPSED: f32 = 30.0;
const EXPANDED: f32 = 90.0;

/// Expanded rows by key, owned by the screen above the list.
#[derive(Clone)]
struct ExpandedRows(Signal<HashMap<usize, bool>>);

/// Counts row component creations, to show rows are remounted.
#[derive(Clone)]
struct Creations(Arc<AtomicUsize>);

struct ExpandableRow {
  expanded: Option<ExpandedRows>,
}

impl Component for ExpandableRow {
  type Props = usize;

  fn create(ctx: &mut Ctx) -> Self {
    if let Some(Creations(count)) = ctx.use_context::<Creations>() {
      count.fetch_add(1, Ordering::SeqCst);
    }
    Self {
      expanded: ctx.use_context::<ExpandedRows>(),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let key = *ctx.props::<Self::Props>();
    let expanded = self
      .expanded
      .as_ref()
      .is_some_and(|rows| rows.0.with(|rows| rows.get(&key).copied().unwrap_or(false)));
    let height = if expanded { EXPANDED } else { COLLAPSED };
    Rect::new(100.0, height).id(format!("row-{key}"))
  }
}

#[derive(Clone, lurq::DevtoolsInspectable)]
struct ScreenProps(#[devtools_ignore] Arc<(ExpandedRows, Creations)>);

impl PartialEq for ScreenProps {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

/// Provides the expanded-rows map to the rows it mounts through the list.
struct Screen;

impl Component for Screen {
  type Props = ScreenProps;

  fn create(ctx: &mut Ctx) -> Self {
    let (expanded, creations) = (*ctx.props::<Self::Props>().0).clone();
    ctx.provide(expanded);
    ctx.provide(creations);
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    VirtualizedList::new(ctx, 0..500usize)
      .size(100.0, 300.0)
      .overscan_px(0.0)
      .mount_keyed::<ExpandableRow, _, _, _>(|key| *key, |key| *key)
  }
}

fn row_height(tree: &mut Tree, key: usize) -> Option<f32> {
  tree
    .get_element_by_id_mut(&format!("row-{key}"))
    .and_then(|row| row.bounds())
    .map(|bounds| bounds.height)
}

fn settle(tree: &mut Tree) {
  for _ in 0..4 {
    run_pass(tree);
  }
}

#[test]
fn row_state_kept_above_the_list_survives_scrolling_the_row_out_and_back() {
  let expanded = ExpandedRows(Signal::new(HashMap::new()));
  let creations = Arc::new(AtomicUsize::new(0));
  let mut tree = Tree::new();
  tree.mount_root::<Screen>(
    &mut App::new(),
    ScreenProps(Arc::new((expanded.clone(), Creations(creations.clone())))),
  );
  settle(&mut tree);

  expanded.0.update(|rows| {
    rows.insert(2, true);
  });
  settle(&mut tree);
  assert_eq!(row_height(&mut tree, 2), Some(EXPANDED));

  // Far enough that row 2 leaves the rendered window and is unmounted.
  tree.scroll(10.0, 10.0, 0.0, -6000.0, ScrollPhase::Scroll);
  settle(&mut tree);
  assert_eq!(row_height(&mut tree, 2), None, "row 2 is unmounted");
  let created_before_return = creations.load(Ordering::SeqCst);

  tree.scroll(10.0, 10.0, 0.0, 10_000.0, ScrollPhase::Scroll);
  settle(&mut tree);
  assert!(
    creations.load(Ordering::SeqCst) > created_before_return,
    "rows coming back are created again"
  );
  assert_eq!(row_height(&mut tree, 2), Some(EXPANDED), "row 2 is still expanded");
}
