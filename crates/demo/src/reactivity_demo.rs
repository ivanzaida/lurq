use lurq::{
  app::{component::Component, ctx::Ctx},
  core::Signal,
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{CursorIcon, Element, color::Color, dimension::Dimension},
};

use crate::style::{BG, BORDER, PRIMARY, SURFACE, TEXT, TEXT_MUTED, text};

const FILL_WIDTH: Dimension = Dimension::Pct(100.0);
const CONTENT_PAD: f32 = 32.0;
const CARD_RADIUS: f32 = 8.0;

pub(crate) struct ReactivityDemo {
  count: Signal<i32>,
  memo_count: Signal<i32>,
  render_count: Signal<i32>,
  batch_a: Signal<i32>,
  batch_b: Signal<i32>,
  batch_c: Signal<i32>,
  unbatched_renders: Signal<i32>,
  batched_renders: Signal<i32>,
}

impl Component for ReactivityDemo {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      count: ctx.signal(0),
      memo_count: ctx.signal(7),
      render_count: ctx.signal(0),
      batch_a: ctx.signal(1),
      batch_b: ctx.signal(2),
      batch_c: ctx.signal(3),
      unbatched_renders: ctx.signal(0),
      batched_renders: ctx.signal(0),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let count = self.count.get();
    let memo_count = self.memo_count.get();
    let doubled = memo_count * 2;
    let is_even = memo_count % 2 == 0;
    let render_count = self.render_count.get();
    let a = self.batch_a.get();
    let b = self.batch_b.get();
    let c = self.batch_c.get();
    let unbatched = self.unbatched_renders.get();
    let batched = self.batched_renders.get();

    self.render_count.set(render_count + 1);

    let count_sig = self.count.clone();
    let count_sig2 = self.count.clone();
    let count_sig3 = self.count.clone();
    let count_sig4 = self.count.clone();

    let ba = self.batch_a.clone();
    let bb = self.batch_b.clone();
    let bc = self.batch_c.clone();
    let ur = self.unbatched_renders.clone();

    let ba2 = self.batch_a.clone();
    let bb2 = self.batch_b.clone();
    let bc2 = self.batch_c.clone();
    let br = self.batched_renders.clone();

    lurq::components::Column::new()
      .spacing(24.0)
      .child(text("Reactivity", 28.0, FontWeight::Bold, TEXT).width(FILL_WIDTH))
      .child(section_title("Signals"))
      .child(signals_card(count, count_sig, count_sig2, count_sig3, count_sig4))
      .child(section_title("Memo (Derived Values)"))
      .child(memo_card(memo_count, doubled, is_even, render_count))
      .child(section_title("Batch Updates"))
      .child(batch_card(
        a, b, c, unbatched, batched, ba, bb, bc, ur, ba2, bb2, bc2, br,
      ))
      .padding(CONTENT_PAD)
      .width(FILL_WIDTH)
      .fill(BG)
  }
}

fn section_title(label: &str) -> Element {
  text(label, 18.0, FontWeight::Bold, TEXT).width(FILL_WIDTH).into()
}

fn card_frame() -> lurq::components::Column {
  lurq::components::Column::new()
    .width(FILL_WIDTH)
    .fill(SURFACE)
    .rounded(CARD_RADIUS)
    .border_inside(1.0, Color::from_hex(BORDER))
}

fn btn(
  label: &str,
  color: &str,
  width: f32,
  handler: impl Fn(&lurq::app::events::MouseEvent) + Send + Sync + 'static,
) -> Element {
  lurq::components::Row::new()
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(text(
      label,
      if width > 60.0 { 12.0 } else { 20.0 },
      FontWeight::Bold,
      "#ffffff",
    ))
    .size(width, if width > 60.0 { 32.0 } else { 48.0 })
    .fill(color)
    .rounded(if width > 60.0 { 6.0 } else { 8.0 })
    .cursor(CursorIcon::Pointer)
    .on_click(handler)
    .into()
}

