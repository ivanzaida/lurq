//! A flex child shrunk by its Row/Column must be laid out again at the shrunk
//! size: only resizing its box leaves the inner layout (scroll viewports,
//! nested flex distribution, bottom-aligned children) at the unshrunk size.

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::{Column, Row, ScrollHorizontal, ScrollVertical, Spacer, Text},
  core::Signal,
  layout::{Constraints, Size, layout_kind::ScrollState, layout_result::LayoutResult},
  node::Element,
};

use super::PassLayoutExt;

const WIDTH: f32 = 200.0;
const HEIGHT: f32 = 300.0;
const HEADER: f32 = 50.0;
const FOOTER: f32 = 30.0;
const ROW: f32 = 40.0;
const ROWS: usize = 12;

fn rows_column() -> Column {
  Column::new()
    .spacing(0.0)
    .with_children((0..ROWS).map(|_| Spacer::new().height(ROW)))
}

fn rows_total() -> f32 {
  ROW * ROWS as f32
}

fn remaining_height() -> f32 {
  HEIGHT - HEADER - FOOTER
}

fn page(middle: impl Into<Element>) -> Column {
  Column::new()
    .spacing(0.0)
    .size(WIDTH, HEIGHT)
    .child(Spacer::new().height(HEADER))
    .child(middle)
    .child(Spacer::new().height(FOOTER))
}

fn layout(tree: &mut Tree) -> LayoutResult {
  tree
    .pass_layout(Constraints::loose(Size::new(WIDTH * 2.0, HEIGHT * 2.0)))
    .expect("layout result")
}

/// Absolute top/bottom of the last row of a scroll area whose box starts at
/// `scroll_top`.
fn last_row_span(scroll: &LayoutResult, scroll_top: f32) -> (f32, f32) {
  let content = &scroll.children[0];
  let last = content.result.children.last().expect("rows");
  let top = scroll_top + content.offset.y + last.offset.y;
  (top, top + last.result.size.height)
}

/// `scroll_path` returns the scroll area's absolute top and its layout.
fn assert_scrolls_to_end(
  tree: &mut Tree,
  state: &ScrollState,
  scroll_path: impl Fn(&LayoutResult) -> (f32, LayoutResult),
) {
  let viewport = remaining_height();
  let (_, scroll) = scroll_path(&layout(tree));
  assert_eq!(scroll.size.height, viewport, "shrunk scroll box height");
  assert_eq!(state.content_height(), rows_total(), "scroll content height");
  assert_eq!(state.viewport_height(), viewport, "scroll viewport height");

  state.scroll_to_bottom_pending();
  layout(tree);
  assert_eq!(state.scroll_y(), rows_total() - viewport, "max scroll offset");

  let (scroll_top, scroll) = scroll_path(&layout(tree));
  let (last_top, last_bottom) = last_row_span(&scroll, scroll_top);
  assert!(
    last_top >= scroll_top,
    "last row starts inside the viewport: {last_top}"
  );
  assert!(
    (last_bottom - (scroll_top + viewport)).abs() < 0.01,
    "last row ends at the viewport bottom: {last_bottom} vs {}",
    scroll_top + viewport
  );
}

#[test]
fn shrunk_scroll_area_scrolls_to_last_row() {
  let mut tree = Tree::new();
  let state = ScrollState::new();
  tree.set_root(page(
    ScrollVertical::new(rows_column())
      .flex_shrink(1.0)
      .with_scroll_state(state.clone()),
  ));

  assert_scrolls_to_end(&mut tree, &state, |result| {
    (result.children[1].offset.y, (*result.children[1].result).clone())
  });
}

#[test]
fn scroll_area_in_shrunk_wrapper_scrolls_to_last_row() {
  let mut tree = Tree::new();
  let state = ScrollState::new();
  let scroll = ScrollVertical::new(rows_column())
    .flex_shrink(1.0)
    .with_scroll_state(state.clone());
  tree.set_root(page(Column::new().spacing(0.0).flex_shrink(1.0).child(scroll)));

  assert_scrolls_to_end(&mut tree, &state, |result| {
    let wrapper = &result.children[1];
    let scroll = &wrapper.result.children[0];
    (wrapper.offset.y + scroll.offset.y, (*scroll.result).clone())
  });
}

