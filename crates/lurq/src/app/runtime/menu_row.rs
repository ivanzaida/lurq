//! A row of menus: a focus group marked [`FocusGroup::menu_row`](crate::node::FocusGroup::menu_row), each of
//! whose elements opens a menu hung from it, as the File, Edit and View of an app's menubar.
//!
//! While a menu of the row is open and has the focus, Left and Right that nothing in the menu took (a submenu's
//! Left closes the submenu, an item's Right opens its own) close it, with the menus opened from it, and press the
//! element of the row before or after the one it hangs from: that one opens its menu, which takes the focus.
//!
//! Alt with a letter presses the element of the row whose label starts with the letter, wherever the focus is.
//! Alt pressed and let go with nothing between puts the focus on the row, at its entry, and from the row (or a menu
//! of it) gives the focus back to what had it. On a Mac, where Option with a letter types a character and an app's
//! menus are in the system's menu bar, Alt does neither.

use super::{
  FocusTarget, MouseModifiers, Tree, find_node_by_id, find_node_by_path, find_path_by_id, focus_visible,
  overlay_layers, tab_navigation, typeahead,
};
use crate::{
  core::ElementRef as OwnedElementRef,
  node::{Node, SyntheticNodeRole},
};

/// Where the focus is, seen from the page: the path of the element it is on, or of the anchor of the outermost
/// layer it is in, with the overlay host's layers it is in.
struct Place {
  /// The path in the tree (from the overlay host, where there is one) of the focused element, or of the anchor the
  /// layer it is in hangs from, followed outward to the page.
  path: Vec<usize>,
  /// The index among the overlay host's children of the outermost layer the focus is in, if it is in one.
  layer: Option<usize>,
}

impl Tree {
  /// Notes a key pressed: whether it is Alt alone, so far, which its release then acts on.
  pub(super) fn note_alt_press(&mut self, key: &str, ctrl: bool, meta: bool) {
    self.alt_alone = key == "Alt" && !ctrl && !meta;
  }

  /// A key that a row of menus takes: Left or Right in a menu opened from the row, or Alt with a letter. Returns
  /// whether it took the key.
  pub(super) fn menu_row_key(&mut self, key: &str, alt: bool, modifiers: MouseModifiers) -> bool {
    if alt {
      return self.menu_row_mnemonic(key, modifiers);
    }
    let back = match key {
      "ArrowLeft" => true,
      "ArrowRight" => false,
      _ => return false,
    };
    let Some(place) = self.focus_place() else {
      return false;
    };
    // The focus is in a menu: on the row itself the group's own keys move it.
    let Some(layer) = place.layer else {
      return false;
    };
    let Some(root) = &self.root else {
      return false;
    };
    let Some(group) = menu_row_around(root, &place.path) else {
      return false;
    };
    let Some(group_node) = find_node_by_path(root, &group) else {
      return false;
    };
    let stops = tab_navigation::group_stops(group_node, &self.dropped_nodes);
    let Some(current) = stop_at(root, &stops, &place.path) else {
      return false;
    };
    let wrap = group_node.focus_group_value().is_some_and(|group| group.wrap);
    let last = stops.len() - 1;
    let next = match (back, current) {
      (true, 0) if wrap => last,
      (true, 0) => 0,
      (true, current) => current - 1,
      (false, current) if current == last && wrap => 0,
      (false, current) if current == last => last,
      (false, current) => current + 1,
    };
    // The key is the row's even where there is no neighbour to go to.
    if next != current {
      let target = stops[next];
      self.close_layers_from(layer);
      self.press_in_row(target, modifiers);
    }
    true
  }

  /// Alt with `key`: the element of the first row of menus whose label starts with it is pressed, after the one
  /// that has the focus where several start with it.
  fn menu_row_mnemonic(&mut self, key: &str, modifiers: MouseModifiers) -> bool {
    // On a Mac, Option with a letter types a character, and an app's menus are the system's.
    if cfg!(target_os = "macos") {
      return false;
    }
    let mut chars = key.chars();
    let (Some(letter), None) = (chars.next(), chars.next()) else {
      return false;
    };
    if !letter.is_alphanumeric() {
      return false;
    }
    let letter: String = letter.to_lowercase().collect();
    let Some((group, stops)) = self.first_menu_row() else {
      return false;
    };
    let place = self.focus_place();
    let Some(root) = &self.root else {
      return false;
    };
    let Some(group_node) = find_node_by_path(root, &group) else {
      return false;
    };
    // From the element the focus is on, or the one whose menu has it: the same letter again goes on to the next.
    let current = place
      .as_ref()
      .filter(|place| place.path.starts_with(&group))
      .and_then(|place| stop_at(root, &stops, &place.path));
    let start = current.map_or(0, |current| current + 1);
    let Some(target) = (0..stops.len())
      .map(|step| stops[(start + step) % stops.len()])
      .find(|stop| {
        find_node_by_id(group_node, stop.input_id)
          .and_then(typeahead::label)
          .is_some_and(|label| label.to_lowercase().starts_with(&letter))
      })
    else {
      return false;
    };
    if let Some(Place {
      layer: Some(layer),
      path,
    }) = place
      && path.starts_with(&group)
    {
      self.close_layers_from(layer);
    }
    self.press_in_row(target, modifiers);
    true
  }

