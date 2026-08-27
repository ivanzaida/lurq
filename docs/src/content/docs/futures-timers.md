---
title: Futures And Timers
description: Async data fetching with futures, imperative actions, and timer-based scheduling.
---

# Futures And Timers

## Futures

`ctx.future` runs an async operation that automatically re-executes when its dependencies change. The returned `FutureHandle` exposes a reactive `Signal<FutureState<T, E>>`.

Use `ctx.future` for finite async work that resolves to one result, such as loading a page, querying an endpoint, or submitting a request. Do not use it to manually chain a continuous subscription by changing a dependency after every completion. For watch receivers, sockets, event feeds, and other multi-item sources, use [`ctx.stream`](#streams).

```rust
use lurq::{
  app::{component::Component, ctx::{Ctx, FutureStatus}},
  components::Text,
  node::Element,
};

struct UserProfile;

impl Component for UserProfile {
  type Props = ();

  fn create(_ctx: &mut Ctx) -> Self { Self }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let handle = ctx.future((), |_| async {
      Ok::<_, String>("loaded data".to_owned())
    });
    let state = handle.state().get();
    match state.status {
      FutureStatus::Pending => Text::new("Loading..."),
      FutureStatus::Fulfilled => Text::new(&state.data.unwrap()),
      FutureStatus::Rejected => Text::new(&format!("Error: {}", state.error.unwrap())),
      FutureStatus::Idle => Text::new(""),
    }
  }
}
```

### Dependency-Driven Re-execution

The first argument is a dependency value. When it changes between renders, the future restarts.

```rust
fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
  let page = self.page.get();
  let handle = ctx.future(page, |page| async move {
    fetch_page(page).await
  });
  // ...
}
```

The dependency must implement `Clone + PartialEq + Send + Sync + 'static`. Use a tuple to combine multiple deps.

`ctx.future` restarts when the dependency value is different on a later render. If a future result is consumed and that render then changes the dependency, the new future is only created by a following render. That is fine for finite request/retry flows, but it is the wrong shape for continuous streams.

### FutureState

| Field | Type | Description |
| --- | --- | --- |
| `status` | `FutureStatus` | `Idle`, `Pending`, `Fulfilled`, or `Rejected`. |
| `data` | `Option<T>` | Present on `Fulfilled`; preserved during re-fetch. |
| `error` | `Option<E>` | Present on `Rejected`. |

Convenience methods: `is_idle()`, `is_pending()`, `is_fulfilled()`, `is_rejected()`.

### FutureHandle

| Method | Description |
| --- | --- |
| `.state()` | Returns a `Signal<FutureState<T, E>>` for reactive reads. |
| `.cancel()` | Cancels the in-flight future. |
| `.is_active()` | Whether a future is currently running. |

## Streams

`ctx.stream` runs a continuous async producer. The producer receives a `StreamEmitter<T, E>` and can call `.emit(value)` repeatedly. The returned `StreamHandle` exposes the same reactive `Signal<FutureState<T, E>>` shape as futures, but the task stays alive after each emitted item.

Use streams for `watch::Receiver`, websocket subscriptions, event feeds, filesystem watchers, and other sources that can produce more than one value.

```rust
use lurq::{
  app::{
    component::Component,
    ctx::{Ctx, FutureStatus, StreamEmitter},
  },
  components::Text,
  node::Element,
};
use tokio::sync::watch;

struct ServerEvents;

impl Component for ServerEvents {
  type Props = watch::Receiver<String>;

  fn create(_ctx: &mut Ctx) -> Self { Self }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let receiver = ctx.props::<Self::Props>().clone();
    let handle = ctx.stream((), move |_, emitter: StreamEmitter<String, String>| {
      let mut receiver = receiver.clone();
      async move {
        loop {
          if receiver.changed().await.is_err() {
            break;
          }
          if !emitter.emit(receiver.borrow().clone()) {
            break;
          }
        }
      }
    });

    let state = handle.state().get();
    match state.status {
      FutureStatus::Fulfilled => Text::new(&state.data.unwrap()),
      FutureStatus::Pending => Text::new("Waiting..."),
      FutureStatus::Rejected => Text::new(&format!("Stream error: {}", state.error.unwrap())),
      FutureStatus::Idle => Text::new(""),
    }
  }
}
```

### StreamHandle

| Method | Description |
| --- | --- |
| `.state()` | Returns a `Signal<FutureState<T, E>>` containing the latest emitted item or error. |
| `.cancel()` | Cancels the stream task. |
| `.is_active()` | Whether the stream task is currently running. |

### StreamEmitter

| Method | Description |
| --- | --- |
| `.emit(value)` | Publishes a fulfilled item to the UI state. Returns `false` if the receiver was dropped. |
| `.reject(error)` | Publishes a rejected state while keeping the stream task alive. Returns `false` if the receiver was dropped. |

The dependency argument works the same way as `ctx.future`: changing it between renders cancels the existing stream task and starts a new one.

## Future Actions

`ctx.future_action` creates a future that does not run automatically. Instead, you call `.run(args)` to start it — useful for form submissions, button-triggered requests, or any operation that should not run on every render.

```rust
fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
  let action = ctx.future_action(|query: String| async move {
    search(query).await
  });
  let state = action.state().get();

  Column::new()
    .child(Button::new("Search").on_click({
      let action = action.clone();
      move |_| action.run("lurq".to_owned())
    }))
    .child(match state.status {
      FutureStatus::Idle => Text::new("Press search"),
      FutureStatus::Pending => Text::new("Searching..."),
      FutureStatus::Fulfilled => Text::new(&state.data.unwrap()),
      FutureStatus::Rejected => Text::new("Failed"),
    })
}
```

`FutureAction` has the same `.state()`, `.cancel()`, and `.is_active()` methods as `FutureHandle`, plus `.run(args)`.

When using the `form` feature, `FormProps::submit_action(action)` wires a `FutureAction<FormValues, _, FormErrors>` into a mounted form. It validates before running the action, exposes `form.submitting()`, blocks duplicate submits while pending, and maps rejected `FormErrors` back into field errors.

## Tokio Integration

Enable the `tokio` feature to run futures on a real async runtime instead of polling them manually each frame.

```toml
lurq = { version = "0.18", features = ["tokio"] }
```

Pass a tokio handle when creating the `App`:

```rust
let tokio_rt = tokio::runtime::Runtime::new().unwrap();
let app = App::new().with_tokio_handle(tokio_rt.handle().clone());
```

With a tokio handle, futures and streams spawn onto the tokio runtime and complete independently. Results are delivered back to the UI thread on the next `tree.tick_futures()` call.

Without the `tokio` feature, futures and streams are polled cooperatively during `tree.tick_futures()`.

## Timers

### Timeout

A one-shot timer that fires once after a duration.

```rust
use std::time::Duration;

fn create(ctx: &mut Ctx) -> Self {
  let count = ctx.signal(0);
  let timeout = ctx.create_timeout(Duration::from_secs(2), {
    let count = count.clone();
    move || count.update(|n| *n += 1)
  });
  timeout.start();
  Self { count, _timeout: timeout }
}
```

| Method | Description |
| --- | --- |
| `.start()` | Arm the timer. If already armed, does nothing. |
| `.restart()` | Reset the deadline to `now + duration`. |
| `.cancel()` | Stop the timer without firing. |
| `.is_active()` | Whether the timer is armed. |

A timeout fires at most once. After firing it becomes inactive.

### Interval

A repeating timer that fires every `duration`.

```rust
let interval = ctx.create_interval(Duration::from_millis(500), {
  let tick = tick.clone();
  move || tick.update(|n| *n += 1)
});
interval.start();
```

| Method | Description |
| --- | --- |
| `.start()` | Arm the interval. |
| `.restart()` | Reset the next fire to `now + duration`. |
| `.stop()` | Stop repeating. |
| `.is_active()` | Whether the interval is running. |

### Lifecycle

Create timers in `Component::create` and store them in the component struct. The runtime ticks timers each frame via `tree.tick_timers()`. When a timer fires it invokes its callback, which typically updates a signal, causing a re-render.
