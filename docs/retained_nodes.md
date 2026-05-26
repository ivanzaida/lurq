# Runtime And Retained Tree

## Overview

The public API works with `Element`. Internally, the runtime stores a crate-private retained node tree for layout, rendering, hit testing, and component reconciliation.

The internal tree is rebuilt only where needed:

- `set_root(element)` installs a static element tree.
- `mount_root::<Component>(props)` installs a root component.
- Reactive state writes mark owning components dirty.
- Before layout/render/event lookup, runtime rebuilds dirty component subtrees.

## Runtime Setup

```rust
use lurq::{
  app::{Runtime, wgpu_render::WgpuRenderEngine, winit_shell::WinitWindow},
};

let mut runtime = Runtime::new();
runtime.set_render_engine(Box::new(WgpuRenderEngine::new()));
runtime.mount_root::<App>(());

WinitWindow::new(runtime)
  .with_title("lurq demo")
  .run();
```

## Window Tick Callback

`WinitWindow::on_tick` runs a callback with mutable runtime access while the window event loop is active.

```rust
WinitWindow::new(runtime)
  .with_title("lurq demo")
  .on_tick(|rt: &mut Runtime| {
    // Update runtime-managed state here.
  })
  .run();
```

When a tick callback is installed, the winit event loop uses polling so ticks continue without input events. After each tick, the window checks `Runtime::needs_redraw()` and requests redraw when needed.

## Element Lookup

Use `find_element` to search the current tree and get the element plus its computed rect.

```rust
let found = runtime.find_element(|el| {
  el.color() == Some(Color::from_hex("#22c55e"))
});

if let Some(found) = found {
  println!("x={}, y={}", found.rect.x, found.rect.y);
}
```

`ElementRect` contains both absolute and parent-relative coordinates.

```rust
pub struct ElementRect {
  pub x: f32,
  pub y: f32,
  pub relative_x: f32,
  pub relative_y: f32,
  pub width: f32,
  pub height: f32,
}
```

- `x` and `y` are absolute window-space coordinates.
- `relative_x` and `relative_y` are relative to the parent layout origin.
- `width` and `height` are the computed layout size.
- `center()` returns the center point of the rect.

`find_element` updates dirty component output and layout before returning.

## Mutable Element Rects

Use `find_element_mut` when runtime code needs to change an element's layout rect.

```rust
{
  let mut found = runtime
    .find_element_mut(|el| el.color() == Some(Color::from_hex("#22c55e")))
    .unwrap();

  found.rect.relative_x = 15.0;
  found.rect.relative_y = 20.0;
  found.rect.width = 30.0;
  found.rect.height = 40.0;
} // mutation is applied when `found` is dropped
```

The mutable handle writes back on drop. If the rect changed, runtime stores a runtime rect override, invalidates layout, clears cached layout, and marks the runtime for redraw.

Use `relative_x` and `relative_y` for mutation. Absolute `x` and `y` are derived from parent position plus the relative offset.

## Redraw Flow

The runtime sets `needs_redraw` when input, reactive updates, scroll updates, or runtime rect mutations change what should be drawn. The winit shell calls `request_redraw()` when needed.

During redraw, `runtime.pass(window)` performs the render pass through the configured render engine.

## Internal Dirty Tracking

Internal nodes cache layout results. Visual and layout-affecting changes invalidate the relevant caches. Runtime rect mutation invalidates the tree because a child override can affect parent positioning and hit testing.

Components are still the source of truth for reactive UI. Runtime rect mutation is an imperative escape hatch for direct runtime manipulation, animation, or tooling.
