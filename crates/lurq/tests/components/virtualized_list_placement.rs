//! A virtualized list that moves inside its parent (a sibling above it
//! appears or disappears) and a list capped by `max_height`.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Column, Rect, VirtualizedList},
  core::Signal,
  layout::{layout_kind::ScrollState, layout_result::LayoutResult},
  node::{Element, color::Color},
};
use tracing::{
  Event, Metadata, Subscriber,
  field::{Field, Visit},
  span,
};

use crate::support::{RenderSnapshot, render_pass_with_app};

const ROW_H: f32 = 50.0;
const NOTICE_H: f32 = 120.0;
const LIST_W: f32 = 100.0;
const PAGE_H: f32 = 400.0;

/// Collects `lurq::vlist` viewport-hole warnings emitted on this thread.
#[derive(Clone, Default)]
struct HoleWarnings(Arc<Mutex<Vec<String>>>);

struct MessageVisitor<'a>(&'a mut String);

impl Visit for MessageVisitor<'_> {
  fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
    if field.name() == "message" {
      *self.0 = format!("{value:?}");
    }
  }
}

impl Subscriber for HoleWarnings {
  fn enabled(&self, metadata: &Metadata<'_>) -> bool {
    metadata.target() == "lurq::vlist"
  }

  fn new_span(&self, _span: &span::Attributes<'_>) -> span::Id {
    span::Id::from_u64(1)
  }

  fn record(&self, _span: &span::Id, _values: &span::Record<'_>) {}

  fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {}

  fn event(&self, event: &Event<'_>) {
    let mut message = String::new();
    event.record(&mut MessageVisitor(&mut message));
    if message.contains("viewport hole") {
      self.0.lock().unwrap().push(message);
    }
  }

  fn enter(&self, _span: &span::Id) {}

  fn exit(&self, _span: &span::Id) {}
}

#[derive(Clone)]
struct PageSignals {
  notice: Signal<bool>,
  rows: Signal<usize>,
}

impl PartialEq for PageSignals {
  fn eq(&self, other: &Self) -> bool {
    self.notice.id() == other.notice.id() && self.rows.id() == other.rows.id()
  }
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for PageSignals {}

fn row_color(id: usize) -> Color {
  Color::new(id as u8, 90, 160, 255)
}

struct ListRow;

impl Component for ListRow {
  type Props = usize;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let id = *ctx.props::<Self::Props>();
    Rect::new(LIST_W, ROW_H).background(row_color(id))
  }
}

/// A fixed-height page: an optional notice above a list that fills the rest.
struct MovingListPage;

impl Component for MovingListPage {
  type Props = PageSignals;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let signals = ctx.props::<Self::Props>().clone();
    let mut page = Column::new().spacing(0.0).size(LIST_W, PAGE_H);
    if signals.notice.get() {
      page = page.child(Rect::new(LIST_W, NOTICE_H).background(Color::new(250, 200, 0, 255)));
    }
    let list = VirtualizedList::new(ctx, 0..signals.rows.get())
      .flex(1.0)
      .overscan_px(0.0)
      .mount_keyed::<ListRow, _, _, _>(|id| *id, |id| *id);
    page.child(list)
  }
}

#[derive(Clone)]
struct CappedProps {
  rows: usize,
  scroll_state: ScrollState,
}

impl PartialEq for CappedProps {
  fn eq(&self, other: &Self) -> bool {
    self.rows == other.rows
  }
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for CappedProps {}

/// A list capped by `max_height` and otherwise sized by its content.
struct CappedListPage;

impl Component for CappedListPage {
  type Props = CappedProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let CappedProps { rows, scroll_state } = ctx.props::<Self::Props>().clone();
    let list = VirtualizedList::new(ctx, 0..rows)
      .max_height(200.0)
      .overscan_px(0.0)
      .with_scroll_state(scroll_state)
      .mount_keyed::<ListRow, _, _, _>(|id| *id, |id| *id);
    Column::new().spacing(0.0).width(LIST_W).child(list)
  }
}

/// Every row that intersects the list viewport at `list_top` is painted at
/// its row position (the list is not scrolled in these tests).
fn assert_rows_painted(snapshot: &RenderSnapshot, list_top: f32, rows: usize, label: &str) {
  let viewport_h = PAGE_H - list_top;
  for id in 0..rows {
    let y = list_top + id as f32 * ROW_H;
    if y >= list_top + viewport_h {
      break;
    }
    let painted = snapshot
      .rects
      .iter()
      .any(|rect| rect.color == row_color(id) && (rect.y - y).abs() < 0.5);
    assert!(painted, "{label}: row {id} not painted at y={y}");
  }
}

