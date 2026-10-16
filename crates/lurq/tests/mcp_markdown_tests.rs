#![cfg(all(feature = "mcp", feature = "markdown"))]
//! An MCP client sees a Markdown link as a `link` named by its words, reads
//! the paragraph's text without the link's trailing runs, and follows the
//! link as synthetic input, with the button and modifiers it asks for.

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

#[derive(Clone, Default)]
struct Props {
  followed: Followed,
  activated: Activated,
}

impl PartialEq for Props {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.followed, &other.followed) && Arc::ptr_eq(&self.activated, &other.activated)
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
    let Props { followed, activated } = ctx.props::<Props>().clone();
    Markdown::mount(
      ctx,
      MarkdownProps::new("Open [PR 42](https://github.com/acme/pull/42) to review.")
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
