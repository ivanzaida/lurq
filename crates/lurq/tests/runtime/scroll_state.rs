use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    Tree,
    component::Component,
    ctx::Ctx,
    events::{MouseButton, ScrollPhase},
  },
  components::{Column, Rect, Row, ScrollHorizontal, ScrollVertical},
  core::{ElementRef as CoreElementRef, Signal},
  layout::layout_kind::ScrollState,
  node::{Element, color::Color},
};

use crate::support::{pointer_click, run_pass};

const CONTENT_COLOR: Color = Color::new(255, 0, 255, 255);

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