  /// Alt pressed and let go alone: the focus goes to the first row of menus, at its entry, and from the row or a
  /// menu opened from it back to what had it before. Returns whether there is a row.
  pub(super) fn menu_row_alt(&mut self) -> bool {
    if cfg!(target_os = "macos") {
      return false;
    }
    let Some((group, stops)) = self.first_menu_row() else {
      return false;
    };
    let Some(&first) = stops.first() else {
      return false;
    };
    let place = self.focus_place();
    if let Some(place) = place.filter(|place| place.path.starts_with(&group)) {
      if let Some(layer) = place.layer {
        self.close_layers_from(layer);
      }
      let back = self.menu_row_return.take().filter(|back| {
        self
          .root
          .as_ref()
          .is_some_and(|root| find_path_by_id(root, back.input_id).is_some())
      });
      match back {
        Some(back) => self.focus_node(back),
        None => {
          self.blur_focus();
        }
      }
      return true;
    }
    let Some(root) = &self.root else {
      return false;
    };
    let entry = stops
      .iter()
      .copied()
      .find(|stop| find_path_by_id(root, stop.input_id).is_some_and(|path| entry_at(root, &path)))
      .unwrap_or(first);
    self.menu_row_return = self
      .focused_node
      .zip(self.focused_event_node)
      .map(|(input_id, event_id)| FocusTarget { input_id, event_id });
    self.set_input_modality(focus_visible::InputModality::Keyboard);
    self.focus_node(entry);
    self.scroll_focused_into_view();
    true
  }

  /// Focuses `target`, an element of a row of menus, from the keyboard, and presses it: it opens its menu.
  fn press_in_row(&mut self, target: FocusTarget, modifiers: MouseModifiers) {
    self.set_input_modality(focus_visible::InputModality::Keyboard);
    self.focus_node(target);
    self.scroll_focused_into_view();
    self.activate_focused_button(modifiers);
    self.needs_redraw = true;
  }

  /// Closes the overlays of the host's layer `layer` and of those over it: a menu, and the submenus opened from it.
  fn close_layers_from(&mut self, layer: usize) {
    for entry in &self.overlay_dismiss_entries {
      if entry.layer >= layer && entry.open.get_untracked() {
        entry.open.set(false);
      }
    }
  }

  /// Where the focus is, from the page: in a layer, the anchor it hangs from, followed out to the page.
  fn focus_place(&self) -> Option<Place> {
    let root = self.root.as_ref()?;
    let mut path = self.focused_path.clone()?;
    if !root.has_synthetic_role(SyntheticNodeRole::OverlayHost) {
      return Some(Place { path, layer: None });
    }
    let mut layer = None;
    // Each step goes out one layer; a cycle of anchors cannot be longer than the layers there are.
    for _ in 0..=self.overlay_layer_anchors.len() {
      let index = *path.first()?;
      if index == 0 {
        return Some(Place { path, layer });
      }
      layer = Some(index);
      let anchor: &OwnedElementRef = self.overlay_layer_anchors.get(index - 1)?.as_ref()?;
      path = overlay_layers::path_to_ref(root, anchor)?;
    }
    None
  }

  /// The path of the first row of menus of the page that has an element to stop at, with its elements.
  fn first_menu_row(&self) -> Option<(Vec<usize>, Vec<FocusTarget>)> {
    let root = self.root.as_ref()?;
    let (page, prefix) = if root.has_synthetic_role(SyntheticNodeRole::OverlayHost) {
      (root.children().first()?, vec![0])
    } else {
      (root, Vec::new())
    };
    let mut found = None;
    find_menu_rows(page, &mut prefix.clone(), &mut |path, node| {
      if found.is_some() {
        return;
      }
      let stops = tab_navigation::group_stops(node, &self.dropped_nodes);
      if !stops.is_empty() {
        found = Some((path.to_vec(), stops));
      }
    });
    found
  }
}

/// The path of the innermost row of menus around the node at `path`, where nothing between them is a group of no
/// keys.
fn menu_row_around(root: &Node, path: &[usize]) -> Option<Vec<usize>> {
  let mut found = None;
  let mut node = root;
  for (depth, &index) in path.iter().enumerate() {
    match node.focus_group_value() {
      Some(group) if group.is_none() => found = None,
      Some(group) if group.menu_row => found = Some(path[..depth].to_vec()),
      _ => {}
    }
    node = node.children().get(index)?;
  }
  found
}

/// Calls `found` with each row of menus in `node`, in the order of the tree, and its path; one that is disabled is
/// passed by, with what is in it.
fn find_menu_rows(node: &Node, path: &mut Vec<usize>, found: &mut impl FnMut(&[usize], &Node)) {
  if node.is_disabled() {
    return;
  }
  if node.focus_group_value().is_some_and(|group| group.menu_row) {
    found(path, node);
    return;
  }
  for (index, child) in node.children().iter().enumerate() {
    path.push(index);
    find_menu_rows(child, path, found);
    path.pop();
  }
}

/// Which of `stops` the node at `path` is, or holds.
fn stop_at(root: &Node, stops: &[FocusTarget], path: &[usize]) -> Option<usize> {
  let node = find_node_by_path(root, path)?;
  stops
    .iter()
    .position(|stop| stop.input_id == node.node_id() || find_path_by_id(node, stop.input_id).is_some())
}

/// Whether the node at `path` is marked as the entry of its group (`focus_entry`).
fn entry_at(root: &Node, path: &[usize]) -> bool {
  find_node_by_path(root, path).is_some_and(Node::is_focus_entry)
}
