---
title: Typed Component API
description: Public typed builders for layout, visuals, events, drag and drop, and components.
---

# Typed Component API

## Import

```rust
use lurq::{
  components::{
    Checkbox, Column, DragContainer, DragContainerProps, Draggable, DraggableProps, DropZone, DropZoneProps, Rect, Row,
    Slider, Spacer, Stack, Text, TextInput,
  },
  layout::{Alignment, StackAlignment},
  node::{CheckboxStyle, Element, TextTransformMode, color::Color},
};
```

Typed components are the public UI builders. `Element` is the erased return/transport type used by the runtime and component system. The internal `Node` type is crate-private.

## Constructors

| Constructor | Description |
|-------------|-------------|
| `Row::new()` | Horizontal container |
| `Column::new()` | Vertical container |
| `Stack::new()` | Overlay container; later children paint on top |
| `Text::new("content")` | Text with default style |
| `Text::styled("content", style)` | Text with custom `TextStyle` |
| `TextInput::new(signal)` | Controlled editable text input |
| `Checkbox::new(signal)` | Controlled boolean checkbox |
| `Slider::new(signal)` | Controlled integer slider |
| `Rect::new(width, height)` | Fixed-size rectangle leaf |
| `Spacer::new()` | Empty leaf, often used with `.flex(1.0)` |
| `ScrollVertical::new(child)` | Vertical scroll container |
| `ScrollHorizontal::new(child)` | Horizontal scroll container |
| `ScrollBoth::new(child)` | Two-axis scroll container |
| `DragContainer::mount(ctx, DragContainerProps::new(), child)` | Drag surface that bounds descendant draggables by default |
| `Draggable::mount(ctx, DraggableProps::new(), child)` | Blank component wrapper that makes its child draggable |
| `DropZone::mount(ctx, DropZoneProps::new(), child)` | Blank component wrapper that makes its child a drop target |

## Children

```rust
lurq::components::Column::new()
  .child(lurq::components::Text::new("one"))
  .child(lurq::components::Text::new("two"))
  .child(lurq::components::Text::new("three"))
```

```rust
let items = vec![lurq::components::Text::new("a"), lurq::components::Text::new("b"), lurq::components::Text::new("c")];
lurq::components::Column::new().with_children(items)
```

## Containers

```rust
lurq::components::Row::new()
  .spacing(12.0)
  .align_items(Alignment::Center)
  .child(lurq::components::Rect::new(40.0, 40.0))
```

```rust
lurq::components::Stack::new()
  .stack_align(StackAlignment::BottomEnd)
  .child(lurq::components::Rect::new(200.0, 120.0))
```

## Sizing

```rust
lurq::components::Spacer::new().size(200.0, 100.0)
lurq::components::Spacer::new().width(200.0)
lurq::components::Spacer::new().height(100.0)
lurq::components::Rect::new(80.0, 80.0)
```

Plain `f32` values are pixel shorthand. Use `Dimension` directly for non-pixel sizing:

```rust
use lurq::node::dimension::Dimension;

lurq::components::Spacer::new().width(Dimension::Pct(50.0))
lurq::components::Spacer::new().height(Dimension::Auto)
```

## Visuals

```rust
lurq::components::Rect::new(100.0, 50.0)
  .background("#3b82f6")
  .rounded(8.0)
  .border_inside(1.0, Color::from_hex("#1d4ed8"))
```

```rust
use lurq::node::CursorIcon;

lurq::components::Rect::new(100.0, 50.0).cursor(CursorIcon::Pointer)
lurq::components::Rect::new(100.0, 50.0).hovered(|style| style.cursor(CursorIcon::Text))
```

## Padding

```rust
lurq::components::Column::new().padding(16.0)
lurq::components::Column::new().padding_horizontal(16.0).padding_vertical(8.0)
lurq::components::Column::new().padding_left(10.0)
lurq::components::Column::new().padding_right(10.0)
lurq::components::Column::new().padding_top(10.0)
lurq::components::Column::new().padding_bottom(10.0)
```

## Flex

```rust
lurq::components::Row::new()
  .child(lurq::components::Rect::new(100.0, 50.0))
  .child(lurq::components::Spacer::new().flex(1.0))
  .child(lurq::components::Rect::new(100.0, 50.0))
```

Flex applies inside `Row` and `Column`.

## Relative Positioning

```rust
lurq::components::Rect::new(50.0, 50.0).relative(10.0, 20.0)
```

`relative(x, y)` is an alias for `offset(x, y)`. It shifts the element visually without changing the space it takes in parent layout.

## Absolute Positioning

Absolute positioning is supported in `Stack`.

```rust
lurq::components::Stack::new()
  .child(
    lurq::components::Rect::new(300.0, 120.0)
      .background("#f8fafc")
      .rounded(12.0),
  )
  .child(
    lurq::components::Rect::new(86.0, 34.0)
      .background("#f97316")
      .rounded(8.0)
      .absolute(190.0, 24.0, 86.0, 34.0),
  )
  .child(
    lurq::components::Text::new("absolute")
      .absolute_position(201.0, 31.0),
  )
```

Rules:

- `absolute(x, y, width, height)` positions the child at `(x, y)` inside the stack and forces its size.
- `absolute_position(x, y)` positions the child but lets it keep its measured size.
- Absolute children do not affect stack size.
- There is no z-index API. Rendering follows child order; later stack children paint on top.

## Alignment Override

```rust
lurq::components::Column::new()
  .align_items(Alignment::Start)
  .child(lurq::components::Text::new("left"))
  .child(lurq::components::Text::new("right").align(Alignment::End))
```

## Events

