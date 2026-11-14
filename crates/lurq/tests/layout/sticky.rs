//! Sticky positioning: an element held at an edge of the scroll container it
//! is in while that scrolls, for as long as the box of its parent has room
//! for it.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::Ctx,
    events::{MouseButton, ScrollPhase},
  },
  components::{Column, Rect, Row, ScrollHorizontal, ScrollVertical},
  core::Signal,
  layout::{layout_kind::Sticky, quad::QuadContent},
  node::{Element, color::Color},
};

use crate::support::{TestSurface, pointer_click};

const HEADER: Color = Color::new(200, 30, 30, 255);
const ROW: Color = Color::new(30, 30, 200, 255);

type Scene = Arc<dyn Fn() -> Element + Send + Sync>;

#[derive(Clone, lurq::DevtoolsInspectable)]
struct Props {
  #[devtools_ignore]
  scene: Scene,
}

impl PartialEq for Props {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.scene, &other.scene)
  }
}

struct Page;

impl Component for Page {
  type Props = Props;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    (ctx.props::<Props>().scene)()
  }
}

struct Stage {
  app: App,
  tree: Tree,
}

impl Stage {
  fn new(scene: impl Fn() -> Element + Send + Sync + 'static) -> Self {
    let mut app = App::new();
    let mut tree = Tree::new();
    tree.mount_root::<Page>(&mut app, Props { scene: Arc::new(scene) });
    let mut stage = Self { app, tree };
    stage.pass();
    stage
  }

  fn pass(&mut self) {
    for _ in 0..2 {
      self.tree.request_redraw();
      self.tree.pass(&mut self.app, &TestSurface);
    }
  }

  /// Scrolls the container under (100, 50) down by `dy` and right by `dx`.
  fn scroll(&mut self, dx: f32, dy: f32) {
    self.tree.scroll(100.0, 50.0, -dx, -dy, ScrollPhase::Scroll);
    self.pass();
  }

  fn top_left(&mut self, id: &str) -> (f32, f32) {
    let bounds = self
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.bounds())
      .unwrap_or_else(|| panic!("#{id} should be laid out"));
    (bounds.x, bounds.y)
  }

  /// Where the first rectangle of `color` is in the paint order.
  fn painted_at(&self, color: Color) -> usize {
    self
      .tree
      .resolve_quads(self.tree.last_layout().unwrap())
      .iter()
      .position(|quad| matches!(&quad.content, QuadContent::Rect { color: fill, .. } if *fill == color))
      .expect("the rectangle is painted")
  }
}

/// A list 200 by 100 that scrolls: 30 of lead, a section of a header of 20 and three rows of 40, and a tail.
/// The section is at 30..170 of the content.
fn list(header: impl Fn(Rect) -> Rect + Send + Sync + 'static) -> Stage {
  Stage::new(move || {
    let mut section = Column::new()
      .id("section")
      .child(header(Rect::new(200.0, 20.0).id("header").background(HEADER)));
    for index in 0..3 {
      section = section.child(Rect::new(200.0, 40.0).id(format!("row-{index}")).background(ROW));
    }
    let content = Column::new()
      .child(Rect::new(200.0, 30.0))
      .child(section)
      .child(Rect::new(200.0, 400.0));
    ScrollVertical::new(content).size(200.0, 100.0).into()
  })
}

#[test]
fn a_sticky_header_is_in_flow_until_scrolling_would_take_it_past_the_top() {
  let mut stage = list(|header| header.sticky(Sticky::top(0.0)));
  assert_eq!(stage.top_left("header"), (0.0, 30.0));
  assert_eq!(stage.top_left("row-0"), (0.0, 50.0), "it keeps its room in flow");

  stage.scroll(0.0, 20.0);
  assert_eq!(stage.top_left("header").1, 10.0, "still on its way to the top");

  stage.scroll(0.0, 30.0);
  assert_eq!(stage.top_left("header").1, 0.0, "held at the top");
  assert_eq!(stage.top_left("row-0").1, 0.0, "while its rows scroll under it");

  stage.scroll(0.0, 60.0);
  assert_eq!(stage.top_left("header").1, 0.0);
  assert_eq!(stage.top_left("row-1").1, -20.0);
}

#[test]
fn the_end_of_its_parent_takes_a_sticky_header_with_it() {
  let mut stage = list(|header| header.sticky(Sticky::top(0.0)));

  // The section ends at 170 of the content: scrolled by 160, ten pixels of it are left.
  stage.scroll(0.0, 160.0);
  assert_eq!(stage.top_left("section").1, -130.0);
  assert_eq!(stage.top_left("header").1, -10.0, "it does not leave the section");

  stage.scroll(0.0, 100.0);
  assert_eq!(stage.top_left("header").1, -110.0, "and is gone with it");
}

