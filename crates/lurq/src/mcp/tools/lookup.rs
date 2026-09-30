//! `lurq_find_by_id` / `lurq_find_by_class`: live lookups that append fresh refs.

use super::{
  canvas_items,
  read_tree::format_ref_line,
  resolve::locate_node,
  semantics::{inspection_attrs, is_interactive, semantic_name, semantic_role},
  windows::{requested_window, window_tree_mut},
};
use crate::{
  app::Tree,
  core::NodeId,
  mcp::{
    McpState,
    shared::{McpToolOutput, McpToolResult, RefRecord},
  },
  node::node::Node,
};

/// Owned metadata for a node found by live lookup, captured while the
/// `ElementRef` borrow is alive so bounds can be resolved afterwards.
pub(super) struct LookupHit {
  pub(super) node_id: NodeId,
  pub(super) tag: String,
  pub(super) text: Option<String>,
  pub(super) role: String,
  pub(super) name: Option<String>,
  pub(super) element_id: Option<String>,
  pub(super) classes: Vec<String>,
  pub(super) attrs: Vec<(String, String)>,
  pub(super) interactive: bool,
}

pub(super) fn lookup_hit(node: &Node) -> LookupHit {
  LookupHit {
    node_id: node.node_id(),
    tag: node.tag_name().to_owned(),
    text: node.inspection_text(),
    role: semantic_role(node),
    name: semantic_name(node),
    element_id: node.element_id().map(|id| id.to_owned()),
    classes: node.class_list().iter().map(|class| class.to_string()).collect(),
    attrs: inspection_attrs(node),
    interactive: is_interactive(node),
  }
}

/// Mint a fresh actionable ref for a lookup hit. Appended to the ref table,
/// leaving the window's `read_tree` refs valid.
pub(super) fn register_lookup_ref(target: &Tree, window: &str, hit: LookupHit, state: &McpState) -> RefRecord {
  let scale = target.scale_factor();
  let bounds = locate_node(target, hit.node_id).map(|[x, y, width, height]| {
    [
      (x * scale).round(),
      (y * scale).round(),
      (width * scale).round(),
      (height * scale).round(),
    ]
  });
  let mut refs = state.shared.refs.lock().unwrap();
  let record = RefRecord {
    id: refs.mint(),
    window: window.to_owned(),
    node_id: hit.node_id,
    tag: hit.tag,
    text: hit.text,
    role: hit.role,
    name: hit.name,
    element_id: hit.element_id,
    classes: hit.classes,
    attrs: hit.attrs,
    bounds,
    interactive: hit.interactive,
    canvas_item: None,
  };
  refs.append(vec![record.clone()]);
  record
}

pub(super) fn find_by_id_tool(tree: &mut Tree, state: &McpState, args: &serde_json::Value) -> McpToolResult {
  let id = args
    .get("id")
    .and_then(|value| value.as_str())
    .ok_or("`id` is required")?;
  let window = requested_window(args);
  let target = window_tree_mut(tree, &window, state.include_devtools)?;
  let hit = target.get_element_by_id(id).map(|element| lookup_hit(element.node));
  let record = match hit {
    Some(hit) => register_lookup_ref(target, &window, hit, state),
    None => {
      let Some(item) = canvas_items::find_item_by_id(target, id) else {
        return Ok(McpToolOutput::Text(format!(
          "no element or canvas item with id {id:?} in window {window:?}"
        )));
      };
      let mut refs = state.shared.refs.lock().unwrap();
      let record = canvas_items::lookup_record(refs.mint(), &window, item, target.scale_factor());
      refs.append(vec![record.clone()]);
      record
    }
  };
  Ok(McpToolOutput::Text(format_ref_line(&record)))
}

pub(super) fn find_by_class_tool(tree: &mut Tree, state: &McpState, args: &serde_json::Value) -> McpToolResult {
  let class = args
    .get("class")
    .and_then(|value| value.as_str())
    .ok_or("`class` is required")?;
  let window = requested_window(args);
  let target = window_tree_mut(tree, &window, state.include_devtools)?;
  let hits: Vec<LookupHit> = target
    .get_elements_by_class_name(class)
    .into_iter()
    .map(|element| lookup_hit(element.node))
    .collect();
  if hits.is_empty() {
    return Ok(McpToolOutput::Text(format!(
      "no elements with class {class:?} in window {window:?}"
    )));
  }
  let lines: Vec<String> = hits
    .into_iter()
    .map(|hit| format_ref_line(&register_lookup_ref(target, &window, hit, state)))
    .collect();
  Ok(McpToolOutput::Text(lines.join(
    "
",
  )))
}
