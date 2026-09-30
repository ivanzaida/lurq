//! Resolving refs and node ids against the live tree and layout.

use super::canvas_items;
use crate::{app::Tree, core::NodeId, layout::layout_result::LayoutResult, mcp::McpState, node::node::Node};

/// Absolute logical bounds of a node in its tree, from the last layout pass.
pub(super) fn locate_node(tree: &Tree, node_id: NodeId) -> Option<[f32; 4]> {
  fn walk(node: &Node, layout: &LayoutResult, abs: (f32, f32), node_id: NodeId) -> Option<[f32; 4]> {
    if node.node_id() == node_id {
      return Some([abs.0, abs.1, layout.size.width, layout.size.height]);
    }
    for (index, child) in node.children().iter().enumerate() {
      let child_layout = layout.children.get(index)?;
      if let Some(found) = walk(
        child,
        &child_layout.result,
        (abs.0 + child_layout.offset.x, abs.1 + child_layout.offset.y),
        node_id,
      ) {
        return Some(found);
      }
    }
    None
  }
  let root = tree.root()?;
  let layout = tree.last_layout()?;
  walk(root.node, layout, (0.0, 0.0), node_id)
}

pub(super) fn find_node(tree: &Tree, node_id: NodeId) -> Option<&Node> {
  fn walk(node: &Node, node_id: NodeId) -> Option<&Node> {
    if node.node_id() == node_id {
      return Some(node);
    }
    node.children().iter().find_map(|child| walk(child, node_id))
  }
  walk(tree.root()?.node, node_id)
}

pub(super) struct ResolvedRef {
  pub(super) window: String,
  pub(super) node_id: NodeId,
  pub(super) canvas_item: Option<String>,
}

pub(super) fn resolve_ref(state: &McpState, ref_id: &str) -> Result<ResolvedRef, String> {
  let refs = state.shared.refs.lock().unwrap();
  match refs.get(ref_id) {
    Some(record) => Ok(ResolvedRef {
      window: record.window.clone(),
      node_id: record.node_id,
      canvas_item: record.canvas_item.clone(),
    }),
    None => Err(format!(
      "unknown or stale ref {ref_id:?}; refs are replaced by each lurq_read_tree — call it again"
    )),
  }
}

/// Live logical bounds of a ref: its node from the last layout, or a canvas
/// item from its canvas's current placement.
pub(super) fn ref_bounds(tree: &Tree, resolved: &ResolvedRef, ref_id: &str) -> Result<[f32; 4], String> {
  let stale = || format!("ref {ref_id:?} no longer resolves to a live element; call lurq_read_tree again");
  match &resolved.canvas_item {
    Some(item_id) => find_node(tree, resolved.node_id)
      .and_then(|canvas| canvas_items::item_bounds(canvas, item_id))
      .ok_or_else(stale),
    None => locate_node(tree, resolved.node_id).ok_or_else(stale),
  }
}

/// Physical-pixel center of a ref, resolved against the live layout.
pub(super) fn ref_center_physical(tree: &Tree, resolved: &ResolvedRef, ref_id: &str) -> Result<(f32, f32), String> {
  let [x, y, width, height] = ref_bounds(tree, resolved, ref_id)?;
  let scale = tree.scale_factor();
  Ok(((x + width * 0.5) * scale, (y + height * 0.5) * scale))
}
