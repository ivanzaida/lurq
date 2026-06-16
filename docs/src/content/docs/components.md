---
title: Components
description: Component structure, props, mounting, state, effects, and lifecycle.
---

# Components

## Overview

Components are structs that implement `Component`. They hold persistent state and return an `Element` tree from
`render`.

See [Ctx](./ctx/) for the full `Ctx` API used inside `create` and `render`, and [Reactivity](./reactivity/) for signals,
stores, memos, effects, and contexts.

```rust
use lurq::{
  app::{component::Component, ctx::Ctx},
  core::Signal,
  layout::{Alignment, text_style::{FontWeight, TextStyle}},
  node::{Element, color::Color},
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
    let dec = self.count.clone();
    let inc = self.count.clone();
    let value = self.count.get();

    lurq::components::Row::new()
      .spacing(12.0)
      .align_items(Alignment::Center)
      .child(
        lurq::components::Rect::new(36.0, 36.0)
          .background("#ef4444")
          .rounded(6.0)
          .on_click(move |_| dec.update(|n| *n -= 1)),
      )
      .child(lurq::components::Text::styled(
        &format!("{value}"),
        TextStyle {
          font_size: 24.0,
          weight: FontWeight::Bold,
          color: Color::from_hex("#1e293b"),
          ..TextStyle::default()
        },
      ))
      .child(
        lurq::components::Rect::new(36.0, 36.0)
          .background("#22c55e")
          .rounded(6.0)
          .on_click(move |_| inc.update(|n| *n += 1)),
      )
  }
}
```

## Component Trait

```rust
pub trait Component: Send + Sync + 'static {
  type Props: Send + PartialEq + 'static;

  fn create(ctx: &mut Ctx) -> Self;
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element>;

  fn on_mounted(&self) {}
  fn on_unmounted(&self) {}
}
```

| Method         | Called                                   | Purpose                                   |
|----------------|------------------------------------------|-------------------------------------------|
| `create`       | Once, when the component is mounted      | Initialize persistent state               |
| `render`       | On mount and when the component is dirty | Return the current element tree           |
| `on_mounted`   | After first render                       | Setup hooks that need a mounted component |
| `on_unmounted` | Before the component is removed          | Cleanup                                   |

## Props

Components receive props through the `Props` associated type. The current props are stored on the component context and
can be read with `ctx.props::<Self::Props>()`.

```rust
struct Greeting {
  name: String,
}

#[derive(Clone, PartialEq)]
struct GreetingProps {
  name: String,
}

impl Component for Greeting {
  type Props = GreetingProps;

  fn create(ctx: &mut Ctx) -> Self {
    let props = ctx.props::<Self::Props>();
    Self { name: props.name.clone() }
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Text::new(&format!("Hello, {}!", self.name))
  }
}
```

Use `()` for components with no props. Reused components rerender when their props compare unequal, so custom props must
implement `PartialEq`.

When the `devtools` feature is enabled, props must also implement `DevtoolsInspectable`.

```rust
#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct GreetingProps {
  name: String,
}
```

## Mounting Children

Mount child components inside `render` with `Ctx`.

```rust
fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
  lurq::components::Column::new()
    .spacing(16.0)
    .child(ctx.mount::<Header>(HeaderProps { title: "App" }))
    .child(ctx.mount::<Counter>(()))
    .child(ctx.mount::<Footer>(()))
}
```

### Unkeyed Mounts

`ctx.mount::<C>(props)` matches children by position and component type. If the same component type stays at the same
slot, its instance is reused. The child rerenders when its props change or its own context is dirty.

### Keyed Mounts

`ctx.mount_keyed::<C>(key, props)` matches children by key and type. Use keyed mounts for lists that can reorder.

```rust
lurq::components::Column::new().with_children(
self .items.get().iter().map( | item| {
ctx.mount_keyed::< TodoItem > ( & item.id, item.clone())
})
)
```

### Slot Children

Use `mount_with` or `mount_keyed_with` when a component needs children supplied by its parent.

```rust
ctx.mount_with::<Panel>(PanelProps { title: "Tools" }, vec![
  lurq::components::Text::new("content"),
])
```

## Built-In DnD Components

