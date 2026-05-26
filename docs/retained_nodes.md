# Retained Node Tree

## Overview

The node tree is persistent. Once built, it stays alive. Properties on nodes use `Guard<T>` — reads are free, writes set a dirty flag. The layout/render system checks dirty flags and only recomputes what changed.

## Guard<T>

```rust
let mut color = Guard::new(Color::from_hex("#3b82f6"));

// Read — no overhead
let c: &Color = &*color;

// Write — marks dirty
*color = Color::from_hex("#ef4444");
```

## Node Properties

Node properties that can change at runtime are wrapped in `Guard<T>`:
- `color: Guard<Option<Color>>`
- `border_radius: Guard<Option<BorderRadius>>`
- `border: Guard<Option<Border>>`
- `scrollbar_style: Guard<Option<ScrollBarStyle>>`

Text content uses `Guard<String>` so text updates don't rebuild the node.

## Component Model

Components build the node tree once in `create`. The tree persists.

```rust
struct Counter {
  count: Signal<i32>,
  label: Guard<String>,
}

impl Component for Counter {
  fn create(ctx: &mut Ctx, _: ()) -> Self {
    Self {
      count: ctx.signal(0),
      label: Guard::new("0".into()),
    }
  }

  fn setup(&mut self, ctx: &mut Ctx) -> Node {
    // Build tree once
    let count = self.count.clone();
    column()
      .child(text_guard(&mut self.label))
      .child(rect(36.0, 36.0).fill("#22c55e").on_click(move |_| {
        count.update(|n| *n += 1);
      }))
  }
}
```

When `count` signal fires, the component updates its guards:
```rust
// Effect or watcher updates the guard
ctx.watch(&self.count, |val| {
  self.label.set(format!("{}", val));
});
```

The runtime sees `label.is_changed()` and only re-measures that text node.

## Layout Dirty Tracking

Each `pass()`:
1. Walk the node tree
2. Check `Guard::is_changed()` on each node's properties
3. Only re-layout subtrees with dirty nodes
4. Clear all dirty flags after layout

## No Rebuild

- `pass()` never calls `render()` again
- The node tree from `create` persists
- Signal changes → guard writes → dirty flags → incremental layout
