---
title: Runtime And Retained Tree
description: Retained tree behavior, dirty tracking, layout caching, element lookup, and redraw flow.
---

# Runtime And Retained Tree

This page is the lower-level reference for `Tree`. Start with [Mental Model](./mental-model/) and [App Runtime](./app-runtime/) first.

## Public Types

`lurq` uses these runtime-facing types:

- `App`: fonts, theme, resources, profiling setting.
- `Tree`: retained UI tree, components, layout, input, render engine, DevTools, profiling.
- `Element`: public erased UI value returned from components.
- `Node`: crate-private retained layout/render/input node.
- `Ctx`: per-component retained context.

There is no public `Runtime` type. Older notes may use that name; read it as `Tree`.

## Root Modes

Static root:

```rust
let mut tree = lurq::app::Tree::new();
tree.set_root(lurq::components::Text::new("static"));
```

Component root:

```rust
tree.mount_root::<RootComponent>(app.theme().clone(), RootProps);
```

Root props can be updated without replacing the component type:

```rust
tree.update_root_props::<RootComponent>(RootProps { enabled: true });
```

Call `tree.rebuild()` when code needs to force a root component render.

## Render Engine Ownership

Use `set_render_engine_factory`.

```rust
tree.set_render_engine_factory(|| Box::new(lurq::app::wgpu_render::WgpuRenderEngine::new()));
```

The factory creates the render engine for the main tree and any secondary trees. This is required because each OS window needs its own renderer state.

## Dirty Tracking

Dirty tracking happens at component context boundaries.

State changes mark the owning context dirty:

- `Signal::set` and `Signal::update`,
- `Store::set` and `Store::update`,
- `Lens::set` and `Lens::update`,
- memo output changes,
- reactive context changes,
- imperative element-ref layout mutations.

Before layout, rendering, hit testing, or element lookup, `Tree` rebuilds dirty component subtrees. Clean component subtrees keep their previous retained output.

## Prop Reconciliation

Mounted components are reused when identity matches.

For unkeyed mounts:

```rust
ctx.mount::<Child>(props)
```

Identity is component type plus slot position.

For keyed mounts:

```rust
ctx.mount_keyed::<Child>("stable-key", props)
```

Identity is component type plus key. Use keyed mounts for lists that can reorder.

If new props are unequal to stored props, the child context is marked dirty.

## Retained Node IDs

Every retained node has a `NodeId`. IDs are assigned by the tree id generator and released when nodes are removed.

Node IDs are used for:

- event targets,
- hit testing,
- element lookup,
- DevTools tree rows,
- overlay selection,
- drag/drop source and target IDs.

## Layout Cache

Layout is cached on retained nodes. Cache invalidation happens when layout-affecting state changes:

- size/frame constraints,
- padding,
- flex parameters,
- scroll state,
- text input value,
- text selection/caret runtime state,
- style state that affects layout,
- element-ref rect overrides,
- root resize or scale changes.

Smart relayout should stop as soon as an ancestor can still contain the changed child size. Full-tree invalidation should be reserved for changes that can affect ancestors, hit testing, or global viewport assumptions.

## Element Lookup

`find_element` gives read access to the current tree plus computed bounds.

```rust
let found = tree.find_element(|element| {
  element.text_content() == Some("Submit")
});

if let Some(found) = found {
  let rect = found.bounds();
  println!("{}x{}", rect.width, rect.height);
}
```

`find_element_mut` gives a mutable element ref for imperative rect overrides:

```rust
let handle = tree
  .find_element_mut(|element| element.text_content() == Some("Panel"))
  .unwrap();

handle.set_relative_bounds(12.0, 24.0, 300.0, 180.0);
```

Use this sparingly. Declarative component state should remain the default way to move UI.

## Input Dispatch

The shell forwards input into `Tree`:

```rust
tree.mouse_move(x, y);
tree.mouse_down(x, y, MouseButton::Left);
tree.mouse_up(x, y, MouseButton::Left);
tree.click(x, y, MouseButton::Left);
tree.scroll(x, y, delta_x, delta_y, ScrollPhase::Scroll);
tree.key_down(key, code, shift, ctrl, alt);
```

The tree resolves target nodes from the latest layout and updates hover, active, focus, drag, scroll, cursor, text selection, and text input editing state. Hit testing uses visual coordinates, so transformed text and transformed parents can still receive pointer selection from their painted position.

## Redraw

`Tree::needs_redraw()` reports whether the shell should request a frame. `WinitWindow` handles this automatically.

Manual shells should follow this pattern:

```rust
if tree.needs_redraw() {
  window.request_redraw();
}
```

During redraw:

```rust
tree.clear_needs_redraw();
tree.pass(&mut app, &surface);
```

## Last Layout And Profiles

Use `last_layout()` to inspect the last computed layout.

```rust
if let Some(layout) = tree.last_layout() {
  println!("root size: {}x{}", layout.size.width, layout.size.height);
}
```

Use `last_profile()` for frame timings and memory counters.

```rust
let profile = tree.last_profile();
println!("layout: {:?}", profile.layout);
```

## Secondary Windows

Secondary windows are owned by the main `Tree`. DevTools uses this path, but the concept is generic inside the runtime.

The important rule: secondary trees should render with the same render engine factory, not share the same render engine instance. Renderer instances are window/surface specific.
