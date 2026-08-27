//! Built-in tool execution. Everything here runs on the event-loop thread
//! during the shell's drain (or a headless harness's explicit drain), with
//! full `&mut Tree` / `&mut App` access — the same powers and the same
//! no-blocking constraint as event handlers.

use std::sync::{Arc, Mutex};

use super::{
  McpState, McpToolCtx, McpWaitEntry, McpWaitMode,
  registry::{BuiltinTool, ToolKind},
  shared::{McpReply, McpRequest, McpToolOutput, McpToolResult, RefRecord},
};
use crate::{
  app::{
    App, Tree,
    events::MouseButton,
    synthetic_input::{self, SyntheticInput, SyntheticModifiers},
  },
  core::NodeId,
  layout::{layout_kind::LayoutKind, layout_result::LayoutResult},
  node::{node::Node, node_kind::NodeKind},
};

pub(crate) fn execute(tree: &mut Tree, app: &mut App, state: &McpState, request: McpRequest) {
  let McpRequest { tool, args, reply } = request;
  let Some(registered) = state.registry.find(&tool) else {
    let _ = reply.send(Err(format!("unknown tool: {tool}")));
    return;
  };
  // Scopes are runtime-mutable; re-check at execution time, not just listing.
  if !state.shared.is_enabled() || !state.shared.has_scope(&registered.scope) || state.shared.is_denied(&tool) {
    let _ = reply.send(Err(format!("tool {tool} is not currently available")));
    return;
  }

  match &registered.kind {
    ToolKind::Builtin(builtin) => execute_builtin(*builtin, tree, app, state, args, reply),
    ToolKind::Sync(handler) => {
      let mut ctx = McpToolCtx { tree, app };
      let result = handler(&mut ctx, args).map(McpToolOutput::Json);
      let _ = reply.send(result);
    }
    // Async tools run on the server runtime and never cross the channel.
    ToolKind::Async(_) => {
      let _ = reply.send(Err(format!("tool {tool} is async and must not be routed to the app")));
    }
  }
}

fn execute_builtin(
  builtin: BuiltinTool,
  tree: &mut Tree,
  app: &mut App,
  state: &McpState,
  args: serde_json::Value,
  reply: McpReply,
) {
  let _ = app;
  match builtin {
    BuiltinTool::Windows => {
      let _ = reply.send(windows_tool(tree, state));
    }
    BuiltinTool::ReadTree => {
      let _ = reply.send(read_tree_tool(tree, state, &args));
    }
    BuiltinTool::Screenshot => screenshot_tool(tree, state, &args, reply),
    BuiltinTool::Wait => wait_tool(tree, state, &args, reply),
    BuiltinTool::Interact => {
      let _ = reply.send(interact_tool(tree, state, &args));
    }
    BuiltinTool::SetValue => {
      let _ = reply.send(set_value_tool(tree, state, &args));
    }
    BuiltinTool::Resize => {
      let _ = reply.send(resize_tool(tree, state, &args));
    }
    BuiltinTool::ReadState => {
      let _ = reply.send(read_state_tool(tree, state, &args));
    }
    BuiltinTool::Navigate => {
      let _ = reply.send(navigate_tool(state, &args));
    }
    // Served on the server thread; never routed here.
    BuiltinTool::Find | BuiltinTool::Logs => {
      let _ = reply.send(Err("this tool is served without an app roundtrip".into()));
    }
  }
}

// ---------------------------------------------------------------------------
// Window addressing

fn requested_window(args: &serde_json::Value) -> String {
  args
    .get("window")
    .and_then(|value| value.as_str())
    .filter(|window| !window.is_empty())
    .unwrap_or("main")
    .to_owned()
}

fn is_devtools_index(tree: &Tree, index: usize) -> bool {
  #[cfg(feature = "devtools")]
  {
    tree
      .devtools
      .as_ref()
      .is_some_and(|devtools| devtools.secondary_index == index)
  }
  #[cfg(not(feature = "devtools"))]
  {
    let _ = (tree, index);
    false
  }
}

