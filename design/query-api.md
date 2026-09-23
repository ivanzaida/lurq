# Query API proposal

Status: core released in 0.19.3 behind the optional `query` feature and included in 0.20.0. This document records the original proposal and remaining design work. See the [Queries guide](../docs/src/content/docs/queries.md) for the implemented API and current behavior; the proposal's open questions and recommendations below are historical.

Implemented: descriptor macros, shared clients and observers, typed cache identity, exact/family invalidation, refresh, freshness/retention, cooperative and Tokio execution, and metadata inspection. Client defaults use `QueryClient::with_options(QueryClientOptions { ... })`. Saved handles become empty and inert after eviction. Client clones can be shared across trees; `new()` creates an independent cache. Payloads do not require DevTools inspection traits. A dedicated mutation API, prefetch, persistence, and a DevTools query panel remain future work.

Date: 2026-09-13. Repository baseline: lurq 0.18.2.

## 1. Direction from the discussion

Provide shared async data through named query functions and a one-line component API:

```rust
#[lurq::query]
async fn get_user(id: UserId) -> Result<User, ApiError> {
  api::get_user(id).await
}

// In a component's render method:
let user = ctx.query(get_user(user_id));
```

Use the same query expression to invalidate its cached result:

```rust
let queries = ctx.query_client();

queries.invalidate(get_user(user_id));
queries.invalidate(get_user::all());
```

The initial builder-heavy API with manually repeated cache keys was rejected. This document develops the subsequent function-based proposal. Lifecycle rules, defaults, internal types, and implementation stages below are recommendations for review, rather than separately agreed decisions.

## 2. Query definitions

A query is an annotated async function returning `Result<T, E>`. Its body performs a finite read, such as an HTTP request, database query, file read, or IPC call.

```rust
#[lurq::query(stale_time = "30s")]
async fn get_user(id: UserId) -> Result<User, ApiError> {
  api::get_user(id).await
}

#[lurq::query]
async fn list_users(filter: UserFilter, page: u32) -> Result<Vec<User>, ApiError> {
  api::list_users(filter, page).await
}
```

The macro changes the public function into a typed descriptor constructor. Calling `get_user(id)` describes a query; it does not execute the body, access a cache, or start a request. `ctx.query(...)` binds that descriptor to a client and observes the result.

This transformation should be explicit in the documentation and generated API docs. The descriptor is not directly awaitable in the initial proposal. Ordinary async callers can use the underlying application service, such as `api::get_user(id).await`.

The macro also generates a same-named companion module containing the family selector, such as `get_user::all()`. The constructor and companion module must preserve the declared visibility. A conflicting user-defined module should produce a useful macro diagnostic.

Initially support free functions with owned, concrete arguments and a concrete `Result<T, E>` return type. Generic query functions, borrowed arguments, and methods can be considered after the basic API works.

## 3. Cache identity

Within one `QueryClient`, an entry is identified by the query definition and the complete argument tuple.

```rust
get_user(42)                    // One entry.
get_user(43)                    // Another entry.
list_users(filter.clone(), 1)   // Page 1 for this filter.
list_users(filter, 2)           // Page 2 for this filter.
```

Repeated calls for the same definition and equal arguments share cached data and any current request. Two query functions with identical arguments or return types remain separate families.

Recommended argument bounds are `Clone + Eq + Hash + Send + Sync + 'static`. Hashing accelerates lookup; full argument equality resolves collisions. Do not use a bare hash, display string, function address, or serialized JSON as the only identity.

Every input that changes the result must be represented by the arguments or by the client's scope. Examples include filters, pagination, tenant, and locale. Mutable service configuration or an authenticated session needs an explicit scope policy; hidden dependencies must not silently reuse results from a previous scope.

## 4. Providing a client

Create and provide a client once from a stable root component:

```rust
fn create(ctx: &mut Ctx) -> Self {
  ctx.provide(QueryClient::new());
  Self
}
```

Descendants use `ctx.query(...)` and `ctx.query_client()`. Cloning a client shares its cache. Creating another client creates an isolated cache. The recommended missing-provider behavior is a clear diagnostic naming `QueryClient` and showing the setup above.

The caller chooses the scope across trees as well. Pass clones of one `QueryClient` as root props and provide them in each tree to share results, requests, and invalidation. Pass separate `QueryClient::new()` values for independent caches. The client already uses `Arc` internally.

Each tree has a weak runtime registration and delivers notifications to its own observers on its own thread. Shared polling is serialized. Closing one tree detaches its registration without cancelling requests needed by surviving trees. When the last tree closes, cancel pending work but preserve successful data while the caller retains a client; enforce retention before reusing that cache on reattachment.

A client should be scoped to an application session or an intentionally isolated subtree. Replacing the client must rebind observers even when their query descriptors are unchanged. This provides a foundation for changing accounts without reusing the previous account's data.

Defaults belong on the client. An annotation such as `#[lurq::query(stale_time = "30s")]` overrides the corresponding client default for that definition. The exact client configuration API remains open; component code should remain a simple `ctx.query(get_user(id))` call.

