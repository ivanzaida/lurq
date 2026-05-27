mod layout_demo;
mod sidebar;
mod style;

use lurq::{
  app::{Runtime, component::Component, ctx::Ctx, wgpu_render::WgpuRenderEngine, winit_shell::WinitWindow},
  components::{Rect, Row},
  core::Signal,
  layout::{
    Alignment,
    scrollbar::{ScrollBarStyle, ScrollBarVisibility},
  },
  node::{Element, color::Color},
};

use crate::{
  layout_demo::layout_content,
  sidebar::sidebar,
  style::{ACCENT, BG, PRIMARY, SURFACE_DARK},
};

const SIDEBAR_WIDTH: f32 = 200.0;

struct Child;

impl Component for Child {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    println!("child rerender!");
    Rect::new(0.0, 0.0)
  }
}

struct DemoApp {
  signal: Signal<u32>,
}

impl Component for DemoApp {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self { signal: ctx.signal(0) }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    Row::new()
      .align_items(Alignment::Stretch)
      .on_click({
        let c = self.signal.clone();
        move |_| {
          c.update(|c| *c += 1);
        }
      })
      .child(
        lurq::components::ScrollVertical::new(sidebar())
          .scrollbar(ScrollBarStyle {
            visible: ScrollBarVisibility::Auto,
            width: 6.0,
            thumb_color: Color::from_hex(PRIMARY),
            thumb_radius: 4.0,
            ..ScrollBarStyle::default()
          })
          .scrollbar_hovered(|style| style.with_thumb_color(Color::from_hex(ACCENT)))
          .width(SIDEBAR_WIDTH)
          .fill(SURFACE_DARK),
      )
      .child(
        lurq::components::ScrollVertical::new(layout_content())
          .scrollbar(ScrollBarStyle {
            visible: ScrollBarVisibility::Auto,
            width: 7.0,
            thumb_color: Color::from_hex(PRIMARY),
            thumb_radius: 4.0,
            ..ScrollBarStyle::default()
          })
          .scrollbar_hovered(|style| style.with_thumb_color(Color::from_hex(ACCENT)))
          .fill(BG)
          .flex(1.0),
      )
      .child(_ctx.mount::<Child>(()))
      .fill(BG)
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
