---
title: Styling And Events
description: Visual modifiers, state styles, cursors, inputs, event handlers, scroll, and drag and drop.
---

# Styling And Events

Most visual and input behavior is expressed as chainable modifiers on typed components.

Use [Theme](./theme/) roles for shared app semantics such as palette colors, text variants, radii, spacing, and compound form controls. Use concrete values for isolated one-off visuals.

## Visual Modifiers

```rust
use lurq::{components::Rect, node::color::Color};

Rect::new(120.0, 40.0)
  .background("#2563eb")
  .rounded(8.0)
  .border_inside(1.0, Color::from_hex("#1d4ed8"))
```

Common visual modifiers:

| Modifier | Purpose |
| --- | --- |
| `.background(color)` | Background color from a concrete color or `PaletteColor`. |
| `.rounded(radius)` | Uniform corner radius from `f32` or `RadiusSize`. |
| `.corner_radius_*` | Per-corner radius from `f32` or `RadiusSize`. |
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
  .background("#2563eb")
  .rounded(6.0)
  .cursor(CursorIcon::Pointer)
  .hovered(|style| style.background("#3b82f6"))
  .active(|style| style.background("#1d4ed8"))
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

## Text Selection

Plain text is not selectable by default. Opt in with `.selectable(true)`:

```rust
lurq::components::Text::new("Drag, double-click, or triple-click this text")
  .selectable(true)
```

Selectable text supports pointer drag ranges, double-click word selection, and triple-click line selection. Multiline and wrapped text render one selection highlight per selected row. Selection is visual-coordinate aware, so text inside transformed parents can still be selected from the painted position.

With the `clipboard` feature enabled, `Ctrl+C` and `Ctrl+Insert` copy the current selectable text selection to the system clipboard.

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
let volume = ctx.signal(50);
let name = ctx.signal(String::new());

Column::new()
  .child(lurq::components::Checkbox::new(checked.clone()))
  .child(lurq::components::Slider::new(volume.clone()).range(0, 100))
  .child(lurq::components::TextInput::new(name.clone()).placeholder("Name"))
```

Input updates write back to their signals, which rerenders the owning component.

### Checkbox Styling

Checkboxes accept normal element modifiers such as `.size()`, `.background()`, `.border_inside()`, `.rounded()`, `.cursor()`, `.hovered()`, and `.focused()`. Generic `.background()` styles the unchecked box. Checked visuals use checkbox-specific styles so the checked state can have its own color or indicator.

```rust
use lurq::{components::Checkbox, core::Signal, node::color::Color};

let enabled = Signal::new(true);

Checkbox::new(enabled)
  .size(20.0, 20.0)
  .background("#ffffff")
  .border_inside(1.0, Color::from_hex("#94a3b8"))
  .rounded(4.0)
  .checked_box(|style| {
    style
      .background("#2563eb")
      .border_inside(1.0, Color::from_hex("#1d4ed8"))
      .rounded(4.0)
  })
  .box_hovered(|style| style.border_inside(1.0, Color::from_hex("#38bdf8")))
  .checked_box_hovered(|style| style.background("#1d4ed8"))
```

With the `image` feature enabled, checked boxes can render an indicator image centered inside the box:

```rust
use lurq::{components::Checkbox, images::ImageData};

let check = ImageData::from_file("assets/check.png").unwrap();

Checkbox::new(enabled)
  .checked_box(|style| {
    style
      .background("#16a34a")
      .indicator_image(check)
      .indicator_size(12.0, 12.0)
      .indicator_contain()
  })
```

With `image` and `resources`, the indicator can come from the app resource loader:

```rust
Checkbox::new(enabled)
  .checked_box(|style| style.indicator_image("ui/check.png").indicator_size(12.0, 12.0))
```

### Text Input Editing

`TextInput` keeps editing state internally while the string value remains signal-owned. Clicking focuses the input and places the caret. Dragging selects a range; double-click selects a word; triple-click selects a line. Multiline inputs support vertical caret movement and per-row selection highlights.

Keyboard editing supports character insertion, `Backspace`, `Delete`, arrow keys, `Home`, `End`, `Ctrl+A`, `Ctrl+Z`, `Ctrl+Y`, and `Ctrl+Shift+Z`. Hold `Shift` with movement keys to extend the selection; hold `Ctrl` with horizontal movement to jump by words.

With the `clipboard` feature enabled, text inputs also support `Ctrl+C`, `Ctrl+X`, `Ctrl+V`, `Ctrl+Insert`, `Shift+Insert`, and `Shift+Delete`. Without `clipboard`, those shortcuts do not read or write the system clipboard.

### Slider Styling

Sliders use integer signals. Pointer input maps the track position to the nearest integer in the configured range, and arrow keys nudge by `1`.

The slider frame still accepts normal modifiers like `.width()`, `.height()`, `.cursor()`, and `.focused()`. Track and thumb visuals are styled separately with `SliderPartStyle`.

```rust
use lurq::{components::Slider, core::Signal, node::color::Color};

let value = Signal::new(68);

Slider::new(value)
  .range(0, 100)
  .width(260.0)
  .height(34.0)
  .track(|style| {
    style
      .size(220.0, 2.0)
      .background("#334155")
      .rounded(1.0)
      .border_center(1.0, Color::from_hex("#64748b"))
  })
  .track_hovered(|style| {
    style
      .height(4.0)
      .background("#475569")
      .border_center(1.0, Color::from_hex("#93c5fd"))
  })
  .thumb(|style| {
    style
      .size(12.0, 12.0)
      .background("#f97316")
      .rounded(6.0)
      .border_inside(2.0, Color::from_hex("#0f172a"))
  })
  .thumb_hovered(|style| {
    style
      .size(14.0, 14.0)
      .background("#fb923c")
      .rounded(7.0)
      .border_inside(2.0, Color::from_hex("#f8fafc"))
  })
```

The track and thumb support width, height, background color, border, corner radius, image backgrounds, and hover overrides. Corner radius accepts `f32` or `RadiusSize`. Hover dimensions are included in the slider's preferred size, so a larger hover thumb does not resize surrounding layout when the pointer enters.

The thumb is centered on the track line, not on the slider frame. A `2px` track with a `10px` or `14px` thumb keeps the thumb vertically centered on that thin track.

Image-backed slider parts use the same `image` feature as node background images:

```rust
use lurq::{components::Slider, images::ImageData};

let track = ImageData::from_file("assets/track.png").unwrap();
let thumb = ImageData::from_file("assets/thumb.png").unwrap();

Slider::new(value)
  .track(|style| style.height(2.0).background_image(track).background_cover())
  .thumb(|style| style.size(16.0, 16.0).background_image(thumb).background_cover())
```

With `image` and `resources`, pass resource paths instead:

```rust
Slider::new(value)
  .track(|style| style.background_image("ui/slider-track.png").background_cover())
  .thumb(|style| style.background_image("ui/slider-thumb.png").background_cover())
```

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
    .background("#2563eb")
    .absolute_position(24.0, 24.0),
);

let zone = DropZone::mount(
  ctx,
  DropZoneProps::new().on_drop(|event| {
    println!("source {:?} dropped on {:?}", event.source_id, event.target_id);
  }),
  Rect::new(160.0, 100.0)
    .background("#16a34a33")
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
