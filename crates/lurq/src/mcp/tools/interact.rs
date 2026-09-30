//! `lurq_interact`: synthetic pointer and keyboard input, and `scroll_to`.

use super::{
  resolve::{ref_center_physical, ref_scroll_bounds, resolve_ref},
  windows::{requested_window, window_tree_mut},
};
use crate::{
  app::{
    App, Tree,
    events::MouseButton,
    synthetic_input::{self, SyntheticInput, SyntheticModifiers},
  },
  core::NodeId,
  layout::{layout_kind::LayoutKind, layout_result::LayoutResult},
  mcp::{
    McpState,
    shared::{McpToolOutput, McpToolResult},
  },
  node::node::Node,
};

pub(super) fn parse_modifiers(args: &serde_json::Value) -> SyntheticModifiers {
  let mut modifiers = SyntheticModifiers::default();
  if let Some(list) = args.get("modifiers").and_then(|value| value.as_array()) {
    for entry in list.iter().filter_map(|value| value.as_str()) {
      match entry {
        "shift" => modifiers.shift = true,
        "ctrl" => modifiers.ctrl = true,
        "alt" => modifiers.alt = true,
        "meta" => modifiers.meta = true,
        _ => {}
      }
    }
  }
  modifiers
}

pub(super) fn parse_button(args: &serde_json::Value) -> MouseButton {
  match args.get("button").and_then(|value| value.as_str()) {
    Some("right") => MouseButton::Right,
    Some("middle") => MouseButton::Middle,
    _ => MouseButton::Left,
  }
}

pub(super) fn arg_f32(args: &serde_json::Value, name: &str) -> Option<f32> {
  args
    .get(name)
    .and_then(|value| value.as_f64())
    .map(|value| value as f32)
}

/// Resolve the action's target window and point (physical px). Ref wins over
/// coordinates and carries its own window.
pub(super) fn resolve_point(
  tree: &mut Tree,
  state: &McpState,
  args: &serde_json::Value,
  ref_key: &str,
  x_key: &str,
  y_key: &str,
) -> Result<(String, Option<(f32, f32)>), String> {
  if let Some(ref_id) = args.get(ref_key).and_then(|value| value.as_str()) {
    let resolved = resolve_ref(state, ref_id)?;
    let target = window_tree_mut(tree, &resolved.window, state.include_devtools)?;
    let point = ref_center_physical(target, &resolved, ref_id)?;
    return Ok((resolved.window, Some(point)));
  }
  let window = requested_window(args);
  match (arg_f32(args, x_key), arg_f32(args, y_key)) {
    (Some(x), Some(y)) => Ok((window, Some((x, y)))),
    _ => Ok((window, None)),
  }
}