/// Secondary indexes visible to MCP, in order.
fn visible_secondary_indexes(tree: &Tree, include_devtools: bool) -> Vec<usize> {
  (0..tree.secondary_window_count())
    .filter(|index| tree.secondary_window(*index).is_some())
    .filter(|index| include_devtools || !is_devtools_index(tree, *index))
    .collect()
}

fn find_secondary_index(tree: &Tree, window: &str, include_devtools: bool) -> Option<usize> {
  visible_secondary_indexes(tree, include_devtools)
    .into_iter()
    .find(|index| {
      tree
        .secondary_window(*index)
        .is_some_and(|secondary| secondary.name() == Some(window) || format!("w{}", secondary.id()) == window)
    })
}

/// Resolve a `window` argument to its tree. `"main"` is the root tree;
/// secondaries resolve by name or `w<id>`; closed or unknown windows error
/// as gone rather than falling back to a different window.
fn window_tree_mut<'t>(root: &'t mut Tree, window: &str, include_devtools: bool) -> Result<&'t mut Tree, String> {
  if window == "main" {
    return Ok(root);
  }
  if window == "focused" {
    if root.window().info().is_focused {
      return Ok(root);
    }
    let focused = visible_secondary_indexes(root, include_devtools)
      .into_iter()
      .find(|index| {
        root
          .secondary_window(*index)
          .is_some_and(|secondary| secondary.tree().window().info().is_focused)
      });
    return match focused {
      Some(index) => Ok(
        root
          .secondary_window_mut(index)
          .expect("index just resolved")
          .tree_mut(),
      ),
      // Focus races with real user activity; fall back to the main window.
      None => Ok(root),
    };
  }
  match find_secondary_index(root, window, include_devtools) {
    Some(index) => Ok(
      root
        .secondary_window_mut(index)
        .expect("index just resolved")
        .tree_mut(),
    ),
    None => Err(format!(
      "window {window:?} not found or closed; list windows with lurq_windows"
    )),
  }
}

fn windows_tool(tree: &Tree, state: &McpState) -> McpToolResult {
  let mut windows = Vec::new();
  let main_info = tree.window().info();
  windows.push(serde_json::json!({
    "id": "main",
    "kind": "main",
    "open": true,
    "focused": main_info.is_focused,
    "width": main_info.resolved_width.round(),
    "height": main_info.resolved_height.round(),
    "scale_factor": main_info.scale_factor,
  }));
  for index in visible_secondary_indexes(tree, state.include_devtools) {
    let Some(secondary) = tree.secondary_window(index) else {
      continue;
    };
    let info = secondary.tree().window().info();
    windows.push(serde_json::json!({
      "id": format!("w{}", secondary.id()),
      "name": secondary.name(),
      "title": secondary.title(),
      "kind": if is_devtools_index(tree, index) { "devtools" } else { "secondary" },
      "open": true,
      "focused": info.is_focused,
      "width": info.resolved_width.round(),
      "height": info.resolved_height.round(),
      "scale_factor": info.scale_factor,
    }));
  }
  Ok(McpToolOutput::Json(serde_json::json!({ "windows": windows })))
}

// ---------------------------------------------------------------------------
// Tree snapshot / refs

fn is_interactive(node: &Node) -> bool {
  let events = &node.events;
  !events.on_click.is_empty()
    || !events.on_mouse_click.is_empty()
    || !events.on_dblclick.is_empty()
    || !events.on_mouse_down.is_empty()
    || !events.on_mouse_up.is_empty()
    || !events.on_drag_start.is_empty()
    || !events.on_drop.is_empty()
    || !events.on_key_down.is_empty()
    || matches!(
      node.node_kind(),
      NodeKind::TextInput { .. } | NodeKind::Checkbox { .. } | NodeKind::Slider { .. } | NodeKind::Select { .. }
    )
    || matches!(node.layout_kind(), LayoutKind::ScrollModifier { .. })
}

