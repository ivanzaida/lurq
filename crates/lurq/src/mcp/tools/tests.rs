use std::sync::Arc;

use super::*;
use crate::mcp::{
  Scope,
  registry::ToolRegistry,
  shared::{McpShared, McpToolResult},
};

struct TestSurface;

impl raw_window_handle::HasWindowHandle for TestSurface {
  fn window_handle(&self) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
    let handle = raw_window_handle::Win32WindowHandle::new(std::num::NonZeroIsize::new(1).unwrap());
    Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(handle.into()) })
  }
}

impl raw_window_handle::HasDisplayHandle for TestSurface {
  fn display_handle(&self) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
    Ok(unsafe { raw_window_handle::DisplayHandle::borrow_raw(raw_window_handle::WindowsDisplayHandle::new().into()) })
  }
}

#[test]
fn semantic_inspect_finds_button_by_child_label_and_act_invokes_ref() {
  use std::sync::atomic::{AtomicUsize, Ordering};

  use crate::components::{Button, Column};

  let count = Arc::new(AtomicUsize::new(0));
  let mut tree = Tree::new();
  let mut app = App::new();
  let state = state();
  tree.resize(400, 200);
  tree.set_root(
    Column::new()
      .child(Button::new("Save").id("save").on_click({
        let count = count.clone();
        move |_| {
          count.fetch_add(1, Ordering::SeqCst);
        }
      }))
      .child(Button::new("Save").id("save-other")),
  );
  tree.pass(&mut app, &TestSurface);

  let inspected = json(call(
    &mut tree,
    &mut app,
    &state,
    "lurq_inspect",
    serde_json::json!({"query": "save", "role": "button"}),
  ));
  let matches = inspected["matches"].as_array().unwrap();
  assert_eq!(matches.len(), 2);
  let save = matches.iter().find(|element| element["id"] == "save").unwrap();
  assert_eq!(save["name"], "Save");
  assert_eq!(save["actions"], serde_json::json!(["invoke"]));
  let ref_id = save["ref"].as_str().unwrap().to_owned();

  let result = json(call(
    &mut tree,
    &mut app,
    &state,
    "lurq_act",
    serde_json::json!({"ref": ref_id, "action": "invoke"}),
  ));
  assert_eq!(result["dispatched"], true);
  assert_eq!(count.load(Ordering::SeqCst), 1);

  // A new snapshot replaces its predecessor's refs, so an old handle cannot
  // accidentally invoke a different element after a UI change.
  json(call(&mut tree, &mut app, &state, "lurq_inspect", serde_json::json!({})));
  assert!(
    call(
      &mut tree,
      &mut app,
      &state,
      "lurq_act",
      serde_json::json!({"ref": ref_id, "action": "invoke"}),
    )
    .is_err()
  );
}

#[test]
fn semantic_inspect_keeps_masked_input_value_private() {
  use crate::{components::TextInput, core::Signal};
  let secret = "private-inspect-secret";
  let mut tree = Tree::new();
  let mut app = App::new();
  let state = state();
  tree.set_root(
    TextInput::new(Signal::new(secret.to_owned()))
      .mask()
      .placeholder("Password"),
  );
  let result = json(call(&mut tree, &mut app, &state, "lurq_inspect", serde_json::json!({})));
  assert!(!result.to_string().contains(secret));
  assert_eq!(result["tree"]["role"], "textbox");
  assert_eq!(result["tree"]["name"], "Password");
  assert_eq!(result["tree"]["state"]["masked"], true);
}

#[test]
fn semantic_act_rejects_a_ref_whose_name_changed() {
  use crate::components::{Button, Text};
  let mut tree = Tree::new();
  let mut app = App::new();
  let state = state();
  tree.set_root(Button::empty().child(Text::new("Save").id("label")));
  let inspected = json(call(
    &mut tree,
    &mut app,
    &state,
    "lurq_inspect",
    serde_json::json!({"role": "button"}),
  ));
  let ref_id = inspected["matches"][0]["ref"].as_str().unwrap().to_owned();
  tree.get_element_by_id_mut("label").unwrap().set_text_content("Delete");
  let error = call(
    &mut tree,
    &mut app,
    &state,
    "lurq_act",
    serde_json::json!({"ref": ref_id, "action": "invoke"}),
  )
  .err()
  .unwrap();
  assert!(error.contains("changed role or name"), "{error}");
}

pub(super) fn output_text(result: McpToolResult) -> String {
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
pub(super) fn state() -> McpState {
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
pub(super) fn call(
  tree: &mut Tree,
  app: &mut App,
  state: &McpState,
  tool: &str,
  args: serde_json::Value,
) -> McpToolResult {
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
pub(super) fn json(result: McpToolResult) -> serde_json::Value {
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
#[test]
fn windows_report_the_title_set_at_runtime() {
  let mut tree = Tree::new();
  let mut app = App::new();
  let state = state();
  let windows = |tree: &mut Tree, app: &mut App| json(call(tree, app, &state, "lurq_windows", serde_json::json!({})));
  assert_eq!(
    windows(&mut tree, &mut app)["windows"][0]["title"],
    serde_json::Value::Null
  );
  tree.window().handle().set_title("Tasks - Orchester");
  assert_eq!(windows(&mut tree, &mut app)["windows"][0]["title"], "Tasks - Orchester");
}

#[test]
fn lookups_before_layout_say_the_bounds_are_unknown() {
  let mut tree = Tree::new();
  let mut app = App::new();
  let state = state();
  tree.set_root(crate::components::Rect::new(10.0, 10.0).id("fresh"));
  let found = output_text(call(
    &mut tree,
    &mut app,
    &state,
    "lurq_find_by_id",
    serde_json::json!({"id": "fresh"}),
  ));
  assert!(found.contains("(bounds unknown: not laid out yet)"), "{found}");
  assert!(!found.contains("@0,0"), "{found}");
}
