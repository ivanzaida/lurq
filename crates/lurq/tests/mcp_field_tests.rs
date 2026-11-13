//! What a field and the controls of the design system say to what reads the tree, read through the embedded MCP
//! server as an agent reads it: the role, the name, whether it is checked, and the field's description. These are
//! `a11y_` attributes, which a node keeps only with the `devtools` or `mcp` feature, so the ui tests cannot see them.
#![cfg(all(feature = "mcp", feature = "form"))]

mod mcp_client;

use std::time::{Duration, Instant};

use lurq::{
  app::{App, Tree, component::Component, ctx::Ctx},
  components::Column,
  core::Signal,
  mcp::McpConfig,
  node::Element,
  ui::{Checkbox, Field, Input, RadioGroup, Switch},
};
use mcp_client::Client;
use serde_json::{Value, json};

struct Form {
  email: Signal<String>,
  plan: Signal<u8>,
}

impl Component for Form {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      email: ctx.signal("ivan@".to_owned()),
      plan: ctx.signal(2),
    }
  }

  fn render(&self, _: &mut Ctx) -> impl Into<Element> {
    Column::new()
      .padding(8.0)
      .spacing(12.0)
      .child(Field::new("Email", Input::new(self.email.clone()).id("email")).error("Enter a full email address."))
      .child(
        Field::new("Terms", Checkbox::new("I agree to the terms").checked(true).id("terms"))
          .description("Read them first."),
      )
      .child(
        Field::new(
          "Plan",
          RadioGroup::new(self.plan.clone(), [(1, "Free"), (2, "Pro")]).id("plan"),
        )
        .description("You can change it later."),
      )
      .child(
        Field::new(
          "Sync",
          Switch::new("Sync in background").label_hidden(true).on(true).id("sync"),
        )
        .horizontal(true),
      )
  }
}

/// The one element of `role` the inspector finds.
fn one(client: &mut Client, role: &str) -> Value {
  let found = client.tool_json("lurq_inspect", json!({"role": role}));
  let matches = found["matches"].as_array().unwrap_or_else(|| panic!("{found}"));
  assert_eq!(matches.len(), 1, "one {role}: {found}");
  matches[0].clone()
}

#[test]
fn a_field_names_and_describes_its_control_and_each_control_says_what_it_is() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.resize(600, 400);
  tree.mount_root::<Form>(&mut app, ());
  tree.pass_headless(&mut app);
  let mcp = tree.enable_mcp(McpConfig::new().app_name("lurq-field-test"));
  let (port, token) = (mcp.port(), mcp.token().to_owned());

  let agent = std::thread::spawn(move || {
    let mut client = Client::new(port, token);
    client.initialize();
    let found = ["textbox", "checkbox", "radiogroup", "switch"].map(|role| one(&mut client, role));
    let radios = client.tool_json("lurq_inspect", json!({"role": "radio"}));
    (found, radios)
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
  let ([input, checkbox, group, switch], radios) = agent.join().expect("agent thread");

  // The input is called by the field's label, and its error is its description.
  assert_eq!(input["id"], "email");
  assert_eq!(input["name"], "Email");
  assert_eq!(input["description"], "Enter a full email address.");
  assert_eq!(input["state"]["value"], "ivan@");
  let attrs = input["attrs"].to_string();
  assert!(
    attrs.contains(r#"["a11y_invalid","true"]"#),
    "the field's error judges it: {attrs}"
  );

  // The checkbox's own label is shown, so it is its name; the field's description is its description.
  assert_eq!(checkbox["id"], "terms");
  assert_eq!(checkbox["name"], "I agree to the terms");
  assert_eq!(checkbox["state"]["checked"], true);
  assert_eq!(checkbox["description"], "Read them first.");

  // The group is called by the field's label and described, not each radio in it.
  assert_eq!(group["id"], "plan");
  assert_eq!(group["name"], "Plan");
  assert_eq!(group["description"], "You can change it later.");
  let radios = radios["matches"].as_array().unwrap();
  let named: Vec<(&str, bool)> = radios
    .iter()
    .map(|radio| (radio["name"].as_str().unwrap(), radio["state"]["checked"] == true))
    .collect();
  assert_eq!(named, [("Free", false), ("Pro", true)]);
  assert!(radios.iter().all(|radio| radio.get("description").is_none()));

  // The switch's own label is hidden: the label a person reads is the field's, and that is its name.
  assert_eq!(switch["id"], "sync");
  assert_eq!(switch["name"], "Sync");
  assert_eq!(switch["state"]["checked"], true);
  assert!(switch.get("description").is_none(), "the field has none");
}
