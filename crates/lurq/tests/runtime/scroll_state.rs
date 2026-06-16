use std::sync::{
  Arc, Mutex,
  atomic::{AtomicUsize, Ordering},
};

#[cfg(feature = "markdown")]
use lurq::components::{Markdown, MarkdownProps};
#[cfg(feature = "markdown")]
use lurq::node::dimension::Dimension;
use lurq::{
  app::{
    Tree,
    component::Component,
    ctx::{Ctx, Overlay, Placement},
    events::{MouseButton, ScrollPhase},
  },
  components::{Column, Rect, Row, ScrollHorizontal, ScrollVertical, VirtualListState},
  core::{ElementRef as CoreElementRef, Signal},
  layout::{layout_kind::ScrollState, scrollbar::ScrollBarStyle},
  node::{Element, color::Color},
};

use crate::support::{pointer_click, run_pass};

const CONTENT_COLOR: Color = Color::new(255, 0, 255, 255);
#[cfg(feature = "markdown")]
const VIRTUAL_LIST_FOLLOWING_ROW_COLOR: Color = Color::new(0, 255, 255, 255);

#[derive(lurq::DevtoolsInspectable)]
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

impl<T> std::fmt::Debug for Shared<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_tuple("Shared").field(&(Arc::as_ptr(&self.0) as usize)).finish()
  }
}

struct ScrollRerender {
  ticks: Signal<u32>,
}

impl Component for ScrollRerender {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self { ticks: ctx.signal(0) }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let _ = self.ticks.get();
    let ticks = self.ticks.clone();

    ScrollVertical::new(Rect::new(100.0, 400.0).background(CONTENT_COLOR))
      .on_click(move |_| ticks.update(|ticks| *ticks += 1))
      .size(100.0, 100.0)
  }
}

#[test]
fn scroll_state_survives_signal_driven_rerender() {
  let mut runtime = Tree::new();
  runtime.mount_root::<ScrollRerender>(&mut lurq::app::App::new(), ());

  run_pass(&mut runtime);
  runtime.scroll(10.0, 10.0, 0.0, -60.0, ScrollPhase::Scroll);
  run_pass(&mut runtime);

  let content = runtime
    .find_element(|element| element.color() == Some(CONTENT_COLOR))
    .unwrap();
  assert_eq!(content.bounds().y, -60.0);

  pointer_click(&mut runtime, 10.0, 10.0, MouseButton::Left);
  run_pass(&mut runtime);

  let content = runtime
    .find_element(|element| element.color() == Some(CONTENT_COLOR))
    .unwrap();
  assert_eq!(content.bounds().y, -60.0);
}

struct ScrollbarDragReactiveRoot {
  ticks: Signal<u32>,
  renders: Arc<AtomicUsize>,
}

impl Component for ScrollbarDragReactiveRoot {
  type Props = Shared<AtomicUsize>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      ticks: ctx.signal(0),
      renders: ctx.props::<Self::Props>().0.clone(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    self.renders.fetch_add(1, Ordering::SeqCst);
    let _ = self.ticks.get();
    let ticks = self.ticks.clone();

    ScrollVertical::new(Rect::new(100.0, 400.0).background(CONTENT_COLOR))
      .on_scroll(move |_| ticks.update(|ticks| *ticks += 1))
      .size(100.0, 100.0)
  }
}

struct ScrollReachTopRoot {
  reached: Arc<AtomicUsize>,
}

struct ScrollReachBottomRoot {
  reached: Arc<AtomicUsize>,
}

impl Component for ScrollReachBottomRoot {
  type Props = Shared<AtomicUsize>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      reached: ctx.props::<Self::Props>().0.clone(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let reached = self.reached.clone();
    ScrollVertical::new(Rect::new(100.0, 400.0).background(CONTENT_COLOR))
      .on_scroll_reach_bottom(move |_| {
        reached.fetch_add(1, Ordering::SeqCst);
      })
      .size(100.0, 100.0)
  }
}

impl Component for ScrollReachTopRoot {
  type Props = Shared<AtomicUsize>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      reached: ctx.props::<Self::Props>().0.clone(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let reached = self.reached.clone();
    ScrollVertical::new(Rect::new(100.0, 400.0).background(CONTENT_COLOR))
      .on_scroll_reach_top(move |_| {
        reached.fetch_add(1, Ordering::SeqCst);
      })
      .size(100.0, 100.0)
  }
}

