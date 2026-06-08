---
title: DevTools
description: Enabling DevTools, inspecting components, selecting nodes, profiling renders, and signal debugging.
---

# DevTools

DevTools is feature-gated. Without the `devtools` Cargo feature, the app does not store extra prop trees, signal values, memo values, effect metadata, or profiler snapshots for inspection.

## Enable It

Build with `lurq/devtools` and mount DevTools from the main tree.

```rust
use lurq::app::{App, Tree};

let mut app = App::new();
let mut tree = Tree::new();

lurq::app::devtools::load_fonts(&mut app);
app.set_profiling_enabled(true);

tree.set_render_engine_factory(|| Box::new(lurq::app::wgpu_render::WgpuRenderEngine::new()));
tree.mount_root::<Root>(&mut app, RootProps);
tree.mount_devtools(&mut app);
```

The winit shell sees DevTools as a secondary window. It does not need inspector-specific code.

## What DevTools Shows

The current DevTools UI has three primary tabs:

| Tab | Shows |
| --- | --- |
| Components | Retained component/element tree, selected node details, props, signals, contexts, effects, shape/style rows, and optional overlay. |
| Profiler | Captured render commits, frame timings, render causes, signal changes, memo recomputes, layout status, and perf overlay stats. |
| Signals | Signal list, value, owner component, subscriber count, dependency graph, and change history. |

## Inspectable Props

When `devtools` is enabled, component props must implement `DevtoolsInspectable`.

```rust
#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct InfoCardProps {
  title: &'static str,
  body: &'static str,
  accent: &'static str,
  metadata: Metadata,
}

#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct Metadata {
  count: i32,
  enabled: bool,
}
```

Nested structs that also derive `DevtoolsInspectable` are shown recursively. Scalar values are shown with type and value. Mark sensitive or noisy fields with `#[devtools_ignore]`.

```rust
#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct Credentials {
  user: String,
  #[devtools_ignore]
  token: String,
}
```

Enums show their current variant.

## Inspectable Signals And Memos

With `devtools`, signal and memo values must be inspectable too:

```rust
#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct CounterState {
  count: i32,
}

let state = ctx.signal(CounterState { count: 0 });
let doubled = ctx.memo({
  let state = state.clone();
  move || state.get().count * 2
});
```

DevTools records:

- signal id,
- signal type,
- formatted value,
- owner component,
- subscriber count,
- recent value changes,
- memo recomputes.

## Overlay And Pick Mode

The Components tab can draw an overlay over the selected inspected node. The overlay is controlled from DevTools, while the main tree owns the actual overlay drawing.

Pick mode reverses selection:

1. Click the pick button in DevTools.
2. Click an item in the inspected app.
3. The matching component tree row is selected in DevTools.
4. The tree panel expands ancestors and scrolls the selected row into view.

The main tree exposes this through internal tree methods such as debug overlay selection and node picking; the shell only routes secondary-window pick requests.

## Profiler

Enable profiling on `App`:

```rust
app.set_profiling_enabled(true);
```

The profiler tab uses frame snapshots from the tree. A commit records:

- commit index,
- duration,
- number of rendered components/nodes,
- signal count,
- whether layout recalculated,
- human-readable timestamp,
- render triggers such as signal changes and memo recomputes,
- perf overlay timings when available.

`Tree::last_profile()` returns the latest low-level frame profile for custom tooling.

## Perf Overlay

The perf overlay is separate from DevTools but feeds data that DevTools can show.

```rust
tree.draw_perf_overlay();
```

The overlay enables the frame profiling it needs, samples FPS once per second, and shows frame stage timings such as layout, resolve, glyph, acquire, upload, encode, submit, and present.

## Common Issues

If a prop type fails to compile only with `devtools`, derive or implement `DevtoolsInspectable`.

If signal values show as unknown, check that the signal type implements `DevtoolsInspectable` and that the value is stored through `ctx.signal`, `ctx.store`, or `ctx.memo`.

If DevTools does not repaint after picking while the app window is focused, check the secondary window redraw path in the shell. Secondary windows must request/present frames even when they are not the active OS window.
