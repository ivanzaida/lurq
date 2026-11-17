//! The gallery of lurq's design system: a page for each component, built
//! with the components it shows, in the theme the header chooses.
//!
//! `cargo run -p gallery`, or `cargo run -p gallery -- --route /tokens` to
//! open on a page; `--size 800x600` opens the window at a size, to see a
//! breakpoint; `--device phone` (or `tablet`) runs it as on a phone, the mouse its fingers (`App::emulate`).
//! Built with `--features devtools`, `--devtools` opens DevTools beside it. On Windows `--renderer dx12` draws it
//! with DirectX 12 in place of wgpu, and `--backend vulkan` (or `dx12`, `gl`, `metal`) draws it through that
//! backend of wgpu's. Built with `--features perf_profile`, `--profile-log` writes what each frame
//! cost to `target/perf_profile.log` (or `--profile-log=<file>`): the Glyph Atlas page is its probe.
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
#[cfg(not(target_arch = "wasm32"))]
mod scene;
mod shell;
#[cfg(test)]
mod speed;
mod syntect_code;
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

/// What wgpu draws the gallery through: `--backend vulkan`, `dx12`, `gl` or `metal`, or what lurq takes by
/// itself (Direct3D 12 on Windows). For looking at a page as another backend draws it. lurq's own Direct3D 12
/// renderer is not one of wgpu's: see `renderer`.
#[cfg(not(target_arch = "wasm32"))]
fn backends() -> Option<lurq::app::wgpu_render::WgpuBackends> {
  use lurq::app::wgpu_render::WgpuBackends;
  let named = option("--backend")?;
  Some(match named.to_ascii_lowercase().as_str() {
    "vulkan" => WgpuBackends::VULKAN,
    "dx12" => WgpuBackends::DX12,
    "gl" => WgpuBackends::GL,
    "metal" => WgpuBackends::METAL,
    _ => panic!("--backend is vulkan, dx12, gl or metal"),
  })
}

/// The phone or tablet to run as: `--device phone` or `--device tablet`; `LURQ_DEVICE` does the same.
#[cfg(not(target_arch = "wasm32"))]
fn device() -> Option<lurq::app::Device> {
  option("--device")
    .map(|name| lurq::app::Device::named(&name).unwrap_or_else(|| panic!("--device is phone or tablet, not {name:?}")))
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

/// What draws the window: `--renderer wgpu`, the default, or `--renderer dx12` on Windows.
#[cfg(not(target_arch = "wasm32"))]
fn renderer() -> fn() -> Box<dyn RenderEngine> {
  match option("--renderer").as_deref().map(str::to_ascii_lowercase).as_deref() {
    // The GPU Viewport page's picture is drawn by an extension of the renderer, as an app's own would be.
    None | Some("wgpu") => || {
      let engine = match backends() {
        Some(backends) => WgpuRenderEngine::new().with_backends(backends),
        None => WgpuRenderEngine::new(),
      };
      Box::new(engine.with_frame_extension(scene::Scene::default()))
    },
    #[cfg(windows)]
    Some("dx12" | "d3d12") => || Box::new(lurq::app::dx12_render::Dx12RenderEngine::new()),
    Some(other) => panic!("--renderer is wgpu or, on Windows, dx12, not {other:?}"),
  }
}

/// Where `--profile-log` writes what each frame cost: the file it names, or `target/perf_profile.log` of the
/// repository. Nothing without the option.
#[cfg(all(feature = "perf_profile", not(target_arch = "wasm32")))]
fn profile_log() -> Option<std::path::PathBuf> {
  std::env::args().skip(1).find_map(|arg| {
    if arg == "--profile-log" {
      Some(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/perf_profile.log"))
    } else {
      arg.strip_prefix("--profile-log=").map(std::path::PathBuf::from)
    }
  })
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
  caches.unwrap_or_else(std::env::temp_dir).join("lurq-gallery")
}

fn main() {
  // What a page loads from an address (the Audio Player's sound) is kept on the disk, and is there the next
  // time the gallery is opened.
  #[cfg(not(target_arch = "wasm32"))]
  {
    let mut app = App::new().enable_media_disk_cache(cache_folder().join("media"));
    // What the Persistent Storage page saves is in a file of the gallery's own, there the next time it is opened.
    if let Err(error) = app.set_persistent_storage_path(cache_folder().join("storage.redb")) {
      eprintln!("the gallery's store did not open: {error}");
    }
    run(app, route(), renderer());
  }
  // A page waits for its GPU device and its fonts first; see `web`.
  #[cfg(target_arch = "wasm32")]
  web::start();
}

/// Opens the gallery's window on `route`, drawn by the engines `engine` makes.
fn run(mut app: App, route: String, engine: impl Fn() -> Box<dyn RenderEngine> + 'static) {
  // The languages lurq has no tokenizer of its own for, by syntect's grammars.
  syntect_code::register(&app);
  let mut tree = Tree::new();
  tree.set_render_engine_factory(engine);
  pages::engine::prepare(&mut tree);
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
  #[cfg(not(target_arch = "wasm32"))]
  if let Some(device) = device() {
    app.emulate(device);
  }
  tree.mount_root::<Gallery>(&mut app, GalleryProps { route });
  #[cfg(feature = "devtools")]
  tree.mount_devtools(&mut app);
  let (width, height) = size();
  let window = WinitWindow::new(app, tree)
    .with_title("lurq gallery")
    .with_icon(icon::icon())
    .with_size(width, height)
    .with_min_size(320, 480)
    .with_decorations(false);
  #[cfg(all(feature = "perf_profile", not(target_arch = "wasm32")))]
  let window = match profile_log() {
    Some(path) => {
      use std::io::Write as _;
      if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create the folder of the profile log");
      }
      let mut log = std::io::BufWriter::new(std::fs::File::create(&path).expect("create the profile log"));
      eprintln!("writing what each frame cost to {}", path.display());
      window.on_paint(move |tree, delta, report| {
        let _ = writeln!(
          log,
          "Profile for frame delta={:.2}ms rendered={} layout_recalc={} {}",
          delta.as_secs_f64() * 1000.0,
          report.rendered,
          report.layout_recalculated,
          tree.profile()
        );
        let _ = log.flush();
      })
    }
    None => window,
  };
  window.run();
}
