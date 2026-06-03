---
title: Mental Model
description: How App, Tree, components, Element, layout, input, rendering, and DevTools fit together.
---

# Mental Model

`lurq` separates app services from the retained UI tree.

```text
WinitWindow
  owns App + Tree
  forwards window/input events
  requests redraws

App
  fonts, theme, resource loader, profiling setting

Tree
  root component/static root
  retained nodes and component contexts
  layout cache and last layout
  input, hover, active, focus, drag, scroll, text selection state
  render engine instance/factory
  optional DevTools secondary tree
```

## Render Flow

At a high level, one frame does this:

1. The shell sends window size, scale factor, input events, and redraw events to `Tree`.
2. `Tree` rebuilds dirty component subtrees.
3. The layout engine computes `LayoutResult` from the retained node tree and viewport constraints.
4. The tree resolves render commands from layout.
5. The selected render engine draws the render list.
6. Profiling and DevTools snapshots are updated when enabled.

`Tree::pass(app, surface)` is the low-level pass entry point. `WinitWindow` calls it for normal desktop apps.

## Component Output Is Retained

Components return `impl Into<Element>`, but the runtime stores a retained internal `Node` tree. That retained tree is why `lurq` can:

- reuse component instances across renders,
- keep signal/store/memo/ref/effect state inside each `Ctx`,
- update only dirty component subtrees,
- preserve scroll, hover, active, focus, drag, and text editing state,
- cache layout and redraw only when needed,
- inspect the current tree in DevTools.

## Components Own State

A component struct is persistent. Initialize state in `create`, read current props from `ctx.props()`, and return fresh UI from `render`.

```rust
struct SearchBox {
  query: Signal<String>,
}

impl Component for SearchBox {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self { query: ctx.signal(String::new()) }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::TextInput::new(self.query.clone())
      .placeholder("Search")
      .width(240.0)
  }
}
```

Signals mark the owning context dirty. Refs do not.

## Parent And Child Boundaries

Parents mount children through `Ctx`:

```rust
ctx.mount::<Header>(HeaderProps { title: "Docs" })
ctx.mount_keyed::<RowItem>(&item.id, item.clone())
ctx.mount_with::<Panel>(PanelProps { title: "Details" }, children)
```

Unkeyed mounts are matched by slot position and component type. Keyed mounts are matched by key and component type. Use keyed mounts for lists that reorder.

## Layout Is Constraint-Based

Layout is parent-down, child-up:

1. Parent gives constraints to child.
2. Child chooses a concrete size inside those constraints.
3. Parent positions the child.

`Row`, `Column`, and `Stack` are the main containers. Modifiers such as `.padding(...)`, `.width(...)`, `.flex(...)`, `.align(...)`, `.clip()`, and `.absolute_position(...)` wrap the current element with layout behavior.

## Input Targets Nodes

Input is resolved against the latest layout. The tree tracks hover path, active path, focus, dragging, scroll state, and cursor. Event handlers are attached with node modifiers:

```rust
Text::new("Save")
  .cursor(CursorIcon::Pointer)
  .hovered(|style| style.background("#334155"))
  .active(|style| style.background("#0f172a"))
  .on_click(|event| {
    println!("clicked at {}, {}", event.x, event.y);
  })
```

## DevTools Is Just Another Tree

With the `devtools` feature, `Tree::mount_devtools(theme)` creates a secondary tree that renders with the same render engine factory. The main tree periodically syncs a snapshot into the DevTools tree during `pass()`.

This means DevTools should follow the same layout/render/event rules as any other `lurq` UI. It also means debug metadata is feature-gated so production builds do not store signal values, prop trees, or profiler detail unless `devtools` is enabled.
