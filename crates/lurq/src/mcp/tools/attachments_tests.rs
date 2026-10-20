//! `lurq_interact` pastes and file drags carry their own content, reach the
//! handlers as synthetic input, and never read the person's clipboard.

use std::sync::{Arc, Mutex};

use serde_json::json;

use super::tests::{TestSurface, state};
use crate::{
  app::{
    App, Tree,
    events::{FileDropEvent, PasteEvent, is_synthetic_input},
  },
  components::{Column, Rect, TextInput},
  core::Signal,
  mcp::shared::{McpRequest, McpToolOutput},
};

fn hosted(tree: &mut Tree) -> std::sync::mpsc::Sender<McpRequest> {
  let mut mcp = state();
  let (sender, receiver) = std::sync::mpsc::channel();
  mcp.receiver = receiver;
  tree.mcp = Some(Box::new(mcp));
  sender
}

fn interact(
  tree: &mut Tree,
  app: &mut App,
  sender: &std::sync::mpsc::Sender<McpRequest>,
  args: serde_json::Value,
) -> Result<serde_json::Value, String> {
  let (reply, mut answer) = tokio::sync::oneshot::channel();
  sender
    .send(McpRequest {
      tool: "lurq_interact".into(),
      args,
      reply,
      session: None,
    })
    .unwrap();
  assert!(tree.drain_mcp_requests(app));
  match answer.try_recv().unwrap() {
    Ok(McpToolOutput::Json(value)) => Ok(value),
    Ok(_) => panic!("unexpected non-JSON output"),
    Err(error) => Err(error),
  }
}

#[test]
fn an_mcp_paste_carries_its_text_and_files_as_synthetic_input() {
  let value = Signal::new(String::new());
  let seen = Arc::new(Mutex::new(Vec::new()));
  let mut tree = Tree::new();
  tree.resize(200, 200);
  let log = seen.clone();
  tree.set_root(
    Column::new()
      .width(200.0)
      .height(200.0)
      .child(
        TextInput::new(value.clone())
          .id("input")
          .on_paste(move |event: PasteEvent| {
            log.lock().unwrap().push((
              is_synthetic_input(),
              event.text().map(str::to_owned),
              event.files().len(),
            ));
          }),
      ),
  );
  let mut app = App::new();
  tree.pass(&mut app, &TestSurface);
  let sender = hosted(&mut tree);
  let (x, y) = tree.get_element_by_id_mut("input").unwrap().bounds().unwrap().center();
  interact(&mut tree, &mut app, &sender, json!({"action": "click", "x": x, "y": y})).unwrap();

  let reply = interact(
    &mut tree,
    &mut app,
    &sender,
    json!({"action": "paste", "text": "from an agent", "files": ["C:/shots/a.png"]}),
  )
  .unwrap();

  assert_eq!(reply["text"], true);
  assert_eq!(reply["files"], true);
  assert_eq!(
    *seen.lock().unwrap(),
    [(true, Some("from an agent".to_owned()), 1)],
    "the handler sees the content, marked synthetic"
  );
  assert_eq!(value.get(), "from an agent");
}

#[test]
fn an_mcp_paste_without_content_is_refused_instead_of_reading_the_clipboard() {
  let mut tree = Tree::new();
  tree.resize(200, 200);
  tree.set_root(TextInput::new(Signal::new(String::new())));
  let mut app = App::new();
  tree.pass(&mut app, &TestSurface);
  let sender = hosted(&mut tree);

  let error = interact(&mut tree, &mut app, &sender, json!({"action": "paste"})).unwrap_err();

  assert!(error.contains("never read"), "{error}");
}

#[test]
fn mcp_file_drags_reach_the_element_under_the_point() {
  let dropped = Arc::new(Mutex::new(Vec::new()));
  let mut tree = Tree::new();
  tree.resize(200, 200);
  let log = dropped.clone();
  tree.set_root(
    Column::new()
      .width(200.0)
      .height(200.0)
      .child(Rect::new(100.0, 100.0).on_file_drop(move |event: FileDropEvent| {
        log
          .lock()
          .unwrap()
          .push((is_synthetic_input(), event.paths.len(), event.paths.has_folders()));
      })),
  );
  let mut app = App::new();
  tree.pass(&mut app, &TestSurface);
  let sender = hosted(&mut tree);

  let hover = interact(
    &mut tree,
    &mut app,
    &sender,
    json!({"action": "drag_files", "x": 50, "y": 50, "paths": ["C:/shots/a.png", "C:/shots/b.png"]}),
  )
  .unwrap();
  assert_eq!(hover["paths"], 2);
  assert_eq!(hover["accepted"], true);
  assert!(tree.has_file_drag());

  let leave = interact(&mut tree, &mut app, &sender, json!({"action": "file_drag_leave"})).unwrap();
  assert_eq!(leave["had_drag"], true);
  assert!(!tree.has_file_drag());

  interact(
    &mut tree,
    &mut app,
    &sender,
    json!({"action": "drop_files", "x": 50, "y": 50, "paths": ["C:/shots/a.png", "C:/shots/b.png"]}),
  )
  .unwrap();
  assert_eq!(*dropped.lock().unwrap(), [(true, 2, false)]);
  assert!(!tree.has_file_drag());

  let error = interact(
    &mut tree,
    &mut app,
    &sender,
    json!({"action": "drop_files", "x": 50, "y": 50}),
  )
  .unwrap_err();
  assert!(error.contains("paths"), "{error}");
}
