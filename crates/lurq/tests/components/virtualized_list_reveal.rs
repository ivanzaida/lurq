//! `VirtualizedList::reveal_key`: a list mounted with a reveal key scrolls
//! that row into view on its own, a later key change reveals again, and an
//! already-visible row never moves. An element of a mounted row can also be
//! revealed like any other element. A row taller than the viewport is revealed
//! at its top, once.

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::ScrollPhase},
  components::{Rect, VirtualizedList},
  layout::layout_result::LayoutResult,
  node::{Element, color::Color},
};

use crate::support::run_pass;

const ROW_HEIGHT: f32 = 30.0;
const VIEWPORT_HEIGHT: f32 = 300.0;
const ROWS: usize = 200;

struct RevealRow;

impl Component for RevealRow {
  type Props = usize;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let index = *ctx.props::<Self::Props>();
    Rect::new(100.0, ROW_HEIGHT)
      .id(format!("row-{index}"))
      .background(Color::new((index % 256) as u8, 64, 128, 255))
  }
}

struct RevealRoot;

impl Component for RevealRoot {
  /// Row count and the key to reveal.
  type Props = (usize, Option<String>);

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let (count, reveal) = ctx.props::<Self::Props>().clone();
    VirtualizedList::new(ctx, 0..count)
      .size(100.0, VIEWPORT_HEIGHT)
      .reveal_key(reveal)
      .mount_keyed::<RevealRow, _, _, _>(|index| format!("row-{index}"), |index| *index)
  }
}

fn mount(reveal: Option<&str>) -> Tree {
  let mut tree = Tree::new();
  tree.mount_root::<RevealRoot>(&mut App::new(), (ROWS, reveal.map(str::to_owned)));
  tree
}

fn settle(tree: &mut Tree) {
  for _ in 0..6 {
    run_pass(tree);
  }
}

fn scroll_offset(layout: &LayoutResult) -> f32 {
  -layout.children[0].offset.y
}

fn offset(tree: &Tree) -> f32 {
  scroll_offset(tree.last_layout().expect("the list was laid out"))
}

fn assert_row_visible(tree: &Tree, index: usize) {
  let scroll_y = offset(tree);
  let top = index as f32 * ROW_HEIGHT;
  assert!(
    top >= scroll_y - 0.5 && top + ROW_HEIGHT <= scroll_y + VIEWPORT_HEIGHT + 0.5,
    "row {index} [{top}..{}] is outside the viewport [{scroll_y}..{}]",
    top + ROW_HEIGHT,
    scroll_y + VIEWPORT_HEIGHT
  );
}

#[test]
fn reveal_key_set_at_mount_scrolls_a_row_past_the_first_window() {
  let mut tree = mount(Some("row-150"));
  settle(&mut tree);
  assert_row_visible(&tree, 150);
}

#[test]
fn reveal_key_set_at_mount_scrolls_a_row_inside_the_first_window() {
  // Row 40 is mounted by the first (bootstrap) window but sits below the
  // viewport: the reveal must not be dropped as "already visible".
  let mut tree = mount(Some("row-40"));
  settle(&mut tree);
  assert_row_visible(&tree, 40);
}

#[test]
fn reveal_key_set_at_mount_keeps_a_visible_row_in_place() {
  let mut tree = mount(Some("row-3"));
  settle(&mut tree);
  assert_eq!(offset(&tree), 0.0);
}

#[test]
fn reveal_key_changed_after_mount_reveals_the_new_row() {
  let mut tree = mount(Some("row-150"));
  settle(&mut tree);
  tree.update_root_props::<RevealRoot>((ROWS, Some("row-20".to_owned())));
  settle(&mut tree);
  assert_row_visible(&tree, 20);
  tree.update_root_props::<RevealRoot>((ROWS, Some("row-190".to_owned())));
  settle(&mut tree);
  assert_row_visible(&tree, 190);
}

