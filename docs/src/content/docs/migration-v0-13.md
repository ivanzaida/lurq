---
title: Migration To v0.13
description: Migrating modal, popup, and overlay code to lurq v0.13.
---

# Migration To v0.13

v0.13 removes the legacy context-owned modal API and moves modals, popups, and overlays into normal render declarations. The main migration is to replace imperative `ctx.modal(...)` calls with `Modal` children.

## Modals

`ctx.modal(...)`, `ctx.modal_context()`, `ModalContext`, and the legacy modal host are removed.

Before:

```rust
ctx.modal(self.open.clone(), |ctx| {
  ctx.mount::<SettingsModal>(SettingsModalProps {
    title: "Settings".into(),
  })
});

Column::new().child(Button::new("Open").on_click({
  let open = self.open.clone();
  move |_| open.set(true)
}))
```

After:

```rust
Column::new()
  .child(Button::new("Open").on_click({
    let open = self.open.clone();
    move |_| open.set(true)
  }))
  .child(
    Modal::new(ctx.mount::<SettingsModal>(SettingsModalProps {
      title: "Settings".into(),
    }))
    .open(self.open.clone())
    .target(Root),
  )
```

`Modal` is a normal child declaration. It is layout-neutral, so it does not take space in its parent layout. When open, its content is layered over the selected target.

## Targets

`Modal::target(...)` accepts `Parent`, `Root`, or an `ElementRef`.

```rust
Modal::new(content).open(open.clone()).target(Parent);
Modal::new(content).open(open.clone()).target(Root);
Modal::new(content).open(open.clone()).target(panel_ref);
```

- `Parent` covers the declaring parent bounds and is the default.
- `Root` covers the viewport. Use it for the old app-wide modal behavior.
- `ElementRef` covers a specific element's bounds.

If your old modal expected to cover the whole app, add `.target(Root)`. If you want a modal that only covers a panel or card, use the default `Parent` target by rendering the modal as a child of that container.

## Closing

Modal close state now belongs to your signal.

Before:

```rust
let modal = ctx.modal_context().unwrap().clone();

Button::new("Close").on_click(move |_| modal.close())
```

After:

```rust
let open = self.open.clone();

Button::new("Close").on_click(move |_| open.set(false))
```

Signal-backed modals also close on `Escape` by default.

## Render Timing

The old API received a closure that built modal content through the modal registry. In v0.13, `Modal::new(...)` receives content from the regular render path.

If the modal content is expensive, keep the expensive work inside the mounted modal component or gate it yourself:

```rust
let modal = if self.open.get() {
  Modal::new(ctx.mount::<HeavyModal>(props))
    .open(self.open.clone())
    .target(Root)
} else {
  Modal::new(Stack::new()).open(self.open.clone()).target(Root)
};
```

For most component-backed modals, the direct `Modal::new(ctx.mount::<ModalComponent>(props))` migration is the right shape.

## Scoped Modals

To cover only a parent container, render the modal inside that container and use the default target:

```rust
Stack::new()
  .child(panel_content)
  .child(
    Modal::new(panel_modal_content)
      .open(panel_open.clone())
      .target(Parent),
  )
```

The explicit `.target(Parent)` is optional.

To cover a specific element, attach an element ref to the target and pass the same ref to the modal:

```rust
let panel_ref = ctx.element_ref();

Column::new()
  .ref_element(panel_ref.clone())
  .child(panel_content)
  .child(
    Modal::new(panel_modal_content)
      .open(open.clone())
      .target(panel_ref),
  )
```

## Popups And Overlays

Manual absolute-positioned dropdowns and popovers should migrate to `Popup` when they are anchored to an element.

```rust
Popup::new(anchor_ref, menu_content)
  .open(open.clone())
  .placement(Placement::BottomStart)
  .offset(0.0, 8.0)
  .match_anchor_width(true)
```

`Popover` is an alias for `Popup`. Use low-level `Overlay` only when you need custom anchored layer behavior that `Popup` does not expose.

## Hit Testing

Transparent overlay wrappers should opt into the intended hit-test behavior instead of relying on host-specific event paths.

```rust
Stack::new().hit_test(HitTestBehavior::ContentOnly);
Rect::new(200.0, 100.0).pointer_events_none();
```

- `HitTestBehavior::ContentOnly` lets transparent wrapper space pass through while keeping child content interactive.
- `.pointer_events_none()` makes the node and descendants ignored by hit testing.

## Tests

Tests should assert on user-visible modal, popup, or overlay content rather than legacy host tag names.

Before:

```rust
assert_eq!(tree.root().unwrap().tag_name(), "ModalHost");
```

After:

```rust
let root = tree.root().unwrap();
assert!(find_by_text(root, "Settings").is_some());
```

The runtime may still introduce an internal overlay host for active root overlays, root modals, and popups. That host is implementation detail; application tests should prefer content, layout, and behavior assertions.
