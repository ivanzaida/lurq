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
    NodeKind::RichText { .. } => "text".into(),
    #[cfg(feature = "markdown")]
    NodeKind::InlineLink { .. } => "link".into(),
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
    NodeKind::RichText { .. } => true,
    _ => false,
  }
}

/// Whether the node and all it holds are nothing to tooling: `a11y_hidden` is `true` (a watermark's marks, a
/// chart's axis drawn beside what it says).
pub(super) fn hidden_from_tooling(node: &Node) -> bool {
  node
    .debug_attrs()
    .iter()
    .any(|(key, value)| key.as_ref() == "a11y_hidden" && value.as_ref() == "true")
}

pub(super) fn descendant_label(node: &Node, parts: &mut Vec<String>) {
  if hidden_from_tooling(node) {
    return;
  }
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
  #[cfg(feature = "markdown")]
  if let NodeKind::InlineLink { .. } = node.node_kind() {
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

/// What the app says the node is for, besides its name: a field's description or error, a tooltip's text.
pub(super) fn semantic_description(node: &Node) -> Option<String> {
  node
    .debug_attrs()
    .iter()
    .find(|(key, _)| matches!(key.as_ref(), "a11y_description" | "aria-description"))
    .map(|(_, description)| description.to_string())
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
  if node.is_sensitive_text() {
    attrs.push(("sensitive".to_owned(), "true".to_owned()));
  }
  attrs.extend(canvas_items::canvas_attrs(node));
  attrs.extend(scrollbar_attrs(node));
  attrs
}

/// The bars of a scroll container whose content overflows, each a `scrollbar` with its value as an ARIA
/// scrollbar has it: how far the content is scrolled along that way, from 0 at its start to 100 at its end.
/// `scrollbar_vertical=25` for a container a quarter of the way down. A container whose content fits has none.
pub(super) fn scrollbar_attrs(node: &Node) -> Vec<(String, String)> {
  use crate::layout::layout_kind::ScrollDirection;
  let Some((state, direction)) = node.scroll_container() else {
    return Vec::new();
  };
  let percent = |offset: f32, content: f32, viewport: f32| {
    let max = content - viewport;
    (max > 0.5).then(|| format!("{}", ((offset / max).clamp(0.0, 1.0) * 100.0).round()))
  };
  let mut attrs = Vec::new();
  if matches!(direction, ScrollDirection::Vertical | ScrollDirection::Both)
    && let Some(value) = percent(state.scroll_y(), state.content_height(), state.viewport_height())
  {
    attrs.push(("scrollbar_vertical".to_owned(), value));
  }
  if matches!(direction, ScrollDirection::Horizontal | ScrollDirection::Both)
    && let Some(value) = percent(state.scroll_x(), state.content_width(), state.viewport_width())
  {
    attrs.push(("scrollbar_horizontal".to_owned(), value));
  }
  attrs.extend(scroll_state_attrs(&state, direction));
  attrs
}

/// Where a scroll container's content is, for who looks into why a list shows what it shows: its offset in
/// logical pixels (`scroll_offset=0,844`, across then down) where its content overflows; for a container that
/// scrolls by pages, or whose page something follows, the page nearest its offset and the one it is going to
/// (`scroll_page`, `scroll_target_page`: one number along the way it scrolls, two for one that scrolls both
/// ways); and `scroll_moving=true` while a finger, its momentum, a way to a page or a wheel's scroll has its
/// content.
fn scroll_state_attrs(
  state: &crate::layout::layout_kind::ScrollState,
  direction: crate::layout::layout_kind::ScrollDirection,
) -> Vec<(String, String)> {
  use crate::layout::layout_kind::ScrollDirection;
  let mut attrs = Vec::new();
  let overflows =
    state.content_width() - state.viewport_width() > 0.5 || state.content_height() - state.viewport_height() > 0.5;
  if overflows {
    attrs.push((
      "scroll_offset".to_owned(),
      format!("{},{}", state.scroll_x().round(), state.scroll_y().round()),
    ));
  }
  if overflows && (state.paging() || state.page_watched()) {
    let along = |page: (usize, usize)| match direction {
      ScrollDirection::Horizontal => page.0.to_string(),
      ScrollDirection::Vertical => page.1.to_string(),
      ScrollDirection::Both => format!("{},{}", page.0, page.1),
    };
    attrs.push(("scroll_page".to_owned(), along(state.page())));
    attrs.push(("scroll_target_page".to_owned(), along(state.target_page())));
  }
  if state.moving() {
    attrs.push(("scroll_moving".to_owned(), "true".to_owned()));
  }
  attrs
}

/// App-provided text (ids, classes, roles, tags, attribute names and values,
/// window names) as one outline token: as-is when it is a plain word of
/// letters, digits and `-_.:/`, otherwise quoted with escapes, so it can never
/// end its line, bracket or brace and forge an element or ref.
pub(super) fn token(text: &str) -> std::borrow::Cow<'_, str> {
  let plain = !text.is_empty()
    && text
      .chars()
      .all(|character| character.is_alphanumeric() || matches!(character, '-' | '_' | '.' | ':' | '/'));
  if plain {
    std::borrow::Cow::Borrowed(text)
  } else {
    std::borrow::Cow::Owned(format!("{text:?}"))
  }
}

/// ` {name=value}` with both sides as [`token`]s; values longer than 80
/// characters end in an ellipsis, like element text.
pub(super) fn attr(name: &str, value: &str) -> String {
  format!(" {{{}={}}}", token(name), token(&truncate_text(value, 80)))
}