`DragContainer`, `Draggable`, and `DropZone` are real components. Use their `mount` helpers for the explicit one-child
API.

`Draggable` and `DropZone` are blank behavior wrappers. Each requires exactly one slot child and leaves layout, sizing,
and initial positioning to that child.

`DragContainer` requires exactly one slot child as the drag surface. By default, `DragContainerProps::new()` bounds
descendant draggables to that surface.

```rust
use lurq::components::{DragContainer, DragContainerProps, Draggable, DraggableProps, Rect, Stack};

fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
  let card = Draggable::mount(
    ctx,
    DraggableProps::new().on_drag_move(|event| {
      println!("drag delta: {}, {}", event.delta_x, event.delta_y);
    }),
    Rect::new(64.0, 64.0)
      .background("#3b82f6")
      .absolute_position(24.0, 24.0),
  );

  DragContainer::mount(
    ctx,
    DragContainerProps::new(),
    Stack::new()
      .size(360.0, 220.0)
      .child(card),
  )
}
```

Use `DragContainerProps::new().bounds(DragBounds::None)` for an unbounded drag surface.

`DropZone` marks its single child as a drop target. Visual styling is supplied by that child.

```rust
use lurq::components::{DropZone, DropZoneProps, Rect};

fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
  DropZone::mount(
    ctx,
    DropZoneProps::new().on_drop(|event| {
      println!("dropped from {:?} onto {:?}", event.source_id, event.target_id);
    }),
    Rect::new(140.0, 80.0)
      .background("#22c55e33")
      .border_inside(1.0, lurq::node::color::Color::from_hex("#22c55e")),
  )
}
```

Use `DraggableProps::on_drag_start`, `on_drag_move`, and `on_drag_end` for high-level draggable callbacks. Low-level
node handlers with the same names remain available for custom behavior. The runtime keeps the active drag captured
across rerenders and dispatches `on_drop` to the hit `DropZone` on release.

## State

### Signal

```rust
let count = ctx.signal(0);

count.get();                    // tracked read
count.get_untracked();          // untracked read
count.set(42);                  // replace
count.update( | n| * n += 1);      // mutate in place
count.with( | n| format!("{n}")); // tracked borrow
```

Writing to a signal marks the owning component dirty. Runtime rebuilds dirty component output before layout, rendering,
event dispatch, and element lookup.

### Memo

```rust
let count = ctx.signal(0);
let doubled = ctx.memo({
let count = count.clone();
move | | count.get() * 2
});

let value = doubled.get();
```

A memo tracks signals read during computation and updates dependents only when its value changes.

### Ref

```rust
let handle = ctx.create_ref::<Option<u64> > (None);

handle.set(Some(123));
let current = handle.get();
```

Refs persist across renders but are not reactive.

### Store

Use stores and lenses for structured reactive state.

```rust
let user = ctx.store(User { name: "Ada".into(), age: 36 });
let name = user.lens(
| u| u.name.clone(),
| u, name| u.name = name,
);
name.set("Grace".into());
```

## Effects And Watchers

```rust
let count = ctx.signal(0);

ctx.on_effect({
let count = count.clone();
move | | println ! ("count = {}", count.get())
});
```

Effects run immediately and rerun when any tracked signal read inside the effect changes.

```rust
ctx.watch( & count, | value| {
println ! ("count changed to {value}");
});
```

Watchers run when the watched signal changes.

## Dirty Tracking

The high-level cycle is:

1. Component reads reactive state during `render`.
2. A signal/store/memo update marks the component dirty.
3. Runtime detects dirty components before work that needs a fresh tree.
4. Dirty component subtrees are rendered again.
5. Layout/render/event lookup use the updated internal tree.

Parent components do not force child components to recreate if the child slot still matches.

## Lifecycle

```rust
impl Component for MyComponent {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
    lurq::components::Text::new("mounted")
  }

  fn on_mounted(&self) {
    println!("mounted");
  }

  fn on_unmounted(&self) {
    println!("unmounted");
  }
}
```

## Batch Updates

```rust
ctx.batch(| | {
signal_a.set(1);
signal_b.set(2);
signal_c.set(3);
});
```

Batching coalesces context dirty propagation until the batch ends.
