use lurq::{
  app::{Runtime, component::Component, ctx::Ctx, wgpu_render::WgpuRenderEngine, winit_shell::WinitWindow},
  core::Signal,
  layout::{
    Alignment,
    scrollbar::ScrollBarStyle,
    text_style::{FontStyle, FontWeight, TextStyle},
  },
  node::{Element, color::Color},
};

// --- Counter component ---

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

// --- ScrollList component ---

struct ScrollList {
  scroll: lurq::layout::layout_kind::ScrollState,
}

impl Component for ScrollList {
  type Props = usize;

  fn create(_ctx: &mut Ctx, _props: usize) -> Self {
    Self {
      scroll: lurq::layout::layout_kind::ScrollState::new(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> Element {
    Element::scroll_vertical(
      Element::column()
        .spacing(4.0)
        .align_items(Alignment::Center)
        .with_children((0..20).map(|i| {
          Element::rect(270.0, 32.0)
            .fill(if i % 2 == 0 { "#93c5fd" } else { "#fca5a5" })
            .rounded(4.0)
            .on_click(move |_| println!("Item {} clicked", i))
        }))
        .pad_xy(0.0, 8.0),
    )
    .with_scroll_state(self.scroll.clone())
    .scrollbar(ScrollBarStyle {
      width: 6.0,
      thumb_color: Color::from_hex("#c0c0c0"),
      thumb_hover_color: Color::from_hex("#808080"),
      thumb_radius: 3.0,
      ..ScrollBarStyle::default()
    })
    .size(300.0, 180.0)
    .rounded(8.0)
    .fill("#ffffff")
    .border_inside(1.0, Color::from_hex("#94a3b8"))
  }
}

// --- Root app component ---

struct DemoApp;

impl Component for DemoApp {
  type Props = ();

  fn create(_ctx: &mut Ctx, _: ()) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> Element {
    Element::column()
      .spacing(16.0)
      .align_items(Alignment::Center)
      .child(Element::styled_text(
        "lurq demo",
        TextStyle {
          font_size: 32.0,
          color: Color::from_hex("#1e293b"),
          ..TextStyle::default()
        },
      ))
      .child(ctx.mount::<Counter>(()))
      .child(ctx.mount::<ScrollList>(20))
      .child(
        Element::rect(300.0, 40.0)
          .fill("#3b82f6")
          .rounded(20.0)
          .border_inside(2.0, Color::from_hex("#1d4ed8"))
          .on_click(|_| println!("Blue button clicked!")),
      )
      .child(
        Element::row()
          .spacing(8.0)
          .align_items(Alignment::Center)
          .child(Element::styled_text(
            "Bold",
            TextStyle {
              weight: FontWeight::Bold,
              color: Color::from_hex("#dc2626"),
              ..TextStyle::default()
            },
          ))
          .child(Element::text("and"))
          .child(Element::styled_text(
            "italic",
            TextStyle {
              style: FontStyle::Italic,
              color: Color::from_hex("#2563eb"),
              ..TextStyle::default()
            },
          ))
          .child(Element::text("text.")),
      )
      .pad(32.0)
  }
}

fn main() {
  let mut runtime = Runtime::new();
  runtime.set_render_engine(Box::new(WgpuRenderEngine::new()));
  runtime.mount_root::<DemoApp>(());
  WinitWindow::new(runtime)
    .with_title("lurq demo")
    .on_tick(|_rt: &mut Runtime| {})
    .run();
}
