//! MCP client sessions as the event loop sees them: an id for each session,
//! and a notice when one ends, so that input it holds can be released.
//!
//! rmcp builds one server handler per streamable-HTTP session and drops it
//! when the session ends: the client deletes it, it expires idle, or the server
//! stops. Each handler owns a [`SessionLease`]; its drop is the notice.
//! Stateless requests get a handler of their own too, but carry no session id
//! (`Mcp-Session-Id`), so no call is routed under their lease.

use std::sync::{
  Arc, Mutex,
  atomic::{AtomicBool, AtomicU64, Ordering},
};

use super::shared::McpShared;

/// Identifies one MCP client session for the lifetime of the server.
pub(crate) type SessionId = u64;

/// Session ids, and the sessions that ended since the event loop last looked.
#[derive(Default)]
pub(crate) struct Sessions {
  next: AtomicU64,
  ended: Mutex<Vec<SessionId>>,
}

impl Sessions {
  fn mint(&self) -> SessionId {
    self.next.fetch_add(1, Ordering::Relaxed) + 1
  }

  /// Sessions that ended since the last call.
  pub(crate) fn take_ended(&self) -> Vec<SessionId> {
    std::mem::take(&mut *self.ended.lock().unwrap())
  }
}

/// The session one server handler serves. Dropped with the handler; if any
/// tool call was routed under it, the event loop is told the session ended.
pub(crate) struct SessionLease {
  id: SessionId,
  routed: AtomicBool,
  shared: Arc<McpShared>,
}

impl SessionLease {
  pub(crate) fn new(shared: Arc<McpShared>) -> Self {
    Self {
      id: shared.sessions.mint(),
      routed: AtomicBool::new(false),
      shared,
    }
  }

  /// The id a call from this session is routed with.
  pub(crate) fn route(&self) -> SessionId {
    self.routed.store(true, Ordering::Relaxed);
    self.id
  }
}

impl Drop for SessionLease {
  fn drop(&mut self) {
    if self.routed.load(Ordering::Relaxed) {
      self.shared.sessions.ended.lock().unwrap().push(self.id);
      self.shared.wake();
    }
  }
}
