# Lurq FutureAction modal completion bug report

## Status

Resolved / covered by regression test in `0.15.10`.

The modal completion path is covered by:

- [`tokio_future_action_inside_modal_updates_after_completion`](../crates/lurq/tests/runtime/futures/tokio_future_action_uses_runtime.rs)

The regression opens a modal, clicks a modal button that runs a Tokio-backed `FutureAction`, verifies the modal renders `Pending`, drives the Tokio runtime to completion, ticks futures, and verifies the hosted modal rerenders to `Fulfilled` without external resize/redraw.

## Summary

`FutureAction` completion does not appear to reliably propagate back into a modal-rendered UI when the modal renders progress from the action state directly. In the marketplace app, the create-project modal shows the pending log line (`Indexing game files`) and then remains there, even though the same indexing operation completes in tests.

## Environment

- App: `crates/marketplace`
- Lurq: `0.15.10`
- Enabled Lurq features: `clipboard`, `devtools`, `dx12`, `i18n`, `persistent_storage`, `serde`, `tokio`, `wgpu`, `winit`
- Runtime: Tokio multi-thread runtime passed into `App::new().with_tokio_handle(...)`
- Platform: Windows
- Render shell: `WinitWindow`
- Modal API: `Ctx::Modal` with `ModalTarget::Parent`

## User-visible behavior

1. Open the app.
2. Open the new-project modal.
3. Select a valid game folder.
4. Click `Create project`.
5. The validation log shows:

   ```text
   > Starting project index
   > Folder: ...
   > Indexing game files
   ```

6. The modal does not advance to final success/error log lines.

The indexing operation itself is not the bottleneck. The parser/indexer test against the same data completes successfully:

```powershell
$env:SPOILER_TEST_GAME_DATA='H:\PWElysium\element\data'; cargo test -p marketplace
```

Result:

```text
5 passed; 0 failed
```

## Expected behavior

When the `FutureAction` completes, UI state that reads `action.state()` should observe `FutureStatus::Fulfilled` or `FutureStatus::Rejected`, rerender, and allow the modal to display the completion/error state.

## Actual behavior

The modal visibly reaches the pending state but does not reliably update to completion when completion is handled only by reading `self.create_project.state().get()` during render.

## Original approach

The modal state directly read the future action state during render:

```rust
let state = self.create_project.state().get();

match state.status {
    FutureStatus::Fulfilled => {
        // persist created project, append final logs, refresh recent projects, close modal
    }
    FutureStatus::Rejected => {
        // append error log
    }
    FutureStatus::Pending => {
        // append "Indexing game files"
    }
    FutureStatus::Idle => {}
}
```

This was enough to show the pending line, but the fulfilled/rejected branch did not reliably appear in the modal.

## Current workaround

The app now registers a watcher for the `FutureAction` state and copies fulfilled/rejected completion into separate modal-local signals:

```rust
ctx.watch(&action_state, move |state| match state.status {
    FutureStatus::Fulfilled => {
        if let Some(result) = state.data.as_ref() {
            modal_state.completed_project.set(Some(result.clone()));
            modal_state.completed_error.set(String::new());
        }
    }
    FutureStatus::Rejected => {
        if let Some(error) = state.error.as_ref() {
            modal_state.completed_project.set(None);
            modal_state.completed_error.set(error.clone());
        }
    }
    FutureStatus::Idle | FutureStatus::Pending => {}
});
```

Render then reacts to `completed_project` / `completed_error` instead of relying only on the `FutureAction` state.

## Why this looks like a framework issue

- `FutureAction` reaches `Pending`, because the UI displays the pending log line.
- The same work completes outside the UI path in tests.
- Completion becomes easier to reason about after copying it into normal signals from a watcher.
- Similar modal/render invalidation issues were previously observed: modal open state changed, but the modal only appeared after another external rerender such as window resize.

## Questions for Lurq maintainers

1. Should reading `FutureAction::state().get()` inside render subscribe the component to future completion updates?
2. Should `FutureAction` completion always request/redeliver a redraw when it completes on the Tokio runtime?
3. Are `ctx.watch(...)` callbacks inside modal-rendered components expected to be the recommended pattern for future completion side effects?
4. Can `ModalTarget::Parent` or modal overlay composition affect redraw/subscription propagation?

## Suggested minimal reproduction

1. Create a component rendered inside a `Modal`.
2. Store a `FutureAction<(), String, String>` in component state.
3. On button click, run a Tokio-backed future that sleeps for 1-2 seconds and returns `Ok("done")`.
4. Render text based on `future_action.state().get().status`.
5. Observe whether the modal reliably updates from `Pending` to `Fulfilled` without resizing the window or changing another signal.
