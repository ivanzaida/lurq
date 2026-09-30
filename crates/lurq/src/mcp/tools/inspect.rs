//! `lurq_inspect` (structured semantic tree and matches) and `lurq_act` (semantic actions on its refs).

use super::{
  canvas_items,
  interact::interact_tool,
  resolve::{find_node, locate_node},
  semantics::{can_hover, can_invoke, inspection_attrs, semantic_actions, semantic_name, semantic_role},
  windows::{requested_window, window_tree_mut},
};
use crate::{
  app::{App, Tree, hit_test::hit_test_tree},
  core::NodeId,
  layout::layout_result::LayoutResult,
  mcp::{
    McpState,
    shared::{McpToolOutput, McpToolResult, RefRecord},
  },
  node::{node::Node, node_kind::NodeKind},
};

pub(super) struct InspectCtx<'a> {
  pub(super) window: &'a str,
  pub(super) scale: f32,
  pub(super) query: Option<String>,
  pub(super) role: Option<String>,
  pub(super) max_depth: usize,
  pub(super) max_nodes: usize,
  pub(super) visited: usize,
  pub(super) truncated: bool,
  pub(super) matches: Vec<serde_json::Value>,
  pub(super) records: &'a mut Vec<RefRecord>,
  pub(super) mint: &'a mut dyn FnMut() -> String,
}

impl InspectCtx<'_> {
  /// Keep `value` (with its ancestor `path`) when a query or role filter is set and it matches.
  pub(super) fn record_match(
    &mut self,
    value: &serde_json::Value,
    role: &str,
    name: Option<&str>,
    element_id: Option<&str>,
    classes: &[String],
    path: &[String],
  ) {
    let matches_role = self.role.as_deref().is_none_or(|expected| expected == role);
    let matches_query = self.query.as_ref().is_none_or(|query| {
      name.is_some_and(|name| name.to_lowercase().contains(query))
        || element_id.is_some_and(|id| id.to_lowercase().contains(query))
        || classes.iter().any(|class| class.to_lowercase().contains(query))
    });
    if (self.query.is_some() || self.role.is_some()) && matches_role && matches_query {
      let mut match_value = value.clone();
      match_value["path"] = serde_json::json!(path);
      self.matches.push(match_value);
    }
  }
}

pub(super) fn semantic_state(node: &Node) -> serde_json::Value {
  let mut state = serde_json::Map::new();
  if node.style_state.is_focused() {
    state.insert("focused".into(), serde_json::json!(true));
  }
  match node.node_kind() {
    NodeKind::TextInput { state: input, .. } if input.is_masked() => {
      state.insert("value".into(), serde_json::json!(input.caret_source_text()));
      state.insert("masked".into(), serde_json::json!(true));
    }
    NodeKind::TextInput { state: input, .. } => {
      state.insert("value".into(), serde_json::json!(input.value()));
    }
    NodeKind::Checkbox { state: checkbox } => {
      state.insert("checked".into(), serde_json::json!(checkbox.is_checked()));
    }
    NodeKind::Slider { state: slider } => {
      state.insert("value".into(), serde_json::json!(slider.value_string()));
    }
    NodeKind::Select { state: select } => {
      let labels = select.labels();
      let selected: Vec<_> = select
        .selected_indices()
        .into_iter()
        .filter_map(|index| labels.get(index).map(|label| label.to_string()))
        .collect();
      state.insert("selected".into(), serde_json::json!(selected));
      state.insert("expanded".into(), serde_json::json!(select.is_open()));
    }
    _ => {}
  }
  serde_json::Value::Object(state)
}

