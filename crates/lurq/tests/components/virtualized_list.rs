use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::ScrollPhase},
  components::{Rect, VirtualizedList},
  layout::layout_kind::ScrollState,
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

struct VirtualizedExternalScrollRoot;

impl Component for VirtualizedExternalScrollRoot {
  type Props = (Vec<RowData>, Shared<ScrollState>, Shared<AtomicUsize>);

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let (items, scroll_state, scroll_events) = ctx.props::<Self::Props>().clone();
    let scroll_events = scroll_events.0.clone();
    VirtualizedList::new(ctx, items)
      .size(100.0, 100.0)
      .overscan_px(0.0)
      .with_scroll_state((*scroll_state.0).clone())
      .on_scroll(move |_| {
        scroll_events.fetch_add(1, Ordering::SeqCst);
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
fn virtualized_list_accepts_external_scroll_state() {
  let mut tree = Tree::new();
  let scroll_state = Arc::new(ScrollState::new());
  let scroll_events = Arc::new(AtomicUsize::new(0));
  tree.mount_root::<VirtualizedExternalScrollRoot>(
    &mut App::new(),
    (rows(10), Shared(scroll_state.clone()), Shared(scroll_events.clone())),
  );

  run_pass(&mut tree);
  run_pass(&mut tree);

  tree.scroll(10.0, 10.0, 0.0, -150.0, ScrollPhase::Scroll);
  run_pass(&mut tree);

  let layout = tree.last_layout().unwrap();
  let content = scroll_content(layout);
  assert_eq!(scroll_state.scroll_y(), 150.0);
  assert_eq!(scroll_offset(layout), 150.0);
  assert_eq!(content.children.len(), 4);
  assert_eq!(content.children[0].result.size.height, 150.0);
  assert_eq!(content.children[3].result.size.height, 250.0);
  assert_eq!(scroll_events.load(Ordering::SeqCst), 1);
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
  assert!(scroll_content_child_count(tree.last_layout().unwrap()) < 12);
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

// ── Timing harness (run explicitly) ─────────────────────────────────────
// cargo test -p lurq --features "winit wgpu image svg resources clipboard" \
//   virtualized_list_scroll_timing -- --ignored --nocapture

#[derive(Clone, PartialEq, DevtoolsInspectable)]
struct TimingLine {
  id: usize,
  text: std::sync::Arc<str>,
}

struct TimingRow;

impl Component for TimingRow {
  type Props = TimingLine;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let line = ctx.props::<Self::Props>().clone();
    lurq::components::Row::new()
      .min_width(2400.0)
      .child(
        lurq::components::Row::new()
          .width(48.0)
          .child(lurq::components::Text::new(&(line.id + 1).to_string())),
      )
      .child(
        lurq::components::Text::new(&line.text)
          .nowrap()
          .selectable(true),
      )
  }
}

struct TimingRoot;

impl Component for TimingRoot {
  type Props = Shared<Vec<TimingLine>>;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let items = ctx.props::<Self::Props>().0.as_ref().clone();
    VirtualizedList::new(ctx, items)
      .size(900.0, 700.0)
      .horizontal_scroll(true)
      .mount_keyed::<TimingRow, _, _, _>(|line| line.id, |line| line.clone())
  }
}

#[test]
#[ignore]
fn virtualized_list_scroll_timing() {
  use std::time::Instant;

  let lines: Vec<TimingLine> = (0..10_000)
    .map(|id| TimingLine {
      id,
      text: std::sync::Arc::from(format!(
        "<TerrainRoad id=\"{id}\" texture=\"surfaces/road/autobase_{id}.dds\" \
         u0=\"0.125\" v0=\"0.25\" u1=\"0.875\" v1=\"0.75\" blend=\"true\" layer=\"3\"/>"
      )),
    })
    .collect();

  let mut tree = Tree::new();
  let mut app = App::new();
  tree.mount_root::<TimingRoot>(&mut app, Shared(Arc::new(lines)));

  let mut timed_pass = |tree: &mut Tree, label: &str| {
    let started = Instant::now();
    tree.request_redraw();
    tree.pass(&mut app, &crate::support::TestSurface);
    eprintln!("{label}: {:.2}ms", started.elapsed().as_secs_f64() * 1000.0);
  };

  timed_pass(&mut tree, "bootstrap");
  timed_pass(&mut tree, "settle-1");
  timed_pass(&mut tree, "settle-2");

  for step in 0..12 {
    let started = Instant::now();
    tree.scroll(400.0, 300.0, 0.0, -400.0, ScrollPhase::Scroll);
    let scroll_ms = started.elapsed().as_secs_f64() * 1000.0;
    eprintln!("scroll-{step}: event {scroll_ms:.2}ms");
    timed_pass(&mut tree, &format!("  pass-{step}"));
    timed_pass(&mut tree, &format!("  settle-{step}"));
  }
}

struct PlainTextRow;

impl Component for PlainTextRow {
  type Props = TimingLine;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let line = ctx.props::<Self::Props>().clone();
    lurq::components::Row::new()
      .min_width(2400.0)
      .child(lurq::components::Text::new(&line.text).nowrap())
  }
}

struct RectRow;

impl Component for RectRow {
  type Props = TimingLine;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Rect::new(2400.0, 22.0).background(Color::from_hex("#334455"))
  }
}

