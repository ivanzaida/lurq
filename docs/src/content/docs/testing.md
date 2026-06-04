---
title: Testing
description: How lurq tests layout, rendering, input, reactivity, resources, and DevTools behavior.
---

# Testing

The repo has focused Rust tests under `crates/lurq/tests`. They are the best reference for expected behavior when changing runtime internals.

## Run Tests

```powershell
cargo test -p lurq --features resources
```

With feature combinations:

```powershell
cargo test -p lurq --features "image svg resources devtools"
cargo test -p lurq --features "winit wgpu image svg resources devtools clipboard"
```

Run one area:

```powershell
cargo test -p lurq layout::padding
cargo test -p lurq reactivity::signal
cargo test -p lurq dnd::target_tracking
```

## Layout Tests

Layout tests usually create a `Tree`, install a static root, and run a layout pass with explicit constraints.

```rust
use lurq::{
  app::Tree,
  layout::{Constraints, Size},
};

let mut tree = Tree::new();
tree.set_root(lurq::components::Spacer::new().size(100.0, 50.0));

let result = tree
  .pass_layout(Constraints::loose(Size::new(400.0, 400.0)))
  .unwrap();

assert_eq!(result.size.width, 100.0);
assert_eq!(result.size.height, 50.0);
```

`pass_layout` is a test extension used in the test modules. Production app code normally lets `Tree::pass` drive layout.

## Render Snapshot Tests

`tests/support.rs` defines a capturing render engine that records the generated render list.

Use this style when testing visual output without opening a real GPU window:

```rust
let mut tree = Tree::new();
tree.set_root(lurq::components::Rect::new(100.0, 50.0).background("#22c55e"));

let snapshot = support::render_pass(&mut tree);
assert_eq!(snapshot.rects.len(), 1);
assert_eq!(snapshot.rects[0].width, 100.0);
```

This is useful for border, radius, opacity, image order, SVG order, and render-list regressions.

## Reactivity Tests

Reactivity tests validate state containers independently and through component dirty tracking.

```rust
let signal = lurq::core::Signal::new(0);
signal.update(|value| *value += 1);
assert_eq!(signal.get(), 1);
```

Dirty tracking tests verify that:

- child signal updates do not rerender clean parents,
- parent signal updates do not rerender clean children unnecessarily,
- passed signals mark children dirty when the child reads them,
- prop changes rerender the affected child.

## Input And DnD Tests

Input tests drive the tree directly:

```rust
tree.mouse_move(20.0, 20.0);
tree.mouse_down(20.0, 20.0, MouseButton::Left);
tree.mouse_up(20.0, 20.0, MouseButton::Left);
tree.scroll(20.0, 20.0, 0.0, -120.0, ScrollPhase::Scroll);
tree.key_down("a".into(), "KeyA".into(), false, false, false);
```

Click handlers run from a matching pointer down/up pair; tests should not inject a separate click event.

Use direct tree input for deterministic hover, active, focus, scroll, text input, selectable text, slider, checkbox, drag, and drop behavior.

Text input tests cover caret placement, Unicode-safe deletion, keyboard selection, multiline movement, undo/redo, and double/triple-click selection. Selectable text tests cover drag ranges, word and line selection, and transformed visual-coordinate hit testing.

## DevTools Tests

DevTools tests construct snapshots from the tree and assert collected metadata:

- tag names for built-ins and user components,
- recursive props from `DevtoolsInspectable`,
- signal/memo value history,
- effect metadata,
- context metadata,
- overlay selection behavior,
- pick mode and scroll-into behavior.

Run them with:

```powershell
cargo test -p lurq --features devtools
```

If a test only fails with `devtools`, check trait bounds first. Props, signal values, stores, and memo outputs may need `DevtoolsInspectable`.

## Benchmarks

Benchmarks live in `crates/lurq/benches`:

```powershell
cargo bench -p lurq
```

Current benches cover layout, tree build, and render-list generation. Use them when changing layout caching, smart relayout, retained-node reconciliation, or render command generation.
