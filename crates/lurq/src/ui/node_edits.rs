//! What an app lays on a node of a component besides the style of its part: how the children of the node are
//! aligned, the node's share of a row, where it stands, what it says to tooling. A recipe's `PartStyle` has none
//! of these, so a component that takes them from its own builder (a card that is laid out as a column is) keeps
//! them here and lays them on the node after the part's style.

use std::sync::Arc;

use crate::node::Node;

type Edit = Arc<dyn Fn(&mut Node) + Send + Sync>;

/// The edits of one node, laid in the order they were given: a later one over an earlier one.
#[derive(Clone, Default)]
pub(crate) struct NodeEdits(Vec<Edit>);

impl NodeEdits {
  pub(crate) fn push(&mut self, edit: impl Fn(&mut Node) + Send + Sync + 'static) {
    self.0.push(Arc::new(edit));
  }

  /// Lays every edit on `node`.
  pub(crate) fn apply(&self, node: &mut Node) {
    for edit in &self.0 {
      edit(node);
    }
  }
}