#[test]
fn scrolling_back_puts_a_sticky_header_back_in_its_place() {
  let mut stage = list(|header| header.sticky(Sticky::top(0.0)));
  stage.scroll(0.0, 120.0);
  assert_eq!(stage.top_left("header").1, 0.0);

  stage.scroll(0.0, -100.0);
  assert_eq!(stage.top_left("header").1, 10.0);

  stage.scroll(0.0, -20.0);
  assert_eq!(stage.top_left("header").1, 30.0);
  assert_eq!(stage.top_left("row-0").1, 50.0);
}

#[test]
fn an_inset_holds_it_that_far_inside_the_edge() {
  let mut stage = list(|header| header.sticky(Sticky::top(8.0)));

  stage.scroll(0.0, 20.0);
  assert_eq!(stage.top_left("header").1, 10.0);

  stage.scroll(0.0, 40.0);
  assert_eq!(stage.top_left("header").1, 8.0);
}

#[test]
fn what_sticks_is_painted_over_its_siblings_and_hit_before_them() {
  let clicks = Arc::new(AtomicUsize::new(0));
  let row_clicks = Arc::new(AtomicUsize::new(0));
  let mut stage = Stage::new({
    let (clicks, row_clicks) = (clicks.clone(), row_clicks.clone());
    move || {
      let (clicks, row_clicks) = (clicks.clone(), row_clicks.clone());
      let header = Rect::new(200.0, 20.0)
        .background(HEADER)
        .sticky(Sticky::top(0.0))
        .on_click(move |_| {
          clicks.fetch_add(1, Ordering::SeqCst);
        });
      let rows = Rect::new(200.0, 300.0).background(ROW).on_click(move |_| {
        row_clicks.fetch_add(1, Ordering::SeqCst);
      });
      ScrollVertical::new(Column::new().child(header).child(rows).child(Rect::new(200.0, 300.0)))
        .size(200.0, 100.0)
        .into()
    }
  });
  stage.scroll(0.0, 60.0);
  assert!(
    stage.painted_at(HEADER) > stage.painted_at(ROW),
    "over the rows under it"
  );

  pointer_click(&mut stage.tree, 100.0, 10.0, MouseButton::Left);
  assert_eq!(
    (clicks.load(Ordering::SeqCst), row_clicks.load(Ordering::SeqCst)),
    (1, 0)
  );
  pointer_click(&mut stage.tree, 100.0, 60.0, MouseButton::Left);
  assert_eq!(
    (clicks.load(Ordering::SeqCst), row_clicks.load(Ordering::SeqCst)),
    (1, 1)
  );
}

#[test]
fn a_sticky_footer_is_held_at_the_bottom_while_its_place_is_below_the_viewport() {
  let mut stage = Stage::new(|| {
    let section = Column::new().child(Rect::new(200.0, 300.0).background(ROW)).child(
      Rect::new(200.0, 20.0)
        .id("footer")
        .background(HEADER)
        .sticky(Sticky::bottom(0.0)),
    );
    ScrollVertical::new(Column::new().child(section).child(Rect::new(200.0, 300.0)))
      .size(200.0, 100.0)
      .into()
  });
  assert_eq!(stage.top_left("footer").1, 80.0, "its place is at 300");

  stage.scroll(0.0, 150.0);
  assert_eq!(stage.top_left("footer").1, 80.0);

  stage.scroll(0.0, 100.0);
  assert_eq!(stage.top_left("footer").1, 50.0, "in view, it is in its place again");
}

#[test]
fn a_sticky_column_is_held_at_the_left_of_what_scrolls_sideways() {
  let mut stage = Stage::new(|| {
    let row = Row::new()
      .child(
        Rect::new(40.0, 30.0)
          .id("first")
          .background(HEADER)
          .sticky(Sticky::left(0.0)),
      )
      .child(Rect::new(500.0, 30.0).id("cells").background(ROW));
    ScrollHorizontal::new(row).size(200.0, 100.0).into()
  });
  assert_eq!(stage.top_left("first").0, 0.0);

  stage.scroll(120.0, 0.0);
  assert_eq!(stage.top_left("first").0, 0.0);
  assert_eq!(stage.top_left("cells").0, -80.0);
}

