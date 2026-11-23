//! Issue 3: a shadow on the Tooltip recipe's panel is cut off at the overlay's bounds.
//!
//! Look at the tooltip above the "Hover me" box (forced open, light appearance): its shadow, offset 6, blur 16, black at
//! 40 %, ends in a straight edge about 6 px under the panel instead of fading out.
//!
//! cargo run --example tooltip_shadow

use lurq::{
  app::{
    App, Tree, component::Component, ctx::Ctx, theme::PaletteColor, wgpu_render::WgpuRenderEngine,
    window::Appearance, winit_shell::WinitWindow,
  },
  components::{Column, Row, Text},
  layout::{Alignment, layout_kind::Justify},
  mcp::McpConfig,
  node::{Element, box_shadow::BoxShadow, color::Color, dimension::Dimension},
  ui::{Tooltip, TooltipPart},
};

struct Page;

impl Component for Page {
  type Props = ();
  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let trigger = Row::new().padding(12.0).background(PaletteColor::Muted).child(Text::new("Hover me"));
    let tooltip = Tooltip::new("Publish the payload schema v2 to the registry")
      .force_open(Some(true))
      .style(TooltipPart::Panel, |panel| {
        panel.shadow(BoxShadow::new(0.0, 6.0, 16.0, Color::from_hex("#00000066")))
      })
      .mount(ctx, trigger);
    Column::new()
      .width(Dimension::Pct(100.0))
      .height(Dimension::Pct(100.0))
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .child(tooltip)
  }
}

fn main() {
  let mut app = App::new();
  app.theme().set_preferred_appearance(Some(Appearance::Light));
  let mut tree = Tree::new();
  tree.set_render_engine_factory(|| Box::new(WgpuRenderEngine::new()));
  let mcp = tree.enable_mcp(McpConfig::new().app_name("lurq-repro-tooltip-shadow"));
  println!("pid {} port {}", std::process::id(), mcp.port());
  tree.mount_root::<Page>(&mut app, ());
  WinitWindow::new(app, tree).with_title("tooltip shadow").with_size(520, 360).run();
}
