//! `lurq_wait`: parking a reply until frames are presented or rendering is idle.

use super::windows::{requested_window, window_tree_mut};
use crate::{
  app::Tree,
  mcp::{
    McpState, McpWaitEntry, McpWaitMode,
    shared::{McpReply, McpToolOutput},
  },
};

pub(super) fn wait_tool(tree: &mut Tree, state: &McpState, args: &serde_json::Value, reply: McpReply) {
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
