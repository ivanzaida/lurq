# Ctx

## Overview

`Ctx` is the per-component render context. It is passed to `Component::create` and `Component::render`.

Use it to:

- create reactive state owned by the component
- mount child components
- pass context values down the tree
- read slot children supplied by a parent
- create node refs and interaction state
- register effects, watchers, keyed list slots, and error boundaries

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

  fn create(ctx: &mut Ctx, _: ()) -> Self {
    Self { count: ctx.signal(0) }
  }

  fn render(&self, _ctx: &mut Ctx) -> Element {
    let count = self.count.clone();
    Element::text(&format!("Count: {}", self.count.get()))
      .on_click(move |_| count.update(|n| *n += 1))
  }
}
```

## Dirty State

```rust
ctx.is_dirty();
```

`is_dirty` reports whether this component context is marked dirty. Runtime uses this internally to decide whether a component subtree needs to render again.

Application code usually does not need to call it.

## Manual Root Contexts

```rust
let mut ctx = Ctx::new_root();
```

`new_root` creates a standalone root context. Runtime normally creates and owns root contexts for mounted components, so application code rarely needs this directly. It is useful for tests and low-level component mounting.

Standalone contexts do not have a runtime theme unless one is attached internally by `Runtime`.

## Signals

```rust
let count = ctx.signal(0);

count.get();
count.set(1);
count.update(|n| *n += 1);
```

`ctx.signal(initial)` creates a `Signal<T>` and wires it to the current component. When the signal changes, the component context is marked dirty.

Use `Signal` for state that should trigger a render when it changes.

## Stores And Lenses

```rust
#[derive(Clone)]
struct User {
  name: String,
  age: u32,
}

let user = ctx.store(User { name: "Ada".into(), age: 36 });
let name = user.lens(
  |user| user.name.clone(),
  |user, name| user.name = name,
);

name.set("Grace".into());
```

`ctx.store(initial)` creates structured reactive state. Like signals, store updates mark the owning component dirty.

Use lenses when child code should read or update one field without taking ownership of the whole store value.

## Memos

```rust
let count = ctx.signal(0);
let doubled = ctx.memo({
  let count = count.clone();
  move || count.get() * 2
});

let value = doubled.get();
```

`ctx.memo(f)` creates a derived value. The memo tracks reactive reads inside `f` and recomputes when those dependencies change.

## Refs

```rust
let latest_id = ctx.create_ref::<Option<u64>>(None);
latest_id.set(Some(42));
```

`create_ref` creates persistent non-reactive state. Updating a ref does not mark the component dirty.

Use refs for handles, cached values, counters, or other state that should survive renders but should not cause renders.

## Effects

```rust
let count = ctx.signal(0);

ctx.on_effect({
  let count = count.clone();
  move || println!("count = {}", count.get())
});
```

`on_effect` runs immediately and reruns when any tracked reactive value read inside the effect changes.

Effects are retained by the context, so they live as long as the component context lives.

## Watchers

```rust
let count = ctx.signal(0);

ctx.watch(&count, |value| {
  println!("count changed to {value}");
});
```

`watch` subscribes to a specific signal and keeps the subscription alive for the context lifetime.

Use `watch` when you want an explicit callback for one signal instead of automatic dependency tracking.

## Context Values

### Static Context

```rust
#[derive(Clone)]
struct Locale(String);

ctx.provide(Locale("en-US".into()));
```

Descendants can read the value by type:

```rust
if let Some(locale) = ctx.use_context::<Locale>() {
  println!("locale = {}", locale.0);
}
```

`provide` stores a cloned value by type. `use_context` returns `None` if no ancestor provided that type.

### Reactive Context

```rust
let theme_name = ctx.create_context("light".to_string());
theme_name.set("dark".to_string());
```

Descendants can consume it:

```rust
let theme_name = ctx.consume_context::<String>().unwrap();
let current = theme_name.get();
```

`create_context` stores a `ReactiveContext<T>` and subscribes the creating context to changes. `consume_context` retrieves the reactive context and subscribes the consuming context to changes.

`ReactiveContext<T>` requires `T: Clone + Hash + Send + Sync + 'static` so it can detect meaningful value changes.

## Theme

```rust
let colors = ctx.theme().colors();
let primary = colors.primary;
```

`theme()` returns the current runtime theme. Root and child contexts get the theme from `Runtime`.

Only call `theme()` from a context managed by runtime. A manually-created root context without a theme will panic.

## Slot Children

Parents pass slot children with `mount_with` or `mount_keyed_with`:

```rust
ctx.mount_with::<Panel>(PanelProps { title: "Info" }, vec![
  Element::text("Panel body"),
])
```

The child component reads them through its own context:

```rust
fn render(&self, ctx: &mut Ctx) -> Element {
  let child_count = ctx.children().len();

  Element::column()
    .child(Element::text(&self.title))
    .child(Element::text(&format!("{child_count} slot children")))
}
```

```rust
ctx.has_children();
ctx.children();
```

`children()` returns an empty slice when no slot children were provided.

## Node Refs

```rust
let node_ref = ctx.node_ref();

Element::rect(100.0, 40.0)
  .ref_node(node_ref.clone())
```

After layout, the ref exposes the element rect:

```rust
let (x, y, width, height) = node_ref.rect();
let attached = node_ref.is_attached();
```

Use node refs when code outside normal layout traversal needs an element's measured rect.

## Interaction State

```rust
let state = ctx.interaction();

Element::rect(100.0, 40.0)
  .interactive(state.clone())
  .on_mouse_enter(|| println!("hover"))
```

`InteractionState` tracks runtime interaction flags:

```rust
state.is_hovered();
state.is_active();
state.is_focused();
```

Hover and active are updated by runtime input dispatch. Focus state exists on the type but is not currently updated by the runtime.

## Mounting Child Components

```rust
ctx.mount::<Counter>(());
ctx.mount_keyed::<TodoItem>(todo.id.as_str(), todo.clone());
```

- `mount` matches children by slot position and component type.
- `mount_keyed` matches by slot position, key, and component type.
- Matching children reuse the existing component instance and context.
- Non-matching children are unmounted and replaced.

Use keyed mounts for dynamic lists where identity matters.

## Mounting With Slot Children

```rust
ctx.mount_with::<Panel>(props, vec![Element::text("body")]);
ctx.mount_keyed_with::<Panel>("settings", props, vec![Element::text("body")]);
```

These work like `mount` and `mount_keyed`, but pass slot children into the child context.

## Keyed List Helper

```rust
let elements = ctx.for_each(
  self.items.get(),
  |item| item.id,
  |_ctx, item| {
    Element::row()
      .child(Element::text(&item.title))
  },
);

Element::column().with_children(elements)
```

`for_each` creates keyed child contexts for arbitrary render closures, not just `Component` implementations. It is useful when each list item needs its own local context for child mounts, effects, or refs.

## Error Boundary

```rust
ctx.error_boundary(
  |ctx| risky_component(ctx),
  || Element::text("Something went wrong"),
)
```

`error_boundary` catches panics from the component closure and returns the fallback element instead.

## Render Lifecycle Methods

```rust
ctx.begin_render();
```

`begin_render` resets the child cursor before rendering children. Runtime and the component mounting internals call this as part of normal rendering.

Application components normally should not call render lifecycle methods directly.