struct PlainTimingRoot;

impl Component for PlainTimingRoot {
  type Props = Shared<Vec<TimingLine>>;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let items = ctx.props::<Self::Props>().0.as_ref().clone();
    VirtualizedList::new(ctx, items)
      .size(900.0, 700.0)
      .mount_keyed::<PlainTextRow, _, _, _>(|line| line.id, |line| line.clone())
  }
}

struct RectTimingRoot;

impl Component for RectTimingRoot {
  type Props = Shared<Vec<TimingLine>>;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let items = ctx.props::<Self::Props>().0.as_ref().clone();
    VirtualizedList::new(ctx, items)
      .size(900.0, 700.0)
      .mount_keyed::<RectRow, _, _, _>(|line| line.id, |line| line.clone())
  }
}

fn timing_lines() -> Vec<TimingLine> {
  (0..10_000)
    .map(|id| TimingLine {
      id,
      text: std::sync::Arc::from(format!(
        "<TerrainRoad id=\"{id}\" texture=\"surfaces/road/autobase_{id}.dds\" \
         u0=\"0.125\" v0=\"0.25\" u1=\"0.875\" v1=\"0.75\" blend=\"true\" layer=\"3\"/>"
      )),
    })
    .collect()
}

fn run_timing<R>(label: &str)
where
  R: Component<Props = Shared<Vec<TimingLine>>>,
{
  use std::time::Instant;
  let mut tree = Tree::new();
  let mut app = App::new();
  tree.mount_root::<R>(&mut app, Shared(Arc::new(timing_lines())));

  let mut timed_pass = |tree: &mut Tree, label: String| {
    let started = Instant::now();
    tree.request_redraw();
    tree.pass(&mut app, &crate::support::TestSurface);
    eprintln!("{label}: {:.2}ms", started.elapsed().as_secs_f64() * 1000.0);
  };

  timed_pass(&mut tree, format!("[{label}] bootstrap"));
  timed_pass(&mut tree, format!("[{label}] settle"));
  for step in 0..3 {
    tree.scroll(400.0, 300.0, 0.0, -400.0, ScrollPhase::Scroll);
    timed_pass(&mut tree, format!("[{label}] jump-pass-{step}"));
    timed_pass(&mut tree, format!("[{label}] jump-settle-{step}"));
  }
  for step in 0..3 {
    tree.scroll(400.0, 300.0, 0.0, -160.0, ScrollPhase::Scroll);
    timed_pass(&mut tree, format!("[{label}] step-pass-{step}"));
  }
}

#[test]
#[ignore]
fn virtualized_list_scroll_timing_variants() {
  run_timing::<RectTimingRoot>("rect");
  run_timing::<PlainTimingRoot>("plain-text");
  run_timing::<TimingRoot>("selectable-text");
}
