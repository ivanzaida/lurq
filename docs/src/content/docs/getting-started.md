---
title: Getting Started
description: Feature flags, demo commands, and the smallest useful lurq app.
---

# Getting Started

`lurq` is currently a workspace crate. The main library lives in `crates/lurq`, the demo app lives in `crates/demo`, and derive macros live in `crates/lurq_macros`.

## Run The Demo

The demo is the best executable reference because it exercises layout, sizing, positioning, scrolling, input, events, reactivity, components, contexts, animation, transforms, resources, and DevTools.

```powershell
cargo run -p demo --features "lurq/winit lurq/wgpu lurq/image lurq/svg lurq/resources lurq/devtools lurq/clipboard"
```

On Windows, the demo can also use the DirectX 12 renderer:

```powershell
cargo run -p demo --features "lurq/winit lurq/dx12 lurq/image lurq/svg lurq/resources lurq/devtools lurq/clipboard" -- --renderer dx12
```

The default demo renderer is `wgpu`. Pass `--renderer wgpu` or `--renderer dx12` to choose explicitly.

## Feature Flags

`lurq` keeps optional subsystems behind Cargo features.

| Feature | Enables |
| --- | --- |
| `winit` | The `WinitWindow` shell and desktop event loop integration. |
| `render` | Shared render data types. Usually enabled through `wgpu` or `dx12`. |
| `wgpu` | WGPU render engine. |
| `dx12` | DirectX 12 render engine on Windows. |
| `image` | `Image`, background images, and image decoding. |
| `svg` | `Svg` and SVG tessellation/rendering. |
| `resources` | Async local/remote resource loading. |
| `devtools` | Component metadata, signal values, profiler data, and the DevTools secondary window. |
| `clipboard` | System clipboard integration for text input copy, cut, paste, and selectable text copy shortcuts. |

When `devtools` is enabled, component props and signal values must implement `DevtoolsInspectable`. Derive it on structs and enums you want to inspect:

```rust
#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct CardProps {
  title: &'static str,
  count: i32,
}
```

## Minimal App

Most apps wire three objects:

- `App`: shared runtime services such as fonts, theme, resources, and profiling.
- `Tree`: retained UI tree, component state, layout, input, rendering, devtools, and profiling state.
- `WinitWindow`: desktop shell that owns the event loop and forwards window/input events to the tree.

```rust
use lurq::{
  app::{
    App, Tree,
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
  let app = App::new();
  let mut tree = Tree::new();

  tree.set_render_engine_factory(|| Box::new(WgpuRenderEngine::new()));
  tree.mount_root::<Counter>(app.theme().clone(), ());

  WinitWindow::new(app, tree)
    .with_title("lurq counter")
    .with_size(800, 600)
    .run();
}
```

## Docs Commands

The documentation site uses Yarn and Astro Starlight.

```powershell
cd docs
yarn install
yarn dev
yarn build
```
