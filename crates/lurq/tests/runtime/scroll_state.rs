use lurq::{
  app::{
    Runtime,
    component::Component,
    ctx::Ctx,
    events::{MouseButton, ScrollPhase},
  },
  components::{Rect, ScrollVertical},
  core::Signal,
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