fn signals_card(count: i32, minus: Signal<i32>, plus: Signal<i32>, reset: Signal<i32>, plus10: Signal<i32>) -> Element {
  card_frame()
    .spacing(16.0)
    .align_items(Alignment::Center)
    .child(text("Counter (Signal<i32>)", 13.0, FontWeight::Normal, TEXT_MUTED))
    .child(
      lurq::components::Row::new()
        .spacing(16.0)
        .justify(Justify::Center)
        .align_items(Alignment::Center)
        .child(btn("-", "#EF4444", 48.0, move |_| minus.set(minus.get() - 1)))
        .child(
          lurq::components::Row::new()
            .align_items(Alignment::Center)
            .justify(Justify::Center)
            .child(text(&count.to_string(), 24.0, FontWeight::Bold, TEXT))
            .size(80.0, 48.0)
            .fill("#0F172A")
            .rounded(8.0)
            .border_inside(2.0, Color::from_hex(PRIMARY)),
        )
        .child(btn("+", "#22C55E", 48.0, move |_| plus.set(plus.get() + 1)))
        .width(FILL_WIDTH),
    )
    .child(
      lurq::components::Row::new()
        .spacing(12.0)
        .justify(Justify::Center)
        .width(FILL_WIDTH)
        .child(
          text("[Reset]", 12.0, FontWeight::Normal, PRIMARY)
            .cursor(CursorIcon::Pointer)
            .on_click(move |_| reset.set(0)),
        )
        .child(
          text("[+10]", 12.0, FontWeight::Normal, PRIMARY)
            .cursor(CursorIcon::Pointer)
            .on_click(move |_| plus10.set(plus10.get() + 10)),
        ),
    )
    .padding(24.0)
    .into()
}

fn memo_card(count: i32, doubled: i32, is_even: bool, render_count: i32) -> Element {
  card_frame()
    .spacing(6.0)
    .child(text(&format!("count: {count}"), 15.0, FontWeight::Normal, TEXT))
    .child(text(
      &format!("doubled (memo):  {doubled}"),
      15.0,
      FontWeight::Normal,
      TEXT,
    ))
    .child(text(
      &format!("is_even (memo):  {is_even}"),
      15.0,
      FontWeight::Normal,
      TEXT,
    ))
    .child(text(
      &format!("label (memo):    \"{count} items\""),
      15.0,
      FontWeight::Normal,
      TEXT,
    ))
    .child(text(
      &format!("Render count: {render_count}  (memos skip when equal)"),
      12.0,
      FontWeight::Normal,
      TEXT_MUTED,
    ))
    .padding(24.0)
    .into()
}

fn batch_card(
  a: i32,
  b: i32,
  c: i32,
  unbatched: i32,
  batched: i32,
  ba: Signal<i32>,
  bb: Signal<i32>,
  bc: Signal<i32>,
  ur: Signal<i32>,
  ba2: Signal<i32>,
  bb2: Signal<i32>,
  bc2: Signal<i32>,
  br: Signal<i32>,
) -> Element {
  card_frame()
    .spacing(8.0)
    .child(text(
      &format!("a: {a}    b: {b}    c: {c}"),
      15.0,
      FontWeight::Normal,
      TEXT,
    ))
    .child(
      lurq::components::Row::new()
        .spacing(16.0)
        .align_items(Alignment::Center)
        .child(btn("Update All (no batch)", PRIMARY, 200.0, move |_| {
          ba.set(ba.get() + 1);
          bb.set(bb.get() + 1);
          bc.set(bc.get() + 1);
          ur.set(ur.get() + 3);
        }))
        .child(text(
          &format!("renders: {unbatched}"),
          12.0,
          FontWeight::Normal,
          TEXT_MUTED,
        ))
        .child(btn("Update All (batched)", PRIMARY, 200.0, move |_| {
          ba2.set(ba2.get() + 1);
          bb2.set(bb2.get() + 1);
          bc2.set(bc2.get() + 1);
          br.set(br.get() + 1);
        }))
        .child(text(
          &format!("renders: {batched}"),
          12.0,
          FontWeight::Normal,
          TEXT_MUTED,
        ))
        .width(FILL_WIDTH),
    )
    .padding(24.0)
    .into()
}