pub(super) fn inspect_node(
  ctx: &mut InspectCtx<'_>,
  node: &Node,
  layout: Option<&LayoutResult>,
  abs: (f32, f32),
  depth: usize,
  path: &[String],
) -> Option<serde_json::Value> {
  if ctx.visited >= ctx.max_nodes {
    ctx.truncated = true;
    return None;
  }
  ctx.visited += 1;
  let role = semantic_role(node);
  let name = semantic_name(node);
  let element_id = node.element_id().map(str::to_owned);
  let classes: Vec<String> = node.class_list().iter().map(ToString::to_string).collect();
  let attrs = inspection_attrs(node);
  let bounds = layout.map(|layout| {
    [
      (abs.0 * ctx.scale).round(),
      (abs.1 * ctx.scale).round(),
      (layout.size.width * ctx.scale).round(),
      (layout.size.height * ctx.scale).round(),
    ]
  });
  let ref_id = (ctx.mint)();
  let actions = semantic_actions(can_invoke(node), can_hover(node));
  ctx.records.push(RefRecord {
    id: ref_id.clone(),
    window: ctx.window.to_owned(),
    node_id: node.node_id(),
    tag: node.tag_name().to_owned(),
    text: node.inspection_text(),
    role: role.clone(),
    name: name.clone(),
    element_id: element_id.clone(),
    classes: classes.clone(),
    attrs: attrs.clone(),
    bounds: bounds.unwrap_or([0.0; 4]),
    interactive: can_invoke(node),
    canvas_item: None,
  });
  let mut value = serde_json::json!({
    "ref": ref_id,
    "role": role,
    "name": name,
    "id": element_id,
    "state": semantic_state(node),
    "actions": actions,
    "bounds": bounds,
  });
  if !classes.is_empty() {
    value["classes"] = serde_json::json!(classes);
  }
  if !attrs.is_empty() {
    value["attrs"] = serde_json::json!(attrs);
  }

  ctx.record_match(&value, &role, name.as_deref(), element_id.as_deref(), &classes, path);

  let mut child_path = path.to_vec();
  if let Some(label) = name.as_ref().or(element_id.as_ref()) {
    child_path.push(format!("{role} {label:?}"));
  }
  let mut children = Vec::new();
  if depth < ctx.max_depth {
    children.extend(canvas_items::inspect_items(ctx, node, &child_path));
    for (index, child) in node.children().iter().enumerate() {
      let child_layout = layout.and_then(|layout| layout.children.get(index));
      let child_abs = child_layout
        .map(|child_layout| (abs.0 + child_layout.offset.x, abs.1 + child_layout.offset.y))
        .unwrap_or(abs);
      if let Some(child) = inspect_node(
        ctx,
        child,
        child_layout.map(|child_layout| child_layout.result.as_ref()),
        child_abs,
        depth + 1,
        &child_path,
      ) {
        children.push(child);
      }
    }
  } else if !node.children().is_empty() {
    ctx.truncated = true;
  }
  if !children.is_empty() {
    value["children"] = serde_json::json!(children);
  }
  Some(value)
}

pub(super) fn inspect_tool(tree: &mut Tree, state: &McpState, args: &serde_json::Value) -> McpToolResult {
  let window = requested_window(args);
  let target = window_tree_mut(tree, &window, state.include_devtools)?;
  target.refresh_dirty_subtrees();
  let root = target
    .root()
    .ok_or_else(|| format!("window {window:?} has no mounted tree"))?;
  let mut records = Vec::new();
  let mut refs = state.shared.refs.lock().unwrap();
  let mut mint = || refs.mint();
  let mut ctx = InspectCtx {
    window: &window,
    scale: target.scale_factor(),
    query: args
      .get("query")
      .and_then(|value| value.as_str())
      .map(str::to_lowercase),
    role: args.get("role").and_then(|value| value.as_str()).map(str::to_owned),
    max_depth: args
      .get("max_depth")
      .and_then(|value| value.as_u64())
      .unwrap_or(12)
      .clamp(1, 100) as usize,
    max_nodes: args
      .get("max_nodes")
      .and_then(|value| value.as_u64())
      .unwrap_or(500)
      .clamp(1, 5000) as usize,
    visited: 0,
    truncated: false,
    matches: Vec::new(),
    records: &mut records,
    mint: &mut mint,
  };
  let layout = if target.layout_is_stale() {
    None
  } else {
    target.last_layout()
  };
  let semantic_tree = inspect_node(&mut ctx, root.node, layout, (0.0, 0.0), 0, &[]);
  let searching = ctx.query.is_some() || ctx.role.is_some();
  let result = if searching {
    serde_json::json!({"window": window, "matches": ctx.matches, "truncated": ctx.truncated})
  } else {
    serde_json::json!({"window": window, "tree": semantic_tree, "truncated": ctx.truncated})
  };
  // Ends the ref table borrow held through `mint`.
  drop(ctx);
  refs.replace_window(&window, records);
  Ok(McpToolOutput::Json(result))
}

