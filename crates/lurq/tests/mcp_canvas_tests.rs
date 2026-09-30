//! End to end: a bar chart drawn on a canvas, read and hovered through the
//! embedded MCP server over HTTP, the way an agent drives it.
#![cfg(all(feature = "mcp", feature = "canvas"))]

use std::{
  io::{BufRead, BufReader, Read, Write},
  net::TcpStream,
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

/// Minimal streamable-HTTP MCP client: JSON-RPC over POST, replies as JSON or SSE.
struct Client {
  port: u16,
  token: String,
  session: Option<String>,
  next_id: u64,
}

impl Client {
  fn post(&mut self, body: &Value) -> Option<Value> {
    let mut stream = TcpStream::connect(("127.0.0.1", self.port)).expect("connect to the MCP server");
    stream.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
    let payload = body.to_string();
    let session = self
      .session
      .as_ref()
      .map(|id| format!("mcp-session-id: {id}\r\n"))
      .unwrap_or_default();
    write!(
      stream,
      "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\n\
       Accept: application/json, text/event-stream\r\n{session}Content-Length: {}\r\nConnection: close\r\n\r\n{payload}",
      self.port,
      self.token,
      payload.len()
    )
    .unwrap();
    let mut reader = BufReader::new(stream);
    let mut chunked = false;
    let mut length = None;
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    assert!(line.contains(" 200 ") || line.contains(" 202 "), "HTTP status: {line}");
    loop {
      line.clear();
      reader.read_line(&mut line).unwrap();
      let Some((name, value)) = line.trim_end().split_once(':') else {
        break;
      };
      let (name, value) = (name.to_ascii_lowercase(), value.trim());
      match name.as_str() {
        "mcp-session-id" => self.session = Some(value.to_owned()),
        "transfer-encoding" => chunked = value.eq_ignore_ascii_case("chunked"),
        "content-length" => length = value.parse::<usize>().ok(),
        _ => {}
      }
    }
    let wanted = body.get("id")?.clone();
    let mut received = Vec::new();
    loop {
      if chunked {
        line.clear();
        reader.read_line(&mut line).unwrap();
        let size = usize::from_str_radix(line.trim(), 16).expect("chunk size");
        if size == 0 {
          break;
        }
        let mut chunk = vec![0; size + 2];
        reader.read_exact(&mut chunk).unwrap();
        received.extend_from_slice(&chunk[..size]);
      } else {
        let mut all = vec![0; length.unwrap_or(0)];
        reader.read_exact(&mut all).unwrap();
        received = all;
      }
      let text = String::from_utf8_lossy(&received);
      let messages = text
        .lines()
        .filter_map(|line| line.strip_prefix("data:"))
        .chain(text.trim_start().starts_with('{').then_some(text.as_ref()))
        .filter_map(|data| serde_json::from_str::<Value>(data.trim()).ok());
      if let Some(reply) = messages.into_iter().find(|message| message.get("id") == Some(&wanted)) {
        return Some(reply);
      }
      assert!(chunked, "no JSON-RPC reply in {text}");
    }
    panic!("stream ended without a reply to {wanted}");
  }

  fn initialize(&mut self) {
    self.request(
      "initialize",
      json!({"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "lurq-test", "version": "0"}}),
    );
    self.post(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
  }

  fn request(&mut self, method: &str, params: Value) -> Value {
    self.next_id += 1;
    let reply = self
      .post(&json!({"jsonrpc": "2.0", "id": self.next_id, "method": method, "params": params}))
      .expect("requests get replies");
    reply
      .get("result")
      .cloned()
      .unwrap_or_else(|| panic!("{method} failed: {reply}"))
  }

  /// Call a tool and return its text content.
  fn tool(&mut self, name: &str, arguments: Value) -> String {
    let result = self.request("tools/call", json!({"name": name, "arguments": arguments}));
    assert_ne!(result["isError"], true, "{name} failed: {result}");
    result["content"][0]["text"].as_str().unwrap_or_default().to_owned()
  }
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
    let mut client = Client {
      port,
      token,
      session: None,
      next_id: 0,
    };
    client.initialize();
    let tree_text = client.tool("lurq_read_tree", json!({}));
    let tue = ref_on_line(&tree_text, "#tue");
    let moved = client.tool("lurq_interact", json!({"action": "move", "ref": tue}));
    let after_hover = client.tool("lurq_read_tree", json!({}));
    (tree_text, moved, after_hover)
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
  let (tree_text, moved, after_hover) = agent.join().expect("agent thread");

  // Canvas at (8, 8); "tue" is x 80..120, y 20..110 in the canvas.
  assert!(tree_text.contains("- Canvas #runs-chart ["), "{tree_text}");
  assert!(tree_text.contains("{items=3}"), "{tree_text}");
  let tue_line = tree_text.lines().find(|line| line.contains("#tue")).unwrap();
  assert!(tue_line.contains("- bar #tue [ref_"), "{tue_line}");
  assert!(tue_line.ends_with("\"Tue\" @88,28 40x90 {value=18 runs}"), "{tue_line}");
  let moved: Value = serde_json::from_str(&moved).unwrap();
  assert_eq!((moved["x"].as_f64(), moved["y"].as_f64()), (Some(108.0), Some(73.0)));
  assert!(
    after_hover.contains("#tooltip [") && after_hover.contains("\"Tue: 18 runs\""),
    "{after_hover}"
  );
}