#[test]
fn scroll_reach_top_fires_once_when_crossing_into_top_edge() {
  let mut runtime = Tree::new();
  let reached = Arc::new(AtomicUsize::new(0));
  runtime.mount_root::<ScrollReachTopRoot>(&mut lurq::app::App::new(), Shared(reached.clone()));

  run_pass(&mut runtime);
  runtime.scroll(10.0, 10.0, 0.0, -160.0, ScrollPhase::Scroll);
  run_pass(&mut runtime);
  assert_eq!(reached.load(Ordering::SeqCst), 0);

  runtime.scroll(10.0, 10.0, 0.0, 240.0, ScrollPhase::Scroll);
  run_pass(&mut runtime);
  assert_eq!(reached.load(Ordering::SeqCst), 1);

  runtime.scroll(10.0, 10.0, 0.0, 40.0, ScrollPhase::Scroll);
  run_pass(&mut runtime);
  assert_eq!(reached.load(Ordering::SeqCst), 1);

  runtime.scroll(10.0, 10.0, 0.0, -80.0, ScrollPhase::Scroll);
  run_pass(&mut runtime);
  runtime.scroll(10.0, 10.0, 0.0, 120.0, ScrollPhase::Scroll);
  run_pass(&mut runtime);
  assert_eq!(reached.load(Ordering::SeqCst), 2);
}

#[test]
fn scroll_reach_bottom_fires_once_when_crossing_into_bottom_edge() {
  let mut runtime = Tree::new();
  let reached = Arc::new(AtomicUsize::new(0));
  runtime.mount_root::<ScrollReachBottomRoot>(&mut lurq::app::App::new(), Shared(reached.clone()));

  run_pass(&mut runtime);
  runtime.scroll(10.0, 10.0, 0.0, -360.0, ScrollPhase::Scroll);
  run_pass(&mut runtime);
  assert_eq!(reached.load(Ordering::SeqCst), 1);

  runtime.scroll(10.0, 10.0, 0.0, -40.0, ScrollPhase::Scroll);
  run_pass(&mut runtime);
  assert_eq!(reached.load(Ordering::SeqCst), 1);

  runtime.scroll(10.0, 10.0, 0.0, 80.0, ScrollPhase::Scroll);
  run_pass(&mut runtime);
  runtime.scroll(10.0, 10.0, 0.0, -120.0, ScrollPhase::Scroll);
  run_pass(&mut runtime);
  assert_eq!(reached.load(Ordering::SeqCst), 2);
}

#[test]
fn scrollbar_thumb_drag_defers_reactive_rerender_until_redraw_pass() {
  let mut runtime = Tree::new();
  let renders = Arc::new(AtomicUsize::new(0));
  runtime.mount_root::<ScrollbarDragReactiveRoot>(&mut lurq::app::App::new(), Shared(renders.clone()));

  run_pass(&mut runtime);
  let initial_renders = renders.load(Ordering::SeqCst);

  runtime.mouse_down(94.0, 10.0, MouseButton::Left);
  runtime.mouse_move(94.0, 30.0);
  runtime.mouse_move(94.0, 50.0);

  assert_eq!(renders.load(Ordering::SeqCst), initial_renders);

  run_pass(&mut runtime);
  assert_eq!(renders.load(Ordering::SeqCst), initial_renders + 1);
}

struct ScrollCullingCacheRoot {
  culling: Signal<bool>,
  scroll_state: ScrollState,
  child_renders: Arc<AtomicUsize>,
  child_mounts: Arc<AtomicUsize>,
  child_unmounts: Arc<AtomicUsize>,
}

impl Component for ScrollCullingCacheRoot {
  type Props = (
    Shared<std::sync::Mutex<Option<Signal<bool>>>>,
    Shared<ScrollState>,
    Shared<AtomicUsize>,
    Shared<AtomicUsize>,
    Shared<AtomicUsize>,
  );

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>().clone();
    let culling = ctx.signal(true);
    *props.0.0.lock().unwrap() = Some(culling.clone());
    Self {
      culling,
      scroll_state: (*props.1.0).clone(),
      child_renders: props.2.0,
      child_mounts: props.3.0,
      child_unmounts: props.4.0,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let child = ctx.mount::<ScrollCullingCacheChild>((
      Shared(self.child_renders.clone()),
      Shared(self.child_mounts.clone()),
      Shared(self.child_unmounts.clone()),
    ));
    let content = Column::new().spacing(0.0).child(child).child(Rect::new(100.0, 400.0));

    ScrollVertical::new(content)
      .with_scroll_state(self.scroll_state.clone())
      .culling(self.culling.get())
      .size(100.0, 100.0)
  }
}