#[test]
fn outside_a_scroll_container_a_sticky_element_stays_where_it_is_laid_out() {
  let mut stage = Stage::new(|| {
    Column::new()
      .child(Rect::new(200.0, 30.0))
      .child(Rect::new(200.0, 20.0).id("header").sticky(Sticky::top(0.0)))
      .into()
  });

  assert_eq!(stage.top_left("header"), (0.0, 30.0));
}

#[test]
fn a_sticky_element_follows_the_scroll_container_it_is_in_not_one_further_out() {
  let mut stage = Stage::new(|| {
    let inner = ScrollVertical::new(
      Column::new()
        .child(
          Rect::new(180.0, 20.0)
            .id("header")
            .background(HEADER)
            .sticky(Sticky::top(0.0)),
        )
        .child(Rect::new(180.0, 300.0)),
    )
    .size(180.0, 60.0);
    let content = Column::new()
      .child(Rect::new(200.0, 30.0))
      .child(inner)
      .child(Rect::new(200.0, 400.0));
    ScrollVertical::new(content).size(200.0, 100.0).into()
  });
  assert_eq!(stage.top_left("header").1, 30.0);

  // The inner container is at 30..90. Scrolled, it holds the header at its own top.
  stage.scroll(0.0, 40.0);
  assert_eq!(stage.top_left("header").1, 30.0);

  // The outer one, scrolled from a point below the inner one, takes the inner one and its header along.
  stage.tree.scroll(100.0, 95.0, 0.0, -50.0, ScrollPhase::Scroll);
  stage.pass();
  assert_eq!(stage.top_left("header").1, -20.0);
}

#[test]
fn a_layout_while_scrolled_keeps_what_sticks_stuck() {
  let tall = Signal::new(false);
  let mut stage = Stage::new({
    let tall = tall.clone();
    move || {
      let section = Column::new()
        .child(
          Rect::new(200.0, 20.0)
            .id("header")
            .background(HEADER)
            .sticky(Sticky::top(0.0)),
        )
        .child(
          Rect::new(200.0, if tall.get() { 260.0 } else { 200.0 })
            .id("body")
            .background(ROW),
        );
      let content = Column::new()
        .child(Rect::new(200.0, 30.0))
        .child(section)
        .child(Rect::new(200.0, 400.0));
      ScrollVertical::new(content).size(200.0, 100.0).into()
    }
  });
  stage.scroll(0.0, 80.0);
  assert_eq!(stage.top_left("header").1, 0.0);

  tall.set(true);
  stage.pass();
  assert_eq!(stage.top_left("header").1, 0.0);
  assert_eq!(stage.top_left("body").1, -30.0);

  // The taller section holds its header for longer: it ends at 310 of the content.
  stage.scroll(0.0, 190.0);
  assert_eq!(stage.top_left("header").1, 0.0);
  stage.scroll(0.0, 30.0);
  assert_eq!(stage.top_left("header").1, -10.0);
}

#[test]
fn an_element_that_no_longer_sticks_goes_back_to_its_place() {
  let sticks = Signal::new(true);
  let mut stage = Stage::new({
    let sticks = sticks.clone();
    move || {
      let mut header = Rect::new(200.0, 20.0).id("header").background(HEADER);
      if sticks.get() {
        header = header.sticky(Sticky::top(0.0));
      }
      let section = Column::new()
        .child(header)
        .child(Rect::new(200.0, 200.0).background(ROW));
      let content = Column::new()
        .child(Rect::new(200.0, 30.0))
        .child(section)
        .child(Rect::new(200.0, 400.0));
      ScrollVertical::new(content).size(200.0, 100.0).into()
    }
  });
  stage.scroll(0.0, 80.0);
  assert_eq!(stage.top_left("header").1, 0.0);

  sticks.set(false);
  stage.pass();
  assert_eq!(stage.top_left("header").1, -50.0);

  sticks.set(true);
  stage.pass();
  assert_eq!(stage.top_left("header").1, 0.0);
}

#[test]
fn a_corner_cell_sticks_on_both_axes() {
  let mut stage = Stage::new(|| {
    let grid = Column::new()
      .child(
        Rect::new(40.0, 20.0)
          .id("corner")
          .background(HEADER)
          .sticky(Sticky::top(0.0).with_left(0.0)),
      )
      .child(Rect::new(600.0, 400.0).background(ROW));
    lurq::components::ScrollBoth::new(grid).size(200.0, 100.0).into()
  });

  stage.scroll(70.0, 0.0);
  assert_eq!(stage.top_left("corner"), (0.0, 0.0));
  stage.scroll(0.0, 90.0);
  assert_eq!(stage.top_left("corner"), (0.0, 0.0));
}

