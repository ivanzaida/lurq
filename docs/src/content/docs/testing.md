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

The combinations above do not enable every subsystem. Check feature-specific behavior explicitly:

```powershell
cargo test -p lurq --features query,tokio --test query_tests
cargo test -p lurq --features form,router,persistent_storage,i18n --lib --tests
cargo test -p lurq --all-features --doc
cargo test -p lurq --features canvas --test canvas_tests
cargo check --workspace --all-features --all-targets --locked
```

The publish workflow in `.github/workflows/publish-crates.yml` defines release validation. Native GPU and window checks need the matching platform and a usable graphics environment; see [Canvas 2D](../canvas/#examples-and-checks) and [Window lifecycle](../window-lifecycle-menus/#mcp-and-verification). Ignored hardware tests are not run by an ordinary `cargo test`.

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

This snippet runs inside the layout test modules, where `use super::PassLayoutExt;` imports the helper from `tests/layout/mod.rs`. `pass_layout` is not a public `Tree` method. Production app code normally lets `Tree::pass` drive layout.

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

## Element Lookup And Typed Interaction

Tag nodes with `.id("...")` in the tree under test, then address them directly instead of writing predicates:

```rust
tree.set_root(
  Column::new()
    .child(TextInput::new(value.clone()).id("email"))
    .child(Button::new("Save").id("save").on_click(on_save)),
);
run_pass(&mut tree);

// Signal-backed value write; does NOT fire on_input (DOM `el.value = x`).
tree.get_element_by_id_mut("email").unwrap()
  .as_text_input().unwrap()
  .set_value("ada@example.com");

// DOM el.click(): fires the node's own on_click without hit-testing.
tree.get_element_by_id_mut("save").unwrap().click();
```

`click()` works even when the node is occluded. For pointer-fidelity coverage (hover, capture, hit testing) keep driving `tree.mouse_down` / `tree.mouse_up`, composing coordinates from the handle's `bounds().center()`.

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

Current benches cover layout, tree build, render-list generation, and the Markdown-backed text pipeline. The text benchmark requires its feature explicitly:

```powershell
cargo bench -p lurq --bench text_pipeline --features markdown
```

Its fixtures include the root `README.md`, so documentation changes can alter workload size. Compare the same fixture and source/toolchain metadata before interpreting historical numbers. Use benchmarks when changing layout caching, smart relayout, retained-node reconciliation, render command generation, or text rasterization.

Text pipeline optimization notes and benchmark history live in [Text Pipeline Optimization](/lurq/text-pipeline-optimization/).