pub(super) fn interact_tool(tree: &mut Tree, app: &App, state: &McpState, args: &serde_json::Value) -> McpToolResult {
  let action = args
    .get("action")
    .and_then(|value| value.as_str())
    .ok_or("`action` is required")?;
  let modifiers = parse_modifiers(args);
  let button = parse_button(args);

  let apply_all = |target: &mut Tree, inputs: Vec<SyntheticInput>| {
    for input in inputs {
      synthetic_input::apply(target, &input.with_modifiers(modifiers));
    }
    target.request_redraw();
  };

  match action {
    "request_close" => {
      let window = requested_window(args);
      let target = window_tree_mut(tree, &window, state.include_devtools)?;
      target
        .window()
        .dispatch_close_request(crate::app::CloseRequestSource::App);
      target.request_redraw();
      let close_queued = target.window().close_queued();
      Ok(McpToolOutput::Json(serde_json::json!({
        "ok": true, "window": window, "close_queued": close_queued,
        "stayed_open": !close_queued,
        "note": "Snapshot after handler dispatch; a retained request may proceed later. OS teardown is asynchronous."
      })))
    }
    "menu_activate" => {
      let id = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("menu_activate needs `id`")?;
      let activated = app.shared.menu.dispatch(id, tree.window());
      tree.request_redraw();
      Ok(McpToolOutput::Json(
        serde_json::json!({ "id": id, "activated": activated }),
      ))
    }
    "click" | "double_click" | "move" => {
      let (window, point) = resolve_point(tree, state, args, "ref", "x", "y")?;
      let (x, y) = point.ok_or("this action needs a `ref` or `x`/`y` coordinates")?;
      let target = window_tree_mut(tree, &window, state.include_devtools)?;
      let inputs = match action {
        "click" => vec![SyntheticInput::click_button(x, y, button)],
        "double_click" => vec![
          SyntheticInput::click_button(x, y, button),
          SyntheticInput::click_button(x, y, button),
        ],
        _ => vec![SyntheticInput::mouse_move(x, y)],
      };
      apply_all(target, inputs);
      Ok(McpToolOutput::Json(serde_json::json!({
        "ok": true, "action": action, "window": window, "x": x, "y": y
      })))
    }
    "drag" => {
      let (window, point) = resolve_point(tree, state, args, "ref", "x", "y")?;
      let (from_x, from_y) = point.ok_or("drag needs a start `ref` or `x`/`y`")?;
      let (to_window, to_point) = resolve_point(tree, state, args, "to_ref", "to_x", "to_y")?;
      let (to_x, to_y) = to_point.ok_or("drag needs an end `to_ref` or `to_x`/`to_y`")?;
      if args.get("to_ref").is_some() && to_window != window {
        return Err("drag start and end must be in the same window".into());
      }
      let target = window_tree_mut(tree, &window, state.include_devtools)?;
      let mut inputs = vec![
        SyntheticInput::mouse_move(from_x, from_y),
        SyntheticInput::mouse_down(from_x, from_y, button),
      ];
      // Intermediate motion: drag consumers (scrollbars, sliders, DnD) track
      // the motion stream, not just endpoints.
      const STEPS: u32 = 6;
      for step in 1..=STEPS {
        let t = step as f32 / STEPS as f32;
        inputs.push(SyntheticInput::mouse_move(
          from_x + (to_x - from_x) * t,
          from_y + (to_y - from_y) * t,
        ));
      }
      inputs.push(SyntheticInput::mouse_up(to_x, to_y, button));
      apply_all(target, inputs);
      Ok(McpToolOutput::Json(serde_json::json!({
        "ok": true, "action": "drag", "window": window,
        "from": [from_x, from_y], "to": [to_x, to_y]
      })))
    }
    "wheel" => {
      let (window, point) = resolve_point(tree, state, args, "ref", "x", "y")?;
      let (x, y) = point.ok_or("wheel needs a `ref` or `x`/`y`")?;
      let delta_x = arg_f32(args, "delta_x").unwrap_or(0.0);
      let delta_y = arg_f32(args, "delta_y").unwrap_or(0.0);
      if delta_x == 0.0 && delta_y == 0.0 {
        return Err("wheel needs a non-zero delta_x or delta_y".into());
      }
      let target = window_tree_mut(tree, &window, state.include_devtools)?;
      apply_all(target, vec![SyntheticInput::wheel(x, y, delta_x, delta_y)]);
      Ok(McpToolOutput::Json(
        serde_json::json!({ "ok": true, "action": "wheel", "window": window }),
      ))
    }
    "key" => {
      let key = args
        .get("key")
        .and_then(|value| value.as_str())
        .ok_or("key action needs `key`")?;
      let window = requested_window(args);
      let target = window_tree_mut(tree, &window, state.include_devtools)?;
      apply_all(target, vec![SyntheticInput::key_down(key), SyntheticInput::key_up(key)]);
      Ok(McpToolOutput::Json(
        serde_json::json!({ "ok": true, "action": "key", "key": key, "window": window }),
      ))
    }
    "type" => {
      let text = args
        .get("text")
        .and_then(|value| value.as_str())
        .ok_or("type action needs `text`")?;
      let (window, point) = resolve_point(tree, state, args, "ref", "x", "y")?;
      let target = window_tree_mut(tree, &window, state.include_devtools)?;
      // A ref focuses the input first; otherwise text goes to the current
      // focus.
      let mut inputs = Vec::new();
      if let Some((x, y)) = point
        && args.get("ref").is_some()
      {
        inputs.push(SyntheticInput::click(x, y));
      }
      inputs.extend(SyntheticInput::text(text));
      apply_all(target, inputs);
      Ok(McpToolOutput::Json(
        serde_json::json!({ "ok": true, "action": "type", "window": window }),
      ))
    }
    "scroll_to" => {
      let ref_id = args
        .get("ref")
        .and_then(|value| value.as_str())
        .ok_or("scroll_to needs a `ref`")?;
      let resolved = resolve_ref(state, ref_id)?;
      let target = window_tree_mut(tree, &resolved.window, state.include_devtools)?;
      let item_bounds = match resolved.canvas_item {
        Some(_) => Some(ref_scroll_bounds(target, &resolved, ref_id)?),
        None => None,
      };
      scroll_into_view(target, resolved.node_id, item_bounds, ref_id)?;
      target.request_redraw();
      Ok(McpToolOutput::Json(serde_json::json!({
        "ok": true, "action": "scroll_to", "window": resolved.window,
        "note": "scroll containers adjusted; element positions changed — re-read the tree before coordinate-based actions"
      })))
    }
    other => Err(format!("unknown action {other:?}")),
  }
}