fn list_top(layout: &LayoutResult) -> f32 {
  layout.children.last().expect("list").offset.y
}

fn settle(tree: &mut Tree, app: &mut App) -> RenderSnapshot {
  let mut snapshot = render_pass_with_app(tree, app);
  for _ in 0..3 {
    snapshot = render_pass_with_app(tree, app);
  }
  snapshot
}

fn run_moving_list(steps: &[(bool, usize)]) -> Vec<String> {
  let warnings = HoleWarnings::default();
  let collected = warnings.0.clone();
  tracing::subscriber::with_default(warnings, || {
    let signals = PageSignals {
      notice: Signal::new(steps[0].0),
      rows: Signal::new(steps[0].1),
    };
    let mut app = App::new();
    let mut tree = Tree::new();
    tree.mount_root::<MovingListPage>(&mut app, signals.clone());
    tree.resize(200, 600);
    settle(&mut tree, &mut app);

    for (step, &(notice, rows)) in steps.iter().enumerate().skip(1) {
      signals.notice.set(notice);
      signals.rows.set(rows);
      // The frame that applies the change must already paint every row.
      let snapshot = render_pass_with_app(&mut tree, &mut app);
      let top = list_top(tree.last_layout().expect("layout"));
      let expected_top = if notice { NOTICE_H } else { 0.0 };
      assert_eq!(top, expected_top, "step {step}: list top");
      assert_rows_painted(&snapshot, top, rows, &format!("step {step}, first frame"));
      let snapshot = settle(&mut tree, &mut app);
      assert_rows_painted(&snapshot, top, rows, &format!("step {step}, settled"));
    }
  });
  collected.lock().unwrap().clone()
}

#[test]
fn list_moved_down_by_new_sibling_reports_no_viewport_hole() {
  let warnings = run_moving_list(&[(false, 3), (true, 3)]);
  assert!(warnings.is_empty(), "false viewport-hole warnings: {warnings:#?}");
}

#[test]
fn list_moved_up_by_removed_sibling_reports_no_viewport_hole() {
  let warnings = run_moving_list(&[(true, 3), (false, 3)]);
  assert!(warnings.is_empty(), "false viewport-hole warnings: {warnings:#?}");
}

#[test]
fn list_filled_while_sibling_toggles_reports_no_viewport_hole() {
  let warnings = run_moving_list(&[(true, 0), (false, 8), (true, 0), (true, 8), (false, 2)]);
  assert!(warnings.is_empty(), "false viewport-hole warnings: {warnings:#?}");
}

#[test]
fn list_below_a_sibling_reports_no_viewport_hole_headless() {
  // Headless passes lay out without painting; the probe must not depend on
  // paint-time positions.
  let warnings = HoleWarnings::default();
  let collected = warnings.0.clone();
  tracing::subscriber::with_default(warnings, || {
    let signals = PageSignals {
      notice: Signal::new(true),
      rows: Signal::new(3),
    };
    let mut app = App::new();
    let mut tree = Tree::new();
    tree.mount_root::<MovingListPage>(&mut app, signals);
    for _ in 0..3 {
      tree.pass_headless(&mut app);
    }
  });
  let warnings = collected.lock().unwrap().clone();
  assert!(warnings.is_empty(), "false viewport-hole warnings: {warnings:#?}");
}

#[test]
fn capped_list_sizes_to_shorter_content() {
  let mut app = App::new();
  let mut tree = Tree::new();
  let scroll_state = ScrollState::new();
  let props = CappedProps {
    rows: 3,
    scroll_state: scroll_state.clone(),
  };
  tree.mount_root::<CappedListPage>(&mut app, props);
  settle(&mut tree, &mut app);

  let layout = tree.last_layout().expect("layout");
  assert_eq!(layout.children[0].result.size.height, 3.0 * ROW_H);
  assert_eq!(scroll_state.viewport_height(), 3.0 * ROW_H);
  assert_eq!(scroll_state.content_height(), 3.0 * ROW_H);
}

#[test]
fn capped_list_stops_at_cap_and_scrolls() {
  let mut app = App::new();
  let mut tree = Tree::new();
  let scroll_state = ScrollState::new();
  let props = CappedProps {
    rows: 40,
    scroll_state: scroll_state.clone(),
  };
  tree.mount_root::<CappedListPage>(&mut app, props);
  settle(&mut tree, &mut app);

  let layout = tree.last_layout().expect("layout");
  assert_eq!(layout.children[0].result.size.height, 200.0);
  assert_eq!(scroll_state.viewport_height(), 200.0);
  assert_eq!(scroll_state.content_height(), 40.0 * ROW_H);
}
