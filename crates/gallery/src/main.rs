//! The gallery of lurq's design system: a page for each component, built
//! with the components it shows, in the theme the header chooses.
//!
//! `cargo run -p gallery`, or `cargo run -p gallery -- --route /tokens` to
//! open on a page; `--size 800x600` opens the window at a size, to see a
//! breakpoint.

mod doc;
mod kit;
mod pages;
mod shell;

use lurq::{
  app::{
    App, Tree, component::Component, ctx::Ctx, theme::PaletteColor, wgpu_render::WgpuRenderEngine,
    winit_shell::WinitWindow,
  },
  components::{ChromeBorderPolicy, Router, WindowChrome},
  node::Element,
  router::{RouterHandle, Routes},
};

use crate::{pages::PAGES, shell::Shell};

/// lurq's icon font: the gallery draws its own icons with it.
const ICONS: &[u8] = include_bytes!("../../lurq/assets/lurq-icons.ttf");

#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct GalleryProps {
  route: String,
}

#[cfg(feature = "mcp")]
static MCP: std::sync::OnceLock<lurq::mcp::McpHandle> = std::sync::OnceLock::new();

struct Gallery {
  router: RouterHandle,
}

fn routes(shell: Shell) -> Routes {
  Routes::new().layout(
    "/",
    move |ctx| shell::shell(ctx, &shell),
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
    let shell = Shell::new(ctx);
    let router = ctx.router(routes(shell));
    router.replace(&ctx.props::<GalleryProps>().route);
    #[cfg(feature = "mcp")]
    if let Some(handle) = MCP.get() {
      handle.set_navigator(router.navigator());
    }
    Self { router }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    // The window has no frame of the system's: the gallery draws its title bar from the theme.
    let pages = Router::mount(ctx, self.router.clone());
    WindowChrome::new()
      .border(ChromeBorderPolicy::Visible {
        size: 1.0,
        color: PaletteColor::Border.into(),
      })
      .frame_background(PaletteColor::Background)
      .title_bar(shell::title_bar())
      .content(pages)
      .mount(ctx)
  }
}

/// The value of an option of the command line: `--name value` or `--name=value`.
fn option(name: &str) -> Option<String> {
  let mut args = std::env::args().skip(1);
  while let Some(arg) = args.next() {
    if arg == name {
      return Some(args.next().unwrap_or_else(|| panic!("{name} requires a value")));
    }
    if let Some(value) = arg.strip_prefix(name).and_then(|rest| rest.strip_prefix('=')) {
      return Some(value.to_owned());
    }
  }
  None
}

/// The page to open on: `--route /tokens`, or the overview.
fn route() -> String {
  option("--route").unwrap_or_else(|| "/".to_owned())
}

/// The size to open the window at: `--size 800x600`, or a desktop's.
fn size() -> (u32, u32) {
  let Some(size) = option("--size") else {
    return (1440, 900);
  };
  size
    .split_once('x')
    .and_then(|(width, height)| Some((width.parse().ok()?, height.parse().ok()?)))
    .unwrap_or_else(|| panic!("--size is a width and a height, as in 800x600"))
}

fn main() {
  let mut app = App::new();
  app.load_hinted_font(ICONS.to_vec());
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
  let (width, height) = size();
  WinitWindow::new(app, tree)
    .with_title("lurq gallery")
    .with_size(width, height)
    .with_min_size(720, 480)
    .with_decorations(false)
    .run();
}
