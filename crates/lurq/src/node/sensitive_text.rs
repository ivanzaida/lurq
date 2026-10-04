//! Text that is drawn but kept out of inspection (`Text::sensitive`).

use super::{node::Node, node_kind::NodeKind};

impl Node {
  /// Marks a text node's content as sensitive: it is laid out and painted as usual, and inspectors (DevTools, the
  /// MCP tree) show [`REDACTED`](crate::core::REDACTED) instead.
  pub(crate) fn set_text_sensitive(&mut self) {
    if let NodeKind::Text { state, .. } = &self.node_kind {
      state.set_sensitive();
    }
  }

  #[cfg(any(feature = "mcp", feature = "devtools"))]
  pub(crate) fn is_sensitive_text(&self) -> bool {
    matches!(self.node_kind(), NodeKind::Text { state, .. } if state.is_sensitive())
  }
}
