use lurq::{
  app::{
    Runtime,
    component::Component,
    ctx::Ctx,
    events::{MouseButton, ScrollPhase},
  },
  components::{Column, Rect, Row, ScrollHorizontal, ScrollVertical},
  core::Signal,
  layout::layout_kind::ScrollState,
  node::{Element, color::Color},
};

use crate::support::run_pass;

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
  let mut runtime = Runtime::new();
  runtime.mount_root::<ScrollRerender>(());

  run_pass(&mut runtime);
  runtime.scroll(10.0, 10.0, 0.0, -60.0, ScrollPhase::Scroll);
  run_pass(&mut runtime);

  let content = runtime
    .find_element(|element| element.color() == Some(CONTENT_COLOR))
    .unwrap();
  assert_eq!(content.bounds().y, -60.0);

  runtime.click(10.0, 10.0, MouseButton::Left);
  run_pass(&mut runtime);

  let content = runtime
    .find_element(|element| element.color() == Some(CONTENT_COLOR))
    .unwrap();
  assert_eq!(content.bounds().y, -60.0);
}

#[test]
fn horizontal_scroll_responds_to_wheel_delta_x() {
  let mut runtime = Runtime::new();
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
fn horizontal_scrollbar_thumb_drags_content() {
  let mut runtime = Runtime::new();
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
fn vertical_scroll_bubbles_to_parent_when_child_is_at_edge() {
  let mut runtime = Runtime::new();
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
  let mut runtime = Runtime::new();
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
