//! Interaction groups (`.interaction_group(true)`): an element under the pointer, or with the focus on it or in
//! it, lights the elements in it that have a `within` state style, as an action at the end of a row that shows
//! only while the row is pointed at or focused. An element in two groups answers to the innermost. What is lit
//! knows too whether its group is under the pointer and whether it is held down, for the parts styled by the
//! group's hover or press alone (`within_hovered_style`, `within_pressed_style`): a cell that stays as its row
//! scrolls and covers the row's veil, veiled with the row instead.
//!
//! The tree keeps the states it lit, and after each event and before each paint lights those of the groups that
//! are under the pointer or around the focus now and puts out the rest: a walk of the hovered nodes down from the
//! root, of the focused path, and of what the groups found hold.

use super::Tree;
use crate::node::{Node, interaction_state::InteractionState};

impl Tree {
  /// Lights what the interaction groups under the pointer and around the focus hold, and puts out what the others
  /// held. Returns whether anything changed; a change is drawn again.
  pub(super) fn refresh_interaction_groups(&mut self) -> bool {
    let mut groups: Vec<&Node> = Vec::new();
    if let Some(root) = &self.root {
      // The root may be under the pointer without being marked so; what is in it is.
      if root.is_style_hovered() && root.is_interaction_group() {
        groups.push(root);
      }
      for child in root.children() {
        hovered_groups(child, &mut groups);
      }
      if let Some(path) = &self.focused_path {
        let mut node = root;
        if node.is_interaction_group() {
          groups.push(node);
        }
        for &index in path {
          let Some(child) = node.children().get(index) else {
            break;
          };
          node = child;
          if node.is_interaction_group() {
            groups.push(node);
          }
        }
      }
    }
    if groups.is_empty() && self.within_states.is_empty() {
      return false;
    }
    // A group under the pointer that holds the focus too is found twice: it is lit once.
    let mut unique: Vec<&Node> = Vec::with_capacity(groups.len());
    for group in groups {
      if !unique.iter().any(|kept| std::ptr::eq(*kept, group)) {
        unique.push(group);
      }
    }
    let mut lit: Vec<InteractionState> = Vec::new();
    let mut changed = false;
    for group in unique {
      let states = GroupStates {
        hovered: group.is_style_hovered(),
        pressed: group.style_state.is_active(),
      };
      light(group, true, states, &mut lit, &mut changed);
    }
    for state in std::mem::take(&mut self.within_states) {
      if state.is_within() && !lit.iter().any(|kept| kept.same(&state)) {
        state.set_within(false, false, false);
        changed = true;
      }
    }
    self.within_states = lit;
    if changed {
      self.cached_render_list = None;
      self.needs_redraw = true;
    }
    changed
  }
}

/// The interaction groups among the hovered nodes under `node`, `node` too.
fn hovered_groups<'a>(node: &'a Node, groups: &mut Vec<&'a Node>) {
  if !node.is_style_hovered() {
    return;
  }
  if node.is_interaction_group() {
    groups.push(node);
  }
  for child in node.children() {
    hovered_groups(child, groups);
  }
}

/// Whether a group that is lit is under the pointer and held down.
#[derive(Clone, Copy)]
struct GroupStates {
  hovered: bool,
  pressed: bool,
}

/// Lights what the group `node` holds that has a `within` style, and not what a group inside it holds.
fn light(node: &Node, top: bool, states: GroupStates, lit: &mut Vec<InteractionState>, changed: &mut bool) {
  if !top && node.is_interaction_group() {
    return;
  }
  if node.has_within_style() && !lit.iter().any(|held| held.same(&node.style_state)) {
    let (state, lit_now) = node.set_style_within(true, states.hovered, states.pressed);
    *changed |= lit_now;
    lit.push(state);
  }
  for child in node.children() {
    light(child, false, states, lit, changed);
  }
}
