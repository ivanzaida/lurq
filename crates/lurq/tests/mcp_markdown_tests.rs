#![cfg(all(feature = "mcp", feature = "markdown"))]
//! An MCP client sees a Markdown link as a `link` named by its words, reads
//! the paragraph's text without the link's trailing runs, and follows the
//! link as synthetic input, with the button and modifiers it asks for; and
//! `scroll_to` on a code line wider than its block shows the line's start.

mod mcp_client;

use std::{
  sync::{Arc, Mutex},
  time::{Duration, Instant},
};

use lurq::{
  app::{
    App, Tree,
    component::Component,
    ctx::Ctx,
    events::{MouseButton, is_synthetic_input},
  },
  components::{Markdown, MarkdownLinkPresentation, MarkdownProps, MarkdownRun},
  mcp::McpConfig,
  node::Element,
};
use mcp_client::Client;
use serde_json::json;

type Followed = Arc<Mutex<Vec<(String, bool)>>>;
/// Each activation's button, Ctrl, Meta and whether it was synthetic.
type Activated = Arc<Mutex<Vec<(MouseButton, bool, bool, bool)>>>;

#[derive(Clone)]
struct Props {
  source: Arc<str>,
  followed: Followed,
  activated: Activated,
}

impl Default for Props {
  fn default() -> Self {
    Self {
      source: "Open [PR 42](https://github.com/acme/pull/42) to review.".into(),
      followed: Followed::default(),
      activated: Activated::default(),
    }
  }
}

impl PartialEq for Props {
  fn eq(&self, other: &Self) -> bool {
    self.source == other.source
      && Arc::ptr_eq(&self.followed, &other.followed)
      && Arc::ptr_eq(&self.activated, &other.activated)
  }
}

#[cfg(feature = "devtools")]
impl lurq::app::component::DevtoolsInspectable for Props {}

struct Reply;

impl Component for Reply {
  type Props = Props;

  fn create(_ctx: &mut Ctx) -> Self {
    Self
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let Props {
      source,
      followed,
      activated,
    } = ctx.props::<Props>().clone();
    Markdown::mount(
      ctx,
      MarkdownProps::new(source)
        .link_presentation(|_| MarkdownLinkPresentation::link().trailing([MarkdownRun::plain(" github.com")]))
        .on_link_click(move |link| {
          followed
            .lock()
            .unwrap()
            .push((link.destination().to_owned(), is_synthetic_input()))
        })
        .on_link_activate(move |_, event| {
          activated
            .lock()
            .unwrap()
            .push((event.button, event.ctrl, event.meta, is_synthetic_input()))
        })
        .width(500.0),
    )
  }
}

/// Runs `agent` against a window showing [`Reply`], serving its MCP calls
/// until it finishes.
fn serve<T: Send + 'static>(props: Props, agent: impl FnOnce(Client) -> T + Send + 'static) -> T {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(600, 300);
  tree.mount_root::<Reply>(&mut app, props);
  tree.pass_headless(&mut app);
  let mcp = tree.enable_mcp(McpConfig::new().app_name("lurq-markdown-test"));
  let (port, token) = (mcp.port(), mcp.token().to_owned());

  let agent = std::thread::spawn(move || {
    let mut client = Client::new(port, token);
    client.initialize();
    agent(client)
  });

  let deadline = Instant::now() + Duration::from_secs(60);
  while !agent.is_finished() {
    assert!(Instant::now() < deadline, "the agent did not finish");
    if tree.drain_mcp_requests(&mut app) {
      tree.pass_headless(&mut app);
    }
    std::thread::sleep(Duration::from_millis(2));
  }
  agent.join().expect("the agent")
}

