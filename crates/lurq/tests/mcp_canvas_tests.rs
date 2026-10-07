//! End to end: a bar chart drawn on a canvas, read and hovered through the
//! embedded MCP server over HTTP, the way an agent drives it.
#![cfg(all(feature = "mcp", feature = "canvas"))]

mod mcp_client;

use std::{
  sync::Mutex,
  time::{Duration, Instant},
};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::MouseEvent},
  canvas::{CanvasHandle, CanvasItem, CanvasObserver},
  components::{Canvas, Column, Text},
  core::{ElementRef, Signal},
  mcp::McpConfig,
  node::Element,
};
use mcp_client::Client;
use serde_json::{Value, json};

const BARS: [(&str, &str, f32); 3] = [("mon", "Mon", 12.0), ("tue", "Tue", 18.0), ("wed", "Wed", 7.0)];

struct BarChart {
  reference: ElementRef,
  hovered: Signal<Option<String>>,
  observer: Mutex<Option<CanvasObserver>>,
}

impl Component for BarChart {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      reference: ctx.element_ref(),
      hovered: ctx.signal(None),
      observer: Mutex::new(None),
    }
  }

  fn render(&self, _: &mut Ctx) -> impl Into<Element> {
    let (reference, hovered) = (self.reference.clone(), self.hovered.clone());
    Column::new()
      .padding(8.0)
      .child(
        Canvas::new()
          .software()
          .ref_element(self.reference.clone())
          .id("runs-chart")
          .width(240.0)
          .height(120.0)
          .on_mouse_move(move |event: MouseEvent| {
            let item = reference.as_canvas().and_then(|canvas| {
              canvas
                .point_from_window(event.x, event.y)
                .and_then(|(x, y)| canvas.item_at(x, y))
            });
            let tip =
              item.map(|item| format!("{}: {}", item.label.unwrap_or_default(), item.value.unwrap_or_default()));
            if hovered.get_untracked() != tip {
              hovered.set(tip);
            }
          }),
      )
      .child(Text::new(&self.hovered.get().unwrap_or_default()).id("tooltip"))
  }

  fn after_layout(&self) {
    let mut observer = self.observer.lock().unwrap();
    if observer.is_none() {
      let canvas = self.reference.as_canvas().expect("canvas is bound after layout");
      let target = canvas.clone();
      *observer = Some(canvas.observe_metrics(move |_| draw(&target)));
    }
  }
}

/// Draw the bars and describe each one; both happen on every redraw.
fn draw(canvas: &CanvasHandle) {
  let context = canvas.context_2d();
  context.reset();
  context.set_fill_style("#60a5fa");
  let mut items = Vec::new();
  for (index, (id, label, runs)) in BARS.into_iter().enumerate() {
    let (x, height) = (20.0 + index as f32 * 60.0, runs * 5.0);
    context.fill_rect(x, 110.0 - height, 40.0, height);
    items.push(
      CanvasItem::rect(id, "bar", x, 110.0 - height, 40.0, height)
        .label(label)
        .value(format!("{runs} runs")),
    );
  }
  canvas.set_items(items);
}

fn ref_on_line(text: &str, marker: &str) -> String {
  let line = text
    .lines()
    .find(|line| line.contains(marker))
    .unwrap_or_else(|| panic!("{marker} in {text}"));
  let start = line.find("[ref_").expect("ref on line") + 1;
  line[start..start + line[start..].find(']').unwrap()].to_owned()
}

#[test]
fn an_agent_reads_and_hovers_canvas_bars_through_the_mcp_server() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(600, 400);
  tree.mount_root::<BarChart>(&mut app, ());
  tree.pass_headless(&mut app);
  let mcp = tree.enable_mcp(McpConfig::new().app_name("lurq-canvas-test"));
  let (port, token) = (mcp.port(), mcp.token().to_owned());
  assert_ne!(port, 0, "the MCP server is listening");

  let agent = std::thread::spawn(move || {
    let mut client = Client::new(port, token);
    client.initialize();
    let tree_text = client.tool("lurq_read_tree", json!({}));
    let tue = ref_on_line(&tree_text, "item:tue");
    let moved = client.tool("lurq_interact", json!({"action": "move", "ref": tue}));
    let after_hover = client.tool("lurq_read_tree", json!({}));
    // The semantic route: find the bar by role and name, hover it without coordinates.
    let wed: Value =
      serde_json::from_str(&client.tool("lurq_inspect", json!({"role": "bar", "query": "wed"}))).unwrap();
    let acted = client.tool("lurq_act", json!({"ref": wed["matches"][0]["ref"], "action": "hover"}));
    let tooltip = client.tool("lurq_inspect", json!({"query": "tooltip"}));
    (tree_text, moved, after_hover, (wed, acted, tooltip))
  });

  let deadline = Instant::now() + Duration::from_secs(60);
  while !agent.is_finished() {
    assert!(Instant::now() < deadline, "the agent did not finish");
    if tree.drain_mcp_requests(&mut app) {
      tree.pass_headless(&mut app);
    }
    std::thread::sleep(Duration::from_millis(2));
  }
  tree.shutdown_mcp();
  let (tree_text, moved, after_hover, (wed, acted, tooltip)) = agent.join().expect("agent thread");

  // Canvas at (8, 8); "tue" is x 80..120, y 20..110 in the canvas.
  assert!(tree_text.contains("- Canvas #runs-chart ["), "{tree_text}");
  assert!(tree_text.contains("{items=3}"), "{tree_text}");
  let tue_line = tree_text.lines().find(|line| line.contains("item:tue")).unwrap();
  assert!(tue_line.contains("- bar item:tue [ref_"), "{tue_line}");
  assert!(
    tue_line.ends_with("\"Tue\" @88,28 40x90 {value=\"18 runs\"}"),
    "{tue_line}"
  );
  let moved: Value = serde_json::from_str(&moved).unwrap();
  assert_eq!((moved["x"].as_f64(), moved["y"].as_f64()), (Some(108.0), Some(73.0)));
  assert!(
    after_hover.contains("#tooltip [") && after_hover.contains("\"Tue: 18 runs\""),
    "{after_hover}"
  );

  let wed = &wed["matches"][0];
  assert_eq!(
    (&wed["name"], &wed["state"]["value"]),
    (&json!("Wed"), &json!("7 runs"))
  );
  assert_eq!(wed["bounds"], json!([148.0, 83.0, 40.0, 35.0]));
  assert_eq!(wed["actions"], json!(["hover"]));
  assert_eq!(serde_json::from_str::<Value>(&acted).unwrap()["dispatched"], true);
  let tooltip: Value = serde_json::from_str(&tooltip).unwrap();
  assert_eq!(tooltip["matches"][0]["name"], "Wed: 7 runs", "{tooltip}");
}
