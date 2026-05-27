use lurq::{
  app::{Runtime, component::Component, ctx::Ctx, events::MouseButton},
  core::Signal,
  layout::{
    Alignment,
    text_style::{FontWeight, TextStyle},
  },
  node::{Element, color::Color},
};

struct Counter {
  count: Signal<i32>,
}

impl Component for Counter {
  type Props = ();

  fn create(ctx: &mut Ctx, _: ()) -> Self {
    Self { count: ctx.signal(0) }
  }

  fn render(&self, _ctx: &mut Ctx) -> Element {
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
fn updates_displayed_value_after_increment_and_decrement_clicks() {
  let mut runtime = Runtime::new();
  runtime.mount_root::<Counter>(());
  assert_counter_value(&runtime, "0");

  let increment = runtime
    .find_element(|el| el.color() == Some(Color::from_hex("#22c55e")))
    .unwrap()
    .bounds();
  let (x, y) = increment.center();
  runtime.click(x, y, MouseButton::Left);
  assert_counter_value(&runtime, "1");

  let decrement = runtime
    .find_element(|el| el.color() == Some(Color::from_hex("#ef4444")))
    .unwrap()
    .bounds();
  let (x, y) = decrement.center();
  runtime.click(x, y, MouseButton::Left);
  assert_counter_value(&runtime, "0");
}

fn assert_counter_value(runtime: &Runtime, expected: &str) {
  let value = runtime.root().unwrap().children().iter().nth(1).unwrap().text_content();

  assert_eq!(value, Some(expected));
}
