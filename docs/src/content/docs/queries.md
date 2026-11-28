---
title: Queries
description: Shared async data through named query functions, reactive handles, typed invalidation, mutations and cache writes.
---

# Queries

Enable the optional `query` feature:

```toml
lurq = { version = "0.78.0", features = ["query"] }
```

A query is a named async read whose arguments identify a cached result. Components observing the same query share data and a running request. The client retains results when components unmount, so returning to a screen can reuse its data.

## Define a query

```rust
#[lurq::query(stale_time = "30s")]
async fn get_user(id: u64) -> Result<User, ApiError> {
  api::get_user(id).await
}
```

`User`, `ApiError`, and `api` are application types and services. The query body can read HTTP, a database, files, or IPC; the feature does not add a transport dependency.

The macro makes `get_user(id)` a descriptor constructor. Calling it does not execute the body or start a request. The descriptor is observed through `ctx.query(...)`; ordinary async callers can continue to call the underlying `api::get_user(id).await` service.

Query definitions currently support safe, non-generic async free functions with named, owned arguments and a `Result<T, E>` return type. Arguments require `Clone + Eq + Hash + Send + Sync + 'static`. Values and errors require `Send + Sync + 'static`; they do not require `Clone`, `PartialEq`, or `DevtoolsInspectable`.

The cache identity includes the definition and all arguments. Include result-changing inputs such as tenant, locale, filter, or page. Different query functions remain separate even when their arguments and return types match.

## Provide a client

Create the client once in a stable provider component's `create` method:

```rust
use lurq::query::QueryClient;

fn create(ctx: &mut Ctx) -> Self {
  ctx.provide(QueryClient::new());
  Self
}
```

Descendants use the nearest provided client. A nested provider can isolate a subtree. Cloning a client shares its cache; creating a new client creates an isolated cache. Replacing a provider's client rebinds its query observers.

## Choose which trees share data

`QueryClient` already uses `Arc` internally. Pass clones to trees that should share data; use `new()` for an independent cache:

```rust
let queries = QueryClient::new();

// Root accepts QueryClient as its Props.
first_tree.mount_root::<Root>(&mut app, queries.clone());
second_tree.mount_root::<Root>(&mut app, queries.clone());

// This tree has its own cache and invalidation scope.
isolated_tree.mount_root::<Root>(&mut app, QueryClient::new());
```

In `Root`, declare `type Props = QueryClient` and provide that instance in `create`:

```rust
fn create(ctx: &mut Ctx) -> Self {
  ctx.provide(ctx.props::<QueryClient>().clone());
  Self
}
```

Trees receiving clones share cached results, running requests, and invalidation. Trees receiving different clients remain independent. No additional `Arc<QueryClient>` wrapper is needed.

Each attached tree receives a wakeup when shared work becomes ready, and delivers reactive notifications during its own tick, on its own thread. The client serializes shared request polling so concurrent trees cannot start duplicate requests.

Closing one tree removes its observers and runtime registration. Other trees keep their cache and pending requests. When the last tree closes, pending work is cancelled because no UI driver remains. Successful data stays cached while application code retains the client; retention is checked before reusing entries when a tree attaches again. Dropping the last client reference releases the cache.

## Observe during render

```rust
let user = ctx.query(get_user(user_id));

let label = if let Some(user) = user.data() {
  user.name.clone()
} else if let Some(error) = user.error() {
  error.to_string()
} else {
  "Loading...".to_owned()
};

Text::new(&label)
```

`ctx.query` must be called during render. Lurq retains the observer at its query call position. Rerendering with the same descriptor reuses it; changing arguments switches entries. Removing a query call or unmounting releases that observer.

| Handle method | Result |
| --- | --- |
| `data()` | `Option<Arc<T>>`, the last successful value for this key. |
| `error()` | `Option<Arc<E>>`, the latest failed attempt's error. |
| `loading()` | A request is scheduled or running and there is no cached data. |
| `fetching()` | A request is scheduled or running, including a background refresh. |
| `invalidate()` | Mark this entry stale and refresh if it has mounted observers. |
| `refresh()` | Request a fetch even if the entry is fresh or inactive. |

