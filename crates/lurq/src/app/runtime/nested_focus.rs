//! The keys of a nested focus group ([`FocusGroup::nested`](crate::node::FocusGroup::nested)), a tree: Right
//! opens a closed element and goes into an open one, Left closes an open element and goes out of a closed one,
//! `*` opens every element at the level of the focused one in the element it is in.
//!
//! An element's level and whether it opens are its own (`.focus_level(n)`, `.focus_expanded(open, toggle)`).
//! What an element holds is the stops after it at a higher level, up to the next one at its level or lower:
//! the group may be one flat list of rows, as a tree's is, or elements inside elements, as a sidebar's.
//! What a closed element holds is not there, so it is not among the stops. Opening and closing is the app's:
//! the key calls the element's toggle, and what it holds comes and goes with the next render.
//!
//! A group with [`FocusRows`] (a list that builds only the rows near its view) steps over those rows and not over
//! the stops that are built: Up, Down, Home, End, PageUp, PageDown, the nested keys and typeahead go to any row.
//! The row stepped to is focused at once where it is built, and otherwise asked for (`FocusRows::reveal`) and
//! focused after the layout it is in ([`PendingRowFocus`]); keys pressed meanwhile go on from it.

use std::sync::Arc;

use super::{Tree, find_node_by_id, focus_visible, tab_navigation};
use crate::{
  core::NodeId,
  node::{FocusGroup, FocusRows, FocusStep, Node, step_rows},
};

/// A row a group of rows stepped to that its list had not built: focused once a layout has it.
pub(super) struct PendingRowFocus {
  rows: Arc<dyn FocusRows>,
  row: usize,
}

/// Whether two handles are the rows of one group: the same `Arc`.
fn same_rows(a: &Arc<dyn FocusRows>, b: &Arc<dyn FocusRows>) -> bool {
  std::ptr::eq(Arc::as_ptr(a) as *const (), Arc::as_ptr(b) as *const ())
}

/// The element of the group whose rows are `rows`, with its group.
fn group_of_rows<'a>(node: &'a Node, rows: &Arc<dyn FocusRows>) -> Option<(&'a Node, FocusGroup)> {
  if let Some(group) = node.focus_group_value()
    && node
      .focus_branch()
      .and_then(|branch| branch.rows.as_ref())
      .is_some_and(|held| same_rows(held, rows))
  {
    return Some((node, group));
  }
  node.children().iter().find_map(|child| group_of_rows(child, rows))
}

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
      FocusStep::Back
      | FocusStep::On
      | FocusStep::First
      | FocusStep::Last
      | FocusStep::PageBack
      | FocusStep::PageOn => return false,
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

impl Tree {
  /// A key of the group `group` (`config`), whose rows are `rows`, the focus on its row `current`. Returns whether
  /// the key was the group's.
  pub(super) fn focus_rows_key(
    &mut self,
    group: NodeId,
    rows: Arc<dyn FocusRows>,
    config: FocusGroup,
    current: usize,
    step: FocusStep,
  ) -> bool {
    let count = rows.count();
    if current >= count {
      return false;
    }
    let target = match step {
      FocusStep::Open => match rows.expanded(current) {
        Some(false) => {
          rows.set_expanded(current, true);
          None
        }
        // Into it: the row after it, where that is in it.
        Some(true) => (current + 1 < count && rows.level(current + 1) > rows.level(current)).then_some(current + 1),
        None => None,
      },
      FocusStep::Close => match rows.expanded(current) {
        Some(true) => {
          rows.set_expanded(current, false);
          None
        }
        _ => row_parent(&*rows, current),
      },
      FocusStep::OpenSiblings => {
        let level = rows.level(current);
        let start = row_parent(&*rows, current).map_or(0, |parent| parent + 1);
        let end = (current + 1..count)
          .find(|&row| rows.level(row) < level)
          .unwrap_or(count);
        for row in start..end {
          if rows.level(row) == level && rows.expanded(row) == Some(false) && rows.enabled(row) {
            rows.set_expanded(row, true);
          }
        }
        None
      }
      _ => step_rows(step, current, count, rows.page(), config.wrap, |row| rows.enabled(row)),
    };
    if let Some(target) = target {
      self.focus_row(group, rows, target);
    }
    true
  }

  /// Focuses the row `row` of the group `group`, whose rows are `rows`: its element, where the list built it, or
  /// once a layout has it, the list asked to build it.
  pub(super) fn focus_row(&mut self, group: NodeId, rows: Arc<dyn FocusRows>, row: usize) {
    let built = self
      .root
      .as_ref()
      .and_then(|root| find_node_by_id(root, group))
      .and_then(|group| row_stop(group, row, &self.dropped_nodes));
    self.set_input_modality(focus_visible::InputModality::Keyboard);
    match built {
      Some(target) => {
        self.pending_row_focus = None;
        self.focus_node(target);
        self.scroll_focused_into_view();
      }
      None => {
        rows.reveal(row);
        self.pending_row_focus = Some(PendingRowFocus { rows, row });
      }
    }
  }

  /// A key pressed while a group of rows waits for a row: it goes on from that row. `None` where nothing waits
  /// or the key is not the group's.
  pub(super) fn pending_rows_key(&mut self, key: &str) -> Option<bool> {
    let pending = self.pending_row_focus.as_ref()?;
    let (rows, row) = (pending.rows.clone(), pending.row);
    let (group, config) = group_of_rows(self.root.as_ref()?, &rows)?;
    let group = group.node_id();
    let step = config.step(key)?;
    Some(self.focus_rows_key(group, rows, config, row, step))
  }

  /// After a layout: focuses the row a group of rows waited for, where the list has built it now.
  pub(super) fn apply_pending_row_focus(&mut self) {
    let Some(pending) = &self.pending_row_focus else {
      return;
    };
    let Some(root) = &self.root else {
      return;
    };
    let Some((group, _)) = group_of_rows(root, &pending.rows) else {
      // The group is gone: nothing waits for it.
      self.pending_row_focus = None;
      return;
    };
    let Some(target) = row_stop(group, pending.row, &self.dropped_nodes) else {
      return;
    };
    self.pending_row_focus = None;
    self.focus_node(target);
    if self.focused_node == Some(target.input_id) {
      self.request_scroll_into_view(target.input_id);
    }
  }
}

/// The stop of the row `row` in `group`, where it is built.
fn row_stop(group: &Node, row: usize, dropped: &[NodeId]) -> Option<super::FocusTarget> {
  let (stops, nodes) = tab_navigation::group_stop_nodes(group, dropped);
  nodes
    .iter()
    .position(|node| node.focus_branch().and_then(|branch| branch.row) == Some(row))
    .map(|index| stops[index])
}

/// The row the row `row` is in: the nearest row before it at a lower level.
fn row_parent(rows: &dyn FocusRows, row: usize) -> Option<usize> {
  let level = rows.level(row);
  (0..row).rev().find(|&before| rows.level(before) < level)
}
