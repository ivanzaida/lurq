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

/// A line a table shows: a row, by its place among the rows shown, a group's row, the summary at the end of a
/// group's rows, or the detail of a row open under it, by the row's place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Shown {
  Row(usize),
  Group(usize),
  Summary(usize),
  Detail(usize),
}

/// Rows in a tree, by their ranks: how deep each is (0 at the top), the row it is under, whether rows are under it,
/// and whether it is open.
#[derive(Clone, Debug, Default)]
pub(super) struct Tree {
  pub level: Vec<u32>,
  pub parent: Vec<Option<usize>>,
  pub branch: Vec<bool>,
  pub open: Vec<bool>,
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

/// The lines a table shows, in order, and where each row is among them. Where every line is a row and every row is
/// shown in its order, it holds nothing per row: a table of a million rows reckons its lines as it does ten.
#[derive(Clone, Debug, Default)]
pub(super) struct Lines {
  /// The lines, where any is a group's or a summary's: `None` where each line is the row of its rank.
  lines: Option<Vec<Shown>>,
  /// The place of each row shown among all the rows, in the order shown (a row's place among those shown is its
  /// rank): `None` where every row is shown in its order.
  order: Option<Arc<Vec<usize>>>,
  /// The line of each row shown, by its rank, where the lines are not the rows.
  position_of_rank: Option<Vec<usize>>,
  /// How many rows are shown.
  ranks: usize,
  /// The rank of each row, by its place among all the rows, where they are reordered: reckoned once it is asked.
  rank_of_index: std::sync::OnceLock<Vec<usize>>,
  pub groups: Vec<GroupLine>,
  /// How many columns the rows are grouped by: the level of a row, under the groups.
  pub depth: usize,
  /// The places of the rows the filters let through, in their order, closed groups' too: `None` where no filter
  /// is on. What the count and the grid's summary are of.
  pub passed: Option<Arc<Vec<usize>>>,
  /// Where the rows are a tree: each row's level, its parent, whether it opens and whether it is open.
  pub tree: Option<Arc<Tree>>,
}

impl Lines {
  /// Every row, in its order, and no groups.
  pub fn flat(count: usize) -> Self {
    Self {
      ranks: count,
      ..Self::default()
    }
  }

  /// The rows at `order`, in that order, a tree as `tree` says.
  pub fn tree(order: Vec<usize>, tree: Tree) -> Self {
    Self {
      ranks: order.len(),
      order: Some(Arc::new(order)),
      tree: Some(Arc::new(tree)),
      ..Self::default()
    }
  }

  /// These lines with the detail of each row `open` says under it.
  pub fn with_details(mut self, open: impl Fn(usize) -> bool) -> Self {
    let mut lines = Vec::with_capacity(self.len());
    let mut position_of_rank = vec![0; self.ranks];
    for position in 0..self.len() {
      let Some(line) = self.line(position) else {
        continue;
      };
      lines.push(line);
      if let Shown::Row(rank) = line {
        position_of_rank[rank] = lines.len() - 1;
        if open(rank) {
          lines.push(Shown::Detail(rank));
        }
      }
    }
    self.lines = Some(lines);
    self.position_of_rank = Some(position_of_rank);
    self
  }

  /// The rows at `order`, in that order, and no groups.
  pub fn rows(order: Arc<Vec<usize>>) -> Self {
    Self {
      ranks: order.len(),
      order: Some(order),
      ..Self::default()
    }
  }

  /// How many lines there are.
  pub fn len(&self) -> usize {
    self.lines.as_ref().map_or(self.ranks, Vec::len)
  }

  /// The line at `position`.
  pub fn line(&self, position: usize) -> Option<Shown> {
    match &self.lines {
      Some(lines) => lines.get(position).copied(),
      None => (position < self.ranks).then_some(Shown::Row(position)),
    }
  }

  /// How many rows are shown.
  pub fn ranks(&self) -> usize {
    self.ranks
  }

