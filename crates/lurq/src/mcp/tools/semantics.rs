//! What the tools say about a node: interactivity, semantic role and name, actions, value summaries and inspection
//! attributes.

use super::canvas_items;
use crate::{
  layout::layout_kind::LayoutKind,
  node::{node::Node, node_kind::NodeKind},
};

pub(super) fn is_interactive(node: &Node) -> bool {
  let events = &node.events;
  !events.on_click.is_empty()
    || !events.on_mouse_click.is_empty()
    || !events.on_dblclick.is_empty()
    || !events.on_mouse_down.is_empty()
    || !events.on_mouse_up.is_empty()
    || !events.on_drag_start.is_empty()
    || !events.on_drop.is_empty()
    || !events.on_key_down.is_empty()
    || matches!(
      node.node_kind(),
      NodeKind::TextInput { .. } | NodeKind::Checkbox { .. } | NodeKind::Slider { .. } | NodeKind::Select { .. }
    )
    || matches!(node.layout_kind(), LayoutKind::ScrollModifier { .. })
}

pub(super) fn semantic_role(node: &Node) -> String {
  if let Some((_, role)) = node.debug_attrs().iter().find(|(key, _)| key.as_ref() == "a11y_role") {
    return role.to_string();
  }
  if node.button_kind_value().is_some() {
    return "button".into();
  }
  match node.node_kind() {
    NodeKind::TextInput { .. } => "textbox".into(),
    NodeKind::Checkbox { .. } => "checkbox".into(),
    NodeKind::Slider { .. } => "slider".into(),
    NodeKind::Select { .. } => "combobox".into(),
    NodeKind::Text { .. } => "text".into(),
    #[cfg(feature = "markdown")]
    NodeKind::RichText { .. } => "text".into(),
    // A clickable canvas is a drawing surface, not a button; its items carry the semantics.
    #[cfg(feature = "canvas")]
    NodeKind::Canvas { .. } => "canvas".into(),
    _ if !node.events.on_click.is_empty() || !node.events.on_mouse_click.is_empty() => "button".into(),
    _ => node.tag_name().to_ascii_lowercase(),
  }
}

pub(super) fn is_text_node(node: &Node) -> bool {
  match node.node_kind() {
    NodeKind::Text { .. } => true,
    #[cfg(feature = "markdown")]
    NodeKind::RichText { .. } => true,
    _ => false,
  }
}

pub(super) fn descendant_label(node: &Node, parts: &mut Vec<String>) {
  if is_text_node(node)
    && let Some(text) = node.inspection_text().filter(|text| !text.trim().is_empty())
  {
    parts.push(text);
  }
  for child in node.children() {
    descendant_label(child, parts);
  }
}

pub(super) fn semantic_name(node: &Node) -> Option<String> {
  if let Some((_, name)) = node
    .debug_attrs()
    .iter()
    .find(|(key, _)| matches!(key.as_ref(), "a11y_name" | "aria-label"))
  {
    return Some(name.to_string());
  }
  if let NodeKind::TextInput { state, .. } = node.node_kind() {
    return state.placeholder().map(|placeholder| placeholder.to_string());
  }
  if is_text_node(node) {
    return node.inspection_text().filter(|text| !text.trim().is_empty());
  }
  if semantic_role(node) == "button" {
    let mut parts = Vec::new();
    descendant_label(node, &mut parts);
    let joined = parts.join(" ");
    if !joined.is_empty() {
      return Some(joined);
    }
  }
  None
}

/// Pointer-move consumers: hovering opens their tooltips or highlights.
pub(super) fn can_hover(node: &Node) -> bool {
  !node.events.on_mouse_move.is_empty() || !node.events.on_mouse_enter.is_empty()
}

/// Semantic actions `lurq_act` accepts for an element with these capabilities.
pub(super) fn semantic_actions(invoke: bool, hover: bool) -> Vec<&'static str> {
  [("invoke", invoke), ("hover", hover)]
    .into_iter()
    .filter_map(|(action, available)| available.then_some(action))
    .collect()
}

pub(super) fn can_invoke(node: &Node) -> bool {
  node.button_kind_value().is_some()
    || !node.events.on_click.is_empty()
    || !node.events.on_mouse_click.is_empty()
    || matches!(
      node.node_kind(),
      NodeKind::TextInput { .. } | NodeKind::Checkbox { .. } | NodeKind::Slider { .. } | NodeKind::Select { .. }
    )
}

pub(super) fn node_value_summary(node: &Node) -> Option<String> {
  match node.node_kind() {
    NodeKind::TextInput { state, .. } if state.is_masked() => Some(format!("value={:?}", state.caret_source_text())),
    NodeKind::TextInput { state, .. } => Some(format!("value={:?}", state.value())),
    NodeKind::Checkbox { state } => Some(format!("checked={}", state.is_checked())),
    NodeKind::Slider { state } => Some(format!("value={}", state.value_string())),
    NodeKind::Select { state } => {
      let labels = state.labels();
      let selected = state
        .selected_indices()
        .into_iter()
        .filter_map(|index| labels.get(index).map(|label| label.to_string()))
        .collect::<Vec<_>>();
      Some(format!(
        "selected={:?}{}",
        selected,
        if state.is_open() { " open" } else { "" }
      ))
    }
    _ => None,
  }
}

pub(super) fn truncate_text(text: &str, max: usize) -> String {
  if text.chars().count() <= max {
    return text.to_owned();
  }
  let cut: String = text.chars().take(max).collect();
  format!("{cut}…")
}

pub(super) fn inspection_attrs(node: &Node) -> Vec<(String, String)> {
  let mut attrs: Vec<_> = node
    .debug_attrs()
    .iter()
    .map(|(name, value)| (name.to_string(), value.to_string()))
    .collect();
  if node.is_masked_input() {
    attrs.push(("masked".to_owned(), "true".to_owned()));
  }
  attrs.extend(canvas_items::canvas_attrs(node));
  attrs
}
