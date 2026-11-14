//! The keys of a nested focus group ([`FocusGroup::nested`](crate::node::FocusGroup::nested)), a tree: Right
//! opens a closed element and goes into an open one, Left closes an open element and goes out of a closed one,
//! `*` opens every element at the level of the focused one in the element it is in.
//!
//! An element's level and whether it opens are its own (`.focus_level(n)`, `.focus_expanded(open, toggle)`).
//! What an element holds is the stops after it at a higher level, up to the next one at its level or lower:
//! the group may be one flat list of rows, as a tree's is, or elements inside elements, as a sidebar's.
//! What a closed element holds is not there, so it is not among the stops. Opening and closing is the app's:
//! the key calls the element's toggle, and what it holds comes and goes with the next render.

use std::sync::Arc;

use super::{Tree, find_node_by_id, focus_visible, tab_navigation};
use crate::{core::NodeId, node::FocusStep};

/// One stop of a nested group as its keys read it.
struct Branch {
  level: u32,
  expanded: Option<bool>,
  toggle: Option<Arc<dyn Fn(bool) + Send + Sync>>,
}

impl Tree {
  /// Right, Left or `*` (`step`) in the nested group `group`, the focus on `focused`. Returns whether the key
  /// was the group's: it is wherever the focus is on one of its stops, even where it does nothing (Right on an
  /// element that does not open), so that it goes to nothing else.
  pub(super) fn focus_group_branch_key(&mut self, group: NodeId, focused: NodeId, step: FocusStep) -> bool {
    let Some(root) = &self.root else {
      return false;
    };
    let Some(group) = find_node_by_id(root, group) else {
      return false;
    };
    let (stops, nodes) = tab_navigation::group_stop_nodes(group, &self.dropped_nodes);
    let Some(current) = stops.iter().position(|stop| stop.input_id == focused) else {
      return false;
    };
    let branches: Vec<Branch> = nodes
      .iter()
      .map(|node| match node.focus_branch() {
        Some(branch) => Branch {
          level: branch.level(),
          expanded: branch.expanded,
          toggle: branch.toggle.clone(),
        },
        None => Branch {
          level: 1,
          expanded: None,
          toggle: None,
        },
      })
      .collect();
    let here = &branches[current];
    let mut toggles = Vec::new();
    let mut target = None;
    match step {
      FocusStep::Open => match here.expanded {
        Some(false) => toggles.extend(here.toggle.clone().map(|toggle| (toggle, true))),
        // Into it: the first element it holds, the stop after it, where it holds any.
        Some(true) => {
          target = branches
            .get(current + 1)
            .filter(|next| next.level > here.level)
            .map(|_| current + 1);
        }
        None => {}
      },
      FocusStep::Close => match here.expanded {
        Some(true) => toggles.extend(here.toggle.clone().map(|toggle| (toggle, false))),
        // Out of it: the element it is in.
        _ => target = parent(&branches, current),
      },
      FocusStep::OpenSiblings => {
        let start = parent(&branches, current).map_or(0, |parent| parent + 1);
        let end = (current + 1..branches.len())
          .find(|&index| branches[index].level < here.level)
          .unwrap_or(branches.len());
        toggles.extend(
          branches[start..end]
            .iter()
            .filter(|branch| branch.level == here.level && branch.expanded == Some(false))
            .filter_map(|branch| branch.toggle.clone())
            .map(|toggle| (toggle, true)),
        );
      }
      FocusStep::Back | FocusStep::On | FocusStep::First | FocusStep::Last => return false,
    }
    let target = target.map(|index| stops[index]);
    for (toggle, open) in toggles {
      toggle(open);
    }
    if let Some(target) = target {
      self.set_input_modality(focus_visible::InputModality::Keyboard);
      self.focus_node(target);
      self.scroll_focused_into_view();
    }
    true
  }
}

/// The element the stop `index` is in: the nearest stop before it at a lower level.
fn parent(branches: &[Branch], index: usize) -> Option<usize> {
  let level = branches[index].level;
  (0..index).rev().find(|&before| branches[before].level < level)
}
