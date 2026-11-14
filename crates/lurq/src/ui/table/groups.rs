//! The rows of a table grouped by the values of columns: a group's row, which opens and closes, and the lines a
//! table shows, group rows and the rows of the open groups in their order.

use std::{collections::HashMap, sync::Arc};

use super::{TablePart, glyph, icons, pressable};
use crate::{
  app::events::{KeyboardEvent, MouseEvent},
  components::Row,
  core::ElementRef,
  layout::{
    Alignment,
    layout_kind::{Justify, Sticky},
  },
  node::Element,
  ui::recipe::{Name, ResolvedPart, ResolvedRecipe},
};

/// A group of rows: the values of the columns grouped by, from the first to the group's own. The rows whose
/// status is "Failed" are `["Failed"]`; grouped by the status and then the owner, those of Ada among them are
/// `["Failed", "Ada"]`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct GroupKey {
  pub values: Vec<String>,
}

#[cfg(feature = "devtools")]
impl crate::app::component::DevtoolsInspectable for GroupKey {}

impl GroupKey {
  pub fn new<S: AsRef<str>>(values: impl IntoIterator<Item = S>) -> Self {
    Self {
      values: values.into_iter().map(|value| value.as_ref().to_owned()).collect(),
    }
  }

  /// How deep it is: 1 for a group of the first column grouped by.
  pub fn level(&self) -> usize {
    self.values.len()
  }
}

/// A line a table shows: a row, by its place among the rows shown, or a group's row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Shown {
  Row(usize),
  Group(usize),
}

/// A group's row: what it shows and where it is.
#[derive(Clone, Debug)]
pub(super) struct GroupLine {
  pub key: GroupKey,
  /// The id of the column grouped by.
  pub column: Arc<str>,
  /// How many rows are in it, in the groups in it too.
  pub count: usize,
  pub open: bool,
  /// The places of its rows, among all the rows: what a summary of the group adds up.
  pub rows: Vec<usize>,
}

/// The lines a table shows, in order, and where each row is among them.
#[derive(Clone, Debug, Default)]
pub(super) struct Lines {
  pub lines: Vec<Shown>,
  /// The place of each row shown among all the rows, in the order shown: a row's place among those shown is its
  /// rank.
  pub order: Vec<usize>,
  /// The line of each row shown, by its rank.
  pub position_of_rank: Vec<usize>,
  pub groups: Vec<GroupLine>,
  /// How many columns the rows are grouped by: the level of a row, under the groups.
  pub depth: usize,
}

impl Lines {
  /// Every row, in its order, and no groups.
  pub fn flat(count: usize) -> Self {
    Self {
      lines: (0..count).map(Shown::Row).collect(),
      order: (0..count).collect(),
      position_of_rank: (0..count).collect(),
      groups: Vec::new(),
      depth: 0,
    }
  }

  /// The rows grouped by the values `values` gives each row for each column of `columns`, in the order the groups
  /// are first met, the rows of each in their order; the rows of a closed group are not among the lines.
  pub fn grouped(
    count: usize,
    columns: &[Arc<str>],
    values: &dyn Fn(usize, usize) -> String,
    open: &dyn Fn(&GroupKey) -> bool,
  ) -> Self {
    if columns.is_empty() {
      return Self::flat(count);
    }
    let mut lines = Self {
      depth: columns.len(),
      ..Self::default()
    };
    let rows: Vec<usize> = (0..count).collect();
    lines.add(&rows, 0, &[], columns, values, open);
    lines
  }

  fn add(
    &mut self,
    rows: &[usize],
    level: usize,
    above: &[String],
    columns: &[Arc<str>],
    values: &dyn Fn(usize, usize) -> String,
    open: &dyn Fn(&GroupKey) -> bool,
  ) {
    if level == columns.len() {
      for row in rows {
        self.position_of_rank.push(self.lines.len());
        self.lines.push(Shown::Row(self.order.len()));
        self.order.push(*row);
      }
      return;
    }
    // The groups in the order they are first met, each with its rows in their order.
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    let mut place: HashMap<String, usize> = HashMap::new();
    for row in rows {
      let value = values(*row, level);
      match place.get(&value) {
        Some(at) => groups[*at].1.push(*row),
        None => {
          place.insert(value.clone(), groups.len());
          groups.push((value, vec![*row]));
        }
      }
    }
    for (value, members) in groups {
      let mut path = above.to_vec();
      path.push(value);
      let key = GroupKey { values: path.clone() };
      let is_open = open(&key);
      self.lines.push(Shown::Group(self.groups.len()));
      self.groups.push(GroupLine {
        key,
        column: columns[level].clone(),
        count: members.len(),
        open: is_open,
        rows: members.clone(),
      });
      if is_open {
        self.add(&members, level + 1, &path, columns, values, open);
      }
    }
  }

