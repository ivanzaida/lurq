---
title: Styling And Events
description: Visual modifiers, state styles, cursors, inputs, event handlers, scroll, and drag and drop.
---

# Styling And Events

Most visual and input behavior is expressed as chainable modifiers on typed components.

## Visual Modifiers

```rust
use lurq::{components::Rect, node::color::Color};

Rect::new(120.0, 40.0)
  .fill("#2563eb")
  .rounded(8.0)
  .border_inside(1.0, Color::from_hex("#1d4ed8"))
```

Common visual modifiers:

| Modifier | Purpose |
| --- | --- |
| `.fill(color)` | Background fill. Accepts hex strings through `Into<Color>`. |
| `.background(color)` | Background fill from `Color`. |
| `.rounded(radius)` | Uniform corner radius. |
| `.corner_radius_*` | Per-corner radius. |
| `.border_inside(width, color)` | Border inside the element bounds. |
| `.border_center(width, color)` | Border centered on the element edge. |
| `.border_outside(width, color)` | Border outside the element bounds. |
| `.opacity(value)` | Draw opacity. |
| `.clip()` | Clip descendants to this element. |
| `.overflow_visible()` | Allow descendants to paint outside this element. |

## Hover, Active, And Focus Styles

State styles merge into the base style while the node is hovered, active, or focused.

```rust
use lurq::{components::Text, node::CursorIcon};

Text::new("Save")
  .padding_horizontal(12.0)
  .padding_vertical(8.0)
  .fill("#2563eb")
  .rounded(6.0)
  .cursor(CursorIcon::Pointer)
  .hovered(|style| style.fill("#3b82f6"))
  .active(|style| style.fill("#1d4ed8"))
  .focused(|style| style.border_inside(1.0, "#93c5fd".into()))
```

State styles can affect layout if they change frame, padding, or flex. That is supported, but it can force relayout when interaction state changes.

Use `ctx.interaction()` when component code needs to read the current interaction state:

```rust
let interaction = ctx.interaction();
let hovered = interaction.is_hovered();
```

Attach the state to an element with `.interactive(interaction)` if you need to observe that element's state from component code.

## Mouse Events

```rust
Rect::new(100.0, 40.0)
  .on_mouse_down(|event| println!("down {:?}", event.button))
  .on_mouse_up(|event| println!("up at {}, {}", event.x, event.y))
  .on_click(|event| println!("click target {:?}", event.target_id))
  .on_dblclick(|event| println!("double click {:?}", event.target_id))
  .on_mouse_move(|event| println!("move {}, {}", event.x, event.y))
  .on_mouse_enter(|| println!("enter"))
  .on_mouse_leave(|| println!("leave"))
```

`MouseEvent` includes `x`, `y`, `button`, `kind`, and `target_id`.

## Keyboard And Focus

Keyboard events go to the focused node.

```rust
Text::new("Focusable")
  .on_focus(|| println!("focused"))
  .on_blur(|| println!("blurred"))
  .on_key_down(|event| {
    println!("key={} code={} shift={}", event.key, event.code, event.shift);
  })
```

`KeyboardEvent` includes `key`, `code`, `shift`, `ctrl`, `alt`, and `target_id`.

## Scroll

Wrap content in one of the scroll components:

```rust
use lurq::components::{Column, ScrollVertical, Text};

ScrollVertical::new(
  Column::new()
    .spacing(8.0)
    .child(Text::new("Row 1"))
    .child(Text::new("Row 2")),
)
.on_scroll(|event| println!("delta: {}, {}", event.delta_x, event.delta_y))
```

Customize the scrollbar:

```rust
use lurq::{layout::scrollbar::{ScrollBarStyle, ScrollBarVisibility}, node::color::Color};

ScrollVertical::new(content)
  .scrollbar(ScrollBarStyle {
    visible: ScrollBarVisibility::Auto,
    width: 7.0,
    thumb_color: Color::from_hex("#64748b"),
    thumb_radius: 4.0,
    ..ScrollBarStyle::default()
  })
  .scrollbar_hovered(|style| style.with_thumb_color(Color::from_hex("#94a3b8")))
```

`ScrollEvent` includes `x`, `y`, `delta_x`, `delta_y`, `phase`, and `target_id`.

## Inputs

Inputs are controlled by signals.

```rust
let checked = ctx.signal(false);
let volume = ctx.signal(0.5_f32);
let name = ctx.signal(String::new());

Column::new()
  .child(lurq::components::Checkbox::new(checked.clone()))
  .child(lurq::components::Slider::new(volume.clone()).range(0.0, 1.0))
  .child(lurq::components::TextInput::new(name.clone()).placeholder("Name"))
```

Input updates write back to their signals, which rerenders the owning component.

## Drag And Drop

Use high-level DnD components when you want draggable nodes and drop zones.

```rust
use lurq::components::{
  DragContainer, DragContainerProps, Draggable, DraggableProps, DropZone, DropZoneProps, Rect, Stack,
};

let card = Draggable::mount(
  ctx,
  DraggableProps::new().on_drag_end(|event| {
    println!("drop result: {:?}", event.drop_result);
  }),
  Rect::new(64.0, 64.0)
    .fill("#2563eb")
    .absolute_position(24.0, 24.0),
);

let zone = DropZone::mount(
  ctx,
  DropZoneProps::new().on_drop(|event| {
    println!("source {:?} dropped on {:?}", event.source_id, event.target_id);
  }),
  Rect::new(160.0, 100.0)
    .fill("#16a34a33")
    .absolute_position(180.0, 80.0),
);

DragContainer::mount(
  ctx,
  DragContainerProps::new(),
  Stack::new()
    .size(420.0, 240.0)
    .child(zone)
    .child(card),
)
```

`DragContainerProps::new()` bounds descendant draggables to the container surface. Use `DragBounds::None` when the draggable should not be constrained.

Low-level node drag handlers are also available: `.on_drag_start`, `.on_drag_move`, `.on_drag_end`, and `.on_drop`.
