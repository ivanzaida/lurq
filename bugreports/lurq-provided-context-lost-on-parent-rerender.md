# Lurq Bug Report: Context Provided In `create` Is Lost When The Parent Re-Renders

## Status

Resolved on branch `claude/context-and-scrim` (base `9c61a9e`, 0.22.1).

Fix:

- [`Ctx` keeps the values a component provides outside render and layers them over the inherited contexts.](../crates/lurq/src/app/ctx.rs)
- [`ContextMap::layered` builds that map with a fresh revision.](../crates/lurq/src/core/context.rs)

Regression tests ([`reactivity/provided_context.rs`](../crates/lurq/tests/reactivity/provided_context.rs)):

- `context_provided_in_create_survives_parent_rerender`
- `parent_rerender_without_context_change_does_not_rerender_provider`
- `changed_ancestor_context_reaches_descendants_next_to_provided_one`
- `reactive_context_provided_in_create_still_notifies_after_parent_rerender`
- `value_provided_in_render_is_removed_when_render_stops_providing_it`

## Summary

A component that calls `ctx.provide(value)` or `ctx.create_context(value)` in `create` loses that value as soon as its parent re-renders. Its descendants then get `None` from `use_context` / `consume_context`, and a `ReactiveContext` created there no longer reaches them.

Reported by Orchester. Present since contexts were added; the `QueryClient` special case in `begin_render` was an earlier workaround for the same defect, limited to one type.

## Environment

- Crate: `lurq` 0.22.1, no features required
- Any component tree where a provider component is reused by a parent that re-renders

## Symptoms

1. Parent `P` re-renders (own signal, props, or context change). Child `C` provided `i32` in `create`. Grandchild `G` renders afterwards and `use_context::<i32>()` returns `None`.
2. `C` and `G` re-render on every parent re-render even when nothing they depend on changed, because `C`'s context revision never equals the parent's.
3. A `ReactiveContext` created by `C` stops being visible to `G`, so `G` shows the default value and is no longer subscribed.

## Root Cause

`Ctx::context_map` held one merged map: the values inherited from the parent plus the values the component provided. `mount_inner` compared the child's map revision with the parent's to detect a context change, then overwrote the child's map with a clone of the parent's:

```rust
let context_changed = slot.ctx.context_map.revision() != self.context_map.revision();
slot.ctx.context_map = self.context_map.clone();
```

Nothing recorded which values the child had provided itself, so they were dropped. Providing in `create` also bumped the child's revision, so the comparison reported a change on the first parent re-render even when the inherited contexts had not changed. The same overwrite ran for `for_each` groups and items.

## Reproduction

```rust
struct Provider;
impl Component for Provider {
  type Props = ();
  fn create(ctx: &mut Ctx) -> Self { ctx.provide(7_i32); Self }
  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> { ctx.mount::<Reader>(()) }
}
// Root has a signal and mounts Provider; Reader logs ctx.use_context::<i32>().
root_tick.set(1);
run_pass(&mut tree);   // Provider and Reader re-render; Reader sees None
```

## Fix

`Ctx` now keeps three maps:

- `provided_contexts`: the values the component provided outside render, that is in `create`;
- `base_contexts`: the inherited contexts with `provided_contexts` on top (`ContextMap::layered`);
- `context_map`: what the current render sees, starting from `base_contexts`.

`inherit_contexts` replaces the direct overwrite everywhere a child context is created or reused. It compares the parent's revision with the revision the child last inherited (not with the child's own map), so an unchanged parent no longer re-renders providers. When the parent's map did change, it rebuilds `base_contexts`, which keeps the component's values and gives descendants a new revision.

`provide` and `create_context` outside render store the value in `provided_contexts` and `base_contexts`. During render they only change `context_map`, and `begin_render` resets `context_map` to `base_contexts`. A value provided in `render` therefore belongs to that render and is removed when a later render stops providing it. `Router` and `Outlet` rely on this: `Outlet` reads the ancestor's `OutletDepth` before it provides its own. Keeping render-time values across renders made nested outlets read their own depth and broke four router tests during development.

`form_view_with` still scopes `FormContext` to its closure. The `QueryClient` special case in `begin_render` is removed; the general rule covers it.

## Behavior Changes

- A value provided in `create` stays visible to descendants across ancestor re-renders.
- A value provided in `render` no longer lingers into the component's next render when that render does not provide it again. Before, it lingered until the parent re-rendered.
- Providers whose inherited contexts did not change are no longer re-rendered, with their subtree, on every parent re-render.

## Verification

- `cargo test -p lurq --test reactivity_tests provided_context`: 5 passed; all 5 fail on `9c61a9e`.
- `cargo test -p lurq --features router --test router_tests`, `--features form --test input_tests forms`, `--features query,tokio --test query_tests --test reactivity_tests --test runtime_tests`: pass.
- The `Validate crates` commands and `cargo check --workspace --all-features --all-targets` pass.
