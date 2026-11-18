# Lurq Bug Report: No Direct Focus Call, And An Unkeyed Rebuild Drops The Focus Silently

## Status

Open. Reported 2026-09-10 from the Kontur desktop app (`H:/projects/pencil-web/apps/desktop`, KONTUR-54); **corrected 2026-09-10** during the KONTUR-54 QA follow-ups. The first version claimed lurq exposes no focus query and no usable route to the tree; both claims were wrong. See "Correction". Handoff for the fix is at the end.

## Correction (2026-09-10)

Two statements in the original report were false and are withdrawn:

- **"No focus query."** `core::ElementRef::focused()` (`crates/lurq/src/core/element_ref.rs:103`) is public and is written by the runtime: `set_node_focused` (`crates/lurq/src/app/runtime.rs:8566`) sets it on every focus change alongside the style state. `DevToolsSnapshot::from_tree(&Tree)` is public (behind the `devtools` feature) and carries `focused` per element (`crates/lurq/src/app/devtools/snapshot.rs:112` and `:56`), and MCP prints the same bit as `states=[…,focused]`. Kontur's headless suites now read the ring out of the devtools snapshot instead of out of anything the app publishes about its own focus.
- **"The only public route to the tree is `WinitWindow::on_tick`, which would force continuous polling."** `Ctx::window()` (`crates/lurq/src/app/ctx.rs:2116`) returns a `WindowHandle` with `inject_input` / `inject_inputs` (`crates/lurq/src/app/window.rs:201` and `:206`). Those queue a `WindowCommand` on the same channel the window's own controls use; the shell drains it inside its event loop and applies it through the entry point the matching OS event reaches (`winit_shell.rs:447` and `:964`), and `push_command` calls the shell's waker (`window.rs:390`). Nothing polls, and an injected `Tab` is the `Tab` the user presses.

What remains true is that there is no *direct* focus call: an app has to express "focus this element" as input. That works — `focus_form_tab` moves one candidate forward per `Tab` along a list sorted by `tab_index`, so the distance between two stops is known — and Kontur delivers both of its focus acceptance criteria that way (`apps/desktop/src/auth/focus.rs`, `apps/desktop/tests/auth_forms.rs`). But it is indirect, it can only address focus candidates of the enclosing form, and it is defeated by gap (3) below, which is exactly the moment a newly mounted surface wants the ring. This report now asks for the direct call, not for the possibility.

## Summary

Three gaps in the focus model of the `form` feature:

1. **No direct focus call.** `Tree::focus_node` is private; the public callers are the runtime's own input handling and the tree handles `ElementHandle::focus` / `TextInputHandle::focus`, which need `&mut Tree` — a component has `Ctx`. Moving the ring therefore means synthesising input (see the Correction), which is indirect and unavailable while the runtime's focus is stale.
2. **A rebuild that changes sibling positions clears the focus without a `blur` event.** `refresh_focus_ids` re-resolves the focused path from the focused `NodeId` and calls `clear_focus()` when the id is gone; `clear_focus` does not fire `on_blur`. Ids are preserved positionally (`Node::preserve_ids_from` zips children unless a child carries a `component_key` or a component slot id), so inserting an element above the focused control gives every following sibling a new id, and the focus disappears mid-interaction while the app still believes its `on_focus` handler is current.
3. **A route change leaves `Tab` inert**, because a freed node id is handed out again and makes a stale focus look valid — detailed below.

## Environment

- OS: Windows 11 Pro 10.0.26200, 1.5× scale
- Crate: `lurq`
- Version: `0.18.0`
- Consumer: `kontur-desktop`, features `winit`, `wgpu`, `dx12`, `i18n`, `persistent_storage`, `router`, `form`, `mcp`

## Reproduction — (1) no direct focus call

Searched the whole public surface of 0.18.0: `Ctx` has no `focus`, `core::ElementRef` has `focused()` but no `focus()`, `InteractionState::set_focused` is `pub(crate)`, and `NodeUpdate` exposes only `focusable` / `focused_style`. `Tree::focus_node` and `Tree::blur_focus` are private, so the only handles that can focus a node (`ElementHandle::focus`, `TextInputHandle::focus`) are reachable from a test or a harness holding `&mut Tree`, never from a component.

The route that does exist is synthetic input:

```rust
// In a component, from `Ctx`:
ctx.window().inject_input(SyntheticInput::key_down("Tab"));
```

It is delivered through the window command channel and applied by the shell where the matching OS event would have been, so it is neither a hack nor a polling loop — but it is the wrong shape for the job. It can only reach a focus candidate of the form that already has the ring, the caller has to know the candidate order and count the `Tab`s, and if lurq's own focus is stale (reproduction 3) every one of those `Tab`s is refused. What is missing is `focus(element)`.

## Reproduction — (2) focus lost on an unkeyed rebuild

```rust
// A form whose validation summary appears above the fields.
Form::element(
  FormProps::new(form.clone()),
  Column::new()
    .with_children(error.map(|message| Text::new(&message)))  // appears after a submit
    .child(TextInput::new(email).name("email").single_line())
    .child(Button::new("Sign in").submit()),
)
```

