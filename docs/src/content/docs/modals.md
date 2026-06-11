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

## Stacking Multiple Modals

Modal order is based on when each modal opens, not where the `ctx.modal` call appears in the component tree. A modal opened from inside another modal renders above the older modal.

```rust
ctx.modal(self.settings_open.clone(), |ctx| {
  Column::new()
    .child(Text::new("App settings"))
    .child(Button::new("Close").on_click({
      let modal = ctx.modal_context().unwrap().clone();
      move |_| modal.close()
    }))
});

ctx.modal(self.stream_open.clone(), |_| {
  let settings_open = self.settings_open.clone();

  Column::new()
    .child(Text::new("New stream"))
    .child(Button::new("App settings").on_click(move |_| {
      settings_open.set(true);
    }))
});
```

If `stream_open` is set first and `settings_open` is set later, the app settings modal renders above the new stream modal even though its declaration appears first.

When one or more modals are open, `Escape` key events are routed only through the top modal subtree. This lets each modal attach its own Escape-to-close handler without also closing the modals underneath it.

## Behavior

- When the signal is `false`, the modal closure does not run and no nodes are rendered.
- When the signal is `true`, the modal content is inserted into a `__lurq_modal_host` node that wraps the root, placing the modal above all other content.
- Setting the signal back to `false` removes the modal on the next render pass.
- Multiple modals can be declared from the same component or different components. Each declaration keeps a stable slot while it is rendered, so opening or closing one modal does not shift another modal's identity.
- When more than one modal is open, the modal opened most recently renders above older modals.
- Reopening a closed modal gives it a new stack position, so it becomes the newest top modal.
- When modals are open, `Escape` key events are dispatched only to the top modal subtree.
