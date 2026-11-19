# Lurq Bug Report: `Ctx`'s Cached `App` Pointer Outlives The `App`, So `tick_timers` / `tick_futures` Render Through A Dead Frame

## Status

Open. Reported 2026-09-16 by the Kontur desktop team (KONTUR-112), against `lurq 0.19.4` on Windows.

## Summary

`Ctx` stores the `App` as a raw `NonNull<App>` (`src/app/ctx.rs:1019`, written by `Ctx::set_app_ref`
at `src/app/ctx.rs:1491`). Only two places refresh it: `Tree::mount_root` (`src/app/runtime.rs:1667`)
and `Tree::pass` (`src/app/runtime.rs:1914`) — and `pass` **returns before** the refresh when the
frame is not required (`src/app/runtime.rs:1888`).

Four public entry points take no `&mut App` and can nonetheless re-render components:

| Entry point | Renders via |
| --- | --- |
| `Tree::tick_timers` / `tick_timers_at` | `apply_reactive_updates_after_event` (`runtime.rs:1124`) |
| `Tree::tick_futures` | `apply_reactive_updates_after_event` (`runtime.rs:1146`) |
| `Tree::key_down` / `key_down_with_meta` | the same, after the event |
| `Tree::resize` | the same |

A render reached that way calls `Ctx::app_ref` / `Ctx::app_ref_mut`, which `unsafe`-dereference the
stored pointer. If the `App` has **moved** since the last `pass`, that pointer names a dead stack
frame. `App` is an owned value with no pinning, so moving it is ordinary safe Rust: returning it from
a constructor, storing it in a struct, handing that struct on. Nothing in the public API says the
`App` must not move, and nothing detects that it did.

The result is a use-after-move through a `&mut`: reads give garbage, writes land on whatever now
occupies the address. In our suite it aborts the test process with `STATUS_ACCESS_VIOLATION`
(`0xc0000005`) or `STATUS_HEAP_CORRUPTION` (`0xc0000374`).

`Ctx::persistent_value`, `Ctx::set_persistent_value`, `Ctx::read_bulk`, `Ctx::read_bulk_values`,
`Ctx::write_bulk` and `Ctx::window_opener` all route through `app_ref()`, so this is not limited to
consumers that call `app_ref` by name.

## Environment

- OS: Windows 11 Pro 10.0.26200
- Rust 1.95.0, `x86_64-pc-windows-msvc`, `cargo` dev/debug
- Crate: `lurq` `=0.19.4`. Reproduced with `default-features = false` (no features at all) and with
  Kontur's own set: `winit, wgpu, dx12, i18n, serde, clipboard, persistent_storage, router, tokio,
  form, image, canvas` plus `devtools` in tests
- `cosmic-text 0.12.1`, `raw-window-handle 0.6.2`
- No render engine installed; no GPU involved

## Actual Behaviour

`Ctx` hands a render the address the `App` had at the previous `pass`, not the address it has now.

```text
render #1 (beat 0) was handed 0xa863cf8480
app at mount:      0xa863cf8480
app after move:    0xa863cfc088
render #2 (beat 1) was handed 0xa863cf8480
STALE: render was handed 0xa863cf8480 while the App lives at 0xa863cfc088 (difference 15368 bytes)
```

Written through, it faults. 20 of 20 fresh processes, `0xc0000005`:

```text
ExceptionAddress: write_through_stale_app_ref!core::core_arch::x86::sse2::_mm_load_si128+0x8
   ExceptionCode: c0000005 (Access violation)
Attempt to read from address ffffffffffffffff

hashbrown::raw::RawIterRange<(cosmic_text::attrs::FontMatchAttrs, Arc<Vec<FontMatchKey>>)>::new
hashbrown::raw::RawTableInner::drop_elements
hashbrown::raw::RawTable<..>::clear
std::collections::hash::map::HashMap<..>::clear
cosmic_text::font::system::FontSystem::db_mut
lurq::app::glyph_engine::GlyphEngine::install_fonts
lurq::app::app_state::App::install_fonts
write_through_stale_app_ref!Probe::render                  <-- ctx.app_ref_mut()
lurq::app::runtime::Tree::rebuild                          <-- args read a5a5a5a5`a5a5a59d ...
lurq::app::runtime::Tree::rebuild_if_dirty
lurq::app::runtime::Tree::apply_reactive_updates_after_event
lurq::app::runtime::Tree::tick_timers_at
lurq::app::runtime::Tree::tick_timers
```

`FontSystem::db_mut` clears `font_matches_cache` before it returns the database, so it is the first
thing to touch the bogus `App` and the frame the fault lands in. It is not the defect; anything that
reads a field of the moved-from `App` would do.

The same stack, in Kontur's `editor_text` suite where we found it — the only difference above
`Tree::tick_futures` is that the consumer is a real component:

```text
cosmic_text::font::system::FontSystem::db_mut
lurq::app::glyph_engine::GlyphEngine::install_fonts
lurq::app::app_state::App::install_fonts
kontur_desktop::editor::canvas_assets::FontBook::install
kontur_desktop::editor::canvas::impl$0::render             <-- ctx.app_ref_mut()
lurq::app::ctx::Ctx::mount_inner<kontur_desktop::editor::canvas::CanvasView>
...
lurq::app::runtime::Tree::rebuild
lurq::app::runtime::Tree::rebuild_if_dirty
lurq::app::runtime::Tree::apply_reactive_updates_after_event
lurq::app::runtime::Tree::tick_futures
```

