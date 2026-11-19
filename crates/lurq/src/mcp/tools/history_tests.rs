//! `lurq_interact` goes back and forward as a person does: the actions `history_back` and `history_forward` from the
//! focus or a point, and a click of the mouse's side buttons.

use std::sync::{Arc, Mutex};

use serde_json::json;

use super::tests::{TestSurface, state};
use crate::{
  app::{
    App, Tree,
    events::{HistoryEvent, NavigationSource},
  },
  components::{Column, Rect, Row},
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
) -> serde_json::Value {
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
    Ok(McpToolOutput::Json(value)) => value,
    Ok(_) => panic!("unexpected non-JSON output"),
    Err(error) => panic!("{error}"),
  }
}

type Log = Arc<Mutex<Vec<String>>>;

fn hears(log: &Log, name: &'static str) -> impl Fn(HistoryEvent) + Send + Sync + 'static {
  let log = log.clone();
  move |event: HistoryEvent| {
    assert!(matches!(
      event.source(),
      NavigationSource::App | NavigationSource::Mouse
    ));
    log
      .lock()
      .unwrap()
      .push(format!("{name} {:?} {:?}", event.command(), event.source()));
  }
}

/// Two halves of a 200x100 window, each hearing both commands.
fn halves(log: &Log) -> (Tree, App, std::sync::mpsc::Sender<McpRequest>) {
  let mut tree = Tree::new();
  tree.resize(200, 100);
  tree.set_root(
    Row::new()
      .size(200.0, 100.0)
      .spacing(0.0)
      .child(
        Column::new()
          .size(100.0, 100.0)
          .on_history_back(hears(log, "left"))
          .on_history_forward(hears(log, "left")),
      )
      .child(
        Rect::new(100.0, 100.0)
          .on_history_back(hears(log, "right"))
          .on_history_forward(hears(log, "right")),
      ),
  );
  let mut app = App::new();
  tree.pass(&mut app, &TestSurface);
  let sender = hosted(&mut tree);
  (tree, app, sender)
}

#[test]
fn history_back_and_forward_run_the_command() {
  let log = Log::default();
  let (mut tree, mut app, sender) = halves(&log);

  let answer = interact(
    &mut tree,
    &mut app,
    &sender,
    json!({"action": "history_back", "x": 20, "y": 50}),
  );
  assert_eq!(answer["taken"], true);
  assert_eq!(std::mem::take(&mut *log.lock().unwrap()), ["left Back App"]);

  let answer = interact(&mut tree, &mut app, &sender, json!({"action": "history_forward"}));
  assert_eq!(answer["taken"], true);
  assert_eq!(
    std::mem::take(&mut *log.lock().unwrap()),
    ["right Forward App"],
    "from no point and no focus: the last element that hears it"
  );
}

#[test]
fn a_click_of_the_back_button_goes_back_from_under_the_pointer() {
  let log = Log::default();
  let (mut tree, mut app, sender) = halves(&log);

  interact(
    &mut tree,
    &mut app,
    &sender,
    json!({"action": "click", "x": 150, "y": 50, "button": "back"}),
  );
  assert_eq!(std::mem::take(&mut *log.lock().unwrap()), ["right Back Mouse"]);
  interact(
    &mut tree,
    &mut app,
    &sender,
    json!({"action": "click", "x": 20, "y": 50, "button": "forward"}),
  );
  assert_eq!(std::mem::take(&mut *log.lock().unwrap()), ["left Forward Mouse"]);
}