1. `Tab` into the input, type nothing.
2. Press `Enter`; validation fails and the summary is inserted above the input.
3. Type a character.

Observed: nothing is typed. `focused_node` was cleared during `rebuild_if_dirty` → `refresh_interaction_state` → `refresh_focus_ids`, because the input's node id changed with its position. `on_blur` did not fire, so an app that tracks focus itself still shows a focus ring on the input. A following `Tab` starts again at the first candidate instead of continuing from the input.

Expected: the retained control keeps the focus (its identity did not change — only a sibling was inserted); if lurq does drop the focus, it should fire the `on_blur` of the node that had it.

## Reproduction — (3) a route change leaves Tab dead

With the `router` feature, `render_route_match` mounts the matched route keyed by its index, so moving between two routes replaces the whole subtree. When a control inside the old route held the focus, the runtime does not recover:

1. Render a `Form` on route `/a`. Press `Tab` — the first field is focused.
2. Navigate to `/b`, which renders its own `Form`.
3. Press `Tab`.

Observed: nothing is focused, and `Tab` stays inert however often it is pressed; the surface only becomes keyboard-reachable again after a mouse click on one of its controls. `focused_node` survives the replacement — the freed id is handed out again by the `IdGenerator` — so `refresh_focus_ids` resolves a path to an unrelated node outside the form, and `focus_form_tab` takes the `Some(path)` branch, finds no enclosing form and returns `false`. Reproduced at runtime through MCP and in a headless `Tree` (Kontur's `apps/desktop/tests/auth_forms.rs::switching_mode_while_a_stop_holds_the_ring_leaves_tab_dead_until_a_click` pins the current behaviour so the fix is noticed).

This is also what makes the synthetic-input route of (1) unavailable when it is most wanted: a newly mounted surface asking for its first field injects a `Tab`, and `focus_form_tab` refuses it for the same reason it refuses the user's. Kontur's surface therefore takes the ring on open when the window opens on the auth route, and does not after a mode change — recorded in `docs/evidence/kontur-54/README.md`.

Expected: after a route change the focus is either cleared (so `Tab` starts at the first candidate of the new form) or moved to the corresponding control; a freed node id must never make a stale focus look valid.

## Workaround in use

Every child of the form column carries `.key(...)`, which puts `preserve_ids_from` on its identity-matching path and keeps the ids — and the focus — across the insertion. It works, but it is a rule an app has to know: *any* conditionally rendered sibling silently breaks focus otherwise.

For (1), Kontur injects the exact number of `Tab`s between the stop that holds the ring and the stop it wants (`apps/desktop/src/auth/focus.rs`), which is possible only because the surface states its own `tab_index` order and lurq sorts candidates by it. Kontur also keeps a signal of the focused stop, fed by the stops' own `on_focus`/`on_blur` — not because the ring cannot be read (it can: `ElementRef::focused()`, the devtools snapshot, MCP `states`), but because none of those is reactive, so a `focus=<stop>` describe cannot be rendered from them. A focus *signal* on the tree would remove that mirror too.

## Handoff — fix in lurq

```text
Archetype: implementation in lurq (worktree)
Scope: a direct focus call, a reactive focus query, focus survival across rebuilds and route changes
Out of scope: focus trapping/roving tabindex, accessibility tree
Consumer follow-up: kontur-desktop replaces its injected-Tab focus (`apps/desktop/src/auth/focus.rs`) with the direct call, gains focus-on-open after a mode change, and drops its `focus=<stop>` mirror if the query is reactive
```

1. Add a focus request an app can make from a component: e.g. `Ctx::focus(element_id)` / `ElementRef::focus()` queued on the tree and applied at the next rebuild, so it works for a node that does not exist yet at the time of the call (the case after a route change), and for a control outside the form that has the ring.
2. Make the existing focus query reactive and complete: `core::ElementRef::focused()` and `DevToolsNode::focused` already report the ring, but neither notifies. A `Signal`-backed `Tree::focused_element_id()` (or a focus-changed subscription on `Ctx`) would let an app render the focused element's id without mirroring the events itself.
3. Make focus survive an unkeyed rebuild: match the focused node by identity (id/key/slot) rather than by position in `preserve_ids_from`, or re-resolve `focused_node` against the new tree by element id before falling back to `clear_focus`.
4. When focus genuinely goes away (the node is unmounted), fire the `on_blur` handlers of the node that held it, so an app's own focus state cannot go stale.
5. Validate `focused_node` against the tree it was taken from before reusing it (an id returned to the `IdGenerator` must not resolve a stale focus), and clear the focus when the node it named is gone, so `Tab` starts again at the first candidate of the current form.
6. Tests: focus a control, insert a sibling above it, assert the focus and typing survive; navigate between two routes with a focused control and assert `Tab` still walks the new form; assert `on_blur` fires when the focused node is removed; assert a programmatic focus request applied after a route change lands on the requested element; assert an injected `Tab` on a freshly mounted route focuses the first candidate (the case that fails today).
