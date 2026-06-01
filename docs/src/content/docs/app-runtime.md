---
title: App Runtime
description: App, Tree, render engine factories, windows, frame flow, profiling, and element lookup.
---

# App Runtime

The public runtime surface is split between `App`, `Tree`, and the shell.

## App

`App` stores services shared by a tree pass:

- glyph engine and loaded fonts,
- theme,
- profiling enabled flag,
- optional scale override,
- optional resource loader and decoded image/SVG caches.

```rust
let mut app = lurq::app::App::new();

app.load_font_file(std::path::Path::new("assets/Inter.ttf"));
app.register_font("ui", "Inter");
app.set_profiling_enabled(true);

#[cfg(feature = "resources")]
app.set_resource_root(std::path::PathBuf::from("assets"));
```

## Tree

`Tree` stores the retained UI state. It owns:

- root component or static root element,
- component contexts and reactive subscriptions,
- retained nodes,
- layout state and layout cache,
- render engine instance and render engine factory,
- input state,
- animation and transition engines,
- perf overlay state,
- secondary windows and optional DevTools metadata.

```rust
let mut tree = lurq::app::Tree::new();
tree.mount_root::<RootComponent>(app.theme().clone(), RootProps::default());
```

For static UI without a component root:

```rust
tree.set_root(lurq::components::Text::new("Hello"));
```

## Render Engine Factory

Use a factory, not a prebuilt render engine. Secondary windows, including DevTools, can inherit the same renderer choice by asking the factory for their own engine instance.

```rust
tree.set_render_engine_factory(|| {
  Box::new(lurq::app::wgpu_render::WgpuRenderEngine::new())
});
```

On Windows with `dx12`:

```rust
tree.set_render_engine_factory(|| {
  Box::new(lurq::app::dx12_render::Dx12RenderEngine::new())
});
```

## Winit Shell

`WinitWindow` creates the OS window, forwards events to `Tree`, and calls `Tree::pass`.

```rust
WinitWindow::new(app, tree)
  .with_title("lurq app")
  .with_size(1200, 800)
  .with_min_size(800, 500)
  .on_tick(Tree::request_redraw)
  .run();
```

Use `on_tick` when animations or app code need a steady tick even if there is no input. The shell also keeps ticking while perf overlay or active timelines need redraws.

## Frame And Redraw Flow

`Tree` sets `needs_redraw` when state changes:

- signal/store/memo dependency updates,
- input, hover, active, focus, scroll, drag, or cursor changes,
- animation or transition progress,
- layout-affecting ref mutation,
- perf overlay updates,
- DevTools pick/overlay state.

The shell observes `needs_redraw`, requests a redraw, then calls `Tree::pass(app, window)`.

## Sizing And Scale

The shell updates the tree from the window before each pass:

```rust
tree.set_scale_factor(window.scale_factor() as f32);
tree.resize(size.width, size.height);
```

Tests can override layout constraints directly:

```rust
tree.set_layout_constraints_override(Some(lurq::layout::Constraints::tight(
  lurq::layout::Size::new(800.0, 600.0),
)));
```

## Element Lookup

Use `find_element` when integration code or tests need a computed rect.

```rust
let found = tree.find_element(|el| el.text_content() == Some("Save"));

if let Some(found) = found {
  let bounds = found.bounds();
  println!("{}x{} at {}, {}", bounds.width, bounds.height, bounds.x, bounds.y);
}
```

Use `find_element_mut` only for imperative layout overrides. Declarative component state is the normal path.

## Perf Overlay

The runtime has a built-in frame perf overlay.

```rust
app.set_profiling_enabled(true);
tree.draw_perf_overlay();
```

Profiling data is available through:

```rust
let profile = tree.last_profile();
```

The profile records high-level timings such as layout, resolve, glyph, upload, encode, submit, present, and memory counters used by DevTools.

## DevTools Window

With `devtools` enabled:

```rust
lurq::app::devtools::load_fonts(&mut app);
app.set_profiling_enabled(true);
tree.mount_devtools(app.theme().clone());
```

DevTools is represented as a secondary tree. The shell does not need special inspector logic; it only manages secondary windows and renders each tree.
