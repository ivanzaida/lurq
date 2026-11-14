//! What a scroll container says to what reads the tree, read through the embedded MCP server as an agent reads it:
//! each bar of content that overflows is a scrollbar with how far it is scrolled, from 0 to 100, and a scroll area
//! with a label is a region of that name.
#![cfg(feature = "mcp")]

mod mcp_client;

use std::time::{Duration, Instant};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx, events::ScrollPhase},
  components::Column,
  layout::layout_kind::ScrollState,
  mcp::McpConfig,
  node::Element,
  ui::{ScrollArea, ScrollAreaOrientation},
};
use mcp_client::Client;
use serde_json::{Value, json};

struct Page {
  terms: ScrollState,
}

impl Component for Page {
  type Props = ();

  fn create(_: &mut Ctx) -> Self {
    Self {
      terms: ScrollState::new(),
    }
  }

  fn render(&self, _: &mut Ctx) -> impl Into<Element> {
    Column::new()
      .size(600.0, 400.0)
      .spacing(8.0)
      .child(
        ScrollArea::new()
          .id("terms")
          .label("Terms")
          .scroll_state(self.terms.clone())
          .width(200.0)
          .height(100.0)
          .child(Column::new().size(200.0, 500.0)),
      )
      .child(
        ScrollArea::new()
          .id("both")
          .orientation(ScrollAreaOrientation::Both)
          .width(200.0)
          .height(100.0)
          .child(Column::new().size(400.0, 100.0)),
      )
  }
}

fn attrs(element: &Value) -> String {
  element["attrs"].to_string()
}

#[test]
fn a_scroll_areas_bars_are_scrollbars_with_their_value_and_a_labelled_one_is_a_region() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(600, 400);
  tree.mount_root::<Page>(&mut app, ());
  tree.pass_headless(&mut app);
  // A quarter of the way down: 100 of the 400 it can go, as a touchpad moves it.
  tree.scroll(100.0, 50.0, 0.0, -100.0, ScrollPhase::Scroll);
  tree.pass_headless(&mut app);
  let mcp = tree.enable_mcp(McpConfig::new().app_name("lurq-scroll-test"));
  let (port, token) = (mcp.port(), mcp.token().to_owned());

  let agent = std::thread::spawn(move || {
    let mut client = Client::new(port, token);
    client.initialize();
    let region = client.tool_json("lurq_inspect", json!({"role": "region"}));
    let both = client.tool_json("lurq_inspect", json!({"role": "scrollboth"}));
    (region, both)
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
  let (region, both) = agent.join().expect("agent thread");

  let matches = region["matches"].as_array().unwrap_or_else(|| panic!("{region}"));
  assert_eq!(matches.len(), 1, "{region}");
  let terms = &matches[0];
  assert_eq!(terms["id"], "terms");
  assert_eq!(terms["name"], "Terms");
  assert!(
    attrs(terms).contains(r#"["scrollbar_vertical","25"]"#),
    "a quarter of the way down: {}",
    attrs(terms)
  );
  assert!(!attrs(terms).contains("scrollbar_horizontal"), "it scrolls down only");

  // Both ways, but only across overflows: one bar.
  let both = &both["matches"].as_array().unwrap_or_else(|| panic!("{both}"))[0];
  assert_eq!(both["id"], "both");
  assert!(
    attrs(both).contains(r#"["scrollbar_horizontal","0"]"#),
    "{}",
    attrs(both)
  );
  assert!(!attrs(both).contains("scrollbar_vertical"), "the content fits down");
}
