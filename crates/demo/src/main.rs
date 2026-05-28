mod animation_demo;
mod dnd_demo;
mod layout_demo;
mod positioning_demo;
mod scroll_demo;
mod sidebar;
mod sizing_demo;
mod style;
mod transform_demo;

use lurq::{
  app::{Runtime, component::Component, ctx::Ctx, wgpu_render::WgpuRenderEngine, winit_shell::WinitWindow},
  components::Row,
  core::Signal,
  layout::{
    Alignment,
    scrollbar::{ScrollBarStyle, ScrollBarVisibility},
  },
  node::{Element, color::Color},
};

use crate::{
  animation_demo::animation_content,
  dnd_demo::DndDemo,
  layout_demo::layout_content,
  positioning_demo::PositioningDemo,
  scroll_demo::scroll_content,
  sidebar::{DemoTab, sidebar},
  sizing_demo::sizing_content,
  style::{ACCENT, BG, PRIMARY, SURFACE_DARK},
};

const SIDEBAR_WIDTH: f32 = 200.0;

struct DemoApp {
  selected_tab: Signal<DemoTab>,
}

impl Component for DemoApp {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      selected_tab: ctx.signal(DemoTab::Layout),
    }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let selected_tab = self.selected_tab.get();
    let content = match selected_tab {
      DemoTab::Layout => layout_content(),
      DemoTab::Sizing => sizing_content(),
      DemoTab::Position => _ctx.mount::<PositioningDemo>(()),
      DemoTab::Dnd => _ctx.mount::<DndDemo>(()),
      DemoTab::Animation => animation_content(),
      DemoTab::Transform => transform_demo::transform_content(),
      DemoTab::Scroll => scroll_content(),
    };

    Row::new()
      .align_items(Alignment::Stretch)
      .child(
        lurq::components::ScrollVertical::new(sidebar(selected_tab, self.selected_tab.clone()))
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
        lurq::components::ScrollVertical::new(content)
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
      .fill(BG)
  }
}

fn main() {
  let mut runtime = Runtime::new();
  runtime.set_render_engine(Box::new(WgpuRenderEngine::new()));
  animation_demo::register_keyframes(&mut runtime);
  runtime.mount_root::<DemoApp>(());
  WinitWindow::new(runtime)
    .with_title("lurq demo")
    .on_tick(|_rt: &mut Runtime| {})
    .run();
}