struct ScrollCullingCacheChild {
  renders: Arc<AtomicUsize>,
  mounts: Arc<AtomicUsize>,
  unmounts: Arc<AtomicUsize>,
}

impl Component for ScrollCullingCacheChild {
  type Props = (Shared<AtomicUsize>, Shared<AtomicUsize>, Shared<AtomicUsize>);

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>().clone();
    Self {
      renders: props.0.0,
      mounts: props.1.0,
      unmounts: props.2.0,
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    self.renders.fetch_add(1, Ordering::SeqCst);
    Rect::new(100.0, 50.0).background(CONTENT_COLOR)
  }

  fn on_mounted(&self) {
    self.mounts.fetch_add(1, Ordering::SeqCst);
  }

  fn on_unmounted(&self) {
    self.unmounts.fetch_add(1, Ordering::SeqCst);
  }
}

#[test]
fn scroll_culling_does_not_remount_or_rerender_cached_child_components() {
  let culling = Arc::new(std::sync::Mutex::new(None));
  let scroll_state = ScrollState::new();
  let child_renders = Arc::new(AtomicUsize::new(0));
  let child_mounts = Arc::new(AtomicUsize::new(0));
  let child_unmounts = Arc::new(AtomicUsize::new(0));
  let mut runtime = Tree::new();

  runtime.mount_root::<ScrollCullingCacheRoot>(
    &mut lurq::app::App::new(),
    (
      Shared(culling.clone()),
      Shared(Arc::new(scroll_state.clone())),
      Shared(child_renders.clone()),
      Shared(child_mounts.clone()),
      Shared(child_unmounts.clone()),
    ),
  );
  run_pass(&mut runtime);

  assert_eq!(child_renders.load(Ordering::SeqCst), 1);
  assert_eq!(child_mounts.load(Ordering::SeqCst), 1);
  assert_eq!(child_unmounts.load(Ordering::SeqCst), 0);

  scroll_state.set_scroll_pending(0.0, 80.0);
  run_pass(&mut runtime);
  assert_eq!(child_renders.load(Ordering::SeqCst), 1);
  assert_eq!(child_mounts.load(Ordering::SeqCst), 1);
  assert_eq!(child_unmounts.load(Ordering::SeqCst), 0);

  culling.lock().unwrap().as_ref().unwrap().set(false);
  run_pass(&mut runtime);
  assert_eq!(child_renders.load(Ordering::SeqCst), 1);
  assert_eq!(child_mounts.load(Ordering::SeqCst), 1);
  assert_eq!(child_unmounts.load(Ordering::SeqCst), 0);

  culling.lock().unwrap().as_ref().unwrap().set(true);
  run_pass(&mut runtime);
  assert_eq!(child_renders.load(Ordering::SeqCst), 1);
  assert_eq!(child_mounts.load(Ordering::SeqCst), 1);
  assert_eq!(child_unmounts.load(Ordering::SeqCst), 0);
}

struct VirtualListRoot {
  state: VirtualListState,
  items: Vec<usize>,
  renders: Arc<AtomicUsize>,
  mounts: Arc<AtomicUsize>,
  unmounts: Arc<AtomicUsize>,
}

impl Component for VirtualListRoot {
  type Props = (Shared<AtomicUsize>, Shared<AtomicUsize>, Shared<AtomicUsize>);

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>().clone();
    Self {
      state: VirtualListState::new(ctx)
        .with_overscan(0)
        .with_initial_visible_count(5),
      items: (0..100).collect(),
      renders: props.0.0,
      mounts: props.1.0,
      unmounts: props.2.0,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let renders = self.renders.clone();
    let mounts = self.mounts.clone();
    let unmounts = self.unmounts.clone();
    ctx
      .virtual_list(
        &self.state,
        &self.items,
        |item| *item,
        move |ctx, item| {
          ctx.mount_keyed::<VirtualListRow>(
            &item.to_string(),
            (
              Shared(renders.clone()),
              Shared(mounts.clone()),
              Shared(unmounts.clone()),
            ),
          )
        },
      )
      .scrollbar(ScrollBarStyle::hidden())
      .size(100.0, 100.0)
  }
}

struct VirtualListRow {
  renders: Arc<AtomicUsize>,
  mounts: Arc<AtomicUsize>,
  unmounts: Arc<AtomicUsize>,
}