fn node_value_summary(node: &Node) -> Option<String> {
  match node.node_kind() {
    NodeKind::TextInput { state, .. } => Some(format!("value={:?}", state.value())),
    NodeKind::Checkbox { state } => Some(format!("checked={}", state.is_checked())),
    NodeKind::Slider { state } => Some(format!("value={}", state.value_string())),
    NodeKind::Select { state } => {
      let labels = state.labels();
      let selected = state
        .selected_indices()
        .into_iter()
        .filter_map(|index| labels.get(index).map(|label| label.to_string()))
        .collect::<Vec<_>>();
      Some(format!(
        "selected={:?}{}",
        selected,
        if state.is_open() { " open" } else { "" }
      ))
    }
    _ => None,
  }
}

fn truncate_text(text: &str, max: usize) -> String {
  if text.chars().count() <= max {
    return text.to_owned();
  }
  let cut: String = text.chars().take(max).collect();
  format!("{cut}…")
}

struct SnapshotCtx<'a> {
  window: String,
  scale: f32,
  all: bool,
  max_depth: usize,
  records: &'a mut Vec<RefRecord>,
  mint: &'a mut dyn FnMut() -> String,
}

/// Render one node (and recursively its children) as outline lines.
/// Returns the rendered lines; empty when the branch was pruned.
fn snapshot_node(
  ctx: &mut SnapshotCtx<'_>,
  node: &Node,
  layout: Option<&LayoutResult>,
  abs: (f32, f32),
  depth: usize,
) -> Vec<String> {
  let mut child_lines = Vec::new();
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
  let attrs = node.debug_attrs();
  let text = node.text_content().map(|text| text.to_owned());
  let value = node_value_summary(node);
  // Component nodes with reactive debug state get refs too, so
  // lurq_read_state has something to target.
  #[cfg(feature = "devtools")]
  let stateful = !node.component_signals_debug().is_empty() || !node.component_memos_debug().is_empty();
  #[cfg(not(feature = "devtools"))]
  let stateful = false;
  let interesting = interactive || stateful || !attrs.is_empty() || text.is_some() || value.is_some();
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

  let mut line = format!("{}- {}", "  ".repeat(depth), node.tag_name());

  if interactive || stateful || !attrs.is_empty() {
    let ref_id = (ctx.mint)();
    line.push_str(&format!(" [{ref_id}]"));
    ctx.records.push(RefRecord {
      id: ref_id,
      window: ctx.window.clone(),
      node_id: node.node_id(),
      tag: node.tag_name().to_owned(),
      text: text.clone(),
      attrs: attrs
        .iter()
        .map(|(name, attr_value)| (name.to_string(), attr_value.to_string()))
        .collect(),
      bounds: bounds.unwrap_or([0.0; 4]),
      interactive,
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
    line.push_str(&format!(" {{{name}={attr_value}}}"));
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

fn read_tree_tool(tree: &mut Tree, state: &McpState, args: &serde_json::Value) -> McpToolResult {
  let window = requested_window(args);
  let all = args.get("filter").and_then(|value| value.as_str()) == Some("all");
  let max_depth = args.get("max_depth").and_then(|value| value.as_u64()).unwrap_or(0) as usize;
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
      window: window.clone(),
      scale,
      all,
      max_depth,
      records: &mut records,
      mint: &mut mint,
    };
    snapshot_node(&mut ctx, root.node, target.last_layout(), (0.0, 0.0), 0)
  };
  state.shared.refs.lock().unwrap().replace_window(&window, records);

  let header = format!(
    "window: {window} ({}x{} @{}x)\n",
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
  let mut line = format!("{} [{}] {}", record.id, record.window, record.tag);
  if let Some(text) = &record.text {
    line.push_str(&format!(" {:?}", truncate_text(text, 60)));
  }
  for (name, value) in &record.attrs {
    line.push_str(&format!(" {{{name}={value}}}"));
  }
  let [x, y, width, height] = record.bounds;
  line.push_str(&format!(" @{x:.0},{y:.0} {width:.0}x{height:.0}"));
  if !record.interactive {
    line.push_str(" (not interactive)");
  }
  line
}

// ---------------------------------------------------------------------------
// Node resolution

/// Absolute logical bounds of a node in its tree, from the last layout pass.
fn locate_node(tree: &Tree, node_id: NodeId) -> Option<[f32; 4]> {
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

fn find_node(tree: &Tree, node_id: NodeId) -> Option<&Node> {
  fn walk(node: &Node, node_id: NodeId) -> Option<&Node> {
    if node.node_id() == node_id {
      return Some(node);
    }
    node.children().iter().find_map(|child| walk(child, node_id))
  }
  walk(tree.root()?.node, node_id)
}

struct ResolvedRef {
  window: String,
  node_id: NodeId,
}

fn resolve_ref(state: &McpState, ref_id: &str) -> Result<ResolvedRef, String> {
  let refs = state.shared.refs.lock().unwrap();
  match refs.get(ref_id) {
    Some(record) => Ok(ResolvedRef {
      window: record.window.clone(),
      node_id: record.node_id,
    }),
    None => Err(format!(
      "unknown or stale ref {ref_id:?}; refs are replaced by each lurq_read_tree — call it again"
    )),
  }
}

/// Physical-pixel center of a ref's node, resolved against the live layout.
fn ref_center_physical(tree: &Tree, node_id: NodeId, ref_id: &str) -> Result<(f32, f32), String> {
  let [x, y, width, height] = locate_node(tree, node_id)
    .ok_or_else(|| format!("ref {ref_id:?} no longer resolves to a live element; call lurq_read_tree again"))?;
  let scale = tree.scale_factor();
  Ok(((x + width * 0.5) * scale, (y + height * 0.5) * scale))
}

// ---------------------------------------------------------------------------
// Screenshot

fn screenshot_tool(tree: &mut Tree, state: &McpState, args: &serde_json::Value, reply: McpReply) {
  let result = prepare_screenshot(tree, state, args, reply);
  if let Err((reply, message)) = result {
    let _ = reply.send(Err(message));
  }
}

/// On error, gives the reply back so the caller can resolve it.
fn prepare_screenshot(
  tree: &mut Tree,
  state: &McpState,
  args: &serde_json::Value,
  reply: McpReply,
) -> Result<(), (McpReply, String)> {
  let ref_id = args.get("ref").and_then(|value| value.as_str());

  let (window, region_logical) = if let Some(ref_id) = ref_id {
    let resolved = match resolve_ref(state, ref_id) {
      Ok(resolved) => resolved,
      Err(message) => return Err((reply, message)),
    };
    let target = match window_tree_mut(tree, &resolved.window, state.include_devtools) {
      Ok(target) => target,
      Err(message) => return Err((reply, message)),
    };
    let Some(bounds) = locate_node(target, resolved.node_id) else {
      return Err((
        reply,
        format!("ref {ref_id:?} no longer resolves to a live element; call lurq_read_tree again"),
      ));
    };
    (
      resolved.window,
      Some(crate::app::window::ScreenshotRegion {
        x: bounds[0],
        y: bounds[1],
        width: bounds[2],
        height: bounds[3],
      }),
    )
  } else {
    let window = requested_window(args);
    let region = match args.get("region") {
      Some(region_value) => {
        let field = |name: &str| {
          region_value
            .get(name)
            .and_then(|value| value.as_f64())
            .map(|value| value as f32)
        };
        match (field("x"), field("y"), field("width"), field("height")) {
          (Some(x), Some(y), Some(width), Some(height)) => Some((x, y, width, height)),
          _ => return Err((reply, "region requires numeric x, y, width, height".into())),
        }
      }
      None => None,
    };
    let scale = {
      let target = match window_tree_mut(tree, &window, state.include_devtools) {
        Ok(target) => target,
        Err(message) => return Err((reply, message)),
      };
      target.scale_factor()
    };
    // The MCP surface speaks screenshot pixels (physical); the capture
    // pipeline takes logical regions.
    let region_logical = region.map(|(x, y, width, height)| crate::app::window::ScreenshotRegion {
      x: x / scale,
      y: y / scale,
      width: width / scale,
      height: height / scale,
    });
    (window, region_logical)
  };

  let target = match window_tree_mut(tree, &window, state.include_devtools) {
    Ok(target) => target,
    Err(message) => return Err((reply, message)),
  };

  // The reply parks inside the capture callback; the capture pipeline
  // guarantees the callback fires exactly once (pixels or error), frames
  // later, on a readback thread.
  let slot = Mutex::new(Some(reply));
  let callback = Arc::new(
    move |outcome: Result<crate::app::render_engine::CapturedFrame, String>| {
      let Some(reply) = slot.lock().unwrap().take() else {
        return;
      };
      let result = outcome.and_then(|frame| encode_png(&frame));
      let _ = reply.send(result);
    },
  );
  target.request_screenshot_capture(
    crate::app::render_engine::RenderCaptureTarget::Bytes(callback),
    region_logical,
  );
  Ok(())
}

fn encode_png(frame: &crate::app::render_engine::CapturedFrame) -> McpToolResult {
  use image::ImageEncoder as _;
  let mut png = Vec::new();
  image::codecs::png::PngEncoder::new(&mut png)
    .write_image(&frame.rgba, frame.width, frame.height, image::ExtendedColorType::Rgba8)
    .map_err(|error| format!("failed to encode screenshot PNG: {error}"))?;
  Ok(McpToolOutput::Image {
    data: png,
    mime: "image/png",
  })
}

// ---------------------------------------------------------------------------
// Wait

fn wait_tool(tree: &mut Tree, state: &McpState, args: &serde_json::Value, reply: McpReply) {
  let window = requested_window(args);
  let frames = args.get("frames").and_then(|value| value.as_u64());
  let target = match window_tree_mut(tree, &window, state.include_devtools) {
    Ok(target) => target,
    Err(message) => {
      let _ = reply.send(Err(message));
      return;
    }
  };
  match frames {
    Some(frames) if frames > 0 => {
      target.mcp_wait_entries.push(McpWaitEntry {
        mode: McpWaitMode::Frames(frames as u32),
        reply: Some(reply),
      });
      // Frames have to come from somewhere: keep the render loop producing.
      target.request_redraw();
    }
    _ => {
      if !target.needs_redraw() && !target.has_active_timeline() {
        let _ = reply.send(Ok(McpToolOutput::Text("already idle".into())));
        return;
      }
      target.mcp_wait_entries.push(McpWaitEntry {
        mode: McpWaitMode::Idle,
        reply: Some(reply),
      });
    }
  }
}

// ---------------------------------------------------------------------------
// Interact

fn parse_modifiers(args: &serde_json::Value) -> SyntheticModifiers {
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

fn parse_button(args: &serde_json::Value) -> MouseButton {
  match args.get("button").and_then(|value| value.as_str()) {
    Some("right") => MouseButton::Right,
    Some("middle") => MouseButton::Middle,
    _ => MouseButton::Left,
  }
}

fn arg_f32(args: &serde_json::Value, name: &str) -> Option<f32> {
  args
    .get(name)
    .and_then(|value| value.as_f64())
    .map(|value| value as f32)
}

/// Resolve the action's target window and point (physical px). Ref wins over
/// coordinates and carries its own window.
fn resolve_point(
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
    let point = ref_center_physical(target, resolved.node_id, ref_id)?;
    return Ok((resolved.window, Some(point)));
  }
  let window = requested_window(args);
  match (arg_f32(args, x_key), arg_f32(args, y_key)) {
    (Some(x), Some(y)) => Ok((window, Some((x, y)))),
    _ => Ok((window, None)),
  }
}

fn interact_tool(tree: &mut Tree, state: &McpState, args: &serde_json::Value) -> McpToolResult {
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
      scroll_into_view(target, resolved.node_id, ref_id)?;
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
/// bounds land inside its viewport (innermost adjustments win because outer
/// containers position the viewport, not the node).
fn scroll_into_view(tree: &Tree, node_id: NodeId, ref_id: &str) -> Result<(), String> {
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
  let target = walk(root.node, layout, (0.0, 0.0), node_id, &mut ancestors)
    .ok_or_else(|| format!("ref {ref_id:?} no longer resolves to a live element; call lurq_read_tree again"))?;
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

// ---------------------------------------------------------------------------
// set_value / read_state / resize / navigate

fn set_value_tool(tree: &mut Tree, state: &McpState, args: &serde_json::Value) -> McpToolResult {
  let ref_id = args
    .get("ref")
    .and_then(|value| value.as_str())
    .ok_or("`ref` is required")?;
  let value = args.get("value").cloned().ok_or("`value` is required")?;
  let resolved = resolve_ref(state, ref_id)?;
  let target = window_tree_mut(tree, &resolved.window, state.include_devtools)?;
  let node = find_node(target, resolved.node_id)
    .ok_or_else(|| format!("ref {ref_id:?} no longer resolves to a live element; call lurq_read_tree again"))?;

  let outcome = match node.node_kind() {
    NodeKind::TextInput { state, .. } => match value.as_str() {
      Some(text) => {
        state.set_value_external(text.to_owned());
        Ok(serde_json::json!({ "ok": true, "kind": "TextInput", "value": text }))
      }
      None => Err("TextInput takes a string value".to_owned()),
    },
    NodeKind::Checkbox { state } => match value.as_bool() {
      Some(checked) => {
        if state.is_checked() != checked {
          state.toggle();
        }
        Ok(serde_json::json!({ "ok": true, "kind": "Checkbox", "checked": checked }))
      }
      None => Err("Checkbox takes a boolean value".to_owned()),
    },
    NodeKind::Slider { state } => match value.as_f64() {
      Some(number) => {
        state.set_value_external(number as f32);
        Ok(serde_json::json!({ "ok": true, "kind": "Slider", "value": state.value_string() }))
      }
      None => Err("Slider takes a numeric value".to_owned()),
    },
    NodeKind::Select { state } => {
      let labels = state.labels();
      let index = if let Some(index) = value.as_u64() {
        let index = index as usize;
        if index >= labels.len() {
          return Err(format!(
            "Select has {} options; index {index} is out of range",
            labels.len()
          ));
        }
        index
      } else if let Some(label) = value.as_str() {
        labels
          .iter()
          .position(|candidate| candidate.eq_ignore_ascii_case(label))
          .ok_or_else(|| {
            format!(
              "no option matches {label:?}; options: {:?}",
              labels.iter().map(|label| label.to_string()).collect::<Vec<_>>()
            )
          })?
      } else {
        return Err("Select takes an option label (string) or index (number)".to_owned());
      };
      // commit() fires the app's on_change and closes a single-select menu —
      // the same path an option click takes.
      state.commit(index);
      Ok(
        serde_json::json!({ "ok": true, "kind": "Select", "selected": labels.get(index).map(|label| label.to_string()) }),
      )
    }
    _ => Err(format!(
      "ref {ref_id:?} is a {} — lurq_set_value supports TextInput, Checkbox, Slider, and Select",
      node.tag_name()
    )),
  };

  target.request_redraw();
  outcome.map(McpToolOutput::Json)
}

fn resize_tool(tree: &mut Tree, state: &McpState, args: &serde_json::Value) -> McpToolResult {
  let width = args
    .get("width")
    .and_then(|value| value.as_u64())
    .ok_or("`width` is required")?;
  let height = args
    .get("height")
    .and_then(|value| value.as_u64())
    .ok_or("`height` is required")?;
  if width == 0 || height == 0 {
    return Err("width and height must be positive".into());
  }
  let window = requested_window(args);
  let target = window_tree_mut(tree, &window, state.include_devtools)?;
  target.window().handle().resize(width as u32, height as u32);
  Ok(McpToolOutput::Json(serde_json::json!({
    "ok": true, "window": window, "width": width, "height": height,
    "note": "resize is applied by the OS asynchronously; lurq_wait then lurq_windows to confirm"
  })))
}

#[cfg(feature = "devtools")]
fn read_state_tool(tree: &mut Tree, state: &McpState, args: &serde_json::Value) -> McpToolResult {
  let ref_id = args
    .get("ref")
    .and_then(|value| value.as_str())
    .ok_or("`ref` is required")?;
  let resolved = resolve_ref(state, ref_id)?;
  let target = window_tree_mut(tree, &resolved.window, state.include_devtools)?;
  let node = find_node(target, resolved.node_id)
    .ok_or_else(|| format!("ref {ref_id:?} no longer resolves to a live element; call lurq_read_tree again"))?;

  let signals: Vec<_> = node
    .component_signals_debug()
    .iter()
    .map(|signal| {
      serde_json::json!({
        "id": signal.id,
        "type": signal.type_name.to_string(),
        "value": signal.formatted_value(),
        "subscribers": signal.subscriber_count(),
      })
    })
    .collect();
  let memos: Vec<_> = node
    .component_memos_debug()
    .iter()
    .map(|memo| {
      serde_json::json!({
        "id": memo.id,
        "type": memo.type_name.to_string(),
        "value": memo.formatted_value(),
        "subscribers": memo.subscriber_count(),
      })
    })
    .collect();
  let contexts: Vec<_> = node
    .component_contexts_debug()
    .iter()
    .map(|context| {
      serde_json::json!({
        "kind": format!("{:?}", context.kind),
        "type": context.type_name.to_string(),
      })
    })
    .collect();
  Ok(McpToolOutput::Json(serde_json::json!({
    "ref": ref_id,
    "tag": node.tag_name(),
    "signals": signals,
    "memos": memos,
    "contexts": contexts,
  })))
}

#[cfg(not(feature = "devtools"))]
fn read_state_tool(_tree: &mut Tree, _state: &McpState, _args: &serde_json::Value) -> McpToolResult {
  Err(
    "lurq_read_state needs the app built with lurq's `devtools` feature (it reuses the devtools debug collection)"
      .into(),
  )
}

#[cfg(feature = "router")]
fn navigate_tool(state: &McpState, args: &serde_json::Value) -> McpToolResult {
  let navigator = state.shared.navigator.read().unwrap().clone();
  let Some(navigator) = navigator else {
    return Err(
      "no navigator configured: pass one with McpConfig::navigator(...) or McpHandle::set_navigator(...)".into(),
    );
  };
  if args.get("back").and_then(|value| value.as_bool()) == Some(true) {
    navigator.back();
  } else if args.get("forward").and_then(|value| value.as_bool()) == Some(true) {
    navigator.forward();
  } else if let Some(path) = args.get("path").and_then(|value| value.as_str()) {
    if args.get("replace").and_then(|value| value.as_bool()) == Some(true) {
      navigator.replace(path);
    } else {
      navigator.push(path);
    }
  }
  Ok(McpToolOutput::Json(serde_json::json!({
    "path": navigator.path().get_untracked(),
    "can_back": navigator.can_back(),
    "can_forward": navigator.can_forward(),
  })))
}

#[cfg(not(feature = "router"))]
fn navigate_tool(_state: &McpState, _args: &serde_json::Value) -> McpToolResult {
  Err("lurq_navigate needs the `router` feature".into())
}
