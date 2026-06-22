# Lurq Bug Report: Modal Opens In Tree But Does Not Paint Until External Redraw

## Status

Resolved.

Fix:

- [`Tree::pass()` now treats component-dirty redraws as required frame passes.](../crates/lurq/src/app/runtime.rs)
- [`update_layout()` now preserves the pre-rebuild component dirty state so modal/overlay host reconstruction cannot be skipped by the layout fast path.](../crates/lurq/src/app/runtime.rs)

Regression tests:

- [`signal_opened_parent_modal_requires_and_presents_next_pass_without_explicit_redraw`](../crates/lurq/tests/runtime/pass_report.rs)
- [`signal_opened_root_modal_requires_and_presents_next_pass_without_explicit_redraw`](../crates/lurq/tests/runtime/pass_report.rs)
- [`click_opened_parent_modal_presents_next_pass_after_event_flush`](../crates/lurq/tests/runtime/pass_report.rs)
- [`click_opened_root_modal_presents_next_pass_after_event_flush`](../crates/lurq/tests/runtime/pass_report.rs)

## Summary

Opening a `Modal` from a click handler updates the `Signal<bool>` and rerenders the component tree immediately, but the modal is not visually painted until an external redraw is forced, for example by resizing the window.

The modal content is built with `open=true`, so this does not appear to be an event handler or signal propagation issue. It looks like the modal/overlay host update is not requesting or presenting a redraw after the state change.

## Environment

- OS: Windows
- Crate: `lurq`
- Version: `0.15.8`
- Enabled Lurq features:

```toml
features = [
  "clipboard",
  "dx12",
  "i18n",
  "persistent_storage",
  "serde",
  "wgpu",
  "winit",
]
```

- Renderer: `WgpuRenderEngine`
- Shell: `WinitWindow`
- Window size: `1280x760`

## Expected Behavior

When a click handler sets the modal open signal to `true`, the modal should appear in the next rendered frame without requiring resize, focus changes, or any other external redraw trigger.

## Actual Behavior

After clicking the button:

1. The click handler runs.
2. The modal `open` signal becomes `true`.
3. The startup component rerenders.
4. The modal node/content/dialog are built with `open=true`.
5. Nothing changes visually.
6. If the window is resized, the already-open modal appears.

## Relevant Logs

These logs were added around the button click, startup render, and modal construction:

```text
[marketplace:start] startup screen component created
[marketplace:start] new project modal state created; open=false
[marketplace:start] startup render; layout=Compact; projects=0; filtered=0; search_len=0; modal_open=false
[marketplace:start] new project modal node built; open=false target=Parent
[marketplace:start] new project modal content built; open=false
[marketplace:start] new project modal dialog built; open=false
[marketplace:start] startup render; layout=NarrowDesktop; projects=0; filtered=0; search_len=0; modal_open=false
[marketplace:start] new project modal node built; open=false target=Parent
[marketplace:start] new project modal content built; open=false
[marketplace:start] new project modal dialog built; open=false
[marketplace:start] add game folder clicked; open before set=false
[marketplace:start] add game folder click handled; open after set=true
[marketplace:start] startup render; layout=NarrowDesktop; projects=0; filtered=0; search_len=0; modal_open=true
[marketplace:start] new project modal node built; open=true target=Parent
[marketplace:start] new project modal content built; open=true
[marketplace:start] new project modal dialog built; open=true
```

The final four lines prove that the state and modal subtree are already updated before the resize workaround.

## Reduced Shape Of The App Code

The app root is a component mounted into a Winit/WGPU tree:

```rust
let mut tree = Tree::new();
tree.set_render_engine_factory(|| Box::new(WgpuRenderEngine::new()));
tree.mount_root::<StartupScreen>(&mut app, projects);
tree.request_redraw();

WinitWindow::new(app, tree)
    .with_title("Spoiler Market")
    .with_size(1280, 760)
    .with_min_size(1280, 760)
    .run();
```

The modal state is owned by the root component:

```rust
pub(crate) struct StartupScreen {
    search_query: Signal<String>,
    new_project: NewProjectModalState,
}

pub(crate) struct NewProjectModalState {
    pub(crate) open: Signal<bool>,
    // other fields omitted
}
```

The button flips the signal:

```rust
primary_button(ctx.theme(), "Add game folder", Some(LucideIcon::Box))
    .on_click(move |_| {
        open_new_project.set(true);
    })
```

The modal is mounted as a child of a fullscreen `Stack`:

```rust
Stack::new()
    .width(Dimension::full())
    .height(Dimension::full())
    .background(PaletteColor::SurfaceBase)
    .child(content)
    .child(
        Modal::new(new_project_modal_content(ctx, &new_project))
            .open(new_project.open.clone())
            .target(ModalTarget::Parent)
            .dismiss_on_escape(true),
    )
```

This was also reproduced with `ModalTarget::Root`; the modal still only appeared after resize.

## Suspected Area

The issue appears to be in redraw invalidation/presentation after a modal declaration changes from closed to open.

The component tree rerenders immediately, but the Winit/WGPU window does not present the new modal frame until another window event forces a redraw. This suggests one of:

- opening a `Modal` through `OpenState::Signal` does not request a redraw when the overlay host changes;
- the overlay/modal host is rebuilt in the tree but the Winit shell does not schedule `request_redraw`;
- the render engine receives the update but does not present it until the next external window redraw.

## Workaround

Resizing the window forces a redraw and the modal appears.

A possible app-side workaround would be to explicitly request a redraw after opening the modal, but the click handler only has local UI state and does not currently have a `Tree`/window redraw handle.

## Acceptance Criteria For Fix

1. Click button that sets `modal_open.set(true)`.
2. Modal appears without resizing the window.
3. Closing the modal with escape or a button also visually updates without requiring a forced redraw.
4. Works for both `ModalTarget::Parent` and `ModalTarget::Root`.

Verified with:

```powershell
cargo test -p lurq signal_opened_ --test runtime_tests
cargo test -p lurq --test runtime_tests
cargo test -p lurq --test reactivity_tests modal
```