## 5. Reading a query

`ctx.query(...)` returns a cloneable `QueryHandle<T, E>` bound to that client and argument tuple.

```rust
let user = ctx.query(get_user(user_id));

user.data()      // Option<Arc<User>>
user.loading()   // bool: fetching with no cached data
user.fetching()  // bool: any fetch, including a background refresh
user.error()     // Option<Arc<ApiError>>
user.refresh();
user.invalidate();
```

The four accessors are reactive reads using lurq's existing render tracking. Updates notify observing components. Shared `Arc` values make reads cheap without requiring the fetched value itself to implement `Clone`.

Recommended payload bounds are `Send + Sync + 'static`, with the existing DevTools inspection requirements handled consistently when that feature is enabled. The query implementation should not require deep `PartialEq` comparisons just to publish a result.

Data, errors, and fetch activity are separate state:

| Situation | `data()` | `loading()` | `fetching()` | `error()` |
| --- | --- | --- | --- | --- |
| First fetch | None | true | true | None |
| Successful result | Some | false | false | None |
| Background refresh | Some | false | true | None |
| First fetch failed | None | false | false | Some |
| Refresh failed | Previous data | false | false | Some |

A new attempt clears the previous error while retaining data for that same key. A refresh failure preserves the last successful data and its timestamp.

When arguments change, the observer switches to the new entry. It immediately exposes that entry's cached data if present. Previous data from a different key is not automatically shown as the new result; optional pagination placeholder behavior can be designed separately.

## 6. Invalidation

### One entry

```rust
queries.invalidate(get_user(user_id));
```

This targets the exact query definition and argument tuple. Constructing the selector does not execute the query. Invalidating an entry that has never been cached is a no-op.

### A query family

```rust
queries.invalidate(get_user::all());
queries.invalidate(list_users::all());
```

The generated `all()` selector targets every existing argument combination of that definition within the client. For `list_users`, this includes all cached filters and pages. It does not enumerate possible inputs or create requests for uncached inputs.

### Through an existing handle

```rust
let user = ctx.query(get_user(user_id));
user.invalidate();
```

This is equivalent to invalidating that handle's exact entry through its client. A saved handle remains bound to its original client and key, even if a later render observes another key.

### Effect on the cache

| Target state | Effect |
| --- | --- |
| Observed by at least one mounted query observer | Mark stale and schedule a background refresh. |
| Cached with no mounted observers | Mark stale; refresh when observed again. |
| No cached entry | No work. |

Invalidation preserves existing data and overrides `stale_time`. Repeated invalidations before scheduled work begins should coalesce into one request for the latest invalidation revision.

`invalidate()` means the data may be outdated. `refresh()` requests a fetch immediately, even if the entry is fresh or the handle has no mounted observer. Both enqueue work and return without waiting for the network. A refresh joins an already running request for the current revision rather than duplicating it.

### Invalidation during a request

Track a request generation and an invalidation revision for each entry. A request started before an invalidation must not subsequently mark that entry fresh or publish an obsolete result over newer data.

If the entry remains observed, schedule a replacement request for the latest revision. The executor may cancel the obsolete task or let it finish and discard its result. Multiple invalidations before the replacement starts coalesce. If the entry becomes inactive, keep it stale until the next observer or an explicit refresh.

The same generation checks apply when an entry is evicted or its client is replaced. An old completion cannot recreate an evicted entry or update a new session's cache.

## 7. Invalidation after writes

The application declares which reads a successful write affects:

```rust
// Inside a mutation's success handler:
queries.invalidate(get_user(saved.id));
queries.invalidate(list_users::all());
```

Changing one user may affect a detail view and several filtered lists. Query function names and result types do not establish those relationships automatically.

The first implementation can integrate with the existing `ctx.future_action` API:

```rust
let queries = ctx.query_client();

let save = ctx.future_action(move |changes: UserChanges| {
  let queries = queries.clone();
  async move {
    let saved = api::update_user(changes).await?;

    queries.invalidate(get_user(saved.id));
    queries.invalidate(list_users::all());

    Ok::<_, ApiError>(saved)
  }
});

// In a button or form handler:
save.run(changes);
```

Client commands must be safe to enqueue from async tasks; cache publication and UI notifications happen through the runtime. Invalidation in this example occurs only after success.

This example retains `FutureAction`'s existing restart behavior on another `.run()`. A dedicated mutation API needs a separate concurrency contract: writes must not be implicitly deduplicated, and cancelling a local task cannot undo a write already accepted by a service. Mutation syntax, optimistic updates, and direct cache writes remain follow-up design work.

## 8. Freshness and lifetime

`stale_time` measures freshness from the last successful result. `gc_time` controls retention after the last mounted observer leaves. These are independent clocks.