The four read methods participate in normal reactive render tracking. A refresh retains the previous successful data for the same key. A failed refresh also retains that data and exposes the error. Starting another attempt clears the error. Changing keys exposes the new key's cache entry, rather than displaying the old key's value as the new result.

Cloned handles do not add mounted observers. After an entry is evicted, saved handles are empty and their commands are no-ops; observe the descriptor again to obtain the new entry.

## Invalidate after changes

Use the same descriptor for an exact entry, or the generated `all()` selector for every cached argument combination:

```rust
let queries = ctx.query_client();

queries.invalidate(get_user(user_id));
queries.invalidate(get_user::all());
```

For a paginated query, `list_users::all()` includes all cached filters and pages of `list_users`. It does not invalidate a separate `get_user` definition.

An observed entry refreshes in the background. An inactive entry becomes stale and refreshes when observed again. Missing entries are not created. Existing data remains available while a refresh runs.

Invalidation overrides freshness and makes an older in-flight result obsolete. Repeated invalidations before the next tick coalesce into one replacement request. `refresh()` joins an already running current request. These methods enqueue work and return immediately; the runtime applies commands and publishes results during `Tree::tick_futures()`.

## Mutations

A mutation is a named async write. It gives a component what every write needs: a pending flag for the control, an error to show in place, and the cache told what the write changed.

```rust
#[lurq::mutation(invalidates = [get_task::all(), list_tasks::all()])]
async fn rename_task(id: TaskId, title: String) -> Result<Task, ApiError> {
  api::rename_task(id, title).await
}
```

As with a query, the macro turns the function into a descriptor constructor: calling `rename_task(id, title)` starts nothing. `invalidates` lists the queries a successful run invalidates, exact or by family; a failed run invalidates nothing.

Hold the mutation during render, and run it from a handler with the tuple of its arguments:

```rust
let rename = ctx.mutation(rename_task);

Button::new("Rename")
  .loading(rename.pending())
  .on_click({
    let rename = rename.clone();
    move |_| {
      rename.run((task_id, title.get()));
    }
  });

if let Some(error) = rename.error() {
  // Show it in place.
}
```

| Method | Meaning |
| --- | --- |
| `run(args)` | Ask for a run. Returns at once; `false` only when the run was rejected (see below). |
| `pending()` | A run is waiting or on its way. |
| `error()` | What the last failed run failed with. The next run clears it, and so does a success. |
| `data()` | What the last successful run returned. |
| `reset()` | Forget the last result and error. Runs on their way go on. |

`pending()`, `error()` and `data()` are reactive reads. The handle is `MutationHandle<Args, Data, Error>`, here `MutationHandle<(TaskId, String), Task, ApiError>`, so it can be a prop or a field. Its state belongs to the `ctx.mutation` call and survives re-renders.

A run belongs to the client, not to the component. A run still on its way when its component unmounts finishes, and its effects on the cache apply. A write is never deduplicated and never cancelled: the service may have accepted it already.

### Effects in code

Where the effect needs the arguments or the result, say it on the handle. `on_success` is called with the client, the run's arguments and its result, before the definition's `invalidates` apply:

```rust
let rename = ctx.mutation(rename_task).on_success(|client, (id, _), task| {
  client.set_data(get_task(*id), task.clone()); // write the fresh value
  client.invalidate(list_tasks::all());
});
```

A clone of the handle carries the effects it had when it was cloned, so clone it for a handler after they are set. Effects are called when the run finishes, on the thread of a window that has the client, so what they capture must be `Send + Sync`.

### Runs asked for while one is pending

By default the runs of one handle take turns, in the order they were asked for: no write is lost and none overtakes an earlier one. `concurrency` changes that:

```rust
use lurq::query::MutationConcurrency;

let save = ctx.mutation(save_draft).concurrency(MutationConcurrency::Reject);
```

| Concurrency | A run asked for while another is pending |
| --- | --- |
| `Queue` (default) | waits its turn |
| `Parallel` | starts at once, beside the others |
| `Reject` | is not run, and `run` returns `false` |

Handles of different `ctx.mutation` calls are independent of one another.

### Without a component

`QueryClient::mutate` runs a mutation with nothing to watch it, from a handler of server events or a background task. Its definition's `invalidates` apply.

```rust
queries.mutate(mark_read(notification_id));
```

