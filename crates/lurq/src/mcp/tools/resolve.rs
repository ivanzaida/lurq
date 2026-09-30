//! Resolving refs and node ids against the live tree and layout.

use super::canvas_items;
use crate::{
  app::{Tree, hit_test::hit_test_tree},
  core::NodeId,
  layout::layout_result::LayoutResult,
  mcp::McpState,
  node::node::Node,
};

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

/// Intersection of two `[x, y, width, height]` boxes; `None` when disjoint.
/// Touching or zero-size results are kept.
#[cfg(feature = "canvas")]
pub(super) fn intersect(a: [f32; 4], b: [f32; 4]) -> Option<[f32; 4]> {
  let left = a[0].max(b[0]);
  let top = a[1].max(b[1]);
  let right = (a[0] + a[2]).min(b[0] + b[2]);
  let bottom = (a[1] + a[3]).min(b[1] + b[3]);
  (right >= left && bottom >= top).then_some([left, top, right - left, bottom - top])
}

/// Where a node's content can be seen and hit.
#[cfg(feature = "canvas")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Visibility {
  /// The node is not laid out yet (or not in the tree): no clip is known.
  Unknown,
  /// A clipping ancestor or the window cuts all of it off.
  Hidden,
  /// The window-logical region its content can occupy.
  Region([f32; 4]),
}

/// Where a node's content can be seen and hit: the window, narrowed by every
/// ancestor that clips its overflow (scroll viewports included), each placed
/// with its ancestors' transforms and its own, the way hit testing and canvas
/// placement compose them.
#[cfg(feature = "canvas")]
pub(super) fn visible_clip(tree: &Tree, node_id: NodeId) -> Visibility {
  use crate::{layout::layout_kind::Overflow, node::transform::Transform2D};

  /// `None` when `node_id` is not in this subtree; `Some(None)` when it is but
  /// is clipped away.
  fn walk(
    node: &Node,
    layout: &LayoutResult,
    abs: (f32, f32),
    inherited: Transform2D,
    clip: Option<[f32; 4]>,
    node_id: NodeId,
  ) -> Option<Option<[f32; 4]>> {
    if node.node_id() == node_id {
      return Some(clip);
    }
    let (width, height) = (layout.size.width, layout.size.height);
    let local = node
      .effective_transform()
      .around_origin([abs.0 + width * 0.5, abs.1 + height * 0.5]);
    let composed = inherited.then(&local);
    let clip = if node.overflow == Overflow::Visible {
      clip
    } else {
      clip.and_then(|clip| intersect(clip, transformed_box(composed, [abs.0, abs.1, width, height])))
    };
    node.children().iter().enumerate().find_map(|(index, child)| {
      let child_layout = layout.children.get(index)?;
      let offset = (abs.0 + child_layout.offset.x, abs.1 + child_layout.offset.y);
      walk(child, &child_layout.result, offset, composed, clip, node_id)
    })
  }
  let info = tree.window().info();
  let scale = tree.scale_factor();
  let window = if info.resolved_width > 0.0 && info.resolved_height > 0.0 {
    [0.0, 0.0, info.resolved_width / scale, info.resolved_height / scale]
  } else {
    [f32::MIN / 4.0, f32::MIN / 4.0, f32::MAX / 2.0, f32::MAX / 2.0]
  };
  let (Some(root), Some(layout)) = (tree.root(), tree.last_layout()) else {
    return Visibility::Unknown;
  };
  match walk(
    root.node,
    layout,
    (0.0, 0.0),
    Transform2D::IDENTITY,
    Some(window),
    node_id,
  ) {
    None => Visibility::Unknown,
    Some(None) => Visibility::Hidden,
    Some(Some(region)) => Visibility::Region(region),
  }
}

/// Axis-aligned box enclosing `[x, y, width, height]` after `transform`.
#[cfg(feature = "canvas")]
fn transformed_box(transform: crate::node::transform::Transform2D, [x, y, width, height]: [f32; 4]) -> [f32; 4] {
  if transform.is_identity() {
    return [x, y, width, height];
  }
  let corners = [(x, y), (x + width, y), (x, y + height), (x + width, y + height)]
    .map(|(corner_x, corner_y)| transform.transform_point(corner_x, corner_y));
  let left = corners.iter().map(|corner| corner.0).fold(f32::INFINITY, f32::min);
  let top = corners.iter().map(|corner| corner.1).fold(f32::INFINITY, f32::min);
  let right = corners.iter().map(|corner| corner.0).fold(f32::NEG_INFINITY, f32::max);
  let bottom = corners.iter().map(|corner| corner.1).fold(f32::NEG_INFINITY, f32::max);
  [left, top, right - left, bottom - top]
}

/// Whether a pointer at the window-logical point would hit `node` or one of
/// its descendants.
pub(super) fn hits_node(tree: &Tree, node: &Node, x: f32, y: f32) -> bool {
  fn contains_id(node: &Node, id: NodeId) -> bool {
    node.node_id() == id || node.children().iter().any(|child| contains_id(child, id))
  }
  let (Some(root), Some(layout)) = (tree.root(), tree.last_layout()) else {
    return false;
  };
  let mut hits = Vec::new();
  hit_test_tree(root.node, layout, 0.0, 0.0, x, y, &mut hits);
  hits.iter().any(|(hit, _)| contains_id(node, hit.node_id()))
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

fn stale(ref_id: &str) -> String {
  format!("ref {ref_id:?} no longer resolves to a live element; call lurq_read_tree again")
}

/// Live logical bounds of a ref: its node from the last layout, or the
/// visible part of a canvas item from its canvas's current placement.
pub(super) fn ref_bounds(tree: &Tree, resolved: &ResolvedRef, ref_id: &str) -> Result<[f32; 4], String> {
  item_or_node_bounds(tree, resolved, ref_id, true)
}

/// What `scroll_to` should bring into view: the node, or the part of a canvas
/// item inside its canvas, visible or not.
pub(super) fn ref_scroll_bounds(tree: &Tree, resolved: &ResolvedRef, ref_id: &str) -> Result<[f32; 4], String> {
  item_or_node_bounds(tree, resolved, ref_id, false)
}

fn item_or_node_bounds(tree: &Tree, resolved: &ResolvedRef, ref_id: &str, on_screen: bool) -> Result<[f32; 4], String> {
  match &resolved.canvas_item {
    Some(item_id) => {
      let canvas = find_node(tree, resolved.node_id).ok_or_else(|| stale(ref_id))?;
      canvas_items::item_bounds(tree, canvas, item_id, on_screen, ref_id)
    }
    None => locate_node(tree, resolved.node_id).ok_or_else(|| stale(ref_id)),
  }
}

/// Physical-pixel center of a ref, resolved against the live layout. A canvas
/// item's center must reach its canvas: input there would otherwise land on
/// whatever covers or surrounds it.
pub(super) fn ref_center_physical(tree: &Tree, resolved: &ResolvedRef, ref_id: &str) -> Result<(f32, f32), String> {
  let [x, y, width, height] = ref_bounds(tree, resolved, ref_id)?;
  let center = (x + width * 0.5, y + height * 0.5);
  if resolved.canvas_item.is_some() {
    let canvas = find_node(tree, resolved.node_id).ok_or_else(|| stale(ref_id))?;
    if !hits_node(tree, canvas, center.0, center.1) {
      return Err(format!(
        "canvas item ref {ref_id:?} is covered at its center; scroll_to it or read the tree again"
      ));
    }
  }
  let scale = tree.scale_factor();
  Ok((center.0 * scale, center.1 * scale))
}
