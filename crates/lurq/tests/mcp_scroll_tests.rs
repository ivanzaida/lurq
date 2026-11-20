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

/// A feed of three pages that scrolls by pages, 100 by 100.
struct Feed {
  state: ScrollState,
}

/// The state the feed scrolls by, which the test holds too.
struct FeedProps(ScrollState);

// Props of a component are inspectable by DevTools in a build with it; this test needs nothing shown.
impl lurq::app::component::DevtoolsInspectable for FeedProps {}

impl PartialEq for FeedProps {
  fn eq(&self, _: &Self) -> bool {
    true
  }
}

impl Component for Feed {
  type Props = FeedProps;

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      state: ctx.props::<FeedProps>().0.clone(),
    }
  }

  fn render(&self, _: &mut Ctx) -> impl Into<Element> {
    // In a box of its own: the root of a component is called by the component's name.
    Column::new().size(200.0, 200.0).child(
      lurq::components::ScrollVertical::new(Column::new().size(100.0, 300.0))
        .with_scroll_state(self.state.clone())
        .id("feed")
        .size(100.0, 100.0),
    )
  }
}

/// Serves the tree's MCP requests until `agent` is done, stepping what a finger left moving where `steps`.
fn serve<T: Send + 'static>(tree: &mut Tree, app: &mut App, agent: std::thread::JoinHandle<T>, steps: bool) -> T {
  let deadline = Instant::now() + Duration::from_secs(60);
  while !agent.is_finished() {
    assert!(Instant::now() < deadline, "the agent did not finish");
    if steps {
      tree.finger_tick(Instant::now());
    }
    tree.drain_mcp_requests(app);
    tree.pass_headless(app);
    std::thread::sleep(Duration::from_millis(2));
  }
  agent.join().expect("agent thread")
}

#[test]
fn a_scroll_container_says_where_its_content_is_and_where_it_is_going() {
  use lurq::app::events::TouchPhase;
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(200, 200);
  let state = ScrollState::new();
  state.set_paging(true);
  tree.mount_root::<Feed>(&mut app, FeedProps(state.clone()));
  tree.pass_headless(&mut app);
  let mcp = tree.enable_mcp(McpConfig::new().app_name("lurq-scroll-state-test"));
  let (port, token) = (mcp.port(), mcp.token().to_owned());
  let feed = |client: &mut Client| client.tool_json("lurq_inspect", json!({"role": "scrollvertical"}));

  // At rest on its first page.
  let agent = std::thread::spawn({
    let token = token.clone();
    move || {
      let mut client = Client::new(port, token);
      client.initialize();
      feed(&mut client)
    }
  });
  let at_rest = serve(&mut tree, &mut app, agent, true);
  let said = attrs(&at_rest["matches"][0]);
  assert!(said.contains(r#"["scroll_offset","0,0"]"#), "{said} of {at_rest}");
  assert!(said.contains(r#"["scroll_page","0"]"#), "{said}");
  assert!(said.contains(r#"["scroll_target_page","0"]"#), "{said}");
  assert!(!said.contains("scroll_moving"), "{said}");

  // A short flick up: the finger has lifted 20 into the first page, and the list is on its way to the second.
  let start = Instant::now();
  tree.set_press_clock(Some(start));
  tree.finger(1, TouchPhase::Start, 50.0, 90.0, None);
  for (millis, y) in [(8, 75.0), (16, 65.0), (24, 55.0)] {
    tree.set_press_clock(Some(start + Duration::from_millis(millis)));
    tree.finger(1, TouchPhase::Move, 50.0, y, None);
  }
  tree.set_press_clock(Some(start + Duration::from_millis(28)));
  tree.finger(1, TouchPhase::End, 50.0, 55.0, None);
  tree.set_press_clock(None);
  assert_eq!(state.scroll_y(), 20.0);
  // Asked at once, with nothing stepping what the finger left: it is on its way.
  let agent = std::thread::spawn({
    let token = token.clone();
    move || {
      let mut client = Client::new(port, token);
      client.initialize();
      feed(&mut client)
    }
  });
  let moving = serve(&mut tree, &mut app, agent, false);
  let said = attrs(&moving["matches"][0]);
  assert!(said.contains(r#"["scroll_moving","true"]"#), "it is on its way: {said}");
  assert!(said.contains(r#"["scroll_offset","0,20"]"#), "{said}");
  assert!(
    said.contains(r#"["scroll_target_page","1"]"#),
    "and says where to: {said}"
  );

  // And once time has brought it to its page.
  let agent = std::thread::spawn(move || {
    let mut client = Client::new(port, token);
    client.initialize();
    std::thread::sleep(Duration::from_millis(900));
    feed(&mut client)
  });
  let rested = serve(&mut tree, &mut app, agent, true);
  tree.shutdown_mcp();
  let said = attrs(&rested["matches"][0]);
  assert!(said.contains(r#"["scroll_offset","0,100"]"#), "{said}");
  assert!(said.contains(r#"["scroll_page","1"]"#), "{said}");
  assert!(!said.contains("scroll_moving"), "{said}");
  assert_eq!(state.scroll_y(), 100.0);
}
