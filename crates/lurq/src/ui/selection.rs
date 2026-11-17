//! Selecting in a list of rows: one, or any number with Ctrl and Shift, as a list of files has it. What a tree
//! view's rows, a listbox's options and a table's rows share.
//!
//! The rows are given in the order they are shown, each with whether it takes input. What is selected is the
//! app's: a list of ids, the last of it where a range with Shift starts, and what writes the next one. A row's
//! press tells [`ListSelection::pick`] how it was pressed ([`Pick::of`]); Shift with the keys is a press with Shift
//! of where the focus goes, which the engine makes for a focus group with
//! [`shift_extends`](crate::node::FocusGroup::shift_extends), or, for a list that steps the selection with the
//! keys itself, [`ListSelection::extend`]; Ctrl+A is [`select_all_key`].

use std::sync::Arc;

use crate::{
  app::events::{KeyboardEvent, MouseEvent},
  node::{FocusStep, step_rows},
};

/// How a press selects a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pick {
  /// That row alone.
  Only,
  /// The row in or out of what is selected, the rest as it is.
  Toggle,
  /// The rows from where the selection started to it.
  Range,
}

impl Pick {
  /// How `event` selects, in a list of which any number may be selected (`multiple`) or one: with Shift a range,
  /// with Ctrl (Cmd) one in or out, and Space from the keyboard one in or out where any number are.
  pub(crate) fn of(event: &MouseEvent, multiple: bool) -> Self {
    if event.shift {
      Self::Range
    } else if event.ctrl || event.meta || (event.from_keyboard && multiple) {
      Self::Toggle
    } else {
      Self::Only
    }
  }

  /// How `event` checks in a list of checkboxes: with Shift a range, else the row in or out, the rest as it is.
  pub(crate) fn of_check(event: &MouseEvent) -> Self {
    if event.shift { Self::Range } else { Self::Toggle }
  }
}

/// What writes the next selection.
pub(crate) type Write<T> = Arc<dyn Fn(Vec<T>) + Send + Sync>;

/// The rows of a list as they are shown and what is selected of them, which each row's press reads.
pub(crate) struct ListSelection<T> {
  ids: Vec<T>,
  enabled: Vec<bool>,
  selected: Vec<T>,
  multiple: bool,
  write: Option<Write<T>>,
  /// Whether a range adds to what is selected, as a list of checkboxes has it, in place of being the whole of it.
  adds: bool,
}

impl<T: Clone + PartialEq> ListSelection<T> {
  /// The rows `ids` in order, which of them take input, what is `selected` (where a range starts last), whether
  /// any number may be, and what writes the next selection: nothing is written without it.
  pub(crate) fn new(
    ids: Vec<T>,
    enabled: Vec<bool>,
    selected: Vec<T>,
    multiple: bool,
    write: Option<Write<T>>,
  ) -> Self {
    Self {
      ids,
      enabled,
      selected,
      multiple,
      write,
      adds: false,
    }
  }

  /// Whether a range, by a press with Shift or Shift with the keys, adds its rows to what is selected and keeps the
  /// rest, as in a list of checkboxes, in place of being all that is selected.
  pub(crate) fn adding(mut self, adds: bool) -> Self {
    self.adds = adds;
    self
  }

  /// The id of the row at `index`.
  pub(crate) fn id(&self, index: usize) -> &T {
    &self.ids[index]
  }

  /// A press of the row at `index`. Where one is selected, any press selects that row alone.
  pub(crate) fn pick(&self, index: usize, pick: Pick) {
    let id = &self.ids[index];
    let next = match pick {
      _ if !self.multiple => vec![id.clone()],
      Pick::Only => vec![id.clone()],
      Pick::Toggle => {
        let mut next = self.selected.clone();
        match next.iter().position(|held| held == id) {
          Some(at) => {
            next.remove(at);
          }
          None => next.push(id.clone()),
        }
        next
      }
      Pick::Range => self.range(self.anchor().unwrap_or(index), index),
    };
    self.write(next);
  }

  /// Shift with a key on the row at `from`, where any number may be selected: the rows from where the selection
  /// started to where `step` takes the focus, as the engine steps a focus group over rows that take input, without
  /// going round its ends. PageUp and PageDown go `page` rows, and do nothing without it. The row it reaches need
  /// not be built: a list that builds only the rows near its view reaches any.
  pub(crate) fn extend(&self, from: usize, step: FocusStep, page: Option<usize>) {
    if !self.multiple {
      return;
    }
    let page = match (page, step.is_page()) {
      (Some(page), true) => page,
      (None, true) => return,
      _ => 1,
    };
    let to = step_rows(step, from, self.ids.len(), page, false, |index| self.enabled[index]);
    if let Some(to) = to {
      self.write(self.range(self.anchor().unwrap_or(from), to));
    }
  }

  /// Every row that takes input, where any number may be selected: where the selection started stays where it is.
  pub(crate) fn select_all(&self) {
    if !self.multiple {
      return;
    }
    let anchor = self.anchor();
    let mut all: Vec<T> = (0..self.ids.len())
      .filter(|index| self.enabled[*index] && Some(*index) != anchor)
      .map(|index| self.ids[index].clone())
      .collect();
    all.extend(anchor.map(|anchor| self.ids[anchor].clone()));
    self.write(all);
  }

  /// Where a range starts: the last selected row, where it is shown.
  fn anchor(&self) -> Option<usize> {
    let last = self.selected.last()?;
    self.ids.iter().position(|id| id == last)
  }

  /// The rows from `anchor` to `to` that take input, the anchor last, so that it stays where the next starts.
  fn range(&self, anchor: usize, to: usize) -> Vec<T> {
    let (start, end) = (anchor.min(to), anchor.max(to));
    let mut range: Vec<T> = (start..=end)
      .filter(|index| self.enabled[*index] && *index != anchor)
      .map(|index| self.ids[index].clone())
      .collect();
    range.push(self.ids[anchor].clone());
    if self.adds {
      // What was selected outside the range stays, before it.
      let mut kept: Vec<T> = self.selected.iter().filter(|id| !range.contains(id)).cloned().collect();
      kept.extend(range);
      return kept;
    }
    range
  }

  fn write(&self, next: Vec<T>) {
    if let Some(write) = &self.write
      && next != self.selected
    {
      write(next);
    }
  }
}

/// Ctrl+A (Cmd+A) on a row of a list of which any number may be selected: `select_all`. For `on_key_down` of each
/// row; it takes the key only while the focus is in the row.
pub(crate) fn select_all_key(event: &KeyboardEvent, select_all: &(dyn Fn() + Send + Sync)) {
  if !event.focus_within || event.composing || event.alt || event.shift {
    return;
  }
  if (event.ctrl || event.meta) && matches!(event.key.as_str(), "a" | "A") {
    event.prevent_default();
    select_all();
  }
}