impl Component for VirtualListRow {
  type Props = (Shared<AtomicUsize>, Shared<AtomicUsize>, Shared<AtomicUsize>);

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>().clone();
    Self {
      renders: props.0.0,
      mounts: props.1.0,
      unmounts: props.2.0,
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    self.renders.fetch_add(1, Ordering::SeqCst);
    Rect::new(100.0, 20.0).background(CONTENT_COLOR)
  }

  fn on_mounted(&self) {
    self.mounts.fetch_add(1, Ordering::SeqCst);
  }

  fn on_unmounted(&self) {
    self.unmounts.fetch_add(1, Ordering::SeqCst);
  }
}

struct VirtualListPruneRoot {
  state: VirtualListState,
  item_count: Signal<usize>,
  item_count_out: Arc<Mutex<Option<Signal<usize>>>>,
  renders: Arc<AtomicUsize>,
  mounts: Arc<AtomicUsize>,
  unmounts: Arc<AtomicUsize>,
  drops: Arc<AtomicUsize>,
}

impl Component for VirtualListPruneRoot {
  type Props = (
    Shared<Mutex<Option<Signal<usize>>>>,
    Shared<AtomicUsize>,
    Shared<AtomicUsize>,
    Shared<AtomicUsize>,
    Shared<AtomicUsize>,
  );

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>().clone();
    let item_count = ctx.signal(20);
    *props.0.0.lock().unwrap() = Some(item_count.clone());
    Self {
      state: VirtualListState::new(ctx)
        .with_overscan(0)
        .with_initial_visible_count(5),
      item_count,
      item_count_out: props.0.0,
      renders: props.1.0,
      mounts: props.2.0,
      unmounts: props.3.0,
      drops: props.4.0,
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    *self.item_count_out.lock().unwrap() = Some(self.item_count.clone());
    let items = (0..self.item_count.get()).collect::<Vec<_>>();
    let renders = self.renders.clone();
    let mounts = self.mounts.clone();
    let unmounts = self.unmounts.clone();
    let drops = self.drops.clone();

    ctx
      .virtual_list(
        &self.state,
        &items,
        |item| *item,
        move |ctx, item| {
          ctx.mount_keyed::<VirtualListPruneRow>(
            &item.to_string(),
            (
              Shared(renders.clone()),
              Shared(mounts.clone()),
              Shared(unmounts.clone()),
              Shared(drops.clone()),
            ),
          )
        },
      )
      .scrollbar(ScrollBarStyle::hidden())
      .size(100.0, 100.0)
  }
}

struct VirtualListPruneRow {
  renders: Arc<AtomicUsize>,
  mounts: Arc<AtomicUsize>,
  unmounts: Arc<AtomicUsize>,
  drops: Arc<AtomicUsize>,
}

#[cfg(feature = "markdown")]
struct VirtualListWrappedMarkdownRoot {
  state: VirtualListState,
  items: Vec<usize>,
}

#[cfg(feature = "markdown")]
impl Component for VirtualListWrappedMarkdownRoot {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      state: VirtualListState::new(ctx)
        .with_overscan(0)
        .with_initial_visible_count(2),
      items: vec![0, 1],
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    ctx
      .virtual_list(
        &self.state,
        &self.items,
        |item| *item,
        |ctx, item| -> Element {
          match *item {
            0 => Column::new()
              .width(Dimension::Pct(100.0))
              .padding_bottom(8.0)
              .child(ctx.mount::<Markdown>(
                MarkdownProps::new(
                  "Queued  \n- *first wrapped item with enough words to take width*  \n- second wrapped item  \n- third wrapped item",
                )
                .width(Dimension::Pct(100.0)),
              ))
              .into(),
            _ => Rect::new(100.0, 20.0)
              .background(VIRTUAL_LIST_FOLLOWING_ROW_COLOR)
              .into(),
          }
        },
      )
      .scrollbar(ScrollBarStyle::hidden())
      .size(120.0, 120.0)
  }
}

impl Component for VirtualListPruneRow {
  type Props = (
    Shared<AtomicUsize>,
    Shared<AtomicUsize>,
    Shared<AtomicUsize>,
    Shared<AtomicUsize>,
  );

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>().clone();
    Self {
      renders: props.0.0,
      mounts: props.1.0,
      unmounts: props.2.0,
      drops: props.3.0,
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    self.renders.fetch_add(1, Ordering::SeqCst);
    Rect::new(100.0, 20.0).background(CONTENT_COLOR)
  }

