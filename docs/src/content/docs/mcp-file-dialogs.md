---
title: MCP file selection
description: Request-scoped paths in an application's normal picker handler
---

File selection is explicitly opt-in. Lurq does not intercept `rfd`, open an OS chooser, read files, validate formats, add filename extensions or perform saves. Adapt the same handler used by the normal UI: it either awaits the broker or uses its existing native picker. Never launch both.

```rust,ignore
let dialogs = lurq::mcp::FileDialogBroker::new();
let mcp = tree.enable_mcp(
  lurq::mcp::McpConfig::new()
    .scope(lurq::mcp::Scope::custom("file_dialogs"))
    .file_dialogs(dialogs.clone()),
);
// Give dialogs to the component owning the normal Open button.
// Obtain the actual window handle from ctx.window() in that component.
```

An adapted native handler, using the application's existing `rfd` dependency:

```rust,ignore
async fn pick_design(
  dialogs: &lurq::mcp::FileDialogBroker,
  window: &lurq::app::WindowHandle,
) -> Result<Option<std::path::PathBuf>, lurq::mcp::FileDialogError> {
  use lurq::mcp::{FileDialogError, FileDialogOperation, FileDialogRequest};
  let mut request = FileDialogRequest::new(FileDialogOperation::OpenFile);
  request.title = "Open design".into(); // Use the embedding app's localized copy.
  request.filters = vec![("Design".into(), vec!["kontur".into()])];
  match dialogs.request(window, request)? {
    Some(pending) => match pending.await {
      Ok(selection) => Ok(selection.paths.into_iter().next()),
      Err(FileDialogError::Cancelled) => Ok(None),
      Err(error) => Err(error), // Revocation must NOT launch a native picker.
    },
    None => Ok(rfd::AsyncFileDialog::new()
      .add_filter("Design", &["kontur"])
      .pick_file().await.map(|file| file.path().to_owned())),
  }
}
// The existing handler next checks its captured document/session, reads and
// validates the chosen file, and uses its ordinary transactional admission.
```

Without registration, `request` returns `Ok(None)` and native behavior is unchanged. Once registered, the broker cannot be registered again or revived after shutdown. Intercepted requests stay intercepted across scope changes; errors/cancel do not route them to native dialogs. Dropping the future removes the request immediately. Accepted window close, shutdown, denied dialog tools or scope/serving revocation cancel waiters. A vetoed close request does not cancel them.

With authentication and the `file_dialogs` scope granted, `lurq_file_dialogs` lists pending requests with unique `request_id`, canonical `window` (`main` or `w<ID>`), `operation`, advisory `title`/`filters` and optional `suggested_name`. No tools are added when no broker was registered. Use exactly that identity for `lurq_file_dialog_respond`:

```json
{"request_id":"<UUID>","window":"main","operation":"open_file","action":"select","paths":["H:/designs/screen.kontur"]}
```

Cancel sends `action: "cancel"` without `paths`/`overwrite`. Select accepts one absolute path for `open_file`, `open_folder` or `save_file`, and 1–64 for `open_files`. Unknown/stale/duplicate IDs, wrong window epochs/operations, relative paths and incorrect cardinality are rejected without completing another request. Paths are not normalized. Request metadata/path lengths and 32 pending slots are bounded.

`save_file` selection requires an explicit Boolean `overwrite`. It is returned in `FileDialogSelection` as caller intent, not a toolkit claim that a file exists or replacement is safe. The app must enforce its own existence, permission, overwrite, stale-document and atomic-write/TOCTOU policy before saving. A false flag must not silently replace an existing file; a true flag is not permission to bypass app policy. Filters remain advisory and never replace content validation.

This SDK API cannot steer an unmodified application's direct `rfd` calls. Native chooser visuals, OS ownership and real application import/export remain separate consumer integration checks. No process-global "next path" state or OS input automation is used.