#[test]
fn an_agent_sees_a_markdown_link_by_role_and_name_and_follows_it() {
  let props = Props::default();
  let followed = props.followed.clone();
  let (tree_text, found) = serve(props, |mut client| {
    let tree_text = client.tool("lurq_read_tree", json!({}));
    let found = client.tool_json("lurq_inspect", json!({"role": "link", "query": "PR 42"}));
    // Refs come from the latest read: the inspection's own.
    client.tool(
      "lurq_interact",
      json!({"action": "click", "ref": found["matches"][0]["ref"]}),
    );
    (tree_text, found)
  });

  let matches = found["matches"].as_array().expect("matches");
  assert_eq!(matches.len(), 1, "one link named by its words: {found}");
  assert_eq!(matches[0]["role"], "link", "{found}");
  assert_eq!(matches[0]["name"], "PR 42", "{found}");
  let paragraph = tree_text
    .lines()
    .find(|line| line.contains("Open PR 42 to review."))
    .unwrap_or_else(|| panic!("the paragraph's text without its trailing run: {tree_text}"));
  assert!(!paragraph.contains("github.com"), "{paragraph}");
  assert!(
    !tree_text.contains(['\u{2066}', '\u{2068}', '\u{2069}']),
    "no isolate marks"
  );
  assert_eq!(
    *followed.lock().unwrap(),
    vec![("https://github.com/acme/pull/42".to_owned(), true)],
    "the click reached the app as synthetic input"
  );
}

#[test]
fn an_agent_middle_clicks_a_markdown_link_with_modifiers() {
  let props = Props::default();
  let (followed, activated) = (props.followed.clone(), props.activated.clone());
  serve(props, |mut client| {
    let found = client.tool_json("lurq_inspect", json!({"role": "link", "query": "PR 42"}));
    client.tool(
      "lurq_interact",
      json!({
        "action": "click",
        "ref": found["matches"][0]["ref"],
        "button": "middle",
        "modifiers": ["ctrl", "meta"],
      }),
    );
  });

  assert_eq!(
    *activated.lock().unwrap(),
    vec![(MouseButton::Middle, true, true, true)],
    "the middle click reached the app with its modifiers, as synthetic input"
  );
  assert!(
    followed.lock().unwrap().is_empty(),
    "on_link_click takes left clicks only"
  );
}

const LINE_END: &str = "END-OF-THE-LAST-LINE";

/// The `x` of a match's bounds.
fn x_of(found: &serde_json::Value) -> f64 {
  found["bounds"][0].as_f64().unwrap_or_else(|| panic!("bounds: {found}"))
}

#[test]
fn an_agent_scrolling_to_a_code_line_wider_than_its_block_sees_the_line_start() {
  let long = "- the long line that runs past the column ".repeat(8);
  let props = Props {
    source: format!("```\n{long}\nshort {LINE_END}\n```").into(),
    ..Props::default()
  };
  let (at_start, after_scroll_to, wheeled, back) = serve(props, |mut client| {
    let find = |client: &mut Client| client.tool_json("lurq_inspect", json!({"query": LINE_END}))["matches"][0].clone();
    let at_start = find(&mut client);
    client.tool("lurq_interact", json!({"action": "scroll_to", "ref": at_start["ref"]}));
    let after_scroll_to = find(&mut client);
    // The line's centre is far outside the window: the wheel is over its start.
    client.tool(
      "lurq_interact",
      json!({"action": "wheel", "x": 100.0, "y": 30.0, "delta_x": -200.0}),
    );
    let wheeled = find(&mut client);
    client.tool("lurq_interact", json!({"action": "scroll_to", "ref": wheeled["ref"]}));
    let back = find(&mut client);
    (at_start, after_scroll_to, wheeled, back)
  });
  let start = x_of(&at_start);
  assert_eq!(
    x_of(&after_scroll_to),
    start,
    "a line at its start is left there, not scrolled to its end"
  );
  assert_eq!(
    x_of(&wheeled),
    start - 200.0,
    "a sideways wheel scrolls the block: {wheeled}"
  );
  assert_eq!(
    x_of(&back),
    start - 6.0,
    "scroll_to brings the start back, 8 px inside the box"
  );
}
