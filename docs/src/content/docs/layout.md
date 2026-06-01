---
title: Layout
description: Constraints, containers, modifiers, alignment, flex, scroll, and text layout.
---

# Layout

## Core Idea

Layout is constraints-based and compositional. Public UI code builds typed components such as `Row`, `Column`, `Text`, and `Rect`, then converts them into the erased `Element` type at runtime/component boundaries. Internally, each `Element` wraps a crate-private node tree made of containers, leaves, and modifier nodes.

A plain empty component has no intrinsic size. Size, padding, alignment, offsets, visuals, and scroll behavior are added by wrapping the component in modifiers.

## Modifiers

Modifiers are chainable and wrap the current element.

```rust
lurq::components::Rect::new(80.0, 40.0)
  .padding(12.0)
  .fill("#3b82f6")
  .rounded(8.0)
```

Common modifiers:

| Modifier | Purpose |
|----------|---------|
| `.size(width, height)` | Force width and height |
| `.width(width)` | Force width |
| `.height(height)` | Force height |
| `.padding(...)` / `.padding_horizontal(...)` / `.padding_vertical(...)` | Add insets around the child |
| `.fill(color)` | Fill the element background |
| `.rounded(radius)` | Set border radius |
| `.border_inside(width, color)` | Draw an inside border |
| `.offset(x, y)` | Shift visually without changing parent layout |
| `.relative(x, y)` | Alias for `.offset(x, y)` |
| `.absolute(x, y, width, height)` | Absolute stack positioning with forced size |
| `.absolute_position(x, y)` | Absolute stack positioning with measured size |
| `.align(Alignment)` | Override alignment within parent container |
| `.flex(factor)` | Participate in row/column flex distribution |

Sizing modifiers accept `Dimension` values. Passing a plain `f32` is shorthand for `Dimension::Px(value)`.

```rust
use lurq::node::dimension::Dimension;

lurq::components::Spacer::new().width(120.0)
lurq::components::Spacer::new().width(Dimension::Pct(50.0))
lurq::components::Spacer::new().width(Dimension::Auto)
```

## Constraints Model

Layout follows the same high-level model as Flutter:

1. Parent passes `Constraints` down to each child.
2. Child picks a concrete `Size` within those constraints.
3. Parent positions each child with an offset.

```rust
pub struct Constraints {
  pub min_width: f32,
  pub max_width: f32,
  pub min_height: f32,
  pub max_height: f32,
}
```

Constraint kinds:

- Tight: `min == max`; forces an exact size.
- Loose: `min == 0`; child can choose any size up to max.
- Unbounded: `max == f32::INFINITY`; used by scroll containers on the scroll axis.

Application code normally does not call layout directly. Runtime computes layout when rendering, dispatching input, or looking up elements.

## Containers

### Column

`lurq::components::Column::new()` arranges children top-to-bottom.

```rust
lurq::components::Column::new()
  .spacing(8.0)
  .align_items(Alignment::Center)
  .child(lurq::components::Text::new("A"))
  .child(lurq::components::Text::new("B"))
```

Column layout:

1. Lays out non-flex children with loosened vertical constraints.
2. Sums child heights plus spacing.
3. Uses the max child width.
4. Positions children vertically and applies cross-axis alignment.
5. Distributes remaining height to flex children when present.

### Row

`lurq::components::Row::new()` arranges children left-to-right.

```rust
lurq::components::Row::new()
  .spacing(8.0)
  .align_items(Alignment::Center)
  .child(lurq::components::Text::new("A"))
  .child(lurq::components::Text::new("B"))
```

Row layout is the horizontal equivalent of column layout.

### Stack

`lurq::components::Stack::new()` overlays children. Later children paint on top of earlier children.

```rust
lurq::components::Stack::new()
  .stack_align(StackAlignment::Center)
  .child(lurq::components::Rect::new(200.0, 120.0))
  .child(lurq::components::Rect::new(40.0, 40.0).fill("#ef4444"))
```

Stack layout:

1. Lays out all children with the same constraints.
2. Sizes itself to the max width/height of non-absolute children.
3. Positions normal children using stack alignment or per-child `.align(...)`.
4. Positions absolute children at their explicit `(x, y)` offset.

Absolute children do not contribute to stack size.

## Relative Positioning

```rust
lurq::components::Rect::new(50.0, 50.0).relative(10.0, 20.0)
```

Relative positioning is an offset. It moves the child visually but the parent still reserves space as if the offset were zero. Siblings are not moved by the offset.

## Absolute Positioning

Absolute positioning is intentionally scoped to `Stack`.

```rust
lurq::components::Stack::new()
  .child(lurq::components::Rect::new(300.0, 120.0).fill("#f8fafc"))
  .child(
    lurq::components::Rect::new(80.0, 32.0)
      .fill("#f97316")
      .absolute(190.0, 24.0, 80.0, 32.0),
  )
```

Use:

- `.absolute(x, y, width, height)` when the positioned child should have a forced size.
- `.absolute_position(x, y)` when the positioned child should keep its measured size.

There is no z-index. Rendering order is structural: later stack children paint above earlier children.

## Alignment

### Row And Column Alignment

```rust
lurq::components::Column::new()
  .align_items(Alignment::Center)
  .child(lurq::components::Text::new("centered"))
```

```rust
pub enum Alignment {
  Start,
  Center,
  End,
  Stretch,
}
```

### Per-Child Override

```rust
lurq::components::Column::new()
  .align_items(Alignment::Start)
  .child(lurq::components::Text::new("left"))
  .child(lurq::components::Text::new("right").align(Alignment::End))
```

### Stack Alignment

```rust
lurq::components::Stack::new()
  .stack_align(StackAlignment::BottomEnd)
  .child(lurq::components::Rect::new(40.0, 40.0))
```

```rust
pub enum StackAlignment {
  TopStart,
  TopCenter,
  TopEnd,
  CenterStart,
  Center,
  CenterEnd,
  BottomStart,
  BottomCenter,
  BottomEnd,
}
```

## Flex

Children inside `Row` or `Column` can consume remaining space with `.flex(factor)`.

```rust
lurq::components::Row::new()
  .child(lurq::components::Rect::new(100.0, 50.0))
  .child(lurq::components::Spacer::new().flex(1.0))
  .child(lurq::components::Rect::new(100.0, 50.0))
```

Flex layout:

1. Lay out non-flex children first.
2. Subtract fixed child sizes and spacing from available space.
3. Divide remaining space by flex factor.
4. Lay out flex children with tight constraints for their assigned size.

## Scroll

```rust
lurq::components::ScrollVertical::new(
  lurq::components::Column::new()
    .spacing(4.0)
    .with_children(items),
)
.size(300.0, 180.0)
```

Scroll containers give their child unbounded constraints on the scroll axis and apply scroll offsets during layout/rendering.

## Text

Text is measured by the glyph engine and wraps within its width constraint.

```rust
lurq::components::Text::styled("hello", TextStyle {
  font_size: 18.0,
  ..TextStyle::default()
})
```
