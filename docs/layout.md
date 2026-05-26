# Layout

## Core Idea

Layout is constraints-based and compositional. Public UI code builds `Element` values. Internally, each `Element` wraps a crate-private node tree made of containers, leaves, and modifier nodes.

A plain empty element has no intrinsic size. Size, padding, alignment, offsets, visuals, and scroll behavior are added by wrapping the element in modifiers.

## Modifiers

Modifiers are chainable and wrap the current element.

```rust
Element::rect(80.0, 40.0)
  .pad(12.0)
  .fill("#3b82f6")
  .rounded(8.0)
```

Common modifiers:

| Modifier | Purpose |
|----------|---------|
| `.size(width, height)` | Force width and height |
| `.width(width)` | Force width |
| `.height(height)` | Force height |
| `.pad(...)` / `.pad_xy(...)` | Add insets around the child |
| `.fill(color)` | Fill the element background |
| `.rounded(radius)` | Set border radius |
| `.border_inside(width, color)` | Draw an inside border |
| `.offset(x, y)` | Shift visually without changing parent layout |
| `.relative(x, y)` | Alias for `.offset(x, y)` |
| `.absolute(x, y, width, height)` | Absolute stack positioning with forced size |
| `.absolute_position(x, y)` | Absolute stack positioning with measured size |
| `.align(Alignment)` | Override alignment within parent container |
| `.flex(factor)` | Participate in row/column flex distribution |

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

`Element::column()` arranges children top-to-bottom.

```rust
Element::column()
  .spacing(8.0)
  .align_items(Alignment::Center)
  .child(Element::text("A"))
  .child(Element::text("B"))
```

Column layout:

1. Lays out non-flex children with loosened vertical constraints.
2. Sums child heights plus spacing.
3. Uses the max child width.
4. Positions children vertically and applies cross-axis alignment.
5. Distributes remaining height to flex children when present.

### Row

`Element::row()` arranges children left-to-right.

```rust
Element::row()
  .spacing(8.0)
  .align_items(Alignment::Center)
  .child(Element::text("A"))
  .child(Element::text("B"))
```

Row layout is the horizontal equivalent of column layout.

### Stack

`Element::stack()` overlays children. Later children paint on top of earlier children.

```rust
Element::stack()
  .stack_align(StackAlignment::Center)
  .child(Element::rect(200.0, 120.0))
  .child(Element::rect(40.0, 40.0).fill("#ef4444"))
```

Stack layout:

1. Lays out all children with the same constraints.
2. Sizes itself to the max width/height of non-absolute children.
3. Positions normal children using stack alignment or per-child `.align(...)`.
4. Positions absolute children at their explicit `(x, y)` offset.

Absolute children do not contribute to stack size.

## Relative Positioning

```rust
Element::rect(50.0, 50.0).relative(10.0, 20.0)
```

Relative positioning is an offset. It moves the child visually but the parent still reserves space as if the offset were zero. Siblings are not moved by the offset.

## Absolute Positioning

Absolute positioning is intentionally scoped to `Stack`.

```rust
Element::stack()
  .child(Element::rect(300.0, 120.0).fill("#f8fafc"))
  .child(
    Element::rect(80.0, 32.0)
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
Element::column()
  .align_items(Alignment::Center)
  .child(Element::text("centered"))
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
Element::column()
  .align_items(Alignment::Start)
  .child(Element::text("left"))
  .child(Element::text("right").align(Alignment::End))
```

### Stack Alignment

```rust
Element::stack()
  .stack_align(StackAlignment::BottomEnd)
  .child(Element::rect(40.0, 40.0))
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
Element::row()
  .child(Element::rect(100.0, 50.0))
  .child(Element::spacer().flex(1.0))
  .child(Element::rect(100.0, 50.0))
```

Flex layout:

1. Lay out non-flex children first.
2. Subtract fixed child sizes and spacing from available space.
3. Divide remaining space by flex factor.
4. Lay out flex children with tight constraints for their assigned size.

## Scroll

```rust
Element::scroll_vertical(
  Element::column()
    .spacing(4.0)
    .with_children(items),
)
.size(300.0, 180.0)
```

Scroll containers give their child unbounded constraints on the scroll axis and apply scroll offsets during layout/rendering.

## Text

Text is measured by the glyph engine and wraps within its width constraint.

```rust
Element::styled_text("hello", TextStyle {
  font_size: 18.0,
  ..TextStyle::default()
})
```