## Expected Behaviour

A component render is handed the `App` the caller owns, or the call that can render takes the `App`.
Moving an `App` — a plain owned value — must not turn the next `tick_timers` into undefined
behaviour.

## Minimal Reproduction

`docs/evidence/kontur-112/repro/` in the Kontur repository, two binaries, no Kontur code, no canvas,
no render engine, `lurq` with **no features enabled**.

- `stale_app_ref` — prints the three addresses above and exits 1. Deterministic, never faults.
- `write_through_stale_app_ref` — the same with `ctx.app_ref_mut().install_fonts(..)` in the render
  and the dead frame overwritten first. **20 of 20 fresh processes aborted with `0xc0000005`.**

```rust
struct Probe { beat: Signal<u32>, _timeout: Timeout }

impl Component for Probe {
    type Props = ProbeProps;

    fn create(ctx: &mut Ctx) -> Self {
        let beat = ctx.signal(0u32);
        let ticking = beat.clone();
        let timeout = ctx.create_timeout(Duration::from_millis(1), move || {
            ticking.update(|value| *value += 1);
        });
        timeout.start();
        Self { beat, _timeout: timeout }
    }

    fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
        let beat = self.beat.get();                       // subscribe
        if beat > 0 {
            ctx.app_ref_mut().install_fonts([vec![0u8; 16]], std::iter::empty::<(&str, &str)>());
        }
        Rect::new(10.0, 10.0 + beat as f32)
    }
}

/// An ordinary fixture: build the App, mount into it, one frame, hand both back.
fn mount() -> (App, Tree) {
    let mut app = App::new();
    let mut tree = Tree::new();
    tree.mount_root::<Probe>(&mut app, ProbeProps);
    tree.resize(200, 200);
    tree.request_redraw();
    tree.pass(&mut app, &TestSurface);   // Ctx caches &mut app -> this frame
    (app, tree)                          // the App moves out of it
}

fn main() {
    let (app, mut tree) = mount();
    scribble(24);                        // reuse the frame mount left behind
    std::thread::sleep(Duration::from_millis(20));
    tree.tick_timers();                  // renders Probe through the old address
}
```

The bytes handed to `install_fonts` never have to be a font: `GlyphEngine::install_fonts`
(`src/app/glyph_engine.rs:503`) takes `font_system.db_mut()` before it parses anything, and `db_mut`
is what clears the map.

## Where The Defect Is

In lurq. The reproduction contains no consumer code beyond a five-line component, and every frame
between the fault and `main` is lurq's. Two specific things make it reachable:

1. **`Ctx` caches an address instead of being handed one.** `Ctx::app_ref` / `app_ref_mut` are safe
   functions over an `unsafe` deref of `Ctx::app` (`src/app/ctx.rs:2197` and `:2207`) whose validity
   depends on a caller-side invariant — "do not move the `App`" — that is neither stated nor
   enforceable by the type system today.
2. **`pass` can skip the refresh.** `pass_inner` returns at `src/app/runtime.rs:1888` when the frame
   is not required, so even a consumer that dutifully calls `pass` before every event can be left on
   a stale pointer. Our harness happens to call `request_redraw()` first, which is the only reason
   this second shape did not also bite us.

lurq's own test helper has the same shape: `tests/support.rs:36` builds a **fresh local `App` inside
`run_pass` and lets it die at the end of the function**, so every lurq test that renders outside a
`pass` is dereferencing a dead frame too. It has not faulted because those renders only read.

## Suggested Fix

In rough order of preference:

1. **Take the `App` where you may render it.** `tick_timers(&mut self, app: &mut App)`,
   `tick_futures(&mut self, app: &mut App)`, `key_down(.., app: &mut App)`, `resize(.., app: &mut
   App)`. This is a breaking change and it is the only one that makes the invariant checkable.
2. **Stop caching an address.** Hold the app state behind an `Arc` the `Ctx` can own a handle to, so
   `App` becomes a cheap handle whose moves are harmless.
3. If neither, **pin the `App`**: have `App::new` hand back something that cannot move
   (`Pin<Box<App>>`, or a private constructor plus an `AppHandle`), and refresh the pointer
   unconditionally in `pass` rather than after the `!required` early return.
4. At minimum, **document it**: `Ctx::app_ref`, `Ctx::app_ref_mut`, `Tree::mount_root` and
   `Tree::pass` should all say that the `App` must not move after it is first mounted or passed, and
   `pass` should refresh the pointer before the `!required` return so the rule is only about moves.

## How To Tell It Is Fixed

1. `cargo run --bin stale_app_ref` prints `OK: the tree saw the live App` and exits 0.
2. `cargo run --bin write_through_stale_app_ref` survives 60 consecutive fresh processes.
3. Kontur's `editor_text` suite runs 20 consecutive processes at the harness's default thread count
   with its `App` un-boxed again, 20 of 20 exiting 0.

## Our Workaround

We box the `App` in the headless fixture (`apps/desktop/tests/support/session.rs`) so it has one
address for the life of the test and the cached pointer can never go stale. That is a workaround for
our own fixtures only; it does nothing for an application that owns its `App` by value, and nothing
for lurq's own `tests/support.rs`.