#[test]
fn shrunk_panel_relays_out_grow_scroll_inside() {
  // A shrinkable panel (title + growing scroll) inside a page column: the
  // panel's own flex distribution must run at the shrunk height.
  let mut tree = Tree::new();
  let state = ScrollState::new();
  const TITLE: f32 = 20.0;
  let panel = Column::new()
    .spacing(0.0)
    .flex_shrink(1.0)
    .child(Spacer::new().height(TITLE))
    .child(
      ScrollVertical::new(rows_column())
        .flex_full(1.0, 1.0, Some(rows_total()))
        .with_scroll_state(state.clone()),
    );
  tree.set_root(page(panel));

  let result = layout(&mut tree);
  let panel = &result.children[1];
  assert_eq!(panel.result.size.height, remaining_height());
  let scroll = &panel.result.children[1];
  assert_eq!(scroll.result.size.height, remaining_height() - TITLE);
  assert_eq!(state.viewport_height(), remaining_height() - TITLE);
  assert_eq!(state.content_height(), rows_total());

  state.scroll_to_bottom_pending();
  layout(&mut tree);
  assert_eq!(state.scroll_y(), rows_total() - (remaining_height() - TITLE));
}

#[test]
fn shrunk_column_positions_bottom_child_inside_its_box() {
  // A shrunk column whose last child is pushed down by a growing spacer must
  // keep that child inside the shrunk box.
  let mut tree = Tree::new();
  let inner = Column::new()
    .spacing(0.0)
    .flex_shrink(1.0)
    .child(Spacer::new().height(500.0).flex_full(1.0, 1.0, Some(500.0)))
    .child(Spacer::new().height(FOOTER));
  tree.set_root(page(inner));

  let result = layout(&mut tree);
  let inner = &result.children[1].result;
  assert_eq!(inner.size.height, remaining_height());
  let bottom = &inner.children[1];
  assert_eq!(bottom.offset.y + bottom.result.size.height, remaining_height());
}

#[test]
fn shrunk_horizontal_scroll_in_row_reaches_last_column() {
  let mut tree = Tree::new();
  let state = ScrollState::new();
  let cells = Row::new()
    .spacing(0.0)
    .with_children((0..ROWS).map(|_| Spacer::new().size(ROW, 10.0)));
  let node = Row::new()
    .spacing(0.0)
    .size(HEIGHT, 40.0)
    .child(Spacer::new().width(HEADER))
    .child(
      ScrollHorizontal::new(cells)
        .flex_shrink(1.0)
        .with_scroll_state(state.clone()),
    )
    .child(Spacer::new().width(FOOTER));
  tree.set_root(node);

  let result = layout(&mut tree);
  let viewport = HEIGHT - HEADER - FOOTER;
  assert_eq!(result.children[1].result.size.width, viewport);
  assert_eq!(state.viewport_width(), viewport);
  assert_eq!(state.content_width(), rows_total());
  state.scroll_to_right_pending();
  layout(&mut tree);
  assert_eq!(state.scroll_x(), rows_total() - viewport);
}

#[test]
fn grow_and_shrink_children_share_overflow_and_relayout() {
  // One grow+shrink scroll with a basis and one shrink-only scroll: both
  // overflow the column, both are shrunk, both must report their final
  // viewport.
  let mut tree = Tree::new();
  let first = ScrollState::new();
  let second = ScrollState::new();
  let node = Column::new()
    .spacing(0.0)
    .size(WIDTH, HEIGHT)
    .child(
      ScrollVertical::new(rows_column())
        .flex_full(1.0, 1.0, Some(rows_total()))
        .with_scroll_state(first.clone()),
    )
    .child(
      ScrollVertical::new(rows_column())
        .flex_shrink(1.0)
        .with_scroll_state(second.clone()),
    );
  tree.set_root(node);

  let result = layout(&mut tree);
  let first_height = result.children[0].result.size.height;
  let second_height = result.children[1].result.size.height;
  assert!((first_height + second_height - HEIGHT).abs() < 0.01);
  assert_eq!(first.viewport_height(), first_height);
  assert_eq!(second.viewport_height(), second_height);
  assert_eq!(result.children[1].offset.y, first_height);

  second.scroll_to_bottom_pending();
  layout(&mut tree);
  assert_eq!(second.scroll_y(), rows_total() - second_height);
}

#[test]
fn grown_panel_relays_out_bottom_child() {
  // flex_grow already lays the child out at its final size; keep it that way.
  let mut tree = Tree::new();
  let inner = Column::new()
    .spacing(0.0)
    .flex(1.0)
    .child(Spacer::new().flex(1.0))
    .child(Spacer::new().height(FOOTER));
  tree.set_root(page(inner));

  let result = layout(&mut tree);
  let inner = &result.children[1].result;
  assert_eq!(inner.size.height, remaining_height());
  let bottom = &inner.children[1];
  assert_eq!(bottom.offset.y + bottom.result.size.height, remaining_height());
}

