//! Canvas semantic items ([`crate::canvas::CanvasItem`]) as MCP tree children.
//!
//! Items are listed under their canvas in `lurq_read_tree`, get refs like
//! elements, and resolve to live window bounds through the canvas placement,
//! which already includes padding, scrolling and ancestor transforms. Without
//! the `canvas` feature every function here reports no items.

use super::{RefRecord, SnapshotCtx};
use crate::{app::Tree, core::NodeId, node::node::Node};

/// A canvas item found by id, with what its ref record needs.
pub(super) struct ItemHit {
  pub(super) node_id: NodeId,
  pub(super) record: ItemRecord,
}

pub(super) struct ItemRecord {
  id: String,
  role: String,
  label: Option<String>,
  value: Option<String>,
  bounds: Option<[f32; 4]>,
  interactive: bool,
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
    let interactive = item_interactive(node);
    canvas
      .items_in_window()
      .into_iter()
      .map(|(item, bounds)| ItemRecord {
        id: item.id,
        role: item.role,
        label: item.label,
        value: item.value,
        bounds,
        interactive,
      })
      .collect()
  }
  #[cfg(not(feature = "canvas"))]
  {
    let _ = node;
    Vec::new()
  }
}

/// Items act through their canvas's handlers; hover-only canvases count, since
/// moving the pointer onto an item is how their tooltips open.
#[cfg(feature = "canvas")]
fn item_interactive(node: &Node) -> bool {
  super::is_interactive(node) || !node.events.on_mouse_move.is_empty() || !node.events.on_mouse_enter.is_empty()
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
      record.tag,
      record.canvas_item.as_deref().unwrap_or_default(),
      record.id
    );
    if let Some(label) = &record.text {
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

/// The item's ref: role as tag, item id as `#id`, label as text, value as an attribute.
fn to_ref_record(ref_id: String, window: &str, node_id: NodeId, item: ItemRecord, scale: f32) -> RefRecord {
  RefRecord {
    id: ref_id,
    window: window.to_owned(),
    node_id,
    tag: item.role.clone(),
    text: item.label.clone(),
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
    interactive: item.interactive,
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

/// Live window-logical bounds of a canvas item, re-resolved at action time.
pub(super) fn item_bounds(node: &Node, item_id: &str) -> Option<[f32; 4]> {
  item_records(node)
    .into_iter()
    .find(|record| record.id == item_id)
    .and_then(|record| record.bounds)
}
