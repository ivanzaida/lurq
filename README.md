# lurq

`lurq` is a Rust UI toolkit with typed component builders, retained runtime state, reactive signals, GPU-backed rendering, and an in-app DevTools window.

## Install

```toml
[dependencies]
lurq = "0.10.7"
```

Enable the runtime/rendering features you need:

```toml
[dependencies]
lurq = { version = "0.10.7", features = ["winit", "wgpu"] }
```

Useful optional features:

| Feature | Purpose |
|---------|---------|
| `winit` | Window shell integration |
| `wgpu` | WGPU renderer |
| `dx12` | DirectX 12 renderer on Windows |
| `image` | Image components and image-backed styles |
| `svg` | SVG components |
| `resources` | Resource loader and resource-backed images/SVGs |
| `clipboard` | Clipboard shortcuts for text inputs |
| `devtools` | In-app DevTools window |

## Example

```rust
use lurq::{
  app::{App, Tree, winit_shell::WinitWindow},
  components::{Column, Text},
};

fn main() {
  let app = App::new();
  let mut tree = Tree::new();

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
- API docs: <https://docs.rs/lurq>

## Publishing

Crates are published by GitHub Actions when a pushed version is not already present on crates.io. Add a `CARGO_REGISTRY_TOKEN` repository secret with a crates.io API token, then bump the crate version and push to `master`.

The workflow publishes `lurq_macros` before `lurq` so the versioned macro dependency is available before the main crate is uploaded.

## License

MIT