- Mounting an uncached query schedules its first fetch.
- Mounting a fresh cached query returns the data without fetching.
- Mounting a stale cached query returns the data and schedules a refresh.
- Passing the freshness deadline marks data stale; it does not itself start periodic fetching.
- Ordinary rerenders keep the observer stable and do not repeatedly fetch stale or failed entries.
- When the last observer leaves, start the retention deadline. Let a current valid request finish and populate the cache during that period.
- Eviction cancels remaining work where possible and rejects any later completion. A subsequent observation starts with a new entry.

The client owns entries and tasks. Components own mounted observers. Dropping one observer must not cancel a request another observer needs. Cloning a handle for an event handler does not count as an additional mounted observer or prevent cache eviction indefinitely.

Proposed initial defaults, subject to review:

| Policy | Recommendation |
| --- | --- |
| Freshness | 30 seconds |
| Retention after last observer | 5 minutes |
| Automatic retry | Disabled initially; explicit refresh retries a failure. |
| Refetch on window focus or reconnect | Deferred until native lifecycle integration is designed. |
| Polling | Deferred; staleness is not a polling interval. |

## 9. Integration with lurq

Expose the runtime API under `lurq::query` behind an optional `query` feature, and implement the attribute macro in `lurq_macros`. The query layer should accept futures without requiring an HTTP client or storage backend.

The existing [async implementation](../crates/lurq/src/app/ctx.rs) stores component-local tasks in `Ctx` slots. Finishing a render cancels tasks in slots no longer used. Shared queries need their own client-owned task registry, registered with the runtime independently of any one requesting component.

Reuse the existing [signals and render tracking](../crates/lurq/src/core/signal.rs) for observation. Query observer registration must also participate in render reconciliation, so changing keys, removing a query call, replacing a provider, or unmounting releases the correct observer without accumulating subscriptions.

The runtime integration drives client work while a tree remains attached, even if no requesting component is mounted. This includes command delivery, task completions, cache retention deadlines, and any future retry timers. Both cooperative execution and optional Tokio execution are supported. Shared cache revisions are delivered through per-observer signals on each tree's own tick; a poller in another tree never invokes those observers' reactive callbacks directly.

For Tokio, choose the first available attached runtime at request start. Running tasks keep their chosen runtime after the starting tree detaches. Runtime shutdown makes affected observed requests eligible to start on another available attached runtime. If no live runtime is available after shutdown, wait for one to attach rather than retrying on the stopped runtime in a loop.

The existing [resource cache](../crates/lurq/src/resources/resource_cache.rs) stores byte assets and removes them on TTL expiry. Query entries need typed results, observer lifetimes, retained stale data, and invalidation revisions, so they should have their own cache implementation.

Useful internal concepts are a typed query descriptor, an exact or family selector, a client cache entry, a mounted observer, and a runtime task registration. These types support the small public API; callers should not have to assemble them manually.

## 10. Implementation stages and verification

1. Build the descriptor macro, typed identity, companion family selector, and compile-time diagnostics.
2. Add client provision, shared entries, deduplicated fetching, reactive handles, and observer reconciliation.
3. Add freshness, retention, exact and family invalidation, refresh, and generation checks.
4. Add examples using `future_action` and a DevTools query inspector showing family, status, observer count, and freshness. Argument and payload inspection should follow the existing inspection rules.

Verification should demonstrate behavior rather than only implementation structure:

- Two components requesting the same key perform one fetch and receive the same result.
- Different keys and different definitions remain isolated, including under forced hash collisions.
- Unmounting one observer leaves another observer's request alive.
- Switching keys releases the old observer and does not show its data as the new key's result.
- Fresh remounts reuse data; stale remounts refresh while retaining data.
- Exact invalidation affects one entry; family invalidation affects all cached variants of only that definition.
- Invalidating an inactive or missing entry starts no request.
- Invalidating during a fetch prevents the obsolete result from clearing staleness or overwriting newer data.
- Failed refreshes retain data and do not trigger a rerender-driven retry loop.
- Retention expiry and client replacement reject obsolete completions.
- Retained event-handler handles do not keep entries active indefinitely.
- The same lifecycle works with cooperative execution, Tokio, and DevTools enabled.

## 11. Decisions still open

- Confirm the descriptor-producing macro tradeoff and the generated `get_user::all()` namespace.
- Settle the client configuration syntax and initial freshness/retention defaults.
- Define service injection and session replacement ergonomics without adding non-key service handles to every query's arguments.
- Specify conditional queries and dependent queries while preserving stable observer reconciliation.
- Decide whether the first version includes imperative fetch/prefetch; any such API must use the same descriptors and cache.
- Design mutations, optimistic updates, selective family invalidation, persistence, and infinite queries after the basic lifecycle is established.

## References

- [Lurq futures, streams, and actions](../docs/src/content/docs/futures-timers.md)
- [Lurq context and component lifecycle](../docs/src/content/docs/ctx.md)
- [TanStack Query cache lifecycle and defaults](https://tanstack.com/query/latest/docs/framework/react/guides/important-defaults), which distinguishes freshness from retention.
- [TanStack Query keys](https://tanstack.com/query/latest/docs/framework/react/guides/query-keys), which treats result-changing inputs as part of query identity.
