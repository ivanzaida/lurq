//! Issue 2: an ellipsis cut keeps a trailing space before the `…`.
//!
//! Look at the list of one-line texts: each is cut with an ellipsis in a box of its own width. Where the cut falls
//! right after a word, the text reads `... to …` with a space before the `…`. The last line is the same cut written
//! by hand with no space.
//!
//! cargo run --example ellipsis_space

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, wgpu_render::WgpuRenderEngine, winit_shell::WinitWindow},
  components::{Column, Text, TextOverflow},
  mcp::McpConfig,
  node::{Element, dimension::Dimension},
};

const TITLE: &str = "Publish the payload schema v2 to the registry";

struct Page;

impl Component for Page {
  type Props = ();
  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }
  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let mut column = Column::new().padding(24.0).spacing(6.0);
    // Boxes of growing width: some of them end the cut right after a word.
    for width in (150..=250).step_by(5) {
      column = column.child(
        Text::new(TITLE)
          .nowrap()
          .text_overflow(TextOverflow::Elipsis)
          .width(Dimension::Px(width as f32)),
      );
    }
    column.child(Text::new("expected, no space: Publish the payload schema v2 to…"))
  }
}

fn main() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_render_engine_factory(|| Box::new(WgpuRenderEngine::new()));
  let mcp = tree.enable_mcp(McpConfig::new().app_name("lurq-repro-ellipsis-space"));
  println!("pid {} port {}", std::process::id(), mcp.port());
  tree.mount_root::<Page>(&mut app, ());
  WinitWindow::new(app, tree).with_title("ellipsis space").with_size(520, 640).run();
}
