//! The gallery of lurq's design system: a page for each component, built
//! with the components it shows, in the theme the header chooses.
//!
//! `cargo run -p gallery`, or `cargo run -p gallery -- --route /tokens` to
//! open on a page; `--size 800x600` opens the window at a size, to see a
//! breakpoint. Built with `--features devtools`, `--devtools` opens DevTools beside it. `--backend vulkan` draws
//! it through another of wgpu's backends, and `--backend native-dx12`, built with `--features dx12`, through
//! lurq's own Direct3D 12 renderer.
//!
//! Its pages open in tabs, as a browser's do: the strip is in the title bar, and each tab keeps its own page, its
//! history and its scroll (see `tabs`).

#[cfg(windows)]
mod card_demo;
mod doc;
mod icon;
mod kit;
#[cfg(test)]
mod narrow;
mod pages;
mod playground;
mod shell;
#[cfg(test)]
mod speed;
mod tabs;
#[cfg(target_arch = "wasm32")]
mod web;

use lurq::{
  app::{
    App, Tree, component::Component, ctx::Ctx, render_engine::RenderEngine, theme::PaletteColor,
    wgpu_render::WgpuRenderEngine, winit_shell::WinitWindow,
  },
  components::ChromeBorderPolicy,
  node::Element,
  ui::WindowChrome,
};

use crate::{shell::Shell, tabs::OpenTabs};

#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct GalleryProps {
  route: String,
}

#[cfg(feature = "mcp")]
static MCP: std::sync::OnceLock<lurq::mcp::McpHandle> = std::sync::OnceLock::new();

struct Gallery {
  shell: Shell,
  open: OpenTabs,
}

impl Component for Gallery {
  type Props = GalleryProps;

  fn create(ctx: &mut Ctx) -> Self {
    let shell = Shell::new(ctx);
    let open = OpenTabs::new(&ctx.props::<GalleryProps>().route);
    // In a browser the address names the page: it is opened on, follows the pages and takes the back button. Its
    // own tabs are the browser's, so the gallery has the one there, and no strip.
    open.router().follow_address();
    #[cfg(feature = "devtools")]
    if std::env::args().any(|arg| arg == "--devtools") {
      ctx.window().open_devtools();
    }
    Self { shell, open }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    // What is outside the pages (the side bar, the search, a tool that drives the window) goes where the tab that
    // is shown is: read here, so that this is built again when another tab is shown.
    let router = self.open.router();
    ctx.provide(router.navigator());
    #[cfg(feature = "mcp")]
    if let Some(handle) = MCP.get() {
      handle.set_navigator(router.navigator());
    }
    let content = shell::shell(ctx, &self.shell, &self.open);
    // The window has no frame of the system's: the gallery draws its title bar from the theme, with its tabs in it.
    let chrome = WindowChrome::new()
      .title("lurq gallery")
      .border(ChromeBorderPolicy::Visible {
        size: 1.0,
        color: PaletteColor::Border.into(),
      })
      .content(content);
    #[cfg(not(target_arch = "wasm32"))]
    let chrome = chrome.tabs(self.open.strip(ctx));
    chrome.mount(ctx)
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
#[cfg(not(target_arch = "wasm32"))]
fn route() -> String {
  option("--route").unwrap_or_else(|| "/".to_owned())
}

/// What the gallery draws through: `--backend vulkan`, `dx12`, `gl` or `metal`, or what lurq takes by itself
/// (Direct3D 12 on Windows). For looking at a page as another backend draws it. `native-dx12` is not one of
/// wgpu's: see `engine`.
#[cfg(not(target_arch = "wasm32"))]
fn backends() -> Option<lurq::app::wgpu_render::WgpuBackends> {
  use lurq::app::wgpu_render::WgpuBackends;
  let named = option("--backend")?;
  Some(match named.to_ascii_lowercase().as_str() {
    "vulkan" => WgpuBackends::VULKAN,
    "dx12" => WgpuBackends::DX12,
    "gl" => WgpuBackends::GL,
    "metal" => WgpuBackends::METAL,
    _ => panic!("--backend is vulkan, dx12, gl, metal or native-dx12"),
  })
}

/// The renderer of a window of the gallery: wgpu's, through the backend asked for, or with
/// `--backend native-dx12` lurq's own Direct3D 12 renderer, where the gallery was built with it
/// (`--features dx12`).
#[cfg(not(target_arch = "wasm32"))]
fn engine() -> Box<dyn RenderEngine> {
  if option("--backend").is_some_and(|named| named.eq_ignore_ascii_case("native-dx12")) {
    #[cfg(all(feature = "dx12", windows))]
    return Box::new(lurq::app::dx12_render::Dx12RenderEngine::new());
    #[cfg(not(all(feature = "dx12", windows)))]
    panic!("--backend native-dx12 is of a gallery built on Windows with --features dx12");
  }
  match backends() {
    Some(backends) => Box::new(WgpuRenderEngine::new().with_backends(backends)),
    None => Box::new(WgpuRenderEngine::new()),
  }
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

/// The folder the gallery keeps what it loaded from addresses in, for its next run: its own, among the caches of the
/// user. The app chooses it; lurq picks none.
#[cfg(not(target_arch = "wasm32"))]
fn cache_folder() -> std::path::PathBuf {
  use std::{env::var_os, path::PathBuf};
  let home = |under: &str| var_os("HOME").map(|home| PathBuf::from(home).join(under));
  let caches = if cfg!(windows) {
    var_os("LOCALAPPDATA").map(PathBuf::from)
  } else if cfg!(any(target_os = "macos", target_os = "ios")) {
    home("Library/Caches")
  } else {
    var_os("XDG_CACHE_HOME").map(PathBuf::from).or_else(|| home(".cache"))
  };
  caches
    .unwrap_or_else(std::env::temp_dir)
    .join("lurq-gallery")
    .join("media")
}

fn main() {
  // What a page loads from an address (the Audio Player's sound) is kept on the disk, and is there the next
  // time the gallery is opened.
  #[cfg(not(target_arch = "wasm32"))]
  run(App::new().enable_media_disk_cache(cache_folder()), route(), engine);
  // A page waits for its GPU device and its fonts first; see `web`.
  #[cfg(target_arch = "wasm32")]
  web::start();
}

/// Opens the gallery's window on `route`, drawn by the engines `engine` makes.
fn run(mut app: App, route: String, engine: impl Fn() -> Box<dyn RenderEngine> + 'static) {
  let mut tree = Tree::new();
  tree.set_render_engine_factory(engine);
  #[cfg(feature = "mcp")]
  {
    use lurq::mcp::{McpConfig, Scope};
    let config = McpConfig::new()
      .app_name("lurq-gallery")
      .scopes([Scope::Observe, Scope::Interact, Scope::Navigate]);
    // DevTools is a window of the gallery to what drives it: it is read and captured as the pages are.
    #[cfg(feature = "devtools")]
    let config = config.include_devtools(true);
    let handle = tree.enable_mcp(config);
    eprintln!(
      "MCP listening on http://127.0.0.1:{}/mcp (token {})",
      handle.port(),
      handle.token()
    );
    let _ = MCP.set(handle);
  }
  tree.mount_root::<Gallery>(&mut app, GalleryProps { route });
  #[cfg(feature = "devtools")]
  tree.mount_devtools(&mut app);
  let (width, height) = size();
  WinitWindow::new(app, tree)
    .with_title("lurq gallery")
    .with_icon(icon::icon())
    .with_size(width, height)
    .with_min_size(320, 480)
    .with_decorations(false)
    .run();
}
