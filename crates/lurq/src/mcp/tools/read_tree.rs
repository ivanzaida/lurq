//! `lurq_read_tree`: the indented outline with refs, and the one-line ref format shared with lookups.

use super::{
  canvas_items,
  semantics::{
    attr, inspection_attrs, is_interactive, node_value_summary, semantic_name, semantic_role, token, truncate_text,
  },
  windows::{requested_window, window_tree_mut},
};
use crate::{
  app::Tree,
  layout::layout_result::LayoutResult,
  mcp::{
    McpState,
    shared::{McpToolOutput, McpToolResult, RefRecord},
  },
  node::node::Node,
};

pub(super) struct SnapshotCtx<'a> {
  /// The tree being read, for canvas items' visible regions.
  pub(super) tree: &'a Tree,
  pub(super) window: String,
  pub(super) scale: f32,
  pub(super) all: bool,
  pub(super) max_depth: usize,
  /// Items listed per canvas; the rest are summarized in one line.
  pub(super) max_items: usize,
  pub(super) records: &'a mut Vec<RefRecord>,
  pub(super) mint: &'a mut dyn FnMut() -> String,
}

/// Render one node (and recursively its children) as outline lines.
/// Returns the rendered lines; empty when the branch was pruned.
pub(super) fn snapshot_node(
  ctx: &mut SnapshotCtx<'_>,
  node: &Node,
  layout: Option<&LayoutResult>,
  abs: (f32, f32),
  depth: usize,
) -> Vec<String> {
  let mut child_lines = canvas_items::snapshot_item_lines(ctx, node, depth);
  if ctx.max_depth == 0 || depth < ctx.max_depth {
    let children = node.children();
    for (index, child) in children.iter().enumerate() {
      let child_layout = layout.and_then(|layout| layout.children.get(index));
      let child_abs = match child_layout {
        Some(child_layout) => (abs.0 + child_layout.offset.x, abs.1 + child_layout.offset.y),
        None => abs,
      };
      child_lines.extend(snapshot_node(
        ctx,
        child,
        child_layout.map(|child_layout| child_layout.result.as_ref()),
        child_abs,
        depth + 1,
      ));
    }
  }

  let interactive = is_interactive(node);
  let attrs = inspection_attrs(node);
  let element_id = node.element_id().map(|id| id.to_owned());
  let classes: Vec<String> = node.class_list().iter().map(|class| class.to_string()).collect();
  let labeled = element_id.is_some() || !classes.is_empty();
  let text = node.inspection_text();
  let value = node_value_summary(node);
  let interesting = interactive || labeled || !attrs.is_empty() || text.is_some() || value.is_some();
  if !ctx.all && !interesting && child_lines.is_empty() {
    return Vec::new();
  }

  let bounds = layout.map(|layout| {
    [
      (abs.0 * ctx.scale).round(),
      (abs.1 * ctx.scale).round(),
      (layout.size.width * ctx.scale).round(),
      (layout.size.height * ctx.scale).round(),
    ]
  });

  let mut line = format!("{}- {}", "  ".repeat(depth), token(node.tag_name()));
  if let Some(element_id) = &element_id {
    line.push_str(&format!(" #{}", token(element_id)));
  }
  for class in &classes {
    line.push_str(&format!(" .{}", token(class)));
  }

  if interactive || labeled || !attrs.is_empty() {
    let ref_id = (ctx.mint)();
    line.push_str(&format!(" [{ref_id}]"));
    ctx.records.push(RefRecord {
      id: ref_id,
      window: ctx.window.clone(),
      node_id: node.node_id(),
      tag: node.tag_name().to_owned(),
      text: text.clone(),
      role: semantic_role(node),
      name: semantic_name(node),
      element_id,
      classes,
      attrs: attrs
        .iter()
        .map(|(name, attr_value)| (name.to_string(), attr_value.to_string()))
        .collect(),
      bounds,
      interactive,
      canvas_item: None,
    });
  }

  if let Some(text) = &text {
    line.push_str(&format!(" {:?}", truncate_text(text, 80)));
  }
  if let Some(value) = value {
    line.push(' ');
    line.push_str(&value);
  }
  if let Some([x, y, width, height]) = bounds {
    line.push_str(&format!(" @{x:.0},{y:.0} {width:.0}x{height:.0}"));
  }
  for (name, attr_value) in attrs {
    line.push_str(&attr(&name, &attr_value));
  }
  let mut states = Vec::new();
  if node.style_state.is_hovered() {
    states.push("hovered");
  }
  if node.style_state.is_active() {
    states.push("active");
  }
  if node.style_state.is_focused() {
    states.push("focused");
  }
  if !states.is_empty() {
    line.push_str(&format!(" ({})", states.join(",")));
  }

  let mut lines = vec![line];
  lines.extend(child_lines);
  lines
}

