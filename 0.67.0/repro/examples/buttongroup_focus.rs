//! Issue 4: `ButtonGroup`'s clip cuts the focus ring of its buttons.
//!
//! Look at the first group: its first button has the keyboard focus ring forced on
//! (`.force(Some(Interaction::FocusVisible))`). The ring is drawn outside the button, and the group's clip cuts it at
//! the group's bounds. The second group has no forced state: tab into it through the MCP to see the same on its
//! second button.
//!
//! cargo run --example buttongroup_focus

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, wgpu_render::WgpuRenderEngine, winit_shell::WinitWindow},
  components::Column,
  mcp::McpConfig,
  node::Element,
  ui::{Button, ButtonGroup, Interaction},
};

struct Page;

impl Component for Page {
  type Props = ();
  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let theme = ctx.theme();
    let forced = ButtonGroup::new([Button::new("Request change"), Button::new("").icon("chevron-down")])
      .force(Some(Interaction::FocusVisible))
      .build(&theme);
    let free = ButtonGroup::new([Button::new("Request change"), Button::new("").icon("chevron-down")]).build(&theme);
    Column::new().padding(32.0).spacing(24.0).child(forced).child(free)
  }
}

fn main() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_render_engine_factory(|| Box::new(WgpuRenderEngine::new()));
  let mcp = tree.enable_mcp(McpConfig::new().app_name("lurq-repro-buttongroup-focus"));
  println!("pid {} port {}", std::process::id(), mcp.port());
  tree.mount_root::<Page>(&mut app, ());
  WinitWindow::new(app, tree).with_title("buttongroup focus").with_size(420, 240).run();
}
