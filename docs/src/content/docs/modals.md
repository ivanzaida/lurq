---
title: Modals
description: Declaring modal overlays that render above the component tree.
---

# Modals

## Declaring A Modal

Call `ctx.modal` inside `render` with a `Signal<bool>` that controls visibility. The modal content renders above the normal tree when the signal is `true`.

```rust
use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Text, Button},
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

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let open = self.open.clone();

    ctx.modal(self.open.clone(), |ctx| {
      Column::new()
        .child(Text::new("Modal content"))
        .child(Button::new("Close").on_click({
          let modal = ctx.modal_context().unwrap().clone();
          move |_| modal.close()
        }))
    });

    Button::new("Open modal").on_click(move |_| open.set(true))
  }
}
```

## ModalContext

Inside the modal render closure, `ctx.modal_context()` returns a `ModalContext` with:

| Method | Description |
| --- | --- |
| `.open()` | Set the signal to `true`. |
| `.close()` | Set the signal to `false`. |
| `.is_open()` | Read the current state. |
| `.signal()` | Get the underlying `Signal<bool>`. |

## Behavior

- When the signal is `false`, the modal closure does not run and no nodes are rendered.
- When the signal is `true`, the modal content is inserted into a `__lurq_modal_host` node that wraps the root, placing the modal above all other content.
- Setting the signal back to `false` removes the modal on the next render pass.
- Multiple modals can be declared from different components. Each gets its own slot in the modal host.
