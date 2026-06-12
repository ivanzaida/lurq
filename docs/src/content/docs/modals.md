---
title: Modals
description: Declaring render-flow modal overlays with scoped targets.
---

# Modals

## Declaring A Modal

Use `Modal` as a normal child in your render tree. The modal declaration is layout-neutral, so it does not change parent layout, and its content is layered over the selected target when the open state is `true`.

```rust
use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Button, Column, Modal, Root, Text},
  core::Signal,
  node::Element,
};

struct App {
  open: Signal<bool>,
}

impl Component for App {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self { open: ctx.signal(false) }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let open = self.open.clone();

    Column::new()
      .child(Button::new("Open modal").on_click({
        let open = open.clone();
        move |_| open.set(true)
      }))
      .child(
        Modal::new(
          Column::new()
            .child(Text::new("Modal content"))
            .child(Button::new("Close").on_click({
              let open = open.clone();
              move |_| open.set(false)
            })),
        )
        .open(self.open.clone())
        .target(Root),
      )
  }
}
```

## Targets

`Modal::target(...)` accepts `Parent`, `Root`, or an `ElementRef`.

```rust
Modal::new(content).open(open.clone()).target(Parent);
Modal::new(content).open(open.clone()).target(Root);
Modal::new(content).open(open.clone()).target(panel_ref);
```

- `Parent` covers the declaring parent bounds and is the default target.
- `Root` covers the viewport.
- `ElementRef` covers that element's bounds.

## Behavior

- When the signal is `false`, the modal declaration remains layout-neutral and no modal layer is rendered.
- When the signal is `true`, the modal content is layered above its target.
- Setting the signal back to `false` removes the modal on the next render pass.
- Multiple render-flow modals stack in declaration/layer order.
- Signal-backed modals close on `Escape` by default.
