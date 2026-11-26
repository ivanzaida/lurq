# lurq

`lurq` is a Rust UI toolkit with typed component builders, retained runtime state, reactive signals, GPU-backed rendering, and an in-app DevTools window.

## Platforms

lurq supports **Windows**, **macOS** (the latest release), **iOS** (the latest release) and **the browser**: Chrome,
Edge, Firefox and Safari, through WebGPU, or WebGL2 with the `webgl` feature. In a browser what a page cannot do is
the platform's limit (one thread, no paths, the clipboard read only through the browser, another server's addresses
only where it allows them, no video yet): see [Web](docs/src/content/docs/web.md).

**Linux is not supported.** The crate builds there, so that it publishes and its documentation builds, but it is not
run or tested on Linux, and issues only Linux shows are not worked on.

## Install

```toml
[dependencies]
lurq = "0.76.0"
```

The default features are the desktop set, enough to run the example below:

```toml
[dependencies]
lurq = "0.76.0"
```

| Feature | Default | Purpose |
|---------|---------|---------|
| `winit` | yes | Window shell integration |
| `wgpu` | yes | WGPU renderer |
| `clipboard` | yes | Clipboard shortcuts for text inputs; reading text, images and files from the clipboard |
| `file_dialog` | yes | The system's dialog for files to open |
| `image` | yes | Image components, image decoding and image-backed styles |
| `canvas` | yes | Persistent Canvas 2D drawing through existing element refs (paths, gradients, shadows, blur, blend modes, isolated layers, text, clipping, raw images), charts, and SVG |
| `form` | yes | Form state, validation, and compound form controls |
| `router` | yes | In-process routing, nested layouts, and navigation |
| `query` | yes | Shared typed async queries, caching, and invalidation |
| `webgl` | | WebGL2 for the WGPU renderer in a browser without WebGPU |
| `dx12` | | DirectX 12 renderer on Windows |
| `media` | | Playing a video or a sound |
| `markdown` | | Markdown parsing and components |
| `http` | | Addresses on the network for everything that loads |
| `regex` | | Regular expressions in find |
| `serde` | | JSON translation-resource loading |
| `persistent_storage` | | Typed in-memory or file-backed app storage |
| `tokio` | | Run component futures, streams, and queries on a configured Tokio runtime |
| `devtools` | | In-app DevTools window |
| `mcp` | | Embeddable MCP server for AI agents to drive and inspect a running app |
| `screenshot` | | Capture the next fully composed window frame to PNG; enabled automatically by `devtools` |
| `perf_profile` | | Frame timing and memory counters, including `Tree::last_profile`; enabled automatically by `devtools` |

Translations (`I18n`) are in every build. A browser app turns the default off and names its features; see the getting started guide.

## Example

```rust
use lurq::{
  app::{App, Tree, wgpu_render::WgpuRenderEngine, winit_shell::WinitWindow},
  components::{Column, Text},
};

fn main() {
  let app = App::new();
  let mut tree = Tree::new();
  tree.set_render_engine_factory(|| Box::new(WgpuRenderEngine::new()));

  tree.set_root(
    Column::new()
      .spacing(8.0)
      .child(Text::new("Hello from lurq")),
  );

  WinitWindow::new(app, tree)
    .with_title("lurq")
    .with_size(800, 600)
    .run();
}
```

## Documentation

- Guide: <https://ivanzaida.github.io/lurq/>
- Running an app in a browser: [docs/src/content/docs/web.md](docs/src/content/docs/web.md); what is not done there: [docs/web-gaps.md](docs/web-gaps.md)
- API docs: <https://docs.rs/lurq>
- Local documentation build and maintenance: [docs/README.md](docs/README.md)

## Publishing

Crates are published by GitHub Actions when a pushed version is not already present on crates.io. Add a `CARGO_REGISTRY_TOKEN` repository secret with a crates.io API token, then bump the crate version and push to `master`.

The workflow publishes `lurq_macros` before `lurq` so the versioned macro dependency is available before the main crate is uploaded.

## License

MIT
