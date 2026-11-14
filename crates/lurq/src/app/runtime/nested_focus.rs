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
//! focused after the layout it is in ([`PendingRowFocus`]); keys pressed meanwhile go on from it. In a group whose
//! choice follows the focus ([`FocusGroup::activate`]), or that extends with Shift
//! ([`FocusGroup::shift_extends`]), the row is pressed once it is focused, as an element of a group that builds
//! every one is.
//!
//! A row may hold several stops, the cells of a grid, each saying its column (`.focus_column(c)`): Up and Down go
//! to the stop of the same column in the row they go to ([`ColumnAim`]), and Ctrl+Home and Ctrl+End to the first
//! stop of the first row and the last stop of the last.

use std::sync::Arc;

use super::{MouseModifiers, Tree, find_node_by_id, focus_visible, tab_navigation};
use crate::{
  core::NodeId,
  node::{FocusGroup, FocusRows, FocusStep, Node, step_rows},
};

/// A row a group of rows stepped to that its list had not built: focused once a layout has it.
pub(super) struct PendingRowFocus {
  rows: Arc<dyn FocusRows>,
  row: usize,
  /// Which of the row's stops is focused.
  aim: ColumnAim,
  /// The press to make of it once it is focused, with these keys held.
  press: Option<MouseModifiers>,
}

/// Which stop of a row of several the focus goes to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ColumnAim {
  /// Its first: a row of one stop, or Ctrl+Home.
  First,
  /// Its last: Ctrl+End.
  Last,
  /// The one of this column, or the nearest before it where the row has none of it.
  Same(usize),
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
  /// A key of the group `group` (`config`), whose rows are `rows`, the focus on its row `current`, with
  /// `modifiers` held. Returns whether the key was the group's.
  pub(super) fn focus_rows_key(
    &mut self,
    group: NodeId,
    rows: Arc<dyn FocusRows>,
    config: FocusGroup,
    current: usize,
    step: FocusStep,
    modifiers: MouseModifiers,
    aim: ColumnAim,
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
      // A move with Shift in a group that extends with it presses what it moved to with Shift: a range.
      let press = (config.activate || (config.shift_extends && modifiers.shift)).then_some(modifiers);
      self.focus_row_at(group, rows, target, press, aim);
    } else if matches!(step, FocusStep::First | FocusStep::Last)
      && let Some(stop) = self
        .root
        .as_ref()
        .and_then(|root| find_node_by_id(root, group))
        .and_then(|node| row_stop(node, current, aim, &self.dropped_nodes))
    {
      // Ctrl+Home on the first row, Ctrl+End on the last: the first or the last stop of the row it is on.
      self.set_input_modality(focus_visible::InputModality::Keyboard);
      self.focus_node(stop);
      self.scroll_focused_into_view();
    }
    true
  }

  /// Focuses the row `row` of the group `group`, whose rows are `rows`: its element, where the list built it, or
  /// once a layout has it, the list asked to build it. `press` presses it once it is focused, with those keys.
  pub(super) fn focus_row(
    &mut self,
    group: NodeId,
    rows: Arc<dyn FocusRows>,
    row: usize,
    press: Option<MouseModifiers>,
  ) {
    self.focus_row_at(group, rows, row, press, ColumnAim::First);
  }

  /// Focuses the stop `aim` says of the row `row`: see [`Self::focus_row`].
  pub(super) fn focus_row_at(
    &mut self,
    group: NodeId,
    rows: Arc<dyn FocusRows>,
    row: usize,
    press: Option<MouseModifiers>,
    aim: ColumnAim,
  ) {
    let built = self
      .root
      .as_ref()
      .and_then(|root| find_node_by_id(root, group))
      .and_then(|group| row_stop(group, row, aim, &self.dropped_nodes));
    self.set_input_modality(focus_visible::InputModality::Keyboard);
    match built {
      Some(target) => {
        self.pending_row_focus = None;
        self.focus_node(target);
        self.scroll_focused_into_view();
        if let Some(modifiers) = press {
          self.activate_focused_button(modifiers);
        }
      }
      None => {
        rows.reveal(row);
        self.pending_row_focus = Some(PendingRowFocus { rows, row, aim, press });
      }
    }
  }

  /// A key pressed while a group of rows waits for a row: it goes on from that row. `None` where nothing waits
  /// or the key is not the group's.
  pub(super) fn pending_rows_key(&mut self, key: &str, modifiers: MouseModifiers) -> Option<bool> {
    let pending = self.pending_row_focus.as_ref()?;
    let (rows, row, aim) = (pending.rows.clone(), pending.row, pending.aim);
    let (group, config) = group_of_rows(self.root.as_ref()?, &rows)?;
    let group = group.node_id();
    let step = config.step(key)?;
    Some(self.focus_rows_key(group, rows, config, row, step, modifiers, aim))
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
    let Some(target) = row_stop(group, pending.row, pending.aim, &self.dropped_nodes) else {
      return;
    };
    let press = pending.press;
    self.pending_row_focus = None;
    self.focus_node(target);
    if self.focused_node == Some(target.input_id) {
      self.request_scroll_into_view(target.input_id);
      if let Some(modifiers) = press {
        self.activate_focused_button(modifiers);
      }
    }
  }
}

/// The stop of the row `row` in `group` that `aim` says, where it is built: of a row of several stops, the one
/// of the column aimed at, or the nearest before it.
fn row_stop(group: &Node, row: usize, aim: ColumnAim, dropped: &[NodeId]) -> Option<super::FocusTarget> {
  let (stops, nodes) = tab_navigation::group_stop_nodes(group, dropped);
  let in_row: Vec<(usize, Option<usize>)> = nodes
    .iter()
    .enumerate()
    .filter_map(|(index, node)| {
      let branch = node.focus_branch()?;
      (branch.row == Some(row)).then_some((index, branch.column))
    })
    .collect();
  let index = match aim {
    ColumnAim::First => in_row.first().map(|(index, _)| *index),
    ColumnAim::Last => in_row.last().map(|(index, _)| *index),
    ColumnAim::Same(column) => in_row
      .iter()
      .filter(|(_, at)| at.is_some_and(|at| at <= column))
      .max_by_key(|(_, at)| *at)
      .or_else(|| in_row.first())
      .map(|(index, _)| *index),
  }?;
  Some(stops[index])
}

/// The row the row `row` is in: the nearest row before it at a lower level.
fn row_parent(rows: &dyn FocusRows, row: usize) -> Option<usize> {
  let level = rows.level(row);
  (0..row).rev().find(|&before| rows.level(before) < level)
}
