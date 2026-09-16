use super::*;
use crate::mcp::{Scope, registry::ToolRegistry, shared::McpShared};

fn output_text(result: McpToolResult) -> String {
  match result.unwrap() {
    McpToolOutput::Text(text) => text,
    McpToolOutput::Json(value) => value.to_string(),
    _ => panic!("unexpected output"),
  }
}

#[test]
fn masked_values_are_redacted_in_tree_lookup_refs_and_set_value() {
  use crate::{
    components::{Column, TextInput},
    core::Signal,
  };
  let secret = "audit-secret-Ж-123";
  let custom = "audit-custom-456";
  let mut tree = Tree::new();
  let mut app = App::new();
  let state = state();
  tree.set_root(
    Column::new()
      .child(
        TextInput::new(Signal::new(secret.to_owned()))
          .mask()
          .id("password")
          .class("credentials"),
      )
      .child(
        TextInput::new(Signal::new(custom.to_owned()))
          .mask_char('#')
          .id("custom"),
      )
      .child(TextInput::new(Signal::new("public-value".to_owned())).id("plain"))
      .child(
        TextInput::new(Signal::new(String::new()))
          .mask()
          .placeholder("Type password")
          .id("empty"),
      ),
  );

  for filter in ["all", "interactive"] {
    for max_chars in [1, 60, 30000] {
      let output = output_text(call(
        &mut tree,
        &mut app,
        &state,
        "lurq_read_tree",
        serde_json::json!({"filter": filter, "max_chars": max_chars}),
      ));
      assert!(!output.contains(secret));
      assert!(!output.contains(custom));
      if max_chars == 30000 {
        assert!(output.contains("masked=true"));
        assert!(output.contains("public-value"));
        assert!(output.contains("Type password"));
        assert!(output.contains(&"#".repeat(custom.chars().count())));
      }
    }
  }
  for (tool, args) in [
    ("lurq_find_by_id", serde_json::json!({"id":"password"})),
    ("lurq_find_by_class", serde_json::json!({"class":"credentials"})),
  ] {
    let output = output_text(call(&mut tree, &mut app, &state, tool, args));
    assert!(!output.contains(secret));
    assert!(output.contains("masked=true"));
  }
  // lurq_find searches and formats this same table on the server thread.
  let refs = state.shared.refs.lock().unwrap();
  for record in &refs.records {
    let output = format_ref_line(record);
    assert!(!output.contains(secret));
    assert!(!output.contains(custom));
  }
  let password_ref = refs
    .records
    .iter()
    .find(|r| r.element_id.as_deref() == Some("password"))
    .unwrap()
    .id
    .clone();
  let plain_ref = refs
    .records
    .iter()
    .find(|r| r.element_id.as_deref() == Some("plain"))
    .unwrap()
    .id
    .clone();
  drop(refs);
  let changed = "audit-updated-secret-789";
  let reply = json(call(
    &mut tree,
    &mut app,
    &state,
    "lurq_set_value",
    serde_json::json!({"ref":password_ref, "value":changed}),
  ));
  assert_eq!(reply["masked"], true);
  assert_eq!(reply["value"], "•".repeat(changed.chars().count()));
  assert!(!reply.to_string().contains(changed));
  assert_eq!(
    tree
      .get_element_by_id_mut("password")
      .unwrap()
      .as_text_input()
      .unwrap()
      .value(),
    changed
  );
  let reply = json(call(
    &mut tree,
    &mut app,
    &state,
    "lurq_set_value",
    serde_json::json!({"ref":plain_ref, "value":"still-public"}),
  ));
  assert_eq!(reply["value"], "still-public");
  let input = tree.get_element_by_id_mut("custom").unwrap().as_text_input().unwrap();
  assert!(input.is_masked());
  assert_eq!(input.mask(), Some('#'));
}

#[cfg(feature = "devtools")]
#[test]
fn devtools_redacts_masked_node_text_and_inspector_shape() {
  use crate::{app::devtools::DevToolsSnapshot, components::TextInput, core::Signal};
  let secret = "audit-devtools-secret-Ж";
  let mut tree = Tree::new();
  tree.set_root(TextInput::new(Signal::new(secret.to_owned())).mask_char('#'));
  for snapshot in [
    DevToolsSnapshot::from_tree(&tree),
    DevToolsSnapshot::from_tree_for_selection(&tree, &[]),
  ] {
    let node = snapshot.root.as_ref().unwrap();
    assert!(node.is_masked());
    assert_eq!(node.text.as_deref(), Some("#".repeat(secret.chars().count()).as_str()));
    assert!(!format!("{snapshot:?}").contains(secret));
  }
}
fn state() -> McpState {
  let (_, receiver) = std::sync::mpsc::channel();
  McpState {
    shared: Arc::new(McpShared::new(
      [Scope::Observe, Scope::Interact].into_iter().collect(),
      Default::default(),
      "test".into(),
      "test".into(),
      None,
    )),
    registry: Arc::new(ToolRegistry {
      tools: super::super::registry::builtin_tools(false),
    }),
    receiver,
    include_devtools: false,
    server: None,
    discovery_path: None,
  }
}
fn call(tree: &mut Tree, app: &mut App, state: &McpState, tool: &str, args: serde_json::Value) -> McpToolResult {
  let (reply, mut rx) = tokio::sync::oneshot::channel();
  execute(
    tree,
    app,
    state,
    McpRequest {
      tool: tool.into(),
      args,
      reply,
    },
  );
  rx.try_recv().unwrap()
}
fn json(result: McpToolResult) -> serde_json::Value {
  match result.unwrap() {
    McpToolOutput::Json(v) => v,
    _ => panic!("expected json"),
  }
}
#[test]
fn mcp_close_and_menu_obey_scopes_and_report_dispatch_outcome() {
  let mut tree = Tree::new();
  let mut app = App::new();
  let state = state();
  tree.window().handle().on_close_requested(|r| {
    assert_eq!(r.source(), crate::app::CloseRequestSource::App);
    r.cancel();
  });
  let request = serde_json::json!({"action": "request_close"});
  assert_eq!(
    json(call(&mut tree, &mut app, &state, "lurq_interact", request.clone()))["stayed_open"],
    true
  );
  tree.window().handle().clear_close_requested_handler();
  assert_eq!(
    json(call(&mut tree, &mut app, &state, "lurq_interact", request.clone()))["close_queued"],
    true
  );
  app.set_menu_bar(crate::app::MenuBar::default());
  assert_eq!(
    json(call(&mut tree, &mut app, &state, "lurq_menu", serde_json::json!({})))["model"]["application"]["quit"]["id"],
    "quit"
  );
  assert_eq!(
    json(call(
      &mut tree,
      &mut app,
      &state,
      "lurq_interact",
      serde_json::json!({"action":"menu_activate", "id":"missing"})
    ))["activated"],
    false
  );
  state.shared.remove_scope(&Scope::Interact);
  assert!(call(&mut tree, &mut app, &state, "lurq_interact", request).is_err());
  assert!(call(&mut tree, &mut app, &state, "lurq_menu", serde_json::json!({})).is_ok());
}
