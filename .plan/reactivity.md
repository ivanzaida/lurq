# Reactivity Implementation Plan

## Done

- [x] `Signal<T>` — reactive value with get/set/update/with + get_untracked/with_untracked
- [x] `batch()` — coalesce watcher notifications
- [x] `Signal::subscribe()` — value-change callbacks
- [x] `Signal::watch()` — void watcher callbacks
- [x] `Ctx::signal()` — create signal that marks component dirty on write
- [x] `Ctx::watch()` — explicit watcher on a signal
- [x] `Component::on_mounted()` / `on_unmounted()` — lifecycle hooks as trait default methods
- [x] `Ctx::mount()` / `mount_keyed()` — child component mounting with reuse
- [x] `Component` trait — `create` (once) + `render` (per dirty cycle)
- [x] Dirty tracking — `Arc<AtomicBool>` per component, `any_dirty()` tree walk
- [x] Event handlers — on_click, on_mouse_move, on_mouse_enter/leave, on_key_down/up, on_focus/blur, on_scroll
- [x] Auto-tracking system — thread-local tracking stack, Signal::get/with call track when active
- [x] `Memo<T>` — auto-tracked derived value, re-evaluates on dependency change, only propagates on value change
- [x] `Ref<T>` — non-reactive persistent value with get/set/with/update
- [x] `Ctx::on_effect()` — auto-tracked side effect, runs immediately, re-runs on dependency change

## TODO

### Store + Lenses
- [ ] `Ctx::store(initial) -> Store<T>` — wrapper around Signal for structured state
- [ ] `Store::lens(|s| s.field) -> Lens<T, R>` — field-level reactivity
- [ ] Lens only triggers re-render when projected field changes

### Context (Dependency Injection)
- [ ] `Ctx::provide(value)` — provide non-reactive context to descendants
- [ ] `Ctx::use_context::<T>() -> Option<T>` — consume context
- [ ] `Ctx::create_context(value) -> Context<T>` — reactive context (hash-based change detection)
- [ ] `Ctx::consume_context::<T>() -> Option<Context<T>>` — subscribe to reactive context
- [ ] `Ctx::consume_context_lens::<T, R>(getter, setter)` — lens into reactive context field

### Keyed List Rendering
- [ ] `Ctx::for_each(items, key_fn, component_fn)` — efficient keyed list
- [ ] Reorders/reuses components by key
- [ ] Drops removed, creates added

### Error Boundary
- [ ] `Ctx::error_boundary(component_fn, fallback_fn)` — catch panics in child render
- [ ] Display fallback on error

### Slot Children
- [ ] `Ctx::children() -> Vec<Node>` — access slot children passed by parent
- [ ] `Ctx::has_children() -> bool`
- [ ] `mount_with` / `mount_keyed_with` — pass slot children

### Scoped Styles
- [ ] Style registry on runtime
- [ ] `Ctx::styles(sheet)` — register scoped stylesheet
- [ ] `Ctx::scoped(class) -> String` — namespace class name to component
