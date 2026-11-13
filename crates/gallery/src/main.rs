//! The gallery of lurq's design system: a page for each component, built
//! with the components it shows, in the theme the header chooses.
//!
//! `cargo run -p gallery`, or `cargo run -p gallery -- --route /tokens` to
//! open on a page.

mod doc;
mod kit;
mod pages;
mod shell;

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, wgpu_render::WgpuRenderEngine, winit_shell::WinitWindow},
  components::Router,
  core::Signal,
  node::Element,
  router::{RouterHandle, Routes},
};

use crate::pages::PAGES;

#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct GalleryProps {
  route: String,
}

#[cfg(feature = "mcp")]
static MCP: std::sync::OnceLock<lurq::mcp::McpHandle> = std::sync::OnceLock::new();

struct Gallery {
  router: RouterHandle,
}

fn routes(search: Signal<String>) -> Routes {
  Routes::new().layout(
    "/",
    move |ctx| shell::shell(ctx, &search),
    |mut routes| {
      for page in PAGES {
        routes = routes.route(page.path, page.mount);
      }
      routes.fallback(PAGES[0].mount)
    },
  )
}

impl Component for Gallery {
  type Props = GalleryProps;

  fn create(ctx: &mut Ctx) -> Self {
    let search = ctx.signal(String::new());
    let router = ctx.router(routes(search));
    router.replace(&ctx.props::<GalleryProps>().route);
    #[cfg(feature = "mcp")]
    if let Some(handle) = MCP.get() {
      handle.set_navigator(router.navigator());
    }
    Self { router }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    Router::mount(ctx, self.router.clone())
  }
}

/// The page to open on: `--route /tokens`, or the overview.
fn route() -> String {
  let mut args = std::env::args().skip(1);
  while let Some(arg) = args.next() {
    if arg == "--route" {
      return args.next().unwrap_or_else(|| panic!("--route requires a path"));
    }
    if let Some(route) = arg.strip_prefix("--route=") {
      return route.to_owned();
    }
  }
  "/".to_owned()
}

fn main() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_render_engine_factory(|| Box::new(WgpuRenderEngine::new()));
  #[cfg(feature = "mcp")]
  {
    use lurq::mcp::{McpConfig, Scope};
    let handle = tree.enable_mcp(McpConfig::new().app_name("lurq-gallery").scopes([
      Scope::Observe,
      Scope::Interact,
      Scope::Navigate,
    ]));
    eprintln!(
      "MCP listening on http://127.0.0.1:{}/mcp (token {})",
      handle.port(),
      handle.token()
    );
    let _ = MCP.set(handle);
  }
  tree.mount_root::<Gallery>(&mut app, GalleryProps { route: route() });
  #[cfg(feature = "devtools")]
  tree.mount_devtools(&mut app);
  WinitWindow::new(app, tree)
    .with_title("lurq gallery")
    .with_size(1440, 900)
    .with_min_size(720, 480)
    .run();
}