  fn on_mounted(&self) {
    self.mounts.fetch_add(1, Ordering::SeqCst);
  }

  fn on_unmounted(&self) {
    self.unmounts.fetch_add(1, Ordering::SeqCst);
  }
}

impl Drop for VirtualListPruneRow {
  fn drop(&mut self) {
    self.drops.fetch_add(1, Ordering::SeqCst);
  }
}

#[test]
fn virtual_list_measures_rows_in_bounded_batches_before_virtualizing() {
  let renders = Arc::new(AtomicUsize::new(0));
  let mounts = Arc::new(AtomicUsize::new(0));
  let unmounts = Arc::new(AtomicUsize::new(0));
  let mut runtime = Tree::new();

  runtime.mount_root::<VirtualListRoot>(
    &mut lurq::app::App::new(),
    (
      Shared(renders.clone()),
      Shared(mounts.clone()),
      Shared(unmounts.clone()),
    ),
  );
  run_pass(&mut runtime);

  assert_eq!(renders.load(Ordering::SeqCst), 5);
  assert_eq!(mounts.load(Ordering::SeqCst), 5);
  assert_eq!(unmounts.load(Ordering::SeqCst), 0);

  runtime.tick_timers();
  run_pass(&mut runtime);
  assert_eq!(renders.load(Ordering::SeqCst), 69);
  assert_eq!(mounts.load(Ordering::SeqCst), 69);
  assert_eq!(unmounts.load(Ordering::SeqCst), 0);

  runtime.tick_timers();
  run_pass(&mut runtime);
  assert_eq!(renders.load(Ordering::SeqCst), 100);
  assert_eq!(mounts.load(Ordering::SeqCst), 100);
  assert_eq!(unmounts.load(Ordering::SeqCst), 0);
}

#[test]
fn virtual_list_retains_keyed_rows_when_they_scroll_out_and_back_in() {
  let renders = Arc::new(AtomicUsize::new(0));
  let mounts = Arc::new(AtomicUsize::new(0));
  let unmounts = Arc::new(AtomicUsize::new(0));
  let mut runtime = Tree::new();

  runtime.mount_root::<VirtualListRoot>(
    &mut lurq::app::App::new(),
    (
      Shared(renders.clone()),
      Shared(mounts.clone()),
      Shared(unmounts.clone()),
    ),
  );
  run_pass(&mut runtime);
  runtime.tick_timers();
  run_pass(&mut runtime);
  runtime.tick_timers();
  run_pass(&mut runtime);

  runtime.scroll(10.0, 10.0, 0.0, -160.0, ScrollPhase::Scroll);
  run_pass(&mut runtime);
  assert_eq!(renders.load(Ordering::SeqCst), 100);
  assert_eq!(mounts.load(Ordering::SeqCst), 100);
  assert_eq!(unmounts.load(Ordering::SeqCst), 0);

  runtime.scroll(10.0, 10.0, 0.0, 160.0, ScrollPhase::Scroll);
  run_pass(&mut runtime);
  assert_eq!(renders.load(Ordering::SeqCst), 100);
  assert_eq!(mounts.load(Ordering::SeqCst), 100);
  assert_eq!(unmounts.load(Ordering::SeqCst), 0);
}

#[cfg(feature = "markdown")]
#[test]
fn virtual_list_measures_wrapped_markdown_before_positioning_next_row() {
  let mut runtime = Tree::new();
  runtime.mount_root::<VirtualListWrappedMarkdownRoot>(&mut lurq::app::App::new(), ());

  run_pass(&mut runtime);
  run_pass(&mut runtime);

  let following = runtime
    .find_element(|element| element.color() == Some(VIRTUAL_LIST_FOLLOWING_ROW_COLOR))
    .unwrap();
  assert!(
    following.bounds().y > 70.0,
    "following row should be positioned after the full wrapped markdown height"
  );
}