/// A list 200 by 100 of a header of 20 that stays and 30 rows of 20 under it, its bars told to keep clear of
/// what sticks or not, with the state of its scroll.
fn headed(clear: bool) -> (Tree, lurq::layout::layout_kind::ScrollState) {
  let state = lurq::layout::layout_kind::ScrollState::new();
  let mut rows = Column::new().child(
    Rect::new(200.0, 20.0)
      .id("header")
      .background(HEADER)
      .sticky(Sticky::top(0.0)),
  );
  for index in 0..30 {
    rows = rows.child(Rect::new(200.0, 20.0).id(format!("row-{index}")).background(ROW));
  }
  let mut tree = Tree::new();
  tree.set_root(
    ScrollVertical::new(rows)
      .with_scroll_state(state.clone())
      .scrollbars_clear_of_sticky(clear)
      .size(200.0, 100.0),
  );
  crate::support::render_pass(&mut tree);
  (tree, state)
}

fn vertical_thumb(state: &lurq::layout::layout_kind::ScrollState) -> (f32, f32, f32, f32) {
  state
    .thumb_rect_for_axis(lurq::layout::layout_kind::ScrollAxis::Vertical, &state.style())
    .expect("the rows overflow")
}

#[test]
fn a_container_that_does_not_ask_has_its_bar_over_the_whole_viewport() {
  let (_, state) = headed(false);
  let (_, y, ..) = vertical_thumb(&state);
  // The thumb is the end inset and its own inset from the track's start.
  let inset = state.style().resolved_end_inset() + state.style().thumb_inset;
  assert!((y - inset).abs() < 0.01, "the track starts at the viewport's top: {y}");
}

#[test]
fn a_bar_told_to_keep_clear_of_what_sticks_runs_beside_the_rows_only_and_reaches_their_end() {
  let (mut tree, state) = headed(true);
  // Where the thumb goes: the track's end insets and the thumb's own inset in it.
  let inset = state.style().resolved_end_inset() + state.style().thumb_inset;
  let (x, y, width, height) = vertical_thumb(&state);
  assert!(y >= 20.0 + inset - 0.01, "the thumb starts under the header: {y}");
  // What shows of the rows is 80 of their 600: the thumb is that share of a track of 80 less its insets.
  let track = 80.0 - 2.0 * inset;
  assert!(
    (height - (track * 80.0 / 600.0).max(state.style().min_thumb_length)).abs() < 0.5,
    "{height}"
  );

  // At the end of the scroll the thumb is at the track's end, under the header still.
  tree.scroll(100.0, 50.0, 0.0, -10_000.0, ScrollPhase::Scroll);
  crate::support::render_pass(&mut tree);
  let (_, end_y, _, end_height) = vertical_thumb(&state);
  assert!(
    (end_y + end_height - (100.0 - inset)).abs() < 0.5,
    "at the end: {}",
    end_y + end_height
  );

  // Back at the top, a drag of the thumb to the track's end takes the rows to theirs.
  tree.scroll(100.0, 50.0, 0.0, 10_000.0, ScrollPhase::Scroll);
  crate::support::render_pass(&mut tree);
  assert_eq!(state.scroll_y(), 0.0);
  let (cx, cy) = (x + width / 2.0, y + height / 2.0);
  tree.mouse_move(cx, cy);
  tree.mouse_down(cx, cy, MouseButton::Left);
  tree.mouse_move(cx, cy + (track - height));
  tree.mouse_up(cx, cy + (track - height), MouseButton::Left);
  crate::support::render_pass(&mut tree);
  let end = state.content_height() - state.viewport_height();
  assert!(
    (state.scroll_y() - end).abs() < 0.5,
    "the drag reached the end: {} of {end}",
    state.scroll_y()
  );
}