pub(super) fn act_tool(tree: &mut Tree, app: &App, state: &McpState, args: &serde_json::Value) -> McpToolResult {
  let ref_id = args
    .get("ref")
    .and_then(|value| value.as_str())
    .ok_or("`ref` is required")?;
  let action = args
    .get("action")
    .and_then(|value| value.as_str())
    .ok_or("`action` is required")?;
  if !matches!(action, "invoke" | "hover") {
    return Err(format!("unsupported semantic action {action:?}"));
  }
  let record = state
    .shared
    .refs
    .lock()
    .unwrap()
    .get(ref_id)
    .cloned()
    .ok_or_else(|| format!("unknown or stale ref {ref_id:?}; call lurq_inspect again"))?;
  {
    let target = window_tree_mut(tree, &record.window, state.include_devtools)?;
    target.refresh_dirty_subtrees();
    let stale = || format!("ref {ref_id:?} no longer resolves to a live element; call lurq_inspect again");
    let node = find_node(target, record.node_id).ok_or_else(stale)?;
    // A canvas item's identity is its registered role and label; it acts
    // through its canvas's handlers.
    let (role, name) = match &record.canvas_item {
      Some(item_id) => canvas_items::item_identity(node, item_id).ok_or_else(stale)?,
      None => (semantic_role(node), semantic_name(node)),
    };
    if role != record.role || name != record.name {
      return Err(format!("ref {ref_id:?} changed role or name; call lurq_inspect again"));
    }
    let supported = if action == "invoke" {
      can_invoke(node)
    } else {
      can_hover(node)
    };
    if !supported {
      return Err(format!("ref {ref_id:?} does not support {action}"));
    }
    if target.layout_is_stale() {
      return Err("UI changed since the last layout; wait for a frame and inspect again".into());
    }
    let [x, y, width, height] = match &record.canvas_item {
      Some(item_id) => canvas_items::item_bounds(node, item_id).ok_or_else(stale)?,
      None => {
        let bounds =
          locate_node(target, record.node_id).ok_or("no layout available yet; wait for a frame before invoking")?;
        if bounds[2] <= 0.0 || bounds[3] <= 0.0 {
          return Err(format!("ref {ref_id:?} has no clickable area"));
        }
        bounds
      }
    };
    let (center_x, center_y) = (x + width * 0.5, y + height * 0.5);
    let root = target.root().ok_or("window has no mounted tree")?;
    let layout = target
      .last_layout()
      .ok_or("no layout available yet; wait for a frame before invoking")?;
    let mut hits = Vec::new();
    hit_test_tree(root.node, layout, 0.0, 0.0, center_x, center_y, &mut hits);
    fn contains_id(node: &Node, id: NodeId) -> bool {
      node.node_id() == id || node.children().iter().any(|child| contains_id(child, id))
    }
    if !hits.iter().any(|(hit, _)| contains_id(node, hit.node_id())) {
      return Err(format!(
        "ref {ref_id:?} is not hittable at its center; scroll or inspect again"
      ));
    }
  }
  let input = if action == "invoke" { "click" } else { "move" };
  interact_tool(tree, app, state, &serde_json::json!({"action": input, "ref": ref_id}))?;
  Ok(McpToolOutput::Json(serde_json::json!({
    "dispatched": true,
    "action": action,
    "ref": ref_id,
    "window": record.window,
  })))
}
