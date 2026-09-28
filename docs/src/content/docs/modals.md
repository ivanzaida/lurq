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
- An open modal traps Tab and Shift+Tab: they cycle the topmost modal's stops (elements with `tab_index(0)` or higher, and form controls) and never reach the page behind it, with or without a form. A form inside the modal cycles inside it. See [Modal Focus Trap](../focus-navigation/#modal-focus-trap).

## Popups And Outside Presses

`Popup` (and its alias `Popover`) layers content next to an anchor element. With a signal-backed open state it closes on `Escape` and on a left press outside it:

```rust
use lurq::components::{OutsidePress, Placement, Popup};

Popup::new(anchor_ref.clone(), menu_content)
  .open(open.clone())
  .placement(Placement::BottomStart);

// A light-dismiss popover: the closing press also reaches the element under it.
Popup::new(anchor_ref, hint_content)
  .open(open.clone())
  .outside_press(OutsidePress::PassThrough);
```

A press is outside a popup when it lands neither on the popup, nor on its anchor, nor on a layer opened above it, such as a select menu or a popup opened from inside it. By default (`OutsidePress::Consume`) such a press only closes the popup, like a native menu: the element under the pointer receives no press, release or click, and focus does not move. The next press reaches it. This keeps a press meant to close a menu from also activating whatever lies under it, for example a tab, a list row or a button. `OutsidePress::PassThrough` closes the popup and delivers the press as usual, like a light-dismiss web popover; a handler that prevents the default of that press keeps the popup open.

`Select` menus follow the same rule, set with `Select::outside_press(...)`. `Overlay::outside_press(...)` applies to overlays with `dismiss_on_outside_click(true)`. A popup with `dismiss_on_outside_click(false)` or a static open state never closes on a press and never consumes one. Only left presses close popups; other buttons pass through and leave them open. When one press is outside several popups, it closes all of them, and it is consumed if any of them consumes it.
