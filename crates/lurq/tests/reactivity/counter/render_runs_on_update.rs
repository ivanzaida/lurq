use std::sync::{
  atomic::{AtomicUsize, Ordering},
  Arc,
};

use lurq::{
  app::{component::Component, ctx::Ctx, events::MouseButton, Runtime},
  core::Signal,
  layout::{
    text_style::{FontWeight, TextStyle},
    Alignment,
  },
  node::{color::Color, Element},
};

struct Counter {
  count: Signal<i32>,
  renders: Arc<AtomicUsize>,
}

impl Component for Counter {
  type Props = Arc<AtomicUsize>;

  fn create(ctx: &mut Ctx, renders: Self::Props) -> Self {
    Self {
      count: ctx.signal(0),
      renders,
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> Element {
    self.renders.fetch_add(1, Ordering::Relaxed);

    let c = self.count.clone();
    let c2 = self.count.clone();
    let val = self.count.get();

    Element::row()
      .spacing(12.0)
      .align_items(Alignment::Center)
      .child(
        Element::rect(36.0, 36.0)
          .fill("#ef4444")
          .rounded(6.0)
          .on_click(move |_| c.update(|n| *n -= 1)),
      )
      .child(Element::styled_text(
        &format!("{}", val),
        TextStyle {
          font_size: 24.0,
          weight: FontWeight::Bold,
          color: Color::from_hex("#1e293b"),
          ..TextStyle::default()
        },
      ))
      .child(
        Element::rect(36.0, 36.0)
          .fill("#22c55e")
          .rounded(6.0)
          .on_click(move |_| c2.update(|n| *n += 1)),
      )
  }
}

#[test]
fn rerenders_after_click_updates_signal_value() {
  let renders = Arc::new(AtomicUsize::new(0));
  let mut runtime = Runtime::new();
  runtime.mount_root::<Counter>(renders.clone());

  assert_eq!(renders.load(Ordering::Relaxed), 1);

  let increment = runtime
    .find_element(|el| el.color() == Some(Color::from_hex("#22c55e")))
    .unwrap()
    .rect;
  assert_eq!(renders.load(Ordering::Relaxed), 1);

  let (x, y) = increment.center();
  runtime.click(x, y, MouseButton::Left);

  assert_eq!(renders.load(Ordering::Relaxed), 2);
  assert!(runtime.needs_redraw());
  assert_counter_value(&runtime, "1");

  runtime
    .find_element(|el| el.color() == Some(Color::from_hex("#22c55e")))
    .unwrap();
  assert_eq!(renders.load(Ordering::Relaxed), 2);
}

fn assert_counter_value(runtime: &Runtime, expected: &str) {
  let value = runtime.root().unwrap().children().iter().nth(1).unwrap().text_content();

  assert_eq!(value, Some(expected));
}
