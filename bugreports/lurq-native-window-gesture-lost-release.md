# Lurq Bug Report: Press That Starts A Native Window Move Or Resize Never Releases

## Status

Resolved on branch `claude/mouse-release-after-move` (base `4b41894`, 0.22.0).

Fix:

- [`Tree::mouse_press_taken_by_os` ends a press through the normal mouse-up path without a click.](../crates/lurq/src/app/runtime.rs)
- [The winit shell calls it once `StartDrag` / `StartResize` hands the press to a native loop, for the main and secondary windows.](../crates/lurq/src/app/winit_shell.rs)

Regression tests ([`input/pointer/native_window_gesture.rs`](../crates/lurq/tests/input/pointer/native_window_gesture.rs)):

- `press_taken_by_os_clears_active_without_clicking`
- `click_after_press_taken_by_os_still_fires_once`
- `late_release_after_press_taken_by_os_does_not_click`
- `press_taken_by_os_without_a_held_button_does_nothing`
- `press_taken_by_os_ends_text_selection_drag_and_keeps_next_click`
- `press_taken_by_os_cancels_drag_without_dropping`

## Summary

Pressing the custom `WindowChrome` title bar or a resize handle starts the native window move or resize loop. The loop consumes the left-button release, so lurq sees the press but never the release. Lurq then treats the left button as held after the gesture ends.

Present since custom chrome was added. It was noted as a known issue during the 0.22.0 live-resize work (`d0e7710`); it is not a regression of that change.

## Environment

- Crate: `lurq` 0.22.0, feature `winit`
- Shell: `WinitWindow`, main or secondary window
- Chrome: `WindowChrome` (custom title bar and resize handles), or any app code that calls `WindowHandle::start_drag` / `start_resize` during a press

## Affected Platforms

| Platform | Native path | Release delivered to the app | Affected |
| --- | --- | --- | --- |
| Windows | Posted `WM_NCLBUTTONDOWN` (`HTCAPTION` / `HT*` edges) runs the `DefWindowProc` modal move/size loop | No. The loop captures the mouse and consumes the button-up. | Yes, move and resize |
| macOS | `drag_window` calls `performWindowDragWithEvent:` with the current mouse-down | No. AppKit tracks the drag itself and the view gets no `mouseUp:`. | Yes, move. `drag_resize_window` is unsupported on macOS, so a resize handle keeps its press and gets the real release. |
| X11 / Wayland | `drag_window` / `drag_resize_window` hand the pointer to the window manager or compositor (`_NET_WM_MOVERESIZE`, `xdg_toplevel.move` / `resize`) | No. The pointer grab moves to the WM/compositor. | Yes (custom chrome is not enabled there by default, but the handle methods are public) |

Only Windows was exercised on this host. The macOS and Linux rows come from the platform APIs and the winit 0.30.13 sources, not a device run.

## Symptoms

After moving or resizing the window from custom chrome:

1. The pressed element keeps its active state, so the title bar or handle can keep an active style.
2. If the press started a text-selection, slider, scrollbar, or `on_drag_*` (`Draggable` in a `DragContainer`) session, that session continues and follows the pointer with no button held.
3. The next release completes the stale press. A click can fire on the element the stale press started on, or a drag can drop where the user merely clicks next.
4. A suppression armed by an ended drag can swallow the next genuine click near the same spot.

## Root Cause

The tree tracks a press as `click_press`, `active_path`, and the drag state (`dragging_scroll`, `dragging_slider`, `dragging_text_selection`, `active_drag`). Only `Tree::mouse_up` clears that state. The shell calls it only from winit's `MouseInput { state: Released }`.

`WindowCommand::StartDrag` / `StartResize` are applied right after the `on_mouse_down` handler that queued them. Once the native loop owns the button, no `MouseInput` release reaches winit's client-area handling on any platform. winit 0.30 also exposes no end-of-loop event (`WM_EXITSIZEMOVE` is not forwarded). `Moved` and `Resized` are not a substitute: a press without movement produces neither, and both also fire for programmatic moves.

## Reproduction

Headless, with the public tree API:

```rust
let mut tree = Tree::new();
tree.set_root(Rect::new(200.0, 32.0).ref_element(title.clone()).on_click(|_| clicked()));
run_pass(&mut tree);
tree.mouse_down(x, y, MouseButton::Left); // title bar handler calls window.start_drag()
// the OS loop consumes the release: nothing more is delivered
assert!(title.active());                  // stays active indefinitely
tree.mouse_up(x, y, MouseButton::Left);   // the next genuine release...
// ...fires the click that belonged to the window move
```

In an app: on Windows, open a window with `WindowChrome`, press the title bar, move the window, release. Then move the pointer over text that was being selected or a `Draggable` that the press started; the selection or drag follows the pointer.

## Fix

The shell now knows the exact moment the OS takes the press: when `start_drag` / `start_resize` actually starts a native loop. On Windows that is the posted non-client press; elsewhere it is `drag_window` / `drag_resize_window` returning `Ok`. The shell then calls `Tree::mouse_press_taken_by_os(cursor_x, cursor_y, MouseButton::Left)`. The OS delivers no pointer input to lurq during the loop, so ending the press when the loop starts is observably the same as ending it when the loop ends. This needs no end-of-loop signal, no window subclassing, and no new `unsafe`. It works the same on every platform.

`mouse_press_taken_by_os`:

- does nothing unless that button has a press or drag in progress;
- drops the pending click press, so no click (or double-click bookkeeping) results;
- ends an `on_drag_*` session with `DropResult::Missed` and no drop handlers, so `DropMissBehavior` applies (for example, revert to the start);
- dispatches `MouseEventKind::Up` through the same `dispatch_mouse` path as a real release. This clears active styles and ends scrollbar, slider, and text-selection drags. `on_mouse_up` handlers run;
- clears any click suppression that the ended drag armed, because no click follows it here and the suppression would otherwise swallow the next real click.

If a platform still delivers the release after its loop, the late release finds no press and produces no click.

## Verification

- `cargo test -p lurq --test input_tests native_window_gesture`: 6 passed. With the method body stubbed out, 4 fail (active state, late click, drag drop, selection). With only the suppression reset removed, the selection test fails because the next real click is swallowed.
- The `Validate crates` commands from `.github/workflows/publish-crates.yml` and `cargo check --workspace --all-features --all-targets` pass.

Manual check (Windows, not automatable without global input): run an app with `WindowChrome` and a title bar whose active style differs from its idle style. Drag the title bar and release. The active style must be gone and no title-bar click handler may fire. Repeat with a resize handle. Then click a button once; it must fire exactly once. On macOS, repeat the title-bar case.
