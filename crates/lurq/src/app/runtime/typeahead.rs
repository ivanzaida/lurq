//! Typing a letter in a focus group that has typeahead
//! ([`FocusGroup::typeahead`](crate::node::FocusGroup::typeahead)) moves the
//! focus to the element whose label starts with it, as a native menu or list
//! does.
//!
//! Letters typed within [`TYPEAHEAD_WINDOW`] of one another are one prefix:
//! "s", "a" goes to "Save as" past "Select all". A letter typed again and
//! again goes round the elements that start with it. A pause starts a new
//! prefix. The prefix is looked for from the focused element on, so that the
//! one it is on stays while it still matches; a single letter from the one
//! after it. Nothing that matches leaves the focus where it is.
//!
//! An element's label is its `focus_label`, or, for a button with one text
//! in it, that text.

use std::time::{Duration, Instant};

use super::{FocusTarget, NodeKind, Tree, find_node_by_id, focus_visible, tab_navigation};
use crate::node::Node;

/// How long after a letter the next one goes on the same prefix.
pub(super) const TYPEAHEAD_WINDOW: Duration = Duration::from_millis(1000);

/// What has been typed into a group's typeahead, and when the last of it was.
pub(super) struct Typeahead {
  typed: String,
  at: Instant,
}

impl Tree {
  /// Moves the focus within the innermost focus group around it that has
  /// typeahead to the element whose label starts with what has been typed,
  /// `key` the last of it. Returns whether the group took the key: a single
  /// printable character that is not a space, while the focus is not on a
  /// control that keeps its keys.
  pub(super) fn focus_group_typeahead(&mut self, key: &str) -> bool {
    let mut chars = key.chars();
    let (Some(typed), None) = (chars.next(), chars.next()) else {
      return false;
    };
    if typed.is_whitespace() || typed.is_control() {
      return false;
    }
    let (Some(focused), Some(path)) = (self.focused_node, self.focused_path.clone()) else {
      return false;
    };
    let Some(root) = &self.root else {
      return false;
    };
    let mut group = None;
    let mut node = root;
    for &index in &path {
      if node.focus_group_value().is_some_and(|group| group.typeahead) {
        group = Some(node);
      }
      let Some(child) = node.children().get(index) else {
        return false;
      };
      node = child;
    }
    // A text input takes the letters, a slider and a select their keys.
    if matches!(
      node.node_kind(),
      NodeKind::TextInput { .. } | NodeKind::Slider { .. } | NodeKind::Select { .. }
    ) {
      return false;
    }
    let Some(group) = group else {
      return false;
    };

    let now = self.now();
    let typed: String = typed.to_lowercase().collect();
    let prefix = match self.typeahead.take() {
      Some(Typeahead { typed: before, at }) if now.duration_since(at) <= TYPEAHEAD_WINDOW => before + &typed,
      _ => typed,
    };
    self.typeahead = Some(Typeahead {
      typed: prefix.clone(),
      at: now,
    });

    let stops = tab_navigation::group_stops(group, &self.dropped_nodes);
    let current = stops.iter().position(|stop| stop.input_id == focused);
    let target = find(group, &stops, current, &prefix);
    if let Some(target) = target
      && Some(target.input_id) != Some(focused)
    {
      self.set_input_modality(focus_visible::InputModality::Keyboard);
      self.focus_node(target);
      self.scroll_focused_into_view();
    }
    true
  }
}

/// The stop of `stops` whose label starts with `prefix`, looked for round the
/// group: from `current` on for a prefix of more than one letter, from the one
/// after it for one letter, or one letter typed again and again.
fn find(group: &Node, stops: &[FocusTarget], current: Option<usize>, prefix: &str) -> Option<FocusTarget> {
  let mut letters = prefix.chars();
  let first = letters.next()?;
  let repeated = letters.all(|letter| letter == first);
  let (wanted, start) = match (repeated, current) {
    (true, Some(current)) => (first.to_string(), current + 1),
    (false, Some(current)) => (prefix.to_owned(), current),
    (_, None) => (prefix.to_owned(), 0),
  };
  (0..stops.len())
    .map(|step| stops[(start + step) % stops.len()])
    .find(|stop| label(group, stop.input_id).is_some_and(|label| label.to_lowercase().starts_with(&wanted)))
}

/// The label of the element `id` in `group`: its `focus_label`, or, for a
/// button, the one text in it.
fn label(group: &Node, id: crate::core::NodeId) -> Option<String> {
  let node = find_node_by_id(group, id)?;
  if let Some(label) = node.focus_label() {
    return Some(label.to_owned());
  }
  node.button_kind_value()?;
  let mut texts = Vec::new();
  collect_texts(node, &mut texts);
  match texts.as_slice() {
    [text] => Some((*text).to_owned()),
    _ => None,
  }
}

fn collect_texts<'a>(node: &'a Node, texts: &mut Vec<&'a str>) {
  if let Some(text) = node.text_content() {
    texts.push(text);
  }
  for child in node.children() {
    collect_texts(child, texts);
  }
}
