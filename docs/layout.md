# Layout

## Core Idea

Layout is compositional. A `Node` has no layout properties by default. Layout behavior is added by wrapping a node in modifier nodes. Each modifier does one thing.

## Node

A `Node` is the base unit. It can hold:
- Optional text content
- Optional children
- Optional visual properties (color, border, etc.)

A node with no modifiers has no size, no padding, no alignment. It gets those through wrapping.

## Modifiers

Modifiers wrap a node in a new invisible layout node. They are chainable.

```
Node::new()
  .padding(Padding::all(Px(16.0)))
  .frame(width: Px(200.0), height: Px(100.0))
  .background(Color::from_hex("#ff0000"))
```

Each `.modifier()` call wraps the current node as the child of a new modifier node.

### Available Modifiers

| Modifier | Purpose |
|----------|---------|
| `.padding(Padding)` | Adds insets around the child |
| `.frame(...)` | Sets width, height, min/max constraints |
| `.background(Color)` | Fills the area behind the child |
| `.border(width, color)` | Draws a border around the child |
| `.offset(x, y)` | Shifts the child without affecting layout |
| `.align(Alignment)` | Overrides alignment within parent container |

## Containers

Containers are nodes that arrange children along an axis or in layers. They are just `Node` constructors.

### Column

Arranges children vertically (top to bottom).

```
Node::column(spacing: Px(8.0), align: Center, children)
```

### Row

Arranges children horizontally (left to right).

```
Node::row(spacing: Px(8.0), align: Center, children)
```

### Stack

Layers children on top of each other (z-axis). Last child is on top. Alignment controls how children are positioned within the stack bounds.

```
Node::stack(align: TopLeft, children)
```

This replaces absolute positioning. To place something at a specific spot within a stack, use `.align()` on the child.

## Sizing Model (Flutter-style Constraints)

Layout uses a **constraints-based** model:

1. Parent passes `Constraints` (min_width, max_width, min_height, max_height) down to child
2. Child picks a concrete `Size` (width, height) within those constraints
3. Parent receives the child's chosen size and positions it

### Constraints

```
struct Constraints {
  min_width: f32,
  max_width: f32,
  min_height: f32,
  max_height: f32,
}
```

- A "tight" constraint has min == max (forces exact size)
- A "loose" constraint has min == 0 (child can be anything up to max)
- An "unbounded" constraint has max == f32::INFINITY (e.g. inside a scroll)

### Layout Protocol

Every node implements layout in two steps:

```
fn layout(&self, constraints: Constraints) -> Size
fn position(&self, children_sizes: &[Size]) -> Vec<Offset>
```

- `layout` resolves size given constraints
- `position` determines where each child goes

### How Containers Layout

**Column:**
1. Receives constraints from parent
2. Passes each child a loosened version (unconstrained on the stacking axis)
3. Children report sizes
4. Column sums heights + spacing, uses max child width
5. Positions children top-to-bottom, applying alignment on the cross axis

**Row:** Same but horizontal.

**Stack:**
1. Passes each child the same constraints
2. Takes the max width and max height across all children
3. Positions each child according to stack alignment + per-child `.align()` override

## Alignment

### Container-level

Set on the container, applies to all children as default.

```
Node::column(spacing: Px(8.0), align: AlignItems::Center, children)
```

### Per-child override

A child can override its alignment within the parent.

```
child.align(Alignment::End)
```

### Alignment Values

```
enum Alignment {
  Start,
  Center,
  End,
  Stretch,
}
```

For `Stack`, alignment is 2D:

```
enum StackAlignment {
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

Children inside a `Row` or `Column` can have a flex factor to distribute remaining space.

```
child.flex(1.0)
```

Layout:
1. Lay out non-flex children first, sum their sizes
2. Remaining space = container size - sum - total spacing
3. Distribute remaining space proportionally by flex factor

## Text

Text is a leaf node. It measures its own size based on content and constraints.

```
Node::text("hello")
  .padding(Padding::all(Px(8.0)))
```

Text wraps within the width constraint it receives. Its height grows to fit content.