#[test]
fn reveal_key_of_a_visible_row_does_not_move_the_list() {
  let mut tree = mount(Some("row-150"));
  settle(&mut tree);
  let before = offset(&tree);
  let visible = (before / ROW_HEIGHT).ceil() as usize + 1;
  tree.update_root_props::<RevealRoot>((ROWS, Some(format!("row-{visible}"))));
  settle(&mut tree);
  assert_eq!(offset(&tree), before);
}

#[test]
fn reveal_key_waits_for_items_that_arrive_after_mount() {
  // Data loads after the list mounted with the selection's key: the reveal
  // fires once the row exists.
  let mut tree = Tree::new();
  tree.mount_root::<RevealRoot>(&mut App::new(), (0, Some("row-150".to_owned())));
  settle(&mut tree);
  tree.update_root_props::<RevealRoot>((ROWS, Some("row-150".to_owned())));
  settle(&mut tree);
  assert_row_visible(&tree, 150);
}

#[test]
fn scroll_into_view_reveals_an_element_of_a_mounted_row() {
  // Row 15 ([450..480]) is below the 300 px viewport but inside the overscan,
  // so it is mounted and its element can be revealed directly.
  let mut tree = mount(None);
  settle(&mut tree);
  tree
    .get_element_by_id_mut("row-15")
    .expect("row 15 is mounted in the overscan")
    .scroll_into_view();
  settle(&mut tree);
  assert_eq!(offset(&tree), 15.0 * ROW_HEIGHT + ROW_HEIGHT - VIEWPORT_HEIGHT);
  assert_row_visible(&tree, 15);
  // A row outside viewport and overscan is not mounted: `reveal_key` is the
  // way to reach it.
  assert!(tree.get_element_by_id_mut("row-150").is_none());
}

/// The row of [`TallRoot`] three viewports tall.
const TALL: usize = 100;

struct TallRow;

impl Component for TallRow {
  type Props = usize;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let index = *ctx.props::<Self::Props>();
    let height = if index == TALL {
      VIEWPORT_HEIGHT * 3.0
    } else {
      ROW_HEIGHT
    };
    Rect::new(100.0, height)
      .id(format!("row-{index}"))
      .background(Color::new((index % 256) as u8, 64, 128, 255))
  }
}

/// [`ROWS`] rows, row [`TALL`] taller than the viewport, revealing that row.
struct TallRoot;

impl Component for TallRoot {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    VirtualizedList::new(ctx, 0..ROWS)
      .size(100.0, VIEWPORT_HEIGHT)
      .reveal_key(Some(format!("row-{TALL}")))
      .mount_keyed::<TallRow, _, _, _>(|index| format!("row-{index}"), |index| *index)
  }
}

/// Turns the wheel over the list (a positive `delta` scrolls toward the start) and lays it out once.
fn wheel(tree: &mut Tree, delta: f32) -> f32 {
  tree.scroll(10.0, 10.0, 0.0, delta, ScrollPhase::Scroll);
  run_pass(tree);
  offset(tree)
}

#[test]
fn reveal_key_of_a_row_taller_than_the_viewport_ends_once_its_top_is_shown() {
  let mut tree = Tree::new();
  tree.mount_root::<TallRoot>(&mut App::new(), ());
  settle(&mut tree);
  // The rows above it are placed from estimates, so its top is where the
  // list laid it out, not `TALL * ROW_HEIGHT`.
  let top = tree
    .get_element_by_id_mut(&format!("row-{TALL}"))
    .and_then(|row| row.bounds())
    .expect("the tall row is laid out")
    .y;
  assert!(top.abs() <= 0.5, "the tall row's top is at the viewport's top: {top}");
  let revealed = offset(&tree);
  // The user scrolls up, then down into the tall row: later layouts leave the
  // list where they put it.
  let up = wheel(&mut tree, 120.0);
  assert!(up < revealed, "the wheel scrolled up: {up}");
  settle(&mut tree);
  assert_eq!(offset(&tree), up, "the reveal does not take the list back");
  let down = wheel(&mut tree, -480.0);
  assert!(down > revealed, "the wheel scrolled into the tall row: {down}");
  settle(&mut tree);
  assert_eq!(offset(&tree), down, "the reveal does not take the list back");
}
