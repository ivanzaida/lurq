//! `lurq_set_value`: direct form-control values.

use super::{
  resolve::{find_node, resolve_ref},
  windows::window_tree_mut,
};
use crate::{
  app::Tree,
  mcp::{
    McpState,
    shared::{McpToolOutput, McpToolResult},
  },
  node::node_kind::NodeKind,
};

pub(super) fn set_value_tool(tree: &mut Tree, state: &McpState, args: &serde_json::Value) -> McpToolResult {
  let ref_id = args
    .get("ref")
    .and_then(|value| value.as_str())
    .ok_or("`ref` is required")?;
  let value = args.get("value").cloned().ok_or("`value` is required")?;
  let resolved = resolve_ref(state, ref_id)?;
  let target = window_tree_mut(tree, &resolved.window, state.include_devtools)?;
  let node = find_node(target, resolved.node_id)
    .ok_or_else(|| format!("ref {ref_id:?} no longer resolves to a live element; call lurq_read_tree again"))?;

  let outcome = match node.node_kind() {
    NodeKind::TextInput { state, .. } => match value.as_str() {
      Some(text) => {
        state.set_value_external(text.to_owned());
        if state.is_masked() {
          Ok(serde_json::json!({ "ok": true, "kind": "TextInput", "masked": true, "value": state.caret_source_text() }))
        } else {
          Ok(serde_json::json!({ "ok": true, "kind": "TextInput", "value": text }))
        }
      }
      None => Err("TextInput takes a string value".to_owned()),
    },
    NodeKind::Checkbox { state } => match value.as_bool() {
      Some(checked) => {
        if state.is_checked() != checked {
          state.toggle();
        }
        Ok(serde_json::json!({ "ok": true, "kind": "Checkbox", "checked": checked }))
      }
      None => Err("Checkbox takes a boolean value".to_owned()),
    },
    // One number sets a slider, or the thumb of a range slider nearer it; two numbers set a range slider's low
    // value and its high one.
    NodeKind::Slider { state } => match (value.as_f64(), value.as_array()) {
      (Some(number), _) => {
        state.set_value_external(number as f32);
        Ok(serde_json::json!({ "ok": true, "kind": "Slider", "value": state.value_string() }))
      }
      (None, Some(pair)) if state.is_range() => match pair.as_slice() {
        [low, high] => match (low.as_f64(), high.as_f64()) {
          (Some(low), Some(high)) => {
            state.set_values_external(low as f32, high as f32);
            Ok(serde_json::json!({ "ok": true, "kind": "Slider", "value": state.value_string() }))
          }
          _ => Err("a range slider takes two numbers: [low, high]".to_owned()),
        },
        _ => Err(format!(
          "a range slider takes two numbers, [low, high]; got {} values",
          pair.len()
        )),
      },
      (None, Some(_)) => Err("this Slider has one value: it takes a number, not an array".to_owned()),
      (None, None) if state.is_range() => {
        Err("a range slider takes a number (the nearer thumb) or two numbers, [low, high]".to_owned())
      }
      (None, None) => Err("Slider takes a numeric value".to_owned()),
    },
    NodeKind::Select { state } => {
      let labels = state.labels();
      let index = if let Some(index) = value.as_u64() {
        let index = index as usize;
        if index >= labels.len() {
          return Err(format!(
            "Select has {} options; index {index} is out of range",
            labels.len()
          ));
        }
        index
      } else if let Some(label) = value.as_str() {
        labels
          .iter()
          .position(|candidate| candidate.eq_ignore_ascii_case(label))
          .ok_or_else(|| {
            format!(
              "no option matches {label:?}; options: {:?}",
              labels.iter().map(|label| label.to_string()).collect::<Vec<_>>()
            )
          })?
      } else {
        return Err("Select takes an option label (string) or index (number)".to_owned());
      };
      // commit() fires the app's on_change and closes a single-select menu —
      // the same path an option click takes.
      state.commit(index);
      Ok(
        serde_json::json!({ "ok": true, "kind": "Select", "selected": labels.get(index).map(|label| label.to_string()) }),
      )
    }
    _ => Err(format!(
      "ref {ref_id:?} is a {} — lurq_set_value supports TextInput, Checkbox, Slider, and Select",
      node.tag_name()
    )),
  };

  target.request_redraw();
  outcome.map(McpToolOutput::Json)
}