```rust
lurq::components::Rect::new(100.0, 40.0)
  .background("#3b82f6")
  .on_click(|e| println!("clicked at {}, {}", e.x, e.y))
  .on_drag_start(|e| println!("drag started at {}, {}", e.x, e.y))
  .on_drag_move(|e| println!("drag delta: {}, {}", e.delta_x, e.delta_y))
  .on_drag_end(|e| println!("drag ended at {}, {}", e.x, e.y))
  .on_mouse_enter(|| println!("hover in"))
  .on_mouse_leave(|| println!("hover out"))
  .on_key_down(|e| println!("key: {}", e.key))
```

## Drag And Drop

`DragContainer`, `Draggable`, and `DropZone` are components with explicit one-child `mount` helpers. Each requires exactly one child. `Draggable` and `DropZone` are blank wrappers. `DragContainer` uses its child as the drag surface and applies container policy; `DragContainerProps::new()` bounds descendant draggables to that surface.

```rust
fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
  let drop_zone = lurq::components::DropZone::mount(
    ctx,
    lurq::components::DropZoneProps::new().on_drop(|event| {
      println!("dropped on {:?}", event.target_id);
    }),
    lurq::components::Rect::new(120.0, 80.0)
      .background("#22c55e33")
      .absolute_position(200.0, 110.0),
  );

  let card = lurq::components::Draggable::mount(
    ctx,
    lurq::components::DraggableProps::new().on_drag_move(|event| {
      println!("move by {}, {}", event.delta_x, event.delta_y);
    }),
    lurq::components::Rect::new(64.0, 64.0)
      .background("#3b82f6")
      .absolute_position(24.0, 24.0),
  );

  lurq::components::DragContainer::mount(
    ctx,
    lurq::components::DragContainerProps::new(),
    lurq::components::Stack::new()
      .size(360.0, 220.0)
      .child(drop_zone)
      .child(card),
  )
}
```

## Text Styling

Theme palettes, spacing, radii, and typography use cheap IDs that map to theme token values:

```rust
use lurq::node::{color::Color, dimension::Dimension};

let brand_id = app.theme().register_palette_color(Color::from_hex("#2563eb"));

let gap_id = app.theme().register_spacing(Dimension::Px(8.0));

let card_radius_id = app.theme().register_radius(6.0);

lurq::components::Rect::new(120.0, 40.0)
  .background(brand_id)
  .padding(gap_id)
  .rounded(card_radius_id);

lurq::components::Row::new()
  .spacing(gap_id);
```

Plain text resolves its style from the active theme. `Text::new` uses the theme default text style, and `variant` selects any user-defined typography entry from the theme.

```rust
use lurq::layout::text_style::{FontWeight, TextStyle};

app.theme().set_default_text_style(TextStyle {
  font_size: 16.0,
  ..TextStyle::default()
});

let display_id = app.theme().register_typography_style(
  TextStyle {
    font_size: 32.0,
    weight: FontWeight::Bold,
    ..TextStyle::default()
  },
);

lurq::components::Text::new("Headline")
  .variant(display_id)
```

Missing variants fall back to the default text style. Generated IDs are the default path for custom theme entries; manual IDs are available through `PaletteId::new(...)`, `SpacingId::new(...)`, `RadiusId::new(...)`, and `TypographyId::new(...)` when stable numeric IDs are needed. Lurq does not define or reserve built-in theme token IDs.

Use `Text::styled` for a one-off style that should ignore theme typography:

```rust
use lurq::{
  layout::text_style::{FontStyle, FontWeight, TextStyle},
  node::color::Color,
};

lurq::components::Text::styled("Bold title", TextStyle {
  font_size: 24.0,
  weight: FontWeight::Bold,
  style: FontStyle::Normal,
  color: Color::from_hex("#1e293b"),
  ..TextStyle::default()
})
```

Make plain text selectable when users need copyable or inspectable text:

```rust
lurq::components::Text::new("Selectable text")
  .selectable(true)
```

Selectable text supports drag selection, double-click word selection, and triple-click line selection. `TextInput` has the same pointer selection gestures plus caret movement, undo/redo, and signal-backed edits.

### Transformed Text

Text uses `TextTransformMode::Bitmap` by default. In this mode the glyphs are rasterized normally, then transformed during rendering. This keeps glyph placement in float screen space and is the right default for animated transforms because it does not create a new glyph atlas entry for every angle.

Use `TextTransformMode::Rasterized` for static transformed text when the rotated glyph edges need to stay sharp. This mode bakes the transform into glyph rasterization, disables transform-time hinting for the rotated mask, and emits identity-transform glyph quads. Because the transform is part of the glyph atlas cache key, animated rotations should usually stay on `Bitmap`.

```rust
lurq::components::Text::new("Static rotated label")
  .text_transform_mode(TextTransformMode::Rasterized)
  .transform(lurq::node::transform::Transform2D::rotate_deg(-8.0))
```

## Components

```rust
use lurq::{
  app::{component::Component, ctx::Ctx},
  core::Signal,
  node::Element,
};

struct Counter {
  count: Signal<i32>,
}

impl Component for Counter {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self { count: ctx.signal(0) }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    let count = self.count.clone();
    lurq::components::Column::new()
      .spacing(8.0)
      .child(lurq::components::Text::new(&format!("Count: {}", self.count.get())))
      .child(
        lurq::components::Text::new("Increment")
          .on_click(move |_| count.update(|n| *n += 1)),
      )
  }
}

fn render_parent(ctx: &mut Ctx) -> Element {
  lurq::components::Column::new().child(ctx.mount::<Counter>(())).into()
}
```
