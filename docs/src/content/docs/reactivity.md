---
title: Reactivity
description: Signals, stores, lenses, memos, refs, effects, watchers, contexts, and debug inspectability.
---

# Reactivity

Use reactive state when a value change should update UI. Use refs when a value should persist without rendering.

## Signals

`Signal<T>` is the basic reactive cell.

```rust
let count = ctx.signal(0);

let value = count.get();
count.set(value + 1);
count.update(|value| *value += 1);
```

Reads through `get()` and `with()` are tracked by memos and effects. Reads through `get_untracked()` and `with_untracked()` are not.

```rust
let name = signal.with(|value| value.name.clone());
let current = signal.get_untracked();
```

When a signal created with `ctx.signal(...)` changes, the owning component context is marked dirty. The next pass rerenders that component subtree.

## DevTools Type Bound

Without the `devtools` feature, any `T` can be a signal value.

With `devtools` enabled, `Signal<T>` requires `T: DevtoolsInspectable`. This is intentional: DevTools can show live signal values without storing extra debug data in non-devtools builds.

```rust
#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct CounterState {
  count: i32,
}

let state = ctx.signal(CounterState { count: 0 });
```

Use `#[devtools_ignore]` on fields that should not be displayed.

```rust
#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct Session {
  user: String,
  #[devtools_ignore]
  token: String,
}
```

## Stores And Lenses

`Store<T>` wraps structured state in a signal. Use it when a component owns a larger model.

```rust
#[derive(Clone, PartialEq, lurq::DevtoolsInspectable)]
struct User {
  name: String,
  age: u32,
}

let user = ctx.store(User { name: "Ada".into(), age: 36 });
user.update(|user| user.age += 1);
```

Use `lens` to expose one field to child code.

```rust
let name = user.lens(
  |user| user.name.clone(),
  |user, name| user.name = name,
);

name.set("Grace".into());
```

`Lens` has `get`, `set`, and `update`.

## Memos

`ctx.memo(...)` creates derived reactive state. The closure runs once immediately, tracks signals read inside it, and recomputes when those dependencies change.

```rust
let count = ctx.signal(0);
let doubled = ctx.memo({
  let count = count.clone();
  move || count.get() * 2
});

let value = doubled.get();
```

Memos only publish when the new value is different from the old value, so `T` must implement `PartialEq`.

## Refs

`ctx.create_ref(...)` creates persistent non-reactive state.

```rust
let render_count = ctx.create_ref(0_u64);
render_count.update(|count| *count += 1);
```

Ref updates do not mark the component dirty. Use refs for handles, cached measurements, counters, and imperative coordination.

## Effects

`ctx.on_effect(...)` runs immediately and reruns when tracked values read by the closure change.

```rust
let count = ctx.signal(0);

ctx.on_effect({
  let count = count.clone();
  move || println!("count changed to {}", count.get())
});
```

Effects are retained by the component context and dropped when the context is dropped.

## Watchers

`ctx.watch(&signal, callback)` subscribes directly to one signal.

```rust
ctx.watch(&count, |value| {
  println!("new count: {value}");
});
```

Use an effect when dependencies should be discovered from reads. Use a watcher when you already know the exact signal to observe.

## Batch Updates

`ctx.batch(...)` groups updates so dirty marking is deferred until the closure completes.

```rust
ctx.batch(|| {
  first.set(1);
  second.set(2);
});
```

## Static Context

Static context stores a cloned value by type.

```rust
#[derive(Clone)]
struct Locale(&'static str);

ctx.provide(Locale("en-US"));
```

Descendants read by type:

```rust
if let Some(locale) = ctx.use_context::<Locale>() {
  println!("{}", locale.0);
}
```

Use static context for values that do not need to notify consumers when changed.

## Reactive Context

Reactive context is a typed context value that can notify consumers.

```rust
#[derive(Clone, Hash)]
struct ThemeName(&'static str);

let theme = ctx.create_context(ThemeName("dark"));
theme.set(ThemeName("light"));
```

Descendants consume it:

```rust
let theme = ctx.consume_context::<ThemeName>().unwrap();
let current = theme.get();
```

`ReactiveContext<T>` requires `T: Clone + Hash + Send + Sync + 'static`. Updates notify consumers only when the hash changes.

## Common Patterns

For local component state, create signals in `create`.

For parent-controlled state, pass a `Signal<T>` through props.

For derived display state, use a memo.

For side effects, prefer `on_effect` over doing work directly in `render`.

For context, create or provide at a stable provider component and consume from descendants.