#[test]
fn shrunk_scroll_regrows_when_content_shrinks() {
  // After a shrink pass the child's cached constraints are tight; a content
  // change must still be measured naturally and redistributed.
  let mut tree = Tree::new();
  let state = ScrollState::new();
  let make = |rows: usize| {
    page(
      ScrollVertical::new(
        Column::new()
          .spacing(0.0)
          .with_children((0..rows).map(|_| Spacer::new().height(ROW))),
      )
      .flex_shrink(1.0)
      .with_scroll_state(state.clone()),
    )
  };
  tree.set_root(make(ROWS));
  let result = layout(&mut tree);
  assert_eq!(result.children[1].result.size.height, remaining_height());

  tree.set_root(make(2));
  let result = layout(&mut tree);
  assert_eq!(result.children[1].result.size.height, ROW * 2.0);
  assert_eq!(result.children[2].offset.y, HEADER + ROW * 2.0);
}

fn shrunk_scroll_page(state: &ScrollState, rows: usize) -> Column {
  page(
    ScrollVertical::new(
      Column::new()
        .spacing(0.0)
        .with_children((0..rows).map(|_| Spacer::new().height(ROW))),
    )
    .flex_shrink(1.0)
    .with_scroll_state(state.clone()),
  )
}

#[test]
fn pending_scroll_resolves_against_shrunk_viewport_in_a_relayout_pass() {
  // The pass that resolves the pending scroll also re-measures the shrunk
  // child: the natural-size measurement must not consume it.
  let mut tree = Tree::new();
  let state = ScrollState::new();
  tree.set_root(shrunk_scroll_page(&state, ROWS));
  layout(&mut tree);

  state.scroll_to_bottom_pending();
  tree.set_root(shrunk_scroll_page(&state, ROWS + 2));
  layout(&mut tree);

  let content = ROW * (ROWS + 2) as f32;
  assert_eq!(state.content_height(), content);
  assert_eq!(state.scroll_y(), content - remaining_height());
}

#[test]
fn shrunk_scroll_at_bottom_stays_at_bottom_when_rows_are_appended() {
  let mut tree = Tree::new();
  let state = ScrollState::new();
  tree.set_root(shrunk_scroll_page(&state, ROWS));
  layout(&mut tree);
  state.scroll_to_bottom_pending();
  layout(&mut tree);
  assert_eq!(state.scroll_y(), rows_total() - remaining_height());

  tree.set_root(shrunk_scroll_page(&state, ROWS + 2));
  layout(&mut tree);

  assert_eq!(state.scroll_y(), ROW * (ROWS + 2) as f32 - remaining_height());
}

#[derive(Clone)]
struct RowCount(Signal<usize>);

impl PartialEq for RowCount {
  fn eq(&self, other: &Self) -> bool {
    self.0.id() == other.0.id()
  }
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for RowCount {}

#[derive(Clone)]
struct ShrunkScrollProps {
  rows: Signal<usize>,
  state: ScrollState,
}

impl PartialEq for ShrunkScrollProps {
  fn eq(&self, other: &Self) -> bool {
    self.rows.id() == other.rows.id()
  }
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for ShrunkScrollProps {}

/// Only the scroll content re-renders when the row count changes, so the
/// page and the shrunk scroll area are served from the layout cache.
struct ScrollRows;

impl Component for ScrollRows {
  type Props = RowCount;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    // One text block keeps the node tree's shape and has no fixed frame, so
    // the cached parent layouts are patched instead of rebuilt.
    let rows = ctx.props::<Self::Props>().0.get();
    let lines = vec!["row"; rows].join(
      "
",
    );
    Column::new().spacing(0.0).child(Text::new(&lines))
  }
}

struct ShrunkScrollPage;

impl Component for ShrunkScrollPage {
  type Props = ShrunkScrollProps;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = ctx.props::<Self::Props>().clone();
    let rows = ctx.mount::<ScrollRows>(RowCount(props.rows));
    page(
      ScrollVertical::new(rows)
        .flex_shrink(1.0)
        .with_scroll_state(props.state),
    )
  }
}

#[test]
fn cached_shrunk_scroll_is_redistributed_when_its_content_changes() {
  // After a shrink the child's cached constraints are the tight shrunk size;
  // repairing a changed descendant under them would keep the old height.
  let mut app = App::new();
  let mut tree = Tree::new();
  let rows = Signal::new(40);
  let state = ScrollState::new();
  tree.mount_root::<ShrunkScrollPage>(
    &mut app,
    ShrunkScrollProps {
      rows: rows.clone(),
      state: state.clone(),
    },
  );
  tree.set_layout_constraints_override(Some(Constraints::loose(Size::new(WIDTH * 2.0, HEIGHT * 2.0))));
  tree.pass_headless(&mut app);
  let scroll_height = |tree: &Tree| tree.last_layout().expect("layout").children[1].result.size.height;
  assert_eq!(scroll_height(&tree), remaining_height());
  assert!(state.content_height() > remaining_height());

  rows.set(2);
  tree.pass_headless(&mut app);
  assert!(state.content_height() < remaining_height());
  assert_eq!(scroll_height(&tree), state.content_height());

  rows.set(40);
  tree.pass_headless(&mut app);
  assert_eq!(scroll_height(&tree), remaining_height());
}
