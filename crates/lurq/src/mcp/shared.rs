//! State shared between the event-loop side (tool execution) and the HTTP
//! server thread (listing, auth, `lurq_find`).

use std::{
  collections::HashSet,
  sync::{
    Arc, Mutex, RwLock,
    atomic::{AtomicBool, Ordering},
  },
};

use super::Scope;
use crate::core::NodeId;

/// What a tool call produced. The server layer converts this into MCP
/// content blocks (`Image` becomes a base64 image block).
pub enum McpToolOutput {
  Text(String),
  Json(serde_json::Value),
  Image { data: Vec<u8>, mime: &'static str },
}

/// Tool outcome: `Err` carries a caller-visible message rendered as an MCP
/// tool error (not a protocol error).
pub type McpToolResult = Result<McpToolOutput, String>;

/// The parked reply for an in-flight tool call. Tool handlers either resolve
/// it inline during the drain or park it for a completion event (frame
/// captures, `lurq_wait`).
pub(crate) type McpReply = tokio::sync::oneshot::Sender<McpToolResult>;

/// One tool call crossing from the HTTP server thread to the event loop.
pub(crate) struct McpRequest {
  pub(crate) tool: String,
  pub(crate) args: serde_json::Value,
  pub(crate) reply: McpReply,
}

/// One `ref_N` handed out by `lurq_read_tree`. Records enough for
/// server-side `lurq_find` and for ref-based tool calls to resolve the node
/// (and its window) without a `window` parameter.
#[derive(Clone)]
pub(crate) struct RefRecord {
  pub(crate) id: String,
  pub(crate) window: String,
  pub(crate) node_id: NodeId,
  pub(crate) tag: String,
  pub(crate) text: Option<String>,
  pub(crate) attrs: Vec<(String, String)>,
  /// Physical (screenshot-pixel) bounds at snapshot time. Interaction
  /// re-resolves live bounds by `node_id`; these are for `lurq_find` output.
  pub(crate) bounds: [f32; 4],
  pub(crate) interactive: bool,
}

/// All refs currently handed out, replaced per window on each
/// `lurq_read_tree`. Ref numbering is monotonic so a stale ref errors as
/// unknown instead of silently aliasing a new node.
#[derive(Default)]
pub(crate) struct RefTable {
  pub(crate) next_ref: u64,
  pub(crate) records: Vec<RefRecord>,
}

impl RefTable {
  pub(crate) fn mint(&mut self) -> String {
    self.next_ref += 1;
    format!("ref_{}", self.next_ref)
  }

  pub(crate) fn replace_window(&mut self, window: &str, records: Vec<RefRecord>) {
    self.records.retain(|record| record.window != window);
    self.records.extend(records);
  }

  pub(crate) fn get(&self, id: &str) -> Option<&RefRecord> {
    self.records.iter().find(|record| record.id == id)
  }
}

/// State reachable from both threads and from `McpHandle`.
pub(crate) struct McpShared {
  enabled: AtomicBool,
  scopes: RwLock<HashSet<Scope>>,
  denied_tools: RwLock<HashSet<String>>,
  pub(crate) refs: Mutex<RefTable>,
  /// Wakes the shell's event loop after enqueuing a request; registered by
  /// the shell at startup. Headless harnesses leave it unset and drain
  /// explicitly.
  pub(crate) waker: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
  pub(crate) token: String,
  pub(crate) app_name: String,
  pub(crate) extra_instructions: Option<String>,
  #[cfg(feature = "router")]
  pub(crate) navigator: RwLock<Option<crate::router::Navigator>>,
}

impl McpShared {
  pub(crate) fn new(
    scopes: HashSet<Scope>,
    denied_tools: HashSet<String>,
    token: String,
    app_name: String,
    extra_instructions: Option<String>,
  ) -> Self {
    Self {
      enabled: AtomicBool::new(true),
      scopes: RwLock::new(scopes),
      denied_tools: RwLock::new(denied_tools),
      refs: Mutex::new(RefTable::default()),
      waker: Mutex::new(None),
      token,
      app_name,
      extra_instructions,
      #[cfg(feature = "router")]
      navigator: RwLock::new(None),
    }
  }

  pub(crate) fn is_enabled(&self) -> bool {
    self.enabled.load(Ordering::Relaxed)
  }

  pub(crate) fn set_enabled(&self, enabled: bool) {
    self.enabled.store(enabled, Ordering::Relaxed);
  }

  pub(crate) fn has_scope(&self, scope: &Scope) -> bool {
    self.scopes.read().unwrap().contains(scope)
  }

  pub(crate) fn set_scopes(&self, scopes: HashSet<Scope>) {
    *self.scopes.write().unwrap() = scopes;
  }

  pub(crate) fn add_scope(&self, scope: Scope) {
    self.scopes.write().unwrap().insert(scope);
  }

  pub(crate) fn remove_scope(&self, scope: &Scope) {
    self.scopes.write().unwrap().remove(scope);
  }

  pub(crate) fn is_denied(&self, tool: &str) -> bool {
    self.denied_tools.read().unwrap().contains(tool)
  }

  pub(crate) fn deny_tool(&self, tool: impl Into<String>) {
    self.denied_tools.write().unwrap().insert(tool.into());
  }

  pub(crate) fn allow_tool(&self, tool: &str) {
    self.denied_tools.write().unwrap().remove(tool);
  }

  pub(crate) fn wake(&self) {
    let waker = self.waker.lock().unwrap().clone();
    if let Some(waker) = waker {
      waker();
    }
  }
}
