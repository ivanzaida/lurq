# Components

## Overview

Components are structs that implement the `Component` trait. They hold persistent state and produce a `Node` tree on each render cycle.

```rust
use lurq::app::component::Component;
use lurq::app::ctx::Ctx;
use lurq::core::Signal;
use lurq::node::Node;
use lurq::node::dsl::*;

struct Counter {
  count: Signal<i32>,
}

impl Component for Counter {
  type Props = ();

  fn create(ctx: &mut Ctx, _: ()) -> Self {
    Self { count: ctx.signal(0) }
  }

  fn render(&self, ctx: &mut Ctx) -> Node {
    let count = self.count.clone();
    column()
      .spacing(8.0)
      .child(text(&format!("Count: {}", self.count.get())))
      .child(
        rect(120.0, 40.0)
          .fill("#3b82f6")
          .on_click(move |_| count.update(|n| *n += 1))
      )
  }
}
```

## Component Trait

```rust
pub trait Component: Send + Sync + 'static {
  type Props: Send + 'static;
  fn create(ctx: &mut Ctx, props: Self::Props) -> Self;
  fn render(&self, ctx: &mut Ctx) -> Node;
  fn on_mounted(&self) {}
  fn on_unmounted(&self) {}
}
```

| Method | Called | Purpose |
|--------|--------|---------|
| `create` | Once, on first mount | Initialize state, create signals |
| `render` | Every dirty cycle | Build the node tree |
| `on_mounted` | After first render | Setup (timers, subscriptions) |
| `on_unmounted` | When component is dropped | Cleanup |

## Props

Components receive props through the `Props` associated type. Props are passed when mounting.

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

  fn render(&self, _ctx: &mut Ctx) -> Node {
    text(&format!("Hello, {}!", self.name))
  }
}
```

Use `()` for components with no props.

## Mounting Children

Mount child components inside `render` using `Ctx`:

```rust
fn render(&self, ctx: &mut Ctx) -> Node {
  column()
    .spacing(16.0)
    .child(ctx.mount::<Header>(HeaderProps { title: "App" }))
    .child(ctx.mount::<Counter>(()))
    .child(ctx.mount::<Footer>(()))
}
```

### Unkeyed — `ctx.mount::<C>(props)`

Children are matched by position and type. If the same component type appears at the same position on re-render, the existing instance is reused (no `create`, just `render`).

### Keyed — `ctx.mount_keyed::<C>(key, props)`

Children are matched by key and type. Use for lists where items can reorder.

```rust
for item in &self.items.get() {
  ctx.mount_keyed::<TodoItem>(&item.id, item.clone());
}
```

### Lifecycle on Mount/Unmount

- New child at a position → `create` + `render` + `on_mounted`
- Same child at same position → `render` only
- Child removed (position no longer rendered) → `on_unmounted` + drop

## State

### Signal — Reactive Value

```rust
let count = ctx.signal(0);

count.get()                    // read (tracked)
count.get_untracked()          // read (not tracked)
count.set(42)                  // replace
count.update(|n| *n += 1)     // mutate
count.with(|n| format!("{n}")) // read by reference (tracked)
```

Writing to a signal marks the owning component dirty. On the next frame, `render` is called again.

### Memo — Derived Value

```rust
let count = ctx.signal(0);
let doubled = ctx.memo({
  let count = count.clone();
  move || count.get() * 2
});

doubled.get() // auto-recomputes when count changes
```

Memo tracks which signals are read during computation. It only propagates to dependents when its value actually changes.

### Ref — Non-Reactive Persistent Value

```rust
let timer_handle = ctx.create_ref::<Option<u64>>(None);

timer_handle.set(Some(123));
timer_handle.get()             // does NOT trigger re-render
```

Use for values that need to persist across renders but shouldn't cause re-renders.

## Effects

### Auto-Tracked Effect

```rust
let count = ctx.signal(0);
let name = ctx.signal("world".to_string());

ctx.on_effect({
  let count = count.clone();
  let name = name.clone();
  move || {
    println!("{}: {}", name.get(), count.get());
  }
});
```

Runs immediately. Re-runs whenever any signal read inside it changes.

### Explicit Watcher

```rust
ctx.watch(&count, |val| {
  println!("count changed to {val}");
});
```

Fires the callback whenever the watched signal changes.

## Dirty Tracking

Each component has a dirty flag (`Arc<AtomicBool>`). The cycle:

1. Signal write → dirty flag set
2. Runtime checks `any_dirty()` on the tree
3. Dirty components get `render` called
4. Dirty flag cleared after render

Only dirty subtrees re-render. Parent re-render does not force child re-render unless the child's own signals changed.

## Lifecycle

```rust
impl Component for MyComponent {
  // ...

  fn on_mounted(&self) {
    println!("component mounted");
  }

  fn on_unmounted(&self) {
    println!("component unmounted — cleanup here");
  }
}
```

Both are default trait methods — override only when needed.

## Batch Updates

```rust
use lurq::core::batch;

batch(|| {
  signal_a.set(1);
  signal_b.set(2);
  signal_c.set(3);
  // watchers fire once after the block, not three times
});
```

## Full Example

```rust
use lurq::app::component::Component;
use lurq::app::ctx::Ctx;
use lurq::core::Signal;
use lurq::layout::Alignment;
use lurq::node::Node;
use lurq::node::color::Color;
use lurq::node::dsl::*;

struct App {
  items: Signal<Vec<String>>,
  input: Signal<String>,
}

impl Component for App {
  type Props = ();

  fn create(ctx: &mut Ctx, _: ()) -> Self {
    Self {
      items: ctx.signal(vec!["First item".into()]),
      input: ctx.signal(String::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> Node {
    let items = self.items.clone();
    let input = self.input.clone();

    let add = {
      let items = items.clone();
      let input = input.clone();
      move |_| {
        let val = input.get();
        if !val.is_empty() {
          items.update(|list| list.push(val.clone()));
          input.set(String::new());
        }
      }
    };

    column()
      .spacing(12.0)
      .align_items(Alignment::Start)
      .child(
        row().spacing(8.0)
          .child(rect(200.0, 36.0).fill("#f1f5f9"))
          .child(
            rect(80.0, 36.0)
              .fill("#3b82f6")
              .on_click(add)
          )
      )
      .child(
        column().spacing(4.0).with_children(
          self.items.get().iter().map(|item| {
            text(item)
          }).collect::<Vec<_>>()
        )
      )
      .pad(24.0)
  }
}
```