pub(super) fn read_tree_tool(tree: &mut Tree, state: &McpState, args: &serde_json::Value) -> McpToolResult {
  let window = requested_window(args);
  let all = args.get("filter").and_then(|value| value.as_str()) == Some("all");
  let max_depth = args.get("max_depth").and_then(|value| value.as_u64()).unwrap_or(0) as usize;
  let max_items = args.get("max_items").and_then(|value| value.as_u64()).unwrap_or(200) as usize;
  let max_chars = args.get("max_chars").and_then(|value| value.as_u64()).unwrap_or(30_000) as usize;

  let include_devtools = state.include_devtools;
  let target = window_tree_mut(tree, &window, include_devtools)?;
  let scale = target.scale_factor();
  let info = target.window().info();

  let mut records = Vec::new();
  let lines = {
    let mut refs = state.shared.refs.lock().unwrap();
    let mut mint = || refs.mint();
    let Some(root) = target.root() else {
      return Err(format!("window {window:?} has no mounted tree"));
    };
    let mut ctx = SnapshotCtx {
      tree: target,
      window: window.clone(),
      scale,
      all,
      max_depth,
      max_items,
      records: &mut records,
      mint: &mut mint,
    };
    snapshot_node(&mut ctx, root.node, target.last_layout(), (0.0, 0.0), 0)
  };
  state.shared.refs.lock().unwrap().replace_window(&window, records);

  let header = format!(
    "window: {} ({}x{} @{}x)\n",
    token(&window),
    info.resolved_width.round(),
    info.resolved_height.round(),
    scale
  );
  let mut body = lines.join("\n");
  if body.len() > max_chars {
    let mut cut = max_chars;
    while cut > 0 && !body.is_char_boundary(cut) {
      cut -= 1;
    }
    let dropped = body.len() - cut;
    body.truncate(cut);
    body.push_str(&format!(
      "\n… truncated ({dropped} more chars). Raise max_chars, lower max_depth, or keep filter=interactive."
    ));
  }
  Ok(McpToolOutput::Text(format!("{header}{body}")))
}

pub(crate) fn format_ref_line(record: &RefRecord) -> String {
  let mut line = format!(
    "{} [{}] {} role={}",
    record.id,
    token(&record.window),
    token(&record.tag),
    token(&record.role)
  );
  if let Some(element_id) = &record.element_id {
    line.push_str(&format!(" #{}", token(element_id)));
  }
  for class in &record.classes {
    line.push_str(&format!(" .{}", token(class)));
  }
  if let Some(name) = &record.name {
    line.push_str(&format!(" name={:?}", truncate_text(name, 60)));
  }
  if let Some(text) = &record.text
    && record.name.as_ref() != Some(text)
  {
    line.push_str(&format!(" {:?}", truncate_text(text, 60)));
  }
  for (name, value) in &record.attrs {
    line.push_str(&attr(name, value));
  }
  match (record.bounds, record.canvas_item.is_some()) {
    (Some([x, y, width, height]), _) => line.push_str(&format!(" @{x:.0},{y:.0} {width:.0}x{height:.0}")),
    (None, true) => line.push_str(" (not visible)"),
    (None, false) => line.push_str(" (bounds unknown: not laid out yet)"),
  }
  if !record.interactive {
    line.push_str(" (not interactive)");
  }
  line
}
