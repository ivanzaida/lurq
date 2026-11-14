//! A drop zone driven as an agent drives an app: a press opens the file dialog, which the app's registered
//! broker hands to the agent in place of the system's, and files dropped on it arrive as a person's do.
#![cfg(feature = "mcp")]

mod mcp_client;

use std::{
  sync::{Arc, Mutex},
  time::{Duration, Instant},
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::Column,
  mcp::{FileDialogBroker, McpConfig, Scope},
  node::Element,
  ui::DropZone,
};
use mcp_client::Client;
use serde_json::json;

type Heard = Arc<Mutex<Vec<String>>>;

struct Scene;

#[derive(Clone)]
struct SceneProps {
  heard: Heard,
}

impl PartialEq for SceneProps {
  fn eq(&self, _: &Self) -> bool {
    true
  }
}

impl lurq::app::component::DevtoolsInspectable for SceneProps {}

impl Component for Scene {
  type Props = SceneProps;

  fn create(_: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let heard = ctx.props::<SceneProps>().heard.clone();
    let zone = DropZone::new()
      .accept(["txt"])
      .id("zone")
      .on_files(move |files| {
        heard
          .lock()
          .unwrap()
          .extend(files.iter().map(|file| file.name().to_owned()))
      })
      .mount(ctx);
    Column::new().padding(8.0).child(zone)
  }
}

#[test]
fn a_press_opens_a_dialog_the_agent_answers_and_a_drop_arrives_as_a_persons() {
  let heard: Heard = Arc::default();
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(600, 400);
  tree.mount_root::<Scene>(&mut app, SceneProps { heard: heard.clone() });
  tree.pass_headless(&mut app);
  let broker = FileDialogBroker::new();
  let mcp = tree.enable_mcp(
    McpConfig::new()
      .app_name("lurq-files-test")
      .scope(Scope::custom("file_dialogs"))
      .file_dialogs(broker.clone()),
  );
  let (port, token) = (mcp.port(), mcp.token().to_owned());
  let chosen = std::env::temp_dir().join("lurq-chosen-notes.txt");
  let dropped = std::env::temp_dir().join("lurq-dropped-notes.txt");
  let (chosen_path, dropped_path) = (
    chosen.to_string_lossy().into_owned(),
    dropped.to_string_lossy().into_owned(),
  );

  let agent = std::thread::spawn(move || {
    let mut client = Client::new(port, token);
    client.initialize();
    // The finder answers in lines of text, each starting with the element's ref.
    let found = client.tool("lurq_find_by_id", json!({"id": "zone"}));
    let zone = found
      .split_whitespace()
      .next()
      .unwrap_or_else(|| panic!("the zone's ref: {found}"))
      .to_owned();
    client.tool("lurq_interact", json!({"action": "click", "ref": zone}));
    // The dialog is asked for once the press is handled; the agent waits for it.
    let deadline = Instant::now() + Duration::from_secs(30);
    let request = loop {
      let listed = client.tool_json("lurq_file_dialogs", json!({}));
      if let Some(request) = listed["requests"].as_array().and_then(|requests| requests.first()) {
        break request.clone();
      }
      assert!(Instant::now() < deadline, "no dialog was asked for: {listed}");
      std::thread::sleep(Duration::from_millis(20));
    };
    let answered = client.tool_json(
      "lurq_file_dialog_respond",
      json!({
        "request_id": request["request_id"], "window": request["window"],
        "operation": request["operation"], "action": "select", "paths": [chosen_path],
      }),
    );
    let dropped = client.tool_json(
      "lurq_interact",
      json!({"action": "drop_files", "ref": zone, "paths": [dropped_path]}),
    );
    (request, answered, dropped)
  });
  let deadline = Instant::now() + Duration::from_secs(60);
  while !agent.is_finished() {
    assert!(Instant::now() < deadline, "the agent did not finish");
    // As the shell's loop does: the requests, the futures woken, a pass.
    tree.drain_mcp_requests(&mut app);
    tree.tick_futures();
    tree.pass_headless(&mut app);
    std::thread::sleep(Duration::from_millis(2));
  }
  // The answer reaches the zone's future on the passes after the agent is done.
  for _ in 0..50 {
    tree.tick_futures();
    tree.pass_headless(&mut app);
    if heard.lock().unwrap().len() == 2 {
      break;
    }
    std::thread::sleep(Duration::from_millis(5));
  }
  tree.shutdown_mcp();
  let (request, answered, dropped) = agent.join().expect("agent thread");

  assert_eq!(request["operation"], "open_files", "the zone takes several files");
  assert_eq!(
    request["filters"],
    json!([["txt", ["txt"]]]),
    "what it accepts is the dialog's filter"
  );
  assert_eq!(
    request["title"], "Drop files here",
    "the dialog is titled by the zone's label"
  );
  assert!(answered.get("error").is_none(), "{answered}");
  assert_eq!(dropped["handled"], true, "{dropped}");
  let mut heard = heard.lock().unwrap().clone();
  heard.sort();
  assert_eq!(heard, ["lurq-chosen-notes.txt", "lurq-dropped-notes.txt"]);
}
