//! Handlers reached by MCP tool calls see synthetic input.

use std::sync::{Arc, Mutex};

use serde_json::json;

use super::tests::{TestSurface, state};
use crate::{
  app::{
    App, Tree,
    events::{InputSource, MouseEvent, input_source},
  },
  components::{Column, Rect},
  mcp::shared::McpRequest,
};

#[test]
fn an_mcp_click_reaches_handlers_as_synthetic_input() {
  let seen = Arc::new(Mutex::new(Vec::new()));
  let mut tree = Tree::new();
  tree.resize(200, 200);
  let clicks = seen.clone();
  tree.set_root(
    Column::new()
      .width(200.0)
      .height(200.0)
      .child(Rect::new(100.0, 100.0).on_click(move |_: MouseEvent| clicks.lock().unwrap().push(input_source()))),
  );
  let mut app = App::new();
  tree.pass(&mut app, &TestSurface);
  let mut mcp = state();
  let (sender, receiver) = std::sync::mpsc::channel();
  mcp.receiver = receiver;
  tree.mcp = Some(Box::new(mcp));

  let (reply, mut answer) = tokio::sync::oneshot::channel();
  sender
    .send(McpRequest {
      tool: "lurq_interact".into(),
      args: json!({"action": "click", "x": 50, "y": 50}),
      reply,
      session: None,
    })
    .unwrap();
  assert!(tree.drain_mcp_requests(&mut app));
  assert!(answer.try_recv().unwrap().is_ok());
  assert_eq!(*seen.lock().unwrap(), [InputSource::Synthetic]);
  assert_eq!(input_source(), InputSource::Os);
}
