use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{Runtime, component::Component, ctx::Ctx, events::MouseButton},
  core::Signal,
  layout::{
    Alignment,
    text_style::{FontWeight, TextStyle},
  },
  node::{Element, color::Color},
};

#[derive(Clone)]
struct Shared<T>(Arc<T>);

impl<T> PartialEq for Shared<T> {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.0, &other.0)
  }
}

struct Counter {
  count: Signal<i32>,
  renders: Arc<AtomicUsize>,
}

impl Component for Counter {
  type Props = Shared<AtomicUsize>;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      count: ctx.signal(0),
      renders: ctx.props::<Self::Props>().0.clone(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    self.renders.fetch_add(1, Ordering::Relaxed);

    let c = self.count.clone();
    let c2 = self.count.clone();
    let val = self.count.get();

    lurq::components::Row::new()
      .spacing(12.0)
      .align_items(Alignment::Center)
      .child(
        lurq::components::Rect::new(36.0, 36.0)
          .fill("#ef4444")
          .rounded(6.0)
          .on_click(move |_| c.update(|n| *n -= 1)),
      )
      .child(lurq::components::Text::styled(
        &format!("{}", val),
        TextStyle {
          font_size: 24.0,
          weight: FontWeight::Bold,
          color: Color::from_hex("#1e293b"),
          ..TextStyle::default()
        },
      ))
      .child(
        lurq::components::Rect::new(36.0, 36.0)
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
  runtime.mount_root::<Counter>(Shared(renders.clone()));

  assert_eq!(renders.load(Ordering::Relaxed), 1);

  let increment = runtime
    .find_element(|el| el.color() == Some(Color::from_hex("#22c55e")))
    .unwrap()
    .bounds();
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
