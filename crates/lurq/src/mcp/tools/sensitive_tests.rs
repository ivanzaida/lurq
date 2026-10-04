use super::{
  read_tree::format_ref_line,
  tests::{call, json, output_text, state},
  *,
};
use crate::{
  components::{Button, Column, Text},
  core::{REDACTED, Sensitive},
  layout::quad::QuadContent,
};

const SECRET: &str = "otp-493117-Ж";

fn tree_with(text: Text) -> Tree {
  let mut tree = Tree::new();
  tree.resize(400, 200);
  tree.set_root(
    Column::new()
      .child(text.id("code").class("one-time"))
      .child(Button::empty().child(Text::new(SECRET).sensitive()).id("copy")),
  );
  tree.pass_headless(&mut App::new());
  tree
}

/// Every text quad the last layout paints, with its box.
fn painted_text(tree: &Tree) -> Vec<(String, [f32; 4])> {
  tree
    .painted_quads()
    .into_iter()
    .filter_map(|quad| match quad.content {
      QuadContent::Text { text, .. } => Some((text, [quad.x, quad.y, quad.width, quad.height])),
      _ => None,
    })
    .collect()
}

#[test]
fn sensitive_text_is_redacted_in_every_mcp_tool() {
  let mut tree = tree_with(Text::new(SECRET).sensitive());
  let mut app = App::new();
  let state = state();
  for filter in ["all", "interactive"] {
    let output = output_text(call(
      &mut tree,
      &mut app,
      &state,
      "lurq_read_tree",
      serde_json::json!({"filter": filter}),
    ));
    assert!(!output.contains(SECRET), "{output}");
    assert!(!output.contains("493117"), "{output}");
    if filter == "all" {
      assert!(output.contains(REDACTED), "{output}");
      assert!(output.contains("sensitive=true"), "{output}");
    }
  }
  for (tool, args) in [
    ("lurq_inspect", serde_json::json!({})),
    ("lurq_inspect", serde_json::json!({"role": "button"})),
    ("lurq_find_by_id", serde_json::json!({"id": "code"})),
    ("lurq_find_by_class", serde_json::json!({"class": "one-time"})),
  ] {
    let reply = call(&mut tree, &mut app, &state, tool, args);
    let output = match reply {
      Ok(McpToolOutput::Json(value)) => value.to_string(),
      other => output_text(other),
    };
    assert!(!output.contains(SECRET), "{tool}: {output}");
    assert!(output.contains(REDACTED), "{tool}: {output}");
  }
  // A button named by its sensitive label is named by the marker.
  let buttons = json(call(
    &mut tree,
    &mut app,
    &state,
    "lurq_inspect",
    serde_json::json!({"role": "button"}),
  ));
  assert_eq!(buttons["matches"][0]["name"], REDACTED);
  // lurq_find searches and formats the ref table on the server thread.
  for record in &state.shared.refs.lock().unwrap().records {
    assert!(!format_ref_line(record).contains(SECRET));
  }
}

#[test]
fn sensitive_text_is_laid_out_and_painted_like_any_text() {
  let sensitive = tree_with(Text::new(SECRET).sensitive());
  let plain = tree_with(Text::new(SECRET));
  let painted = painted_text(&sensitive);
  assert!(painted.iter().any(|(text, _)| text == SECRET), "{painted:?}");
  assert_eq!(painted, painted_text(&plain));

  let mut sensitive = sensitive;
  let mut plain = plain;
  let bounds = |tree: &mut Tree| tree.get_element_by_id_mut("code").and_then(|code| code.bounds());
  assert_eq!(bounds(&mut sensitive), bounds(&mut plain));
  // The app still reads its own text.
  assert_eq!(
    sensitive
      .get_element_by_id_mut("code")
      .and_then(|code| code.text_content().map(str::to_owned))
      .as_deref(),
    Some(SECRET)
  );
}

#[test]
fn sensitive_text_stays_redacted_after_its_content_changes() {
  let mut tree = tree_with(Text::new("first").sensitive());
  tree
    .get_element_by_id_mut("code")
    .expect("code")
    .set_text_content(SECRET);
  tree.pass_headless(&mut App::new());
  let output = output_text(call(
    &mut tree,
    &mut App::new(),
    &state(),
    "lurq_read_tree",
    serde_json::json!({}),
  ));
  assert!(!output.contains(SECRET), "{output}");
}

#[test]
fn a_sensitive_value_never_formats_its_content() {
  let value = Sensitive::new(SECRET.to_owned());
  assert_eq!(format!("{value:?}"), format!("Sensitive({REDACTED})"));
  assert_eq!(value.expose(), SECRET);
  let mut fields = Vec::new();
  crate::app::component::DevtoolsInspectable::write_info(&value, &mut fields);
  assert!(!format!("{fields:?}").contains(SECRET));
}

#[cfg(feature = "devtools")]
#[test]
fn devtools_snapshots_and_signal_history_redact_sensitive_text() {
  use std::sync::Arc;

  use parking_lot::Mutex;

  use crate::{
    app::{
      component::{Component, DevtoolsInspectable},
      ctx::Ctx,
      devtools::DevToolsSnapshot,
    },
    core::Signal,
    node::Element,
  };

  /// Hands the component's signal out to the test.
  #[derive(Clone, Default)]
  struct CodeProps(Arc<Mutex<Option<Signal<Sensitive<String>>>>>);
  impl PartialEq for CodeProps {
    fn eq(&self, other: &Self) -> bool {
      Arc::ptr_eq(&self.0, &other.0)
    }
  }
  impl DevtoolsInspectable for CodeProps {}

  struct CodeView {
    code: Signal<Sensitive<String>>,
  }

  impl Component for CodeView {
    type Props = CodeProps;

    fn create(ctx: &mut Ctx) -> Self {
      let code = ctx.signal(Sensitive::new("pending".to_owned()));
      *ctx.props::<CodeProps>().0.lock() = Some(code.clone());
      Self { code }
    }

    fn render(&self, _ctx: &mut Ctx) -> impl Into<Element> {
      Text::new(self.code.get().expose()).sensitive()
    }
  }

  let props = CodeProps::default();
  let mut tree = Tree::new();
  let mut app = App::new();
  tree.mount_root::<CodeView>(&mut app, props.clone());
  tree.pass_headless(&mut app);
  let signal = props.0.lock().clone().expect("the component created its signal");
  signal.set(Sensitive::new(SECRET.to_owned()));
  tree.pass_headless(&mut app);

  for snapshot in [
    DevToolsSnapshot::from_tree(&tree),
    DevToolsSnapshot::from_tree_for_selection(&tree, &[]),
  ] {
    let dump = format!("{snapshot:?}");
    assert!(!dump.contains(SECRET), "{dump}");
    let root = snapshot.root.as_ref().expect("root");
    let code = &root.signals[0];
    assert_eq!(code.formatted_value().as_deref(), Some(REDACTED));
    // Both values format as the marker, so the change leaves no history entry.
    assert!(!format!("{:?}", code.history()).contains(SECRET));
  }
}
