//! `lurq_navigate`: router navigation.

#[cfg(feature = "router")]
use crate::mcp::shared::McpToolOutput;
use crate::mcp::{McpState, shared::McpToolResult};

#[cfg(feature = "router")]
pub(super) fn navigate_tool(state: &McpState, args: &serde_json::Value) -> McpToolResult {
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
pub(super) fn navigate_tool(_state: &McpState, _args: &serde_json::Value) -> McpToolResult {
  Err("lurq_navigate needs the `router` feature".into())
}
