//! Request-scoped file selection. Paths are data; this module never accesses files.
use std::{
  collections::BTreeMap,
  future::Future,
  pin::Pin,
  sync::{Arc, Mutex, Weak},
  task::{Context, Poll},
};

use tokio::sync::oneshot;

use super::{Scope, shared::McpShared};
use crate::app::{WindowHandle, window::DialogWindow};

mod types;
pub use types::{FileDialogError, FileDialogOperation, FileDialogRequest, FileDialogSelection};
#[cfg(test)]
mod tests;

pub(crate) const LIST_TOOL: &str = "lurq_file_dialogs";
pub(crate) const RESPOND_TOOL: &str = "lurq_file_dialog_respond";
const MAX_PENDING: usize = 32;

pub(crate) fn scope() -> Scope {
  Scope::custom("file_dialogs")
}

struct Pending {
  window: DialogWindow,
  request: FileDialogRequest,
  sender: oneshot::Sender<Result<FileDialogSelection, FileDialogError>>,
}

#[derive(Default)]
struct State {
  attached: bool,
  closed: bool,
  shared: Weak<McpShared>,
  pending: BTreeMap<String, Pending>,
}

/// The broker of a tree, as each window of the tree keeps it: what lurq's own
/// [`FileDialog`](crate::files::FileDialog) asks before it opens a system dialog over that window. A window knows
/// the broker of its own tree and no other, so two trees with brokers in one process (an app's tests, run in
/// parallel) never answer each other's dialogs. It does not keep the broker alive.
#[derive(Clone, Default)]
pub(crate) struct ServingBroker(Weak<Mutex<State>>);

impl ServingBroker {
  /// The broker, while it lives.
  pub(crate) fn broker(&self) -> Option<FileDialogBroker> {
    self.0.upgrade().map(FileDialogBroker)
  }

  pub(crate) fn is(&self, other: &Self) -> bool {
    self.0.ptr_eq(&other.0)
  }
}

/// Explicitly opt an application's normal picker handler into MCP selection.
/// Register with `McpConfig::file_dialogs`; before registration requests return
/// `Ok(None)` for native fallback. After registration, cancellation/revocation
/// is an error and must never launch a second native picker.
#[derive(Clone, Default)]
pub struct FileDialogBroker(Arc<Mutex<State>>);

impl FileDialogBroker {
  pub fn new() -> Self {
    Self::default()
  }

  pub(crate) fn attach(&self, shared: &Arc<McpShared>) {
    let mut state = self.0.lock().unwrap();
    assert!(
      !state.attached,
      "a file dialog broker cannot be registered twice or revived"
    );
    state.attached = true;
    state.shared = Arc::downgrade(shared);
  }

  /// The broker as the windows of its tree keep it: see [`ServingBroker`].
  pub(crate) fn serving(&self) -> ServingBroker {
    ServingBroker(Arc::downgrade(&self.0))
  }

  fn permitted(state: &State) -> bool {
    !state.closed
      && state.shared.upgrade().is_some_and(|shared| {
        shared.is_enabled()
          && shared.has_scope(&scope())
          && !shared.is_denied(LIST_TOOL)
          && !shared.is_denied(RESPOND_TOOL)
      })
  }

  /// Register the waiter atomically before exposing its unique request ID.
  /// The supplied handle is the actual window owning this UI operation.
  pub fn request(
    &self,
    window: &WindowHandle,
    request: FileDialogRequest,
  ) -> Result<Option<FileDialogFuture>, FileDialogError> {
    let mut state = self.0.lock().unwrap();
    if !state.attached {
      return Ok(None);
    }
    request.validate()?;
    if !Self::permitted(&state) {
      return Err(FileDialogError::Unavailable);
    }
    let window = window.dialog_window();
    if window.closing() {
      return Err(FileDialogError::WindowClosed);
    }
    if state.pending.len() >= MAX_PENDING {
      return Err(FileDialogError::TooManyPending);
    }
    let id = uuid::Uuid::new_v4().to_string();
    let (sender, receiver) = oneshot::channel();
    state.pending.insert(
      id.clone(),
      Pending {
        window,
        request,
        sender,
      },
    );
    Ok(Some(FileDialogFuture {
      broker: self.clone(),
      id,
      receiver,
    }))
  }