#[test]
fn virtual_list_prunes_retained_rows_when_items_are_removed() {
  let item_count = Arc::new(Mutex::new(None));
  let renders = Arc::new(AtomicUsize::new(0));
  let mounts = Arc::new(AtomicUsize::new(0));
  let unmounts = Arc::new(AtomicUsize::new(0));
  let drops = Arc::new(AtomicUsize::new(0));
  let mut runtime = Tree::new();

  runtime.mount_root::<VirtualListPruneRoot>(
    &mut lurq::app::App::new(),
    (
      Shared(item_count.clone()),
      Shared(renders.clone()),
      Shared(mounts.clone()),
      Shared(unmounts.clone()),
      Shared(drops.clone()),
    ),
  );
  run_pass(&mut runtime);

  for _ in 0..3 {
    runtime.scroll(10.0, 10.0, 0.0, -100.0, ScrollPhase::Scroll);
    run_pass(&mut runtime);
  }

  assert_eq!(mounts.load(Ordering::SeqCst), 20);
  assert_eq!(unmounts.load(Ordering::SeqCst), 0);
  assert_eq!(drops.load(Ordering::SeqCst), 0);

  item_count.lock().unwrap().as_ref().unwrap().set(5);
  run_pass(&mut runtime);

  assert_eq!(unmounts.load(Ordering::SeqCst), 15);
  assert_eq!(drops.load(Ordering::SeqCst), 15);

  for _ in 0..5 {
    runtime.scroll(10.0, 10.0, 0.0, 100.0, ScrollPhase::Scroll);
    run_pass(&mut runtime);
    runtime.scroll(10.0, 10.0, 0.0, -100.0, ScrollPhase::Scroll);
    run_pass(&mut runtime);
  }

  assert_eq!(mounts.load(Ordering::SeqCst), 20);
  assert_eq!(unmounts.load(Ordering::SeqCst), 15);
  assert_eq!(drops.load(Ordering::SeqCst), 15);
}

#[test]
fn horizontal_scroll_responds_to_wheel_delta_x() {
  let mut runtime = Tree::new();
  runtime.set_root(ScrollHorizontal::new(Rect::new(400.0, 100.0).background(CONTENT_COLOR)).size(100.0, 100.0));

  run_pass(&mut runtime);
  runtime.scroll(10.0, 10.0, -60.0, 0.0, ScrollPhase::Scroll);
  run_pass(&mut runtime);

  let content = runtime
    .find_element(|element| element.color() == Some(CONTENT_COLOR))
    .unwrap();
  assert_eq!(content.bounds().x, -60.0);
}

#[test]
fn pending_scroll_to_bottom_resolves_before_first_paint() {
  let mut runtime = Tree::new();
  let state = ScrollState::new();
  state.scroll_to_bottom_pending();

  runtime.set_root(
    ScrollVertical::new(Rect::new(100.0, 400.0).background(CONTENT_COLOR))
      .with_scroll_state(state.clone())
      .size(100.0, 100.0),
  );

  run_pass(&mut runtime);

  let content = runtime
    .find_element(|element| element.color() == Some(CONTENT_COLOR))
    .unwrap();
  assert_eq!(state.scroll_y(), 300.0);
  assert_eq!(content.bounds().y, -300.0);
}

#[test]
fn pending_scroll_to_right_resolves_before_first_paint() {
  let mut runtime = Tree::new();
  let state = ScrollState::new();
  state.scroll_to_right_pending();

  runtime.set_root(
    ScrollHorizontal::new(Rect::new(400.0, 100.0).background(CONTENT_COLOR))
      .with_scroll_state(state.clone())
      .size(100.0, 100.0),
  );

  run_pass(&mut runtime);

  let content = runtime
    .find_element(|element| element.color() == Some(CONTENT_COLOR))
    .unwrap();
  assert_eq!(state.scroll_x(), 300.0);
  assert_eq!(content.bounds().x, -300.0);
}

#[test]
fn overlay_pending_scroll_updates_content_offset_after_measurement() {
  let mut runtime = Tree::new();
  let anchor = CoreElementRef::new();
  let state = ScrollState::new();

  runtime.set_root(
    Column::new()
      .child(Rect::new(100.0, 20.0).background("#22c55e").ref_element(anchor.clone()))
      .child(
        Overlay::new(
          ScrollVertical::new(Rect::new(100.0, 400.0).background(CONTENT_COLOR))
            .with_scroll_state(state.clone())
            .size(100.0, 100.0),
        )
        .anchor(anchor)
        .placement(Placement::BottomStart),
      ),
  );

  run_pass(&mut runtime);
  let initial_y = runtime
    .find_element(|element| element.color() == Some(CONTENT_COLOR))
    .unwrap()
    .bounds()
    .y;

  state.set_scroll_pending(0.0, 200.0);
  run_pass(&mut runtime);

  let content = runtime
    .find_element(|element| element.color() == Some(CONTENT_COLOR))
    .unwrap();
  assert_eq!(state.scroll_y(), 200.0);
  assert_eq!(content.bounds().y, initial_y - 200.0);
}