#[test]
fn a_bar_across_keeps_clear_of_a_first_column_that_stays() {
  let state = lurq::layout::layout_kind::ScrollState::new();
  let row = Row::new()
    .child(Rect::new(60.0, 40.0).background(HEADER).sticky(Sticky::left(0.0)))
    .child(Rect::new(600.0, 40.0).background(ROW));
  let mut tree = Tree::new();
  tree.set_root(
    ScrollHorizontal::new(row)
      .with_scroll_state(state.clone())
      .scrollbars_clear_of_sticky(true)
      .size(200.0, 60.0),
  );
  crate::support::render_pass(&mut tree);
  let inset = state.style().resolved_end_inset() + state.style().thumb_inset;
  let thumb = |state: &lurq::layout::layout_kind::ScrollState| {
    state
      .thumb_rect_for_axis(lurq::layout::layout_kind::ScrollAxis::Horizontal, &state.style())
      .expect("the row overflows")
  };
  let (x, ..) = thumb(&state);
  assert!(x >= 60.0 + inset - 0.01, "the thumb starts after the column: {x}");
  tree.scroll(100.0, 30.0, -10_000.0, 0.0, ScrollPhase::Scroll);
  crate::support::render_pass(&mut tree);
  let (end_x, _, end_width, _) = thumb(&state);
  assert!(end_x >= 60.0 + inset - 0.01);
  assert!((end_x + end_width - (200.0 - inset)).abs() < 0.5, "at the end");
}

#[test]
fn the_bars_end_before_a_column_that_stays_at_the_end_and_its_thumb_is_dragged_there() {
  let state = lurq::layout::layout_kind::ScrollState::new();
  let mut rows = Column::new();
  for index in 0..10 {
    rows = rows.child(
      Row::new().child(Rect::new(400.0, 20.0).background(ROW)).child(
        Rect::new(60.0, 20.0)
          .id(format!("end-{index}"))
          .background(HEADER)
          .sticky(Sticky::right(0.0)),
      ),
    );
  }
  let mut tree = Tree::new();
  tree.set_root(
    lurq::components::ScrollBoth::new(rows)
      .with_scroll_state(state.clone())
      .scrollbars_clear_of_sticky(true)
      .size(200.0, 100.0),
  );
  crate::support::render_pass(&mut tree);
  let thumb = |state: &lurq::layout::layout_kind::ScrollState, axis| {
    state
      .thumb_rect_for_axis(axis, &state.style())
      .expect("the rows overflow both ways")
  };
  use lurq::layout::layout_kind::ScrollAxis::{Horizontal, Vertical};
  // The column that stays is the last 60 of the viewport: neither bar reaches into it.
  for (axis, name) in [(Vertical, "down"), (Horizontal, "across")] {
    let (x, _, width, _) = thumb(&state, axis);
    assert!(x + width <= 140.0 + 0.01, "the bar {name} ends at {}", x + width);
  }
  tree.scroll(100.0, 50.0, -10_000.0, 0.0, ScrollPhase::Scroll);
  crate::support::render_pass(&mut tree);
  let (x, _, width, _) = thumb(&state, Horizontal);
  assert!(
    x + width <= 140.0 + 0.01,
    "scrolled to the end, the bar across ends at {}",
    x + width
  );

  // The thumb is pressed and dragged where it is painted.
  let (x, y, width, height) = thumb(&state, Vertical);
  let (cx, cy) = (x + width / 2.0, y + height / 2.0);
  tree.mouse_move(cx, cy);
  tree.mouse_down(cx, cy, MouseButton::Left);
  tree.mouse_move(cx, cy + 20.0);
  tree.mouse_up(cx, cy + 20.0, MouseButton::Left);
  crate::support::render_pass(&mut tree);
  assert!(state.scroll_y() > 0.0, "the drag scrolled the rows");
}

/// A row 200 by 60 scrolling sideways over content `width` wide, its bar given a lane of its own while it is
/// wider.
fn laned(width: f32) -> lurq::layout::layout_kind::ScrollState {
  let state = lurq::layout::layout_kind::ScrollState::new();
  let mut tree = Tree::new();
  tree.set_root(
    ScrollHorizontal::new(Rect::new(width, 40.0).background(ROW))
      .with_scroll_state(state.clone())
      .reserve_bar_across(true)
      .size(200.0, 60.0),
  );
  crate::support::render_pass(&mut tree);
  state
}

#[test]
fn a_bar_across_given_a_lane_has_one_while_the_content_is_wider_and_none_while_it_fits() {
  let state = laned(600.0);
  let style = state.style();
  let lane = style.width + 2.0 * style.resolved_edge_inset();
  assert!(
    (state.viewport_height() - (60.0 - lane)).abs() < 0.01,
    "{}",
    state.viewport_height()
  );
  let (_, y, ..) = state
    .thumb_rect_for_axis(lurq::layout::layout_kind::ScrollAxis::Horizontal, &style)
    .expect("the content overflows");
  assert!(
    y >= state.viewport_height() - 0.01,
    "the thumb is in the lane, under the viewport: {y}"
  );

  let state = laned(150.0);
  assert_eq!(state.viewport_height(), 60.0, "content that fits has no lane");
}