  pub(crate) fn permission_changed(&self) {
    let mut state = self.0.lock().unwrap();
    if state.attached && !Self::permitted(&state) {
      let pending = std::mem::take(&mut state.pending);
      drop(state);
      Self::cancel_all(pending, FileDialogError::Unavailable);
    }
  }

  pub(crate) fn shutdown(&self) {
    let mut state = self.0.lock().unwrap();
    state.closed = true;
    let pending = std::mem::take(&mut state.pending);
    drop(state);
    Self::cancel_all(pending, FileDialogError::Unavailable);
  }

  fn cancel_all(pending: BTreeMap<String, Pending>, error: FileDialogError) {
    for (_, pending) in pending {
      let _ = pending.sender.send(Err(error));
    }
  }

  /// Run by the ordinary root drain/pass, including when no MCP call arrives. Every window of the tree is told
  /// which broker serves it, one opened since the last pass too.
  pub(crate) fn reconcile(&self, windows: &[(String, crate::app::Window)]) {
    let serving = self.serving();
    for (_, window) in windows {
      window.serve_file_dialogs(&serving);
    }
    let mut state = self.0.lock().unwrap();
    let gone: Vec<_> = state
      .pending
      .iter()
      .filter(|(_, pending)| {
        pending.window.closing() || !windows.iter().any(|(_, window)| pending.window.matches(window))
      })
      .map(|(id, _)| id.clone())
      .collect();
    let pending: Vec<_> = gone.into_iter().filter_map(|id| state.pending.remove(&id)).collect();
    drop(state);
    for pending in pending {
      let _ = pending.sender.send(Err(FileDialogError::WindowClosed));
    }
  }

  pub(crate) fn list(&self, windows: &[(String, crate::app::Window)]) -> serde_json::Value {
    let state = self.0.lock().unwrap();
    let requests: Vec<_> = state
      .pending
      .iter()
      .filter_map(|(id, pending)| {
        let (window, _) = windows.iter().find(|(_, window)| pending.window.matches(window))?;
        Some(serde_json::json!({
          "request_id": id, "window": window,
          "operation": pending.request.operation.as_str(),
          "title": pending.request.title, "filters": pending.request.filters,
          "suggested_name": pending.request.suggested_name,
        }))
      })
      .collect();
    serde_json::json!({ "requests": requests })
  }

  pub(crate) fn respond(
    &self,
    id: &str,
    window: &crate::app::Window,
    operation: FileDialogOperation,
    selection: Option<FileDialogSelection>,
  ) -> Result<(), FileDialogError> {
    let mut state = self.0.lock().unwrap();
    if !Self::permitted(&state) {
      return Err(FileDialogError::Unavailable);
    }
    let pending = state.pending.get(id).ok_or(FileDialogError::StaleRequest)?;
    if !pending.window.matches(window) || pending.window.closing() {
      return Err(FileDialogError::WindowClosed);
    }
    if pending.request.operation != operation {
      return Err(FileDialogError::WrongOperation);
    }
    if let Some(selection) = &selection {
      selection.validate(operation)?;
    }
    let pending = state.pending.remove(id).unwrap();
    drop(state);
    // Removal precedes completion/wake: response vs drop has one winner.
    pending
      .sender
      .send(selection.ok_or(FileDialogError::Cancelled))
      .map_err(|_| FileDialogError::StaleRequest)
  }
}

/// Dropping the future immediately frees its bounded request slot.
pub struct FileDialogFuture {
  broker: FileDialogBroker,
  id: String,
  receiver: oneshot::Receiver<Result<FileDialogSelection, FileDialogError>>,
}

impl Future for FileDialogFuture {
  type Output = Result<FileDialogSelection, FileDialogError>;

  fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    Pin::new(&mut self.receiver)
      .poll(cx)
      .map(|result| result.unwrap_or(Err(FileDialogError::Unavailable)))
  }
}

impl Drop for FileDialogFuture {
  fn drop(&mut self) {
    let pending = self.broker.0.lock().unwrap().pending.remove(&self.id);
    drop(pending);
  }
}
