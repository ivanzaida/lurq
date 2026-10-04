//! Sensitive text quads. A capture made for inspection paints over the
//! glyphs of [`Text::sensitive`](crate::components::Text::sensitive) text
//! (see `app::capture_redaction`), so quad resolution records which quads
//! hold such text; the quads themselves carry the text as usual.

use super::LayoutEngine;
use crate::node::node::Node;

impl LayoutEngine {
  /// Records that the quad about to be pushed at `index` holds `node`'s
  /// content, when that content is sensitive text.
  pub(super) fn record_sensitive_quad(&self, node: &Node, index: usize) {
    if node.is_sensitive_text() {
      self.sensitive_quads.borrow_mut().push(index);
    }
  }

  /// Moves the sensitive text quad indices recorded by the last quad
  /// resolution, ascending, into `indices`.
  pub(crate) fn take_sensitive_quads(&self, indices: &mut Vec<usize>) {
    indices.clear();
    std::mem::swap(indices, &mut self.sensitive_quads.borrow_mut());
  }
}
