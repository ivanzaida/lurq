//! `lurq_pointer`: what the person points with, read and set, so that an agent sees a component's touch form.

use crate::{
  app::{App, Pointer},
  mcp::shared::{McpToolOutput, McpToolResult},
};

pub(super) fn pointer_tool(app: &App, args: &serde_json::Value) -> McpToolResult {
  let pointer = &app.shared.pointer;
  if let Some(follows) = args.get("follow").and_then(serde_json::Value::as_bool) {
    pointer.follow(follows);
  }
  match args.get("pointer").and_then(serde_json::Value::as_str) {
    None => {}
    Some(said) => {
      let to = match said {
        "fine" => Pointer::Fine,
        "coarse" => Pointer::Coarse,
        other => return Err(format!("pointer is \"fine\" or \"coarse\", not \"{other}\"")),
      };
      // Pinned unless told: the agent's own mouse moves would make it fine again.
      let pin = args.get("pin").and_then(serde_json::Value::as_bool).unwrap_or(true);
      pointer.tell(to, pin);
    }
  }
  let (now, pinned, follows) = pointer.state();
  Ok(McpToolOutput::Json(serde_json::json!({
    "pointer": match now {
      Pointer::Fine => "fine",
      Pointer::Coarse => "coarse",
    },
    "touch_form": now == Pointer::Coarse,
    "pinned": pinned,
    "follows": follows,
  })))
}