#[test]
fn overlay_direct_scroll_updates_content_offset_after_reuse() {
  let mut runtime = Tree::new();
  let anchor = CoreElementRef::new();
  let state = ScrollState::new();

  runtime.set_root(
    Column::new()
      .child(Rect::new(100.0, 20.0).background("#22c55e").ref_element(anchor.clone()))
      .child(
        Overlay::new(
          ScrollVertical::new(Rect::new(100.0, 400.0).background(CONTENT_COLOR))
            .with_scroll_state(state.clone())
            .size(100.0, 100.0),
        )
        .anchor(anchor)
        .placement(Placement::BottomStart),
      ),
  );

  run_pass(&mut runtime);
  let initial_y = runtime
    .find_element(|element| element.color() == Some(CONTENT_COLOR))
    .unwrap()
    .bounds()
    .y;

  state.scroll_by(0.0, 200.0);
  run_pass(&mut runtime);

  let content = runtime
    .find_element(|element| element.color() == Some(CONTENT_COLOR))
    .unwrap();
  assert_eq!(state.scroll_y(), 200.0);
  assert_eq!(content.bounds().y, initial_y - 200.0);
}

#[test]
fn stick_to_bottom_only_when_near_end() {
  let mut runtime = Tree::new();
  let state = ScrollState::new();

  runtime.set_root(
    ScrollVertical::new(Rect::new(100.0, 300.0).background(CONTENT_COLOR))
      .with_scroll_state(state.clone())
      .size(100.0, 100.0),
  );
  run_pass(&mut runtime);
  state.set_scroll(0.0, 196.0);

  assert!(state.stick_to_bottom_if_near_end(8.0));
  runtime.set_root(
    ScrollVertical::new(Rect::new(100.0, 400.0).background(CONTENT_COLOR))
      .with_scroll_state(state.clone())
      .size(100.0, 100.0),
  );
  run_pass(&mut runtime);
  assert_eq!(state.scroll_y(), 300.0);

  state.set_scroll(0.0, 50.0);
  assert!(!state.stick_to_bottom_if_near_end(8.0));
  runtime.set_root(
    ScrollVertical::new(Rect::new(100.0, 500.0).background(CONTENT_COLOR))
      .with_scroll_state(state.clone())
      .size(100.0, 100.0),
  );
  run_pass(&mut runtime);
  assert_eq!(state.scroll_y(), 50.0);
}

#[test]
fn preserve_prepend_anchor_compensates_added_content_height() {
  let mut runtime = Tree::new();
  let state = ScrollState::new();

  runtime.set_root(
    ScrollVertical::new(Rect::new(100.0, 300.0).background(CONTENT_COLOR))
      .with_scroll_state(state.clone())
      .size(100.0, 100.0),
  );
  run_pass(&mut runtime);
  state.set_scroll(0.0, 80.0);
  state.preserve_prepend_anchor_pending();

  runtime.set_root(
    ScrollVertical::new(Rect::new(100.0, 350.0).background(CONTENT_COLOR))
      .with_scroll_state(state.clone())
      .size(100.0, 100.0),
  );
  run_pass(&mut runtime);

  let content = runtime
    .find_element(|element| element.color() == Some(CONTENT_COLOR))
    .unwrap();
  assert_eq!(state.scroll_y(), 130.0);
  assert_eq!(content.bounds().y, -130.0);
}

#[test]
fn horizontal_scrollbar_thumb_drags_content() {
  let mut runtime = Tree::new();
  runtime.set_root(ScrollHorizontal::new(Rect::new(400.0, 100.0).background(CONTENT_COLOR)).size(100.0, 100.0));

  run_pass(&mut runtime);
  runtime.mouse_down(10.0, 94.0, MouseButton::Left);
  runtime.mouse_move(34.0, 94.0);
  runtime.mouse_up(34.0, 94.0, MouseButton::Left);
  run_pass(&mut runtime);

  let content = runtime
    .find_element(|element| element.color() == Some(CONTENT_COLOR))
    .unwrap();
  assert_eq!(content.bounds().x, -100.0);
}