  /// The place among all the rows of the row of `rank`.
  pub fn row_of_rank(&self, rank: usize) -> Option<usize> {
    if rank >= self.ranks {
      return None;
    }
    match &self.order {
      Some(order) => order.get(rank).copied(),
      None => Some(rank),
    }
  }

  /// The line of the row of `rank`.
  pub fn position_of(&self, rank: usize) -> Option<usize> {
    if rank >= self.ranks {
      return None;
    }
    match &self.position_of_rank {
      Some(positions) => positions.get(rank).copied(),
      None => Some(rank),
    }
  }

  /// The rank of the row at `index` among all the rows: `None` where it is not shown.
  pub fn rank_of_index(&self, index: usize) -> Option<usize> {
    let Some(order) = &self.order else {
      return (index < self.ranks).then_some(index);
    };
    let ranks = self.rank_of_index.get_or_init(|| {
      let mut ranks = vec![usize::MAX; order.iter().copied().max().map_or(0, |most| most + 1)];
      for (rank, index) in order.iter().enumerate() {
        ranks[*index] = rank;
      }
      ranks
    });
    ranks.get(index).copied().filter(|rank| *rank != usize::MAX)
  }

  /// The rows grouped by the values `values` gives each row for each column of `columns`, in the order the groups
  /// are first met, the rows of each in their order; the rows of a closed group are not among the lines.
  pub fn grouped(
    rows: Vec<usize>,
    columns: &[Arc<str>],
    summaries: bool,
    values: &dyn Fn(usize, usize) -> String,
    open: &dyn Fn(&GroupKey) -> bool,
  ) -> Self {
    if columns.is_empty() {
      return Self::rows(Arc::new(rows));
    }
    let mut built = Built::default();
    built.add(&rows, 0, &[], columns, summaries, values, open);
    Self {
      ranks: built.order.len(),
      lines: Some(built.lines),
      order: Some(Arc::new(built.order)),
      position_of_rank: Some(built.position_of_rank),
      rank_of_index: std::sync::OnceLock::new(),
      groups: built.groups,
      depth: columns.len(),
      passed: None,
      tree: None,
    }
  }

  /// The rank of the row at a line, where it is a row's.
  pub fn rank_at(&self, position: usize) -> Option<usize> {
    match self.line(position)? {
      Shown::Row(rank) => Some(rank),
      Shown::Group(_) | Shown::Summary(_) | Shown::Detail(_) => None,
    }
  }

  /// The level of a line in the nesting of the groups: 1 at the top.
  pub fn level(&self, position: usize) -> u32 {
    match self.line(position) {
      Some(Shown::Group(group)) => self.groups[group].key.level() as u32,
      Some(Shown::Row(rank) | Shown::Detail(rank)) if self.tree.is_some() => {
        self
          .tree
          .as_ref()
          .and_then(|tree| tree.level.get(rank))
          .copied()
          .unwrap_or(0)
          + 1
      }
      _ => self.depth as u32 + 1,
    }
  }
}

/// The lines of grouped rows as they are laid out.
#[derive(Default)]
struct Built {
  lines: Vec<Shown>,
  order: Vec<usize>,
  position_of_rank: Vec<usize>,
  groups: Vec<GroupLine>,
}

impl Built {
  #[allow(clippy::too_many_arguments)]
  fn add(
    &mut self,
    rows: &[usize],
    level: usize,
    above: &[String],
    columns: &[Arc<str>],
    summaries: bool,
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
      let group = self.groups.len();
      self.lines.push(Shown::Group(group));
      self.groups.push(GroupLine {
        key,
        column: columns[level].clone(),
        count: members.len(),
        open: is_open,
        rows: members.clone(),
      });
      if is_open {
        self.add(&members, level + 1, &path, columns, summaries, values, open);
        // A group open ends in the summary of its rows.
        if summaries {
          self.lines.push(Shown::Summary(group));
        }
      }
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
  /// What it is called to what reads the tree: its column, its value and its count.
  pub(super) name: String,
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
    let name = self.name.clone();
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
