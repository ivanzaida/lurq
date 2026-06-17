use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::ScrollPhase},
  components::{Rect, VirtualizedList},
  node::{Element, color::Color},
};
use lurq_macros::DevtoolsInspectable;

use crate::support::run_pass;

struct Shared<T>(Arc<T>);

impl<T> Clone for Shared<T> {
  fn clone(&self) -> Self {
    Self(self.0.clone())
  }
}

impl<T> PartialEq for Shared<T> {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

#[cfg(feature = "devtools")]
impl<T> lurq::app::component::DevtoolsInspectable for Shared<T> {}

#[derive(Clone, PartialEq, DevtoolsInspectable)]
struct RowData {
  id: usize,
  height: f32,
}

struct VirtualizedRoot;

impl Component for VirtualizedRoot {
  type Props = (Vec<RowData>, Shared<AtomicUsize>);

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let (items, bottom_reached) = ctx.props::<Self::Props>().clone();
    let reached = bottom_reached.0.clone();
    VirtualizedList::new(ctx, items)
      .size(100.0, 100.0)
      .overscan_px(0.0)
      .on_bottom_reached(move || {
        reached.fetch_add(1, Ordering::SeqCst);
      })
      .mount_keyed::<MeasuredRow, _, _, _>(|row| row.id, |row| (*row).clone())
  }
}

struct MeasuredRow;

impl Component for MeasuredRow {
  type Props = RowData;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let row = ctx.props::<Self::Props>();
    Rect::new(100.0, row.height).background(row_color(row.id))
  }
}

struct VirtualizedTopRoot;

impl Component for VirtualizedTopRoot {
  type Props = (Vec<RowData>, Shared<AtomicUsize>);

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let (items, top_reached) = ctx.props::<Self::Props>().clone();
    let reached = top_reached.0.clone();
    VirtualizedList::new(ctx, items)
      .size(100.0, 100.0)
      .overscan_px(0.0)
      .on_top_reached(move || {
        reached.fetch_add(1, Ordering::SeqCst);
      })
      .mount_keyed::<MeasuredRow, _, _, _>(|row| row.id, |row| (*row).clone())
  }
}

#[test]
fn virtualized_list_measures_all_rows_then_renders_visible_window() {
  let mut tree = Tree::new();
  let reached = Arc::new(AtomicUsize::new(0));
  tree.mount_root::<VirtualizedRoot>(&mut App::new(), (rows(10), Shared(reached)));

  run_pass(&mut tree);
  let first = tree.last_layout().unwrap();
  assert_eq!(scroll_content_child_count(first), 10);

  run_pass(&mut tree);
  let second = tree.last_layout().unwrap();
  assert_eq!(scroll_content_child_count(second), 3);
  assert_eq!(scroll_content_height(second), 500.0);
}

#[test]
fn virtualized_list_updates_visible_window_after_scroll() {
  let mut tree = Tree::new();
  let reached = Arc::new(AtomicUsize::new(0));
  tree.mount_root::<VirtualizedRoot>(&mut App::new(), (rows(10), Shared(reached)));

  run_pass(&mut tree);
  run_pass(&mut tree);

  tree.scroll(10.0, 10.0, 0.0, -150.0, ScrollPhase::Scroll);
  run_pass(&mut tree);

  let layout = tree.last_layout().unwrap();
  let content = scroll_content(layout);
  assert_eq!(scroll_offset(layout), 150.0);
  assert_eq!(content.children.len(), 4);
  assert_eq!(content.children[0].result.size.height, 150.0);
  assert_eq!(content.children[3].result.size.height, 250.0);
  assert_eq!(scroll_content_height(layout), 500.0);
}

#[test]
fn virtualized_list_forwards_bottom_reached_for_pagination() {
  let mut tree = Tree::new();
  let reached = Arc::new(AtomicUsize::new(0));
  tree.mount_root::<VirtualizedRoot>(&mut App::new(), (rows(10), Shared(reached.clone())));

  run_pass(&mut tree);
  run_pass(&mut tree);

  tree.scroll(10.0, 10.0, 0.0, -1000.0, ScrollPhase::Scroll);
  run_pass(&mut tree);

  assert_eq!(reached.load(Ordering::SeqCst), 1);

  tree.scroll(10.0, 10.0, 0.0, -1000.0, ScrollPhase::Scroll);
  run_pass(&mut tree);

  assert_eq!(reached.load(Ordering::SeqCst), 1);
}