/// Adjust every scroll container on the path to `node_id` so the node's
/// bounds (or `target_bounds` inside it, for a canvas item) land inside its
/// viewport (innermost adjustments win because outer containers position the
/// viewport, not the node).
pub(super) fn scroll_into_view(
  tree: &Tree,
  node_id: NodeId,
  target_bounds: Option<[f32; 4]>,
  ref_id: &str,
) -> Result<(), String> {
  struct ScrollAncestor<'n> {
    state: &'n crate::layout::layout_kind::ScrollState,
    viewport: [f32; 4],
  }

  fn walk<'n>(
    node: &'n Node,
    layout: &LayoutResult,
    abs: (f32, f32),
    node_id: NodeId,
    ancestors: &mut Vec<ScrollAncestor<'n>>,
  ) -> Option<[f32; 4]> {
    if node.node_id() == node_id {
      return Some([abs.0, abs.1, layout.size.width, layout.size.height]);
    }
    let is_scroll = matches!(node.layout_kind(), LayoutKind::ScrollModifier { .. });
    if let LayoutKind::ScrollModifier { state, .. } = node.layout_kind() {
      ancestors.push(ScrollAncestor {
        state,
        viewport: [abs.0, abs.1, layout.size.width, layout.size.height],
      });
    }
    for (index, child) in node.children().iter().enumerate() {
      if let Some(child_layout) = layout.children.get(index)
        && let Some(found) = walk(
          child,
          &child_layout.result,
          (abs.0 + child_layout.offset.x, abs.1 + child_layout.offset.y),
          node_id,
          ancestors,
        )
      {
        return Some(found);
      }
    }
    if is_scroll {
      ancestors.pop();
    }
    None
  }

  let root = tree
    .root()
    .ok_or_else(|| format!("window for ref {ref_id:?} has no mounted tree"))?;
  let layout = tree
    .last_layout()
    .ok_or("no layout available yet; wait for a frame first")?;
  let mut ancestors = Vec::new();
  let node_bounds = walk(root.node, layout, (0.0, 0.0), node_id, &mut ancestors)
    .ok_or_else(|| format!("ref {ref_id:?} no longer resolves to a live element; call lurq_read_tree again"))?;
  let target = target_bounds.unwrap_or(node_bounds);
  if ancestors.is_empty() {
    return Ok(());
  }

  const MARGIN: f32 = 8.0;
  let [target_x, target_y, target_w, target_h] = target;
  for ancestor in ancestors.iter().rev() {
    let [view_x, view_y, view_w, view_h] = ancestor.viewport;
    let mut delta_x = 0.0;
    let mut delta_y = 0.0;
    if target_y < view_y {
      delta_y = target_y - view_y - MARGIN;
    } else if target_y + target_h > view_y + view_h {
      delta_y = (target_y + target_h) - (view_y + view_h) + MARGIN;
    }
    if target_x < view_x {
      delta_x = target_x - view_x - MARGIN;
    } else if target_x + target_w > view_x + view_w {
      delta_x = (target_x + target_w) - (view_x + view_w) + MARGIN;
    }
    if delta_x != 0.0 || delta_y != 0.0 {
      let state = ancestor.state;
      let max_x = (state.content_width() - state.viewport_width()).max(0.0);
      let max_y = (state.content_height() - state.viewport_height()).max(0.0);
      state.set_scroll(
        (state.scroll_x() + delta_x).clamp(0.0, max_x),
        (state.scroll_y() + delta_y).clamp(0.0, max_y),
      );
    }
  }
  Ok(())
}