## Write the cache

The client reads and writes cached values directly:

```rust
queries.get_data(get_task(id));                       // Option<Arc<Task>>, no request
queries.set_data(get_task(id), task);                 // every observer shows it at once
queries.update_data(get_task(id), |task| Task { done: true, ..task.clone() });
```

- `set_data` makes the value the entry's, fresh from now as if fetched. A query nothing has cached yet gets an entry, kept for the retention time.
- `update_data` writes what the closure makes of the cached value. With nothing cached it does nothing, and the closure is not called.
- `get_data` starts no request and is not a reactive read. Components read through `ctx.query`.

A fetch of that query that was on its way when a value is written is dropped: its result is older than the write. Invalidate the query to have the server's value fetched over the written one.

### Optimistic updates

`optimistic` is called as a run starts, before its request is sent. It writes the cache as the write will leave it, and returns what to go back to. `on_error` is handed that value if the run fails:

```rust
let rename = ctx
  .mutation(rename_task)
  .optimistic(|client, (id, title)| {
    let before = client.get_data(get_task(*id));
    client.update_data(get_task(*id), |task| Task { title: title.clone(), ..task.clone() });
    before // the rollback value
  })
  .on_error(|client, (id, _), _error, before| {
    if let Some(before) = before {
      client.set_data(get_task(*id), (*before).clone());
    }
  });
```

Set `optimistic` before `on_error`, which takes its snapshot; the other order panics during render. Without `optimistic`, the fourth argument of `on_error` is `()`.

A queued run makes its optimistic write when its turn comes, over what the runs before it left.

`ctx.future_action` remains for async work that is not a write to shared data: another `.run()` restarts its local task, and it ends with its component.

## Freshness and retention

By default, a successful result is fresh for 30 seconds. Inactive entries are retained for five minutes after their last observer leaves.

```rust
use std::time::Duration;
use lurq::query::{QueryClient, QueryClientOptions};

let queries = QueryClient::with_options(QueryClientOptions {
  stale_time: Duration::from_secs(60),
  gc_time: Duration::from_secs(600),
});
```

A definition can override either setting:

```rust
#[lurq::query(stale_time = "10s", gc_time = "2m")]
async fn get_status() -> Result<Status, ApiError> {
  api::get_status().await
}
```

Duration annotations accept an integer followed by `ns`, `us`, `ms`, `s`, `m`, or `h`.

Freshness and retention are independent. Observing stale data schedules a refresh, but merely passing the freshness deadline does not start polling. Ordinary rerenders do not repeatedly fetch stale data or retry failures. There are no automatic retries or focus/reconnect refreshes in this version.

When the last observer leaves, an existing valid request may finish and populate the cache during the retention period. Eviction cancels unfinished work and rejects obsolete completions. An explicit refresh of an inactive entry restarts its retention period.

## Runtime and inspection

The winit shell drives query commands, completions, and retention deadlines automatically. Queries wake the event loop when a future becomes ready or another thread enqueues invalidation, so idle queries do not require continuous polling. Custom shells should continue calling `Tree::tick_futures()` as part of their tick loop.

With a Tokio handle configured through `App::with_tokio_handle`, queries run on a registered runtime. A shared client chooses the first available attached Tokio runtime when starting a request; an existing task stays on the runtime where it started. Closing its starting tree does not cancel that task while other trees remain attached. If the runtime itself shuts down, observed requests restart on another available attached runtime. If all registered runtimes have shut down, requests wait for a live runtime without busy polling.

Enable both `query` and `tokio` for services that require Tokio. Clients without a configured Tokio runtime poll futures cooperatively; blocking I/O in a query body would block that polling thread. Cache changes wake all attached trees, and each tree publishes its observer notifications on its own UI tick.

Mutations are driven the same way, on a Tokio runtime where one is attached. If that runtime shuts down under a running mutation, the run is not sent again, since whether the write was made is not known: it ends with neither a result nor an error. While no tree is attached to the client, a mutation waits to be finished by the next one that attaches.

`queries.inspect()` returns `QueryInfo` records containing definition name, observer count, freshness, fetch activity, and whether data or an error is present. Inspection does not expose arguments or payload contents. A dedicated DevTools query panel is not included yet.
