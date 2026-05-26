# Components

## Overview

Components are structs that implement `Component`. They hold persistent state and return an `Element` tree from `render`.

See [`ctx.md`](ctx.md) for the full `Ctx` API used inside `create` and `render`.

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

  fn create(ctx: &mut Ctx, _: ()) -> Self {
    Self { count: ctx.signal(0) }
  }

  fn render(&self, _ctx: &mut Ctx) -> Element {
    let dec = self.count.clone();
    let inc = self.count.clone();
    let value = self.count.get();

    Element::row()
      .spacing(12.0)
      .align_items(Alignment::Center)
      .child(
        Element::rect(36.0, 36.0)
          .fill("#ef4444")
          .rounded(6.0)
          .on_click(move |_| dec.update(|n| *n -= 1)),
      )
      .child(Element::styled_text(
        &format!("{value}"),
        TextStyle {
          font_size: 24.0,
          weight: FontWeight::Bold,
          color: Color::from_hex("#1e293b"),
          ..TextStyle::default()
        },
      ))
      .child(
        Element::rect(36.0, 36.0)
          .fill("#22c55e")
          .rounded(6.0)
          .on_click(move |_| inc.update(|n| *n += 1)),
      )
  }
}
```

## Component Trait

```rust
pub trait Component: Send + Sync + 'static {
  type Props: Send + 'static;

  fn create(ctx: &mut Ctx, props: Self::Props) -> Self;
  fn render(&self, ctx: &mut Ctx) -> Element;

  fn on_mounted(&self) {}
  fn on_unmounted(&self) {}
}
```

| Method | Called | Purpose |
|--------|--------|---------|
| `create` | Once, when the component is mounted | Initialize persistent state |
| `render` | On mount and when the component is dirty | Return the current element tree |
| `on_mounted` | After first render | Setup hooks that need a mounted component |
| `on_unmounted` | Before the component is removed | Cleanup |

## Props

Components receive props through the `Props` associated type.

```rust
struct Greeting {
  name: String,
}

struct GreetingProps {
  name: String,
}

impl Component for Greeting {
  type Props = GreetingProps;

  fn create(_ctx: &mut Ctx, props: GreetingProps) -> Self {
    Self { name: props.name }
  }

  fn render(&self, _ctx: &mut Ctx) -> Element {
    Element::text(&format!("Hello, {}!", self.name))
  }
}
```

Use `()` for components with no props.

## Mounting Children

Mount child components inside `render` with `Ctx`.

```rust
fn render(&self, ctx: &mut Ctx) -> Element {
  Element::column()
    .spacing(16.0)
    .child(ctx.mount::<Header>(HeaderProps { title: "App" }))
    .child(ctx.mount::<Counter>(()))
    .child(ctx.mount::<Footer>(()))
}
```

### Unkeyed Mounts

`ctx.mount::<C>(props)` matches children by position and component type. If the same component type stays at the same slot, its instance is reused.

### Keyed Mounts

`ctx.mount_keyed::<C>(key, props)` matches children by key and type. Use keyed mounts for lists that can reorder.

```rust
Element::column().with_children(
  self.items.get().iter().map(|item| {
    ctx.mount_keyed::<TodoItem>(&item.id, item.clone())
  })
)
```

### Slot Children

Use `mount_with` or `mount_keyed_with` when a component needs children supplied by its parent.

```rust
ctx.mount_with::<Panel>(PanelProps { title: "Tools" }, vec![
  Element::text("content"),
])
```

## State

### Signal

```rust
let count = ctx.signal(0);

count.get();                    // tracked read
count.get_untracked();          // untracked read
count.set(42);                  // replace
count.update(|n| *n += 1);      // mutate in place
count.with(|n| format!("{n}")); // tracked borrow
```

Writing to a signal marks the owning component dirty. Runtime rebuilds dirty component output before layout, rendering, event dispatch, and element lookup.

### Memo

```rust
let count = ctx.signal(0);
let doubled = ctx.memo({
  let count = count.clone();
  move || count.get() * 2
});

let value = doubled.get();
```

A memo tracks signals read during computation and updates dependents only when its value changes.

### Ref

```rust
let handle = ctx.create_ref::<Option<u64>>(None);

handle.set(Some(123));
let current = handle.get();
```

Refs persist across renders but are not reactive.

### Store

Use stores and lenses for structured reactive state.

```rust
let user = ctx.store(User { name: "Ada".into(), age: 36 });
let name = user.lens(
  |u| u.name.clone(),
  |u, name| u.name = name,
);
name.set("Grace".into());
```

## Effects And Watchers

```rust
let count = ctx.signal(0);

ctx.on_effect({
  let count = count.clone();
  move || println!("count = {}", count.get())
});
```

Effects run immediately and rerun when any tracked signal read inside the effect changes.

```rust
ctx.watch(&count, |value| {
  println!("count changed to {value}");
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

  fn create(_ctx: &mut Ctx, _: ()) -> Self {
    Self
  }

  fn render(&self, _ctx: &mut Ctx) -> Element {
    Element::text("mounted")
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
use lurq::core::batch;

batch(|| {
  signal_a.set(1);
  signal_b.set(2);
  signal_c.set(3);
});
```

Batching coalesces dirty propagation and watcher notifications until the batch ends.