#[test]
fn scrollbar_drag_release_does_not_click_under_cursor() {
  let mut runtime = Tree::new();
  let clicks = Arc::new(AtomicUsize::new(0));
  let click_count = clicks.clone();

  runtime.set_root(
    ScrollVertical::new(Rect::new(100.0, 400.0).background(CONTENT_COLOR))
      .on_click(move |_| {
        click_count.fetch_add(1, Ordering::SeqCst);
      })
      .size(100.0, 100.0),
  );

  run_pass(&mut runtime);
  runtime.mouse_down(94.0, 10.0, MouseButton::Left);
  runtime.mouse_move(94.0, 50.0);
  runtime.mouse_up(10.0, 50.0, MouseButton::Left);

  assert_eq!(clicks.load(Ordering::SeqCst), 0);
}

#[test]
fn scrollbar_thumb_drag_emits_scroll_event() {
  let mut runtime = Tree::new();
  let scroll_events = Arc::new(AtomicUsize::new(0));
  let scroll_count = scroll_events.clone();

  runtime.set_root(
    ScrollVertical::new(Rect::new(100.0, 400.0).background(CONTENT_COLOR))
      .on_scroll(move |_| {
        scroll_count.fetch_add(1, Ordering::SeqCst);
      })
      .size(100.0, 100.0),
  );

  run_pass(&mut runtime);
  runtime.mouse_down(94.0, 10.0, MouseButton::Left);
  runtime.mouse_move(94.0, 50.0);

  assert_eq!(scroll_events.load(Ordering::SeqCst), 1);
}

#[test]
fn hovering_scrollbar_thumb_does_not_hover_content_underneath() {
  let mut runtime = Tree::new();
  let content_ref = CoreElementRef::new();

  runtime.set_root(
    ScrollVertical::new(
      Rect::new(100.0, 400.0)
        .background(CONTENT_COLOR)
        .ref_element(content_ref.clone()),
    )
    .size(100.0, 100.0),
  );

  run_pass(&mut runtime);
  runtime.mouse_move(94.0, 10.0);
  assert!(!content_ref.hovered());

  runtime.mouse_move(10.0, 10.0);
  assert!(content_ref.hovered());
}

#[test]
fn vertical_scroll_bubbles_to_parent_when_child_is_at_edge() {
  let mut runtime = Tree::new();
  let parent_state = ScrollState::new();
  let child_state = ScrollState::new();

  let child = ScrollVertical::new(Rect::new(100.0, 300.0).background(CONTENT_COLOR))
    .with_scroll_state(child_state.clone())
    .size(100.0, 100.0);
  let parent_content = Column::new().spacing(0.0).child(child).child(Rect::new(100.0, 300.0));
  runtime.set_root(
    ScrollVertical::new(parent_content)
      .with_scroll_state(parent_state.clone())
      .size(100.0, 100.0),
  );

  run_pass(&mut runtime);
  runtime.scroll(10.0, 10.0, 0.0, -200.0, ScrollPhase::Scroll);
  run_pass(&mut runtime);
  assert_eq!(child_state.scroll_y(), 200.0);
  assert_eq!(parent_state.scroll_y(), 0.0);

  runtime.scroll(10.0, 10.0, 0.0, -60.0, ScrollPhase::Scroll);
  run_pass(&mut runtime);
  assert_eq!(child_state.scroll_y(), 200.0);
  assert_eq!(parent_state.scroll_y(), 60.0);
}

#[test]
fn horizontal_scroll_bubbles_to_parent_when_child_is_at_edge() {
  let mut runtime = Tree::new();
  let parent_state = ScrollState::new();
  let child_state = ScrollState::new();

  let child = ScrollHorizontal::new(Rect::new(300.0, 100.0).background(CONTENT_COLOR))
    .with_scroll_state(child_state.clone())
    .size(100.0, 100.0);
  let parent_content = Row::new().spacing(0.0).child(child).child(Rect::new(300.0, 100.0));
  runtime.set_root(
    ScrollHorizontal::new(parent_content)
      .with_scroll_state(parent_state.clone())
      .size(100.0, 100.0),
  );

  run_pass(&mut runtime);
  runtime.scroll(10.0, 10.0, -200.0, 0.0, ScrollPhase::Scroll);
  run_pass(&mut runtime);
  assert_eq!(child_state.scroll_x(), 200.0);
  assert_eq!(parent_state.scroll_x(), 0.0);

  runtime.scroll(10.0, 10.0, -60.0, 0.0, ScrollPhase::Scroll);
  run_pass(&mut runtime);
  assert_eq!(child_state.scroll_x(), 200.0);
  assert_eq!(parent_state.scroll_x(), 60.0);
}
