---
title: Getting Started
description: Feature flags, the gallery, and the smallest useful lurq app.
---

# Getting Started

This guide targets **lurq 0.66.0**. Add the crate; its default features are what a desktop app needs, the window and the renderer used by the example below among them:

```toml
[dependencies]
lurq = "0.66.0"
```

Use a current stable Rust toolchain; the workspace uses edition 2024. In a source checkout, the library lives in `crates/lurq`, the gallery in `crates/gallery`, and macros in `crates/lurq_macros`.

## Platforms

| Platform | Supported |
| --- | --- |
| Windows | Yes |
| macOS | Yes, the latest release |
| iOS | Yes, the latest release ([iOS](../ios/)) |
| The browser | Yes: Chrome, Edge, Firefox and Safari, through WebGPU or WebGL2 ([In a browser](#in-a-browser), [Web](../web/)). One thread, no paths, the clipboard read through the browser, no video yet: [what a page cannot do](../web/#what-a-page-cannot-do) |
| Linux | **No** |

**Linux is not supported.** lurq is built and checked on Linux so that the crate publishes and its documentation builds, but nobody runs it there: there is no Linux machine to try a window, input, the clipboard, file dialogs, fonts or the GPU on, and what lurq does for the other systems (the system's appearance, reduced motion, menus, media, touch) is not written for Linux desktops. An app may build for Linux; what it does there is not tested, and issues that only Linux shows are not worked on.

## Run The Gallery

The gallery is the best executable reference: a page for every component of the design system, and under **Engine** a page for each of the engine's primitives (layout, sizing, position, grid, sticky, scroll, virtualized lists, paint, borders, transforms, theme, images, SVG, canvas, animation, presence, text, rich text, text input, events, reactivity, components, context, persistent storage, drag and drop, the router, the engine's own controls and overlays). Each page starts with a playground whose controls are the props, and shows the code of what it draws.

```powershell
cargo run -p gallery
```

Open it on a page with `--route`, for example the engine's layout:

```powershell
cargo run -p gallery -- --route /engine/layout
```

On Windows, the gallery can also use the DirectX 12 renderer:

```powershell
cargo run -p gallery -- --renderer dx12
```

Its default renderer is `wgpu`; pass `--renderer wgpu` or `--renderer dx12` to choose explicitly. Add `--features perf_profile` and `--profile-log` for frame timings, `--features devtools` for DevTools (F12, or `--devtools` at start), or `--features mcp` to drive it over MCP.

## In A Browser

The same app builds for `wasm32-unknown-unknown` and runs as a page, drawn to a canvas through WebGPU or WebGL2. The gallery of the design system (`crates/gallery`) is the example:

```powershell
python scripts/gallery-web.py --serve
```

It needs the `wasm32-unknown-unknown` target and wasm-bindgen's CLI. [Web](../web/) has the setup, what `main` does in a browser, and what a page does differently from a desktop.

## Feature Flags

`lurq`'s default features are the desktop set: a window, its GPU renderer, the system's clipboard and file dialog, pictures, drawing, forms, routes and queries. An app adds what it uses beyond them:

```toml
lurq = { version = "0.66.0", features = ["markdown", "http"] }
```

| Feature | Default | Enables |
| --- | --- | --- |
| `winit` | yes | The `WinitWindow` shell and desktop event loop integration. |
| `wgpu` | yes | WGPU render engine: Vulkan, Metal, Direct3D 12 or WebGPU. |
| `clipboard` | yes | System clipboard integration for text input copy, cut, paste, and selectable text copy shortcuts; reading text, images and files from the clipboard. |
| `file_dialog` | yes | The system's dialog for files to open, behind `lurq::files::FileDialog`. iOS has its own without the feature: the photo library, the camera and Files. |
| `image` | yes | Pictures from files and bytes: PNG, JPEG, WebP, GIF, BMP and TIFF decoding, `ui::Picture`, background images. |
| `canvas` | yes | Drawing: persistent Canvas 2D with gradients and effects, the charts, `Sparkline`, `Gauge` and `SignaturePad` of `lurq::ui`, and SVG (`Svg`, `SvgData`). |
| `form` | yes | Form handles, validation, submission, and compound form controls. |
| `router` | yes | Routes, links, outlets, and navigation history. |
| `query` | yes | Shared async reads, cache retention, and typed invalidation. |
| `webgl` | | In a browser without WebGPU, the WGPU render engine draws through WebGL2; see [Web](../web/). Changes nothing outside a browser. |
| `dx12` | | DirectX 12 render engine on Windows. |
| `media` | | Playing a video or a sound: `lurq::media`, `ui::VideoPlayer::source` and `ui::AudioPlayer::source`. |
| `markdown` | | Markdown parsing, the `Markdown` component and the rich text editor. |
| `http` | | Addresses on the network, for everything that loads. A file, bytes and a resource of the app are loaded by every build. |
| `regex` | | Regular expressions in `ui::find_matches` (`FindOptions::regex`). |
| `persistent_storage` | | Typed app storage, with in-memory defaults and optional `redb` files. |
| `serde` | | JSON: an app's translations read from JSON, and the Profiler's recording files. |
| `tokio` | | Optional execution on a Tokio handle configured on `App`. |
| `screenshot` | | GPU capture of the next fully composed window frame to PNG; takes `image`. `devtools` and `mcp` enable it. |
| `perf_profile` | | Frame timing/memory instrumentation and `Tree::last_profile`; independent of `devtools`. |
| `counting_allocator` | | `CountingAllocator`, a global allocator that reports allocations per frame to the profiler. |
| `devtools` | | Component metadata, signal values, profiler data, and the DevTools secondary window. |
| `mcp` | | Embeddable [MCP server](../mcp/) so AI agents can drive and inspect a running app. Nothing listens unless the app calls `Tree::enable_mcp`. |

Translations (`I18n`, `ctx.t`) are in every build; see [Internationalization](../i18n/). `raster` and `render` are lurq's own, turned on by `image`, `canvas`, `media` and the renderers: an app does not name them.

An app that leaves out part of the desktop set turns the default off and names what it keeps. The smallest app with a window:

```toml
lurq = { version = "0.66.0", default-features = false, features = ["winit", "wgpu"] }
```

A browser app names its list too: the default is for a desktop. With WebGL2 for a browser without WebGPU:

```toml
lurq = { version = "0.66.0", default-features = false, features = ["webgl", "winit", "clipboard", "image", "canvas", "form", "router", "query"] }
```

When `devtools` is enabled, component props and signal values must implement `DevtoolsInspectable`. Derive it on structs and enums you want to inspect:

```rust
#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct CardProps {
  title: &'static str,
  count: i32,
}
```

## Dev Profile

Text shaping, font parsing, and glyph rasterization run in dependencies (cosmic-text, harfrust, swash, skrifa, fontdb, and others). At `opt-level = 0` they make text-heavy screens noticeably slow in debug builds. Cargo reads `[profile]` sections only from the workspace being built and ignores the ones in dependencies, so lurq cannot set this for you. Add it to your workspace's root `Cargo.toml`:

```toml
# Optimize every dependency in dev builds; your own crates stay at opt-level 0 and debug normally.
[profile.dev.package."*"]
opt-level = 2
```

The wildcard does not match workspace members, so stepping through your own code is unaffected. The first debug build of the dependencies takes longer; after that they come from the build cache. The wildcard also keeps working when lurq's text stack changes, which a list of individual crates does not.

## Minimal App

Most apps wire three objects:

- `App`: shared runtime services such as fonts, theme, resources, storage, and optional Tokio execution.
- `Tree`: retained UI tree, component state, layout, input, rendering, devtools, and profiling state.
- `WinitWindow`: desktop shell that owns the event loop and forwards window/input events to the tree.

```rust
use lurq::{
  app::{
    App, Tree, WindowIcon,
    component::Component,
    ctx::Ctx,
    wgpu_render::WgpuRenderEngine,
    winit_shell::WinitWindow,
  },
  components::{Column, Text},
  core::Signal,
  node::Element,
};

struct Counter {
  count: Signal<i32>,
}

impl Component for Counter {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self { count: ctx.signal(0) }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let count = self.count.clone();

    Column::new()
      .spacing(12.0)
      .child(Text::new(&format!("Count: {}", count.get())))
      .child(Text::new("Click to increment").on_click(move |_| {
        count.update(|value| *value += 1);
      }))
  }
}

fn main() {
  let mut app = App::new();
  let mut tree = Tree::new();

  tree.set_render_engine_factory(|| Box::new(WgpuRenderEngine::new()));
  tree.mount_root::<Counter>(&mut app, ());

  WinitWindow::new(app, tree)
    .with_title("lurq counter")
    .with_size(800, 600)
    .with_icon(WindowIcon::from_rgba(vec![255, 0, 0, 255], 1, 1))
    .run();
}
```

## Docs Commands

The documentation site uses Yarn 1.22.22 and Astro Starlight. Node.js 22.12+ is required by the locked Astro version. Run these commands from the repository root:

```powershell
cd docs
yarn install --frozen-lockfile
yarn dev
yarn build
```

The Rust examples are Markdown code blocks: building the site does not compile them. See [Testing](../testing/) for Rust checks.
