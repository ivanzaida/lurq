mod layout_demo;
mod sidebar;
mod style;

use lurq::{
  app::{component::Component, ctx::Ctx, wgpu_render::WgpuRenderEngine, winit_shell::WinitWindow, Runtime},
  layout::{
    layout_kind::ScrollState,
    scrollbar::{ScrollBarStyle, ScrollBarVisibility},
    Alignment,
  },
  node::{color::Color, Element},
};

use crate::{
  layout_demo::layout_content,
  sidebar::sidebar,
  style::{ACCENT, BG, PRIMARY, SURFACE_DARK},
};

const SIDEBAR_WIDTH: f32 = 200.0;

struct DemoApp {
  sidebar_scroll: ScrollState,
  content_scroll: ScrollState,
}

impl Component for DemoApp {
  type Props = ();

  fn create(_ctx: &mut Ctx, _: ()) -> Self {
    Self {
      sidebar_scroll: ScrollState::new(),
      content_scroll: ScrollState::new(),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> Element {
    let nref = _ctx.node_ref();
    Element::row()
      .align_items(Alignment::Start)
      .ref_node(nref)
      .child(
        Element::scroll_vertical(sidebar())
          .with_scroll_state(self.sidebar_scroll.clone())
          .scrollbar(ScrollBarStyle {
            visible: ScrollBarVisibility::Auto,
            width: 6.0,
            thumb_color: Color::from_hex(PRIMARY),
            thumb_hover_color: Color::from_hex(ACCENT),
            thumb_radius: 4.0,
            ..ScrollBarStyle::default()
          })
          .width(SIDEBAR_WIDTH)
          .fill(SURFACE_DARK),
      )
      .child(
        Element::scroll_vertical(layout_content())
          .with_scroll_state(self.content_scroll.clone())
          .scrollbar(ScrollBarStyle {
            visible: ScrollBarVisibility::Auto,
            width: 7.0,
            thumb_color: Color::from_hex(PRIMARY),
            thumb_hover_color: Color::from_hex(ACCENT),
            thumb_radius: 4.0,
            ..ScrollBarStyle::default()
          })
          .fill(BG)
          .flex(1.0),
      )
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
