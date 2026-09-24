---
title: Focus And Keyboard Navigation
description: Which elements take focus, how Tab moves between them, modal focus traps, focused styles, and testing focus.
---

# Focus And Keyboard Navigation

One element has focus at a time. It receives keyboard events, shows its focused style, and is where Tab and Shift+Tab start. There is one focused state, whatever moved focus there: a click, Tab, `ctx.focus(&element_ref)`, or `ElementHandle::focus()`. lurq has no separate keyboard-only (`:focus-visible`) state.

## Focusable Elements

Buttons, text inputs, checkboxes, sliders, selects, and any element with a `tab_index` are focusable. A click on one focuses it, and a click on its content (a button's label) focuses the nearest focusable ancestor.

- `.focusable(true)` makes any element focusable by click and by request. It does not put the element in the Tab order outside a form; add `.tab_index(0)` for that.
- `.focusable(false)` keeps an element from ever taking focus: not by click, not by Tab, not by `ctx.focus`. Its click handlers still run, and a checkbox still toggles. Use it for toolbar buttons that must leave focus in an editor.

```rust
use lurq::components::Button;

// Clicking it runs the handler; focus stays in the text editor.
Button::new("Bold").focusable(false).on_click(|_| toggle_bold());
```

## Tab Order

`tab_index` follows HTML `tabindex`:

| Element location | `tab_index` not set | `tab_index(0)` | `tab_index(n > 0)` | `tab_index(-1)` |
| --- | --- | --- | --- | --- |
| Inside a form | in Tab order | in layout order | first, by number | skipped |
| Outside any form | not in Tab order | in layout order | first, by number | skipped |

- Positive values come first, in ascending order; equal values keep tree order. Then come `tab_index(0)` elements (and, inside forms, controls without a tab index) in tree order.
- `tab_index(-1)` only removes an element from the Tab order. A click still focuses it, as in HTML.
- A button is one stop: its content is never a separate stop.
- Setting a tab index makes the element focusable, unless it is `focusable(false)`.

Outside forms, only elements that opt in with `tab_index(0)` or higher are stops. Give toolbar, sidebar, and list buttons `tab_index(0)` to make them reachable by keyboard:

```rust
use lurq::components::{Button, Row};

Row::new()
  .child(Button::new("New").tab_index(0).on_click(|_| new_file()))
  .child(Button::new("Open").tab_index(0).on_click(|_| open_file()));
```

## Tab Scope

Tab and Shift+Tab move through one scope and wrap around at its ends:

1. **Focus inside a form**: the form. Tab cycles the form's controls and does not leave it; click or `ctx.focus` elsewhere to leave.
2. **Otherwise, a modal is open**: the topmost open modal (see [Modal Focus Trap](#modal-focus-trap)).
3. **Otherwise**: the whole window, including open popups and overlays after the page, in tree order.

A form inside the window or modal scope takes part under form rules: its controls are stops even without a tab index, in tree order among the scope's other stops. So Tab from a toolbar button reaches the first field of a form that follows it, and from there Tab cycles the form.

When the focused element is not a stop itself (a clicked button without a tab index, or a text input outside a form), Tab continues from its place in the tree, like a browser: Tab goes to the next stop after it, Shift+Tab to the previous one. With nothing focused, Tab goes to the first stop and Shift+Tab to the last.

Tab with no stops in scope outside a modal is not handled, so it reaches other keyboard defaults. An `on_key_down` handler that calls `prevent_default()` on Tab replaces traversal entirely (see [Keyboard And Focus](../styling-events/#keyboard-and-focus)).

Tab traversal does not need the `form` feature. Without it there are no forms, so only the window and modal scopes apply.

## Modal Focus Trap

An open `Modal` confines Tab and Shift+Tab to itself, whether or not it contains a form:

- Focus behind the modal is never a stop. When focus is still on the page behind it (the button that opened it), the next Tab moves into the modal.
- Stops inside the modal follow the same rules: `tab_index(0)` or higher, and form controls inside a form in the modal.
- A modal without stops consumes Tab and keeps focus where it is, so Tab never reaches the page behind it.
- With nested or stacked modals, the topmost one traps.
- When the modal closes, the window scope applies again.

Pointer input is not trapped; a click on the page behind a `Parent`- or element-targeted modal still focuses what it hits.

## Scrolling Focus Into View

When Tab or Shift+Tab moves focus to an element inside a scroll container, every scroll container around it scrolls by the smallest amount that shows the element, innermost first. An element taller or wider than the viewport is aligned to the viewport start. Focus moved by click or by request does not scroll.

## Focused Styles

Every element accepts a focused state style, merged over the base style while it has focus, under the hovered and active styles:

```rust
use lurq::{
  app::theme::{BorderSize, PaletteColor},
  components::Button,
};

Button::new("Save")
  .tab_index(0)
  .border_inside(BorderSize::Sm, PaletteColor::Border)
  .focused(|style| style.border_inside(BorderSize::Sm, PaletteColor::BorderFocus));
```

Controls whose visuals are drawn from part styles have part-level focused styles, layered like their hovered styles. They change paint only; a width or height in them is ignored so focus never moves layout:

| Control | Focused style |
| --- | --- |
| Button, text input, any element | `.focused(...)` / `.focused_style(...)` |
| `Checkbox` | `.box_focused(...)` / `.box_focused_style(...)` |
| `Slider` | `.thumb_focused(...)` / `.thumb_focused_style(...)` |
| `Select` | `SelectStyle::trigger_focused(...)` |

The compound form controls take their focused border from the form theme: `form.input.border_focus`, `form.checkbox.border_focus`, `form.slider.thumb_border_focus`, and `border_focus` on both `form.button` roles, all `PaletteColor::BorderFocus` by default. See [Form Theme](../theme/#form-theme).

## Testing Focus

`Tree::focused_element()` returns the element that has focus, like `document.activeElement`: the control itself, not the wrapper that owns its `on_focus` handler.

`Tree::pass_headless(&mut app)` runs a pass without a window: it rebuilds components and lays out the tree, overlays and modals included, so hit testing, focus, and Tab work, but it draws nothing and never calls the render engine. It needs no window handle and no `unsafe`:

```rust
use lurq::{
  app::{App, Tree},
  components::{Button, Column, Modal, Root},
};

let mut tree = Tree::new();
tree.set_root(
  Column::new()
    .child(Button::new("Toolbar").id("toolbar").tab_index(0))
    .child(
      Modal::new(
        Column::new()
          .child(Button::new("Cancel").id("cancel").tab_index(0))
          .child(Button::new("Confirm").id("confirm").tab_index(0)),
      )
      .target(Root),
    ),
);
tree.pass_headless(&mut App::new());

tree.key_down("Tab".into(), "Tab".into(), false, false, false);
assert_eq!(tree.focused_element().and_then(|element| element.id()), Some("cancel"));
```

Call `tree.resize(width, height)` before the pass to choose the viewport (800×600 by default). Run another `pass_headless` after state changes that re-render (opening a signal-backed modal) and after keyboard scrolling, before reading bounds. See [Testing](../testing/).