  /// The rank of the row at a line, where it is a row's.
  pub fn rank_at(&self, position: usize) -> Option<usize> {
    match self.lines.get(position)? {
      Shown::Row(rank) => Some(*rank),
      Shown::Group(_) => None,
    }
  }

  /// The level of a line in the nesting of the groups: 1 at the top.
  pub fn level(&self, position: usize) -> u32 {
    match self.lines.get(position) {
      Some(Shown::Group(group)) => self.groups[*group].key.level() as u32,
      _ => self.depth as u32 + 1,
    }
  }
}

/// A group's row as a renderer sees it: the group's value and how many rows are in it, at its level, which a press,
/// Enter, Space, Right and Left open and close.
pub struct TableGroup {
  /// Its line among the lines shown, group rows counted, from 0.
  pub position: usize,
  pub key: GroupKey,
  /// The title of the column grouped by.
  pub title: Arc<str>,
  /// How many rows are in it.
  pub count: usize,
  pub open: bool,
  /// Whether Tab enters the rows at it.
  pub entry: bool,
  pub(super) recipe: ResolvedRecipe,
  pub(super) toggle: Arc<dyn Fn(bool) + Send + Sync>,
  pub(super) element: ElementRef,
  pub(super) grid: bool,
  pub(super) indent: f32,
  pub(super) id: Option<Arc<str>>,
}

impl TableGroup {
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  /// The group's value: the last of its key's.
  pub fn value(&self) -> &str {
    self.key.values.last().map_or("", String::as_str)
  }

  /// How the default renderer builds a group's row: its chevron, its value and its count at its level, held at
  /// the start while the rows scroll sideways. It is a row to what reads the tree, at its level, open or not.
  pub fn render(self) -> Element {
    let part = self.part(TablePart::GroupRow);
    let open = self.open;
    let (press, keyed) = (self.toggle.clone(), self.toggle.clone());
    let chevron = glyph(
      &self.part(TablePart::Chevron),
      Name::new(if open {
        icons::CHEVRON_DOWN
      } else {
        icons::CHEVRON_RIGHT
      }),
      false,
    );
    let label = self.part(TablePart::GroupLabel);
    let count = self.part(TablePart::GroupCount);
    let name = format!(
      "{}: {}, {} {}",
      self.title,
      self.value(),
      self.count,
      if self.count == 1 { "row" } else { "rows" }
    );
    let element = self.element.clone();
    let mut row = pressable(&part, false)
      .justify(Justify::Start)
      .align_items(Alignment::Stretch)
      .ref_element(self.element.clone())
      .focus_row(self.position)
      .focus_level(self.key.level() as u32)
      .focus_expanded(open, {
        let toggle = self.toggle.clone();
        move |open| toggle(open)
      })
      .focus_entry(self.entry)
      .focus_label(self.value().to_owned())
      .describe("a11y_role", "row")
      .describe("a11y_rowindex", (self.position + 2).to_string())
      .describe("a11y_level", self.key.level().to_string())
      .describe("a11y_expanded", if open { "true" } else { "false" })
      .describe("a11y_name", name)
      .key(format!("group-{}", self.key.values.join("\u{1f}")))
      .on_click(move |event: MouseEvent| {
        event.stop_propagation();
        press(!open);
      })
      .on_key_down(move |event: KeyboardEvent| {
        if !element.focused() || event.composing || event.alt || event.ctrl || event.meta {
          return;
        }
        if matches!(event.key.as_str(), "Enter" | " ") {
          event.prevent_default();
          keyed(!open);
        }
      });
    if self.grid {
      row = row.tab_index(0);
    }
    if let Some(id) = &self.id {
      row = row.id(id.clone());
    }
    // What it says stays at the start while the rows scroll sideways.
    row
      .child(
        Row::new()
          .align_items(Alignment::Center)
          .spacing(8.0)
          .padding_left(self.indent)
          .sticky(Sticky::left(0.0))
          .child(chevron)
          .child(label.text(self.value()).nowrap())
          .child(count.text(&self.count.to_string()).nowrap()),
      )
      .into()
  }
}
