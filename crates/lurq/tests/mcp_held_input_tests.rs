//! End to end over HTTP, the way an agent drives an app: a drag held across
//! `lurq_interact` calls lands one move per pass, and a press is released for
//! its client when the client ends its MCP session.
#![cfg(feature = "mcp")]

mod mcp_client;
mod support;

use std::{
  sync::{Arc, Mutex},
  thread::JoinHandle,
  time::{Duration, Instant},
};

use lurq::{
  app::{
    App, Tree,
    events::{DragEvent, MouseEvent},
  },
  components::{Column, Rect},
  mcp::McpConfig,
};
use mcp_client::Client;
use serde_json::{Value, json};
use support::TestSurface;

type Log = Arc<Mutex<Vec<String>>>;

fn push(log: &Log, entry: String) {
  log.lock().unwrap().push(entry);
}

/// A 100x100 drag handle at the window's origin, logging what reaches it, served over MCP.
fn app_with_handle(log: &Log) -> (Tree, App, Client) {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(400, 300);
  let (down, start, moved, end, up) = (log.clone(), log.clone(), log.clone(), log.clone(), log.clone());
  tree.set_root(
    Column::new().width(400.0).height(300.0).child(
      Rect::new(100.0, 100.0)
        .on_mouse_down(move |_: MouseEvent| push(&down, "down".into()))
        .on_drag_start(move |_: DragEvent| push(&start, "drag_start".into()))
        .on_drag_move(move |event: DragEvent| push(&moved, format!("drag_move {}", event.total_delta_x)))
        .on_drag_end(move |event: DragEvent| push(&end, format!("drag_end {}", event.x)))
        .on_mouse_up(move |_: MouseEvent| push(&up, "up".into())),
    ),
  );
  tree.pass_headless(&mut app);
  let mcp = tree.enable_mcp(McpConfig::new().app_name("lurq-held-input-test"));
  assert_ne!(mcp.port(), 0, "the MCP server is listening");
  let client = Client::new(mcp.port(), mcp.token().to_owned());
  (tree, app, client)
}

/// Runs the app loop as the winit shell does (drain the calls, then pass) until the agent is done and `settled`
/// holds for the log.
fn serve<T>(tree: &mut Tree, app: &mut App, log: &Log, agent: JoinHandle<T>, settled: impl Fn(&[String]) -> bool) -> T {
  let deadline = Instant::now() + Duration::from_secs(60);
  while !agent.is_finished() || !settled(&log.lock().unwrap()) {
    assert!(
      Instant::now() < deadline,
      "the agent did not finish: {:?}",
      log.lock().unwrap()
    );
    if tree.drain_mcp_requests(app) {
      // The pass the shell presents right after the drain, then the one a redraw requested before it would run.
      // Each pass with something to draw is logged, so a call drawn twice shows twice.
      for _ in 0..2 {
        if tree.pass(app, &TestSurface).required {
          push(log, "pass".into());
        }
      }
    }
    std::thread::sleep(Duration::from_millis(1));
  }
  tree.shutdown_mcp();
  agent.join().expect("agent thread")
}

#[test]
fn a_drag_held_across_calls_moves_once_per_pass() {
  let log = Log::default();
  let (mut tree, mut app, mut client) = app_with_handle(&log);
  let agent = std::thread::spawn(move || {
    client.initialize();
    let pressed = client.tool_json("lurq_interact", json!({"action": "press", "x": 50, "y": 50}));
    let moves: Vec<Value> = [70, 90, 110, 130]
      .into_iter()
      .map(|x| client.tool_json("lurq_interact", json!({"action": "move", "x": x, "y": 50})))
      .collect();
    let released = client.tool_json("lurq_interact", json!({"action": "release"}));
    (pressed, moves, released)
  });
  let (pressed, moves, released) = serve(&mut tree, &mut app, &log, agent, |_| true);

  assert_eq!(pressed["held"], json!({"buttons": ["left"], "keys": []}));
  assert!(moves.iter().all(|moved| moved["held"]["buttons"] == json!(["left"])));
  assert_eq!(released["held"], json!({"buttons": [], "keys": []}));
  assert_eq!(
    *log.lock().unwrap(),
    [
      "down",
      "drag_start",
      "pass",
      "drag_move 20",
      "pass",
      "drag_move 40",
      "pass",
      "drag_move 60",
      "pass",
      "drag_move 80",
      "pass",
      "drag_end 130",
      "pass",
    ]
  );
}

#[test]
fn ending_the_session_releases_what_it_held() {
  let log = Log::default();
  let (mut tree, mut app, mut client) = app_with_handle(&log);
  let (port, token) = (client.port(), client.token().to_owned());
  let agent = std::thread::spawn(move || {
    client.initialize();
    client.tool("lurq_interact", json!({"action": "press", "x": 50, "y": 50}));
    let held = client.tool_json("lurq_windows", json!({}))["windows"][0]["held"].clone();
    client.close();
    // Another client sees the press released once the app has noticed the session ended.
    let mut observer = Client::new(port, token);
    observer.initialize();
    let deadline = Instant::now() + Duration::from_secs(20);
    let after = loop {
      let after = observer.tool_json("lurq_windows", json!({}))["windows"][0]["held"].clone();
      if after["buttons"] == json!([]) || Instant::now() > deadline {
        break after;
      }
      std::thread::sleep(Duration::from_millis(10));
    };
    (held, after)
  });
  let (held, after) = serve(&mut tree, &mut app, &log, agent, |log| {
    log.iter().any(|entry| entry == "up")
  });

  assert_eq!(held, json!({"buttons": ["left"], "keys": []}));
  assert_eq!(after, json!({"buttons": [], "keys": []}));
  let log = log.lock().unwrap();
  let entries: Vec<&str> = log
    .iter()
    .map(String::as_str)
    .filter(|entry| *entry != "pass")
    .collect();
  // The press ends as an abandoned one: the drag ends where the pointer was, then the up event.
  assert_eq!(entries, ["down", "drag_start", "drag_end 50", "up"]);
}