#[test]
fn virtualized_list_forwards_top_reached_for_pagination() {
  let mut tree = Tree::new();
  let reached = Arc::new(AtomicUsize::new(0));
  tree.mount_root::<VirtualizedTopRoot>(&mut App::new(), (rows(10), Shared(reached.clone())));

  run_pass(&mut tree);
  run_pass(&mut tree);

  tree.scroll(10.0, 10.0, 0.0, -150.0, ScrollPhase::Scroll);
  run_pass(&mut tree);
  assert_eq!(reached.load(Ordering::SeqCst), 0);

  tree.scroll(10.0, 10.0, 0.0, 200.0, ScrollPhase::Scroll);
  run_pass(&mut tree);
  assert_eq!(reached.load(Ordering::SeqCst), 1);

  tree.scroll(10.0, 10.0, 0.0, 200.0, ScrollPhase::Scroll);
  run_pass(&mut tree);
  assert_eq!(reached.load(Ordering::SeqCst), 1);
}

#[test]
fn virtualized_list_scrollbar_state_uses_full_measured_extent() {
  let mut tree = Tree::new();
  let reached = Arc::new(AtomicUsize::new(0));
  tree.mount_root::<VirtualizedRoot>(&mut App::new(), (rows(10), Shared(reached)));

  run_pass(&mut tree);
  run_pass(&mut tree);

  tree.scroll(10.0, 10.0, 0.0, -1000.0, ScrollPhase::Scroll);
  run_pass(&mut tree);

  let layout = tree.last_layout().unwrap();
  let content = scroll_content(layout);
  assert_eq!(scroll_offset(layout), 400.0);
  assert_eq!(scroll_content_height(layout), 500.0);
  assert_eq!(content.children.len(), 3);
  assert_eq!(content.children[0].result.size.height, 400.0);
}

#[test]
fn virtualized_list_preserves_anchor_when_rows_are_prepended() {
  let mut tree = Tree::new();
  let reached = Arc::new(AtomicUsize::new(0));
  tree.mount_root::<VirtualizedRoot>(&mut App::new(), (rows(10), Shared(reached.clone())));

  run_pass(&mut tree);
  run_pass(&mut tree);
  tree.scroll(10.0, 10.0, 0.0, -150.0, ScrollPhase::Scroll);
  run_pass(&mut tree);
  assert_eq!(scroll_offset(tree.last_layout().unwrap()), 150.0);

  let mut prepended = vec![RowData { id: 100, height: 50.0 }, RowData { id: 101, height: 50.0 }];
  prepended.extend(rows(10));
  tree.update_root_props::<VirtualizedRoot>((prepended, Shared(reached)));

  run_pass(&mut tree);
  run_pass(&mut tree);

  let layout = tree.last_layout().unwrap();
  assert_eq!(scroll_offset(layout), 250.0);
  assert_eq!(scroll_content_height(layout), 600.0);
}

fn rows(count: usize) -> Vec<RowData> {
  (0..count).map(|id| RowData { id, height: 50.0 }).collect()
}

fn row_color(index: usize) -> Color {
  Color::new(index as u8, 64, 128, 255)
}

fn scroll_content(layout: &lurq::layout::layout_result::LayoutResult) -> &lurq::layout::layout_result::LayoutResult {
  &layout.children[0].result
}

fn scroll_offset(layout: &lurq::layout::layout_result::LayoutResult) -> f32 {
  -layout.children[0].offset.y
}

fn scroll_content_child_count(layout: &lurq::layout::layout_result::LayoutResult) -> usize {
  scroll_content(layout).children.len()
}

fn scroll_content_height(layout: &lurq::layout::layout_result::LayoutResult) -> f32 {
  scroll_content(layout).size.height
}
