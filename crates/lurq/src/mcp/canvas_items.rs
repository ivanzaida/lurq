//! Canvas semantic items ([`crate::canvas::CanvasItem`]) as MCP tree children.
//!
//! Items are listed under their canvas in `lurq_read_tree` and `lurq_inspect`,
//! get refs like elements, and resolve to live window bounds through the
//! canvas placement, which already includes padding, scrolling and ancestor
//! transforms. They act through their canvas's pointer handlers. Without the
//! `canvas` feature every function here reports no items.

use super::{InspectCtx, RefRecord, SnapshotCtx, semantic_actions};
use crate::{app::Tree, core::NodeId, node::node::Node};

/// Tag of item refs in lookup output; the item's own role is its `role`.
const ITEM_TAG: &str = "CanvasItem";

/// A canvas item found by id, with what its ref record needs.
pub(super) struct ItemHit {
  pub(super) node_id: NodeId,
  pub(super) record: ItemRecord,
}

/// One item with its live window-logical bounds and its canvas's capabilities.
pub(super) struct ItemRecord {
  id: String,
  role: String,
  label: Option<String>,
  value: Option<String>,
  bounds: Option<[f32; 4]>,
  invoke: bool,
  hover: bool,
}

/// `items=N` for canvas nodes, so an agent can tell a described canvas from opaque pixels.
pub(super) fn canvas_attrs(node: &Node) -> Option<(String, String)> {
  #[cfg(feature = "canvas")]
  {
    node
      .canvas_handle()
      .map(|canvas| ("items".to_owned(), canvas.items().len().to_string()))
  }
  #[cfg(not(feature = "canvas"))]
  {
    let _ = node;
    None
  }
}

/// Item records of a canvas node, in registration order; empty for other nodes.
fn item_records(node: &Node) -> Vec<ItemRecord> {
  #[cfg(feature = "canvas")]
  {
    let Some(canvas) = node.canvas_handle() else {
      return Vec::new();
    };
    let (invoke, hover) = (super::can_invoke(node), super::can_hover(node));
    canvas
      .items_in_window()
      .into_iter()
      .map(|(item, bounds)| ItemRecord {
        id: item.id,
        role: item.role,
        label: item.label,
        value: item.value,
        bounds,
        invoke,
        hover,
      })
      .collect()
  }
  #[cfg(not(feature = "canvas"))]
  {
    let _ = node;
    Vec::new()
  }
}

/// Outline lines for a canvas node's items, one level below it, minting a ref per item.
pub(super) fn snapshot_item_lines(ctx: &mut SnapshotCtx<'_>, node: &Node, depth: usize) -> Vec<String> {
  if ctx.max_depth != 0 && depth >= ctx.max_depth {
    return Vec::new();
  }
  let records = item_records(node);
  let mut lines = Vec::with_capacity(records.len());
  for item in records {
    let has_bounds = item.bounds.is_some();
    let record = to_ref_record((ctx.mint)(), &ctx.window, node.node_id(), item, ctx.scale);
    let mut line = format!(
      "{}- {} #{} [{}]",
      "  ".repeat(depth + 1),
      record.role,
      record.canvas_item.as_deref().unwrap_or_default(),
      record.id
    );
    if let Some(label) = &record.name {
      line.push_str(&format!(" {label:?}"));
    }
    if has_bounds {
      let [x, y, width, height] = record.bounds;
      line.push_str(&format!(" @{x:.0},{y:.0} {width:.0}x{height:.0}"));
    }
    for (name, value) in &record.attrs {
      line.push_str(&format!(" {{{name}={value}}}"));
    }
    lines.push(line);
    ctx.records.push(record);
  }
  lines
}

/// Semantic nodes for a canvas node's items, as children in `lurq_inspect`.
/// They count toward `max_nodes` and match `query`/`role` like elements.
pub(super) fn inspect_items(ctx: &mut InspectCtx<'_>, node: &Node, path: &[String]) -> Vec<serde_json::Value> {
  let mut values = Vec::new();
  for item in item_records(node) {
    if ctx.visited >= ctx.max_nodes {
      ctx.truncated = true;
      break;
    }
    ctx.visited += 1;
    let has_bounds = item.bounds.is_some();
    let actions = semantic_actions(item.invoke, item.hover);
    let state = match &item.value {
      Some(value) => serde_json::json!({ "value": value }),
      None => serde_json::json!({}),
    };
    let record = to_ref_record((ctx.mint)(), ctx.window, node.node_id(), item, ctx.scale);
    let value = serde_json::json!({
      "ref": record.id,
      "role": record.role,
      "name": record.name,
      "id": record.canvas_item,
      "canvas_item": true,
      "state": state,
      "actions": actions,
      "bounds": has_bounds.then_some(record.bounds),
    });
    ctx.record_match(
      &value,
      &record.role,
      record.name.as_deref(),
      record.canvas_item.as_deref(),
      &[],
      path,
    );
    ctx.records.push(record);
    values.push(value);
  }
  values
}

/// The item's ref: `CanvasItem` tag, its role and label as role and name, its
/// id as `#id`, its value as an attribute, bounds in screenshot pixels.
fn to_ref_record(ref_id: String, window: &str, node_id: NodeId, item: ItemRecord, scale: f32) -> RefRecord {
  RefRecord {
    id: ref_id,
    window: window.to_owned(),
    node_id,
    tag: ITEM_TAG.to_owned(),
    text: None,
    role: item.role,
    name: item.label,
    element_id: Some(item.id.clone()),
    classes: Vec::new(),
    attrs: item
      .value
      .map(|value| vec![("value".to_owned(), value)])
      .unwrap_or_default(),
    bounds: item
      .bounds
      .map_or([0.0; 4], |bounds| bounds.map(|value| (value * scale).round())),
    interactive: item.invoke || item.hover,
    canvas_item: Some(item.id),
  }
}

/// First canvas item with this id in tree order, searched after element ids.
pub(super) fn find_item_by_id(tree: &Tree, id: &str) -> Option<ItemHit> {
  fn walk(node: &Node, id: &str) -> Option<ItemHit> {
    if let Some(record) = item_records(node).into_iter().find(|record| record.id == id) {
      return Some(ItemHit {
        node_id: node.node_id(),
        record,
      });
    }
    node.children().iter().find_map(|child| walk(child, id))
  }
  walk(tree.root()?.node, id)
}

/// A fresh ref record for a lookup hit, in screenshot pixels.
pub(super) fn lookup_record(ref_id: String, window: &str, hit: ItemHit, scale: f32) -> RefRecord {
  to_ref_record(ref_id, window, hit.node_id, hit.record, scale)
}

fn live_item(node: &Node, item_id: &str) -> Option<ItemRecord> {
  item_records(node).into_iter().find(|record| record.id == item_id)
}

/// Live window-logical bounds of a canvas item, re-resolved at action time.
pub(super) fn item_bounds(node: &Node, item_id: &str) -> Option<[f32; 4]> {
  live_item(node, item_id).and_then(|record| record.bounds)
}

/// Live `(role, name)` of a canvas item, compared with its ref before `lurq_act`.
pub(super) fn item_identity(node: &Node, item_id: &str) -> Option<(String, Option<String>)> {
  live_item(node, item_id).map(|record| (record.role, record.label))
}
