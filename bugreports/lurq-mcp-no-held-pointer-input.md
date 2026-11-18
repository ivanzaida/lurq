# Lurq Bug Report: `lurq_interact` Cannot Hold A Pointer Button Or Key Across Frames

## Status

Open: https://github.com/ivanzaida/lurq/issues/37. Found while building the Kontur Desktop frame-time workload (KONTUR-236), lurq 0.34.1 (`3bae59b`); the
MCP tool sources are unchanged in 0.34.2 (`58ca311`).

## Summary

An MCP client cannot keep a pointer button (or a key) pressed across calls, so it cannot drive a continuous drag
whose samples arrive one frame apart. `lurq_interact` `drag` delivers press, six moves and release in a single
UI-thread turn. Every drag-shaped interaction an automation harness can produce is therefore a one-frame gesture.

This blocks measuring frame rate during a drag (node drag, resize handle, marquee, slider, scrollbar thumb,
space/middle-button pan). It is a missing capability, not a crash.

## Environment

- Crate: `lurq` 0.34.1 with features `mcp`, `winit`, `dx12`, `perf_profile`.
- Consumer: Kontur Desktop release build, Windows 11, driven over ordinary MCP (`lurq_interact`).

## Source

- `crates/lurq/src/mcp/registry.rs:244`: the action enum is `click`, `double_click`, `move`, `drag`, `wheel`,
  `key`, `type`, `scroll_to`, `request_close`, `menu_activate`. There is no press-only or release-only action.
- `crates/lurq/src/mcp/tools/interact.rs:134-158`: `drag` builds `mouse_move`, `mouse_down`, `STEPS = 6`
  interpolated `mouse_move`s and `mouse_up`, then `apply_all` (`:84`) applies them back to back and requests one
  redraw. All eight events land between two passes.
- `crates/lurq/src/mcp/tools/interact.rs:178-185`: `key` is `key_down` immediately followed by `key_up`.
- `crates/lurq/src/app/synthetic_input.rs`: `SyntheticInputKind::MouseDown` and `MouseUp` exist and are applied
  through the same entry points as real input, but no MCP action exposes them on their own.

## Reproduction

Reduced to the profiler stream, against any lurq app with MCP and `perf_profile`:

1. `lurq_profile_start {}`.
2. `lurq_interact {"action": "drag", "button": "middle", "x": 100, "y": 100, "to_x": 110, "to_y": 100}`, repeated.
3. `lurq_profile_end {"id": …}` and read the `samples` in `sequence` order.

Observed in Kontur Desktop (lurq 0.34.1, middle-button drags over the canvas, 13 calls): every call produced nine
`input_dispatch` samples of category `pointer` (move, down, six moves, up) interleaved only with `ui_update`
samples, followed by exactly one rendered `pass`. No pass ever ran while the button was down. A standalone
lurq-only repro app was not built; the sequence follows from `interact.rs:134-158` and was seen in that capture.

Expected for a frame-rate workload: a way to press, move once per frame for N frames, and release, so each move
is followed by a pass, as with a physical mouse.

What a client can do today is a sequence of complete `drag` calls, one per sample. For panning that is a usable
proxy, because the press and release change nothing. For a node drag or resize it is a different workload: every
call ends with a release, so the app commits one document edit per sample instead of previewing one gesture.

## Impact

- Kontur's frame-time gate cannot measure drag-move, handle resize or marquee at all, and measures space/middle
  pan only through the per-sample `drag` proxy.
- Any held-key behaviour (Space-to-pan held across a drag, Shift held during a gesture) can only be emulated with
  the `modifiers` field, which applies to the events of one call.

## Proposed Fix

Add held-state actions to `lurq_interact`, keeping `drag` as the one-call convenience:

- `pointer_down` (`x`/`y` or `ref`, `button`): `mouse_move` then `mouse_down`; the button stays pressed.
- `move` while a button is held keeps its current meaning, so drag consumers see `on_drag_move` once per call.
- `pointer_up` (`x`/`y` or `ref`, `button`): `mouse_up`.
- `key_down` / `key_up` (`key`, `modifiers`) for held keys.

Safety: a client that disconnects with a button or key held should get an automatic release on session end, the
same way `Tree::mouse_press_taken_by_os` ends a press the OS took, so no stale press survives the MCP session.

Optionally, `drag` could take `steps` and `frames: true` to spread its moves over successive passes.
