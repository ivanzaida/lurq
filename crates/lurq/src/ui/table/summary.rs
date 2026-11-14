//! A summary of a table's rows: a row at the bottom of the grid that stays there, and one at the end of each group
//! open, with a value for each column that says one: what the app computes, or the sum, the average, the count, the
//! least or the most of a column of numbers.

use std::sync::Arc;

use super::{Content, EditorKind, Geometry, TableCell, TableColumn, TablePart, in_tracks, lead_cell};
use crate::{
  components::Row,
  layout::{Alignment, column_tracks::ColumnTracks, layout_kind::Sticky},
  node::{BackgroundColor, Element},
  ui::recipe::{Name, ResolvedPart, ResolvedRecipe},
};

/// What a summary of a column of numbers is: the grid computes it from the rows it sums up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SummaryKind {
  /// The numbers added up.
  Sum,
  /// Their mean.
  Average,
  /// How many rows there are.
  Count,
  /// The least of them.
  Min,
  /// The most of them.
  Max,
}

impl SummaryKind {
  /// What the summary row says of it, before its value, in English: [`SummaryLabels`] says it otherwise.
  pub fn label(self) -> &'static str {
    match self {
      Self::Sum => "Sum",
      Self::Average => "Average",
      Self::Count => "Count",
      Self::Min => "Min",
      Self::Max => "Max",
    }
  }
}

/// The words of a data grid's summary rows, each the app says otherwise: English unless told.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SummaryLabels {
  pub sum: String,
  pub average: String,
  pub count: String,
  pub min: String,
  pub max: String,
  /// What a summary row is called to what reads the tree.
  pub row: String,
}

impl Default for SummaryLabels {
  fn default() -> Self {
    Self {
      sum: SummaryKind::Sum.label().into(),
      average: SummaryKind::Average.label().into(),
      count: SummaryKind::Count.label().into(),
      min: SummaryKind::Min.label().into(),
      max: SummaryKind::Max.label().into(),
      row: "Summary".into(),
    }
  }
}

impl SummaryLabels {
  /// What the summary of `kind` says before its value.
  pub fn of(&self, kind: SummaryKind) -> &str {
    match kind {
      SummaryKind::Sum => &self.sum,
      SummaryKind::Average => &self.average,
      SummaryKind::Count => &self.count,
      SummaryKind::Min => &self.min,
      SummaryKind::Max => &self.max,
    }
  }
}

type SummaryFn<R> = Arc<dyn Fn(&[&R]) -> String + Send + Sync>;

/// What a column's summary is: one the grid computes, or the app's text.
pub(super) enum Summary<R> {
  Kind(SummaryKind),
  Text(SummaryFn<R>),
}

impl<R> Clone for Summary<R> {
  fn clone(&self) -> Self {
    match self {
      Self::Kind(kind) => Self::Kind(*kind),
      Self::Text(text) => Self::Text(text.clone()),
    }
  }
}

/// The number a column has for a row: the value its editor edits, or the number its text starts with ("12 KB").
fn number_of<R>(column: &TableColumn<R>, row: &R) -> Option<f64> {
  if let Some(EditorKind::Number(number)) = column.editor.as_ref().map(|editor| &editor.kind) {
    return Some(number(row));
  }
  let Content::Text(text) = &column.content else {
    return None;
  };
  let text = text(row);
  let lead: String = text
    .trim_start()
    .chars()
    .take_while(|char| char.is_ascii_digit() || matches!(char, '.' | '-' | '+' | ','))
    .filter(|char| *char != ',')
    .collect();
  lead.parse().ok()
}

/// A number as a summary says it: whole where it is, else to two places.
fn said(number: f64) -> String {
  if number.fract().abs() < 1e-9 {
    format!("{number:.0}")
  } else {
    let text = format!("{number:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
  }
}

/// What the column says of `rows`: its label (a kind's) and its value.
pub(super) fn summarize<R>(
  column: &TableColumn<R>,
  rows: &[&R],
  labels: &SummaryLabels,
) -> Option<(Option<String>, String)> {
  let summary = column.summary.as_ref()?;
  let kind = match summary {
    Summary::Text(text) => return Some((None, text(rows))),
    Summary::Kind(kind) => *kind,
  };
  if kind == SummaryKind::Count {
    return Some((Some(labels.of(kind).to_owned()), rows.len().to_string()));
  }
  let numbers: Vec<f64> = rows.iter().filter_map(|row| number_of(column, row)).collect();
  let value = match kind {
    _ if numbers.is_empty() => String::from("–"),
    SummaryKind::Sum => said(numbers.iter().sum()),
    SummaryKind::Average => said(numbers.iter().sum::<f64>() / numbers.len() as f64),
    SummaryKind::Min => said(numbers.iter().copied().fold(f64::INFINITY, f64::min)),
    SummaryKind::Max => said(numbers.iter().copied().fold(f64::NEG_INFINITY, f64::max)),
    SummaryKind::Count => unreachable!("counted above"),
  };
  Some((Some(labels.of(kind).to_owned()), value))
}

/// A summary row as a renderer sees it: its cells in the tracks of the columns, at the bottom of the grid where it
/// stays, or at the end of a group's rows.
pub struct TableSummary {
  /// Its line among the lines shown, where it ends a group: `None` for the grid's, at its bottom.
  pub position: Option<usize>,
  /// Its cells, a column's each: what the column says of the rows, or nothing.
  pub cells: Vec<TableCell>,
  pub(super) recipe: ResolvedRecipe,
  pub(super) tracks: ColumnTracks,
  pub(super) columns_built: Option<Arc<Vec<bool>>>,
  pub(super) geometry: Geometry,
  pub(super) checkbox: bool,
  /// How wide the track of the rows' numbers is, where the grid has them.
  pub(super) number_lead: f32,
  pub(super) pinned_start: bool,
  pub(super) pinned_end: bool,
  pub(super) fill: Option<BackgroundColor>,
  /// What it is called to what reads the tree.
  pub(super) name: String,
  pub(super) id: Option<Arc<str>>,
}

impl TableSummary {
  pub fn part(&self, part: impl Into<Name>) -> ResolvedPart {
    self.recipe.part(part)
  }

  /// How the default renderer builds a summary row: a cell for each column in its track, the grid's held at the
  /// bottom of what scrolls. It is a row to what reads the tree, its cells cells.
  pub fn render(mut self) -> Element {
    let mut row = Row::new()
      .part(&self.part(TablePart::Summary))
      .spacing(0.0)
      .align_items(Alignment::Stretch)
      .column_tracks(&self.tracks)
      .describe("a11y_role", "row")
      .describe("a11y_name", self.name.clone());
    if self.position.is_none() {
      row = row.sticky(Sticky::bottom(0.0)).key("summary");
    } else {
      row = row.key(format!("summary-{}", self.position.unwrap_or(0)));
    }
    if self.pinned_start {
      row = row.padding_left(0.0);
    }
    if self.pinned_end {
      row = row.padding_right(0.0);
    }
    if let Some(id) = &self.id {
      row = row.id(id.clone());
    }
    let numbered = self.number_lead > 0.0;
    if numbered {
      // The column of numbers says nothing here, and covers what scrolls under it.
      let mut lead = Row::new().sticky(Sticky::left(0.0));
      if let Some(fill) = self.fill.clone() {
        lead = lead.background(fill);
      }
      row = row.child(lead);
    }
    if self.checkbox {
      let mut lead = lead_cell(
        &self.part(TablePart::Select),
        self.geometry,
        self.pinned_start && !numbered,
      );
      if self.pinned_start {
        lead = lead.sticky(Sticky::left(self.number_lead));
      }
      if let Some(fill) = self.fill.clone() {
        lead = lead.background(fill);
      }
      row = row.child(lead);
    }
    let cells = std::mem::take(&mut self.cells);
    let placed = cells.into_iter().map(|cell| (cell.column, cell.render()));
    row
      .with_children(in_tracks(
        self.columns_built.as_ref().map(|built| built.as_slice()),
        placed,
      ))
      .into()
  }
}

/// What a summary cell holds: its label muted, then its value.
pub(super) fn summary_content(recipe: &ResolvedRecipe, said: Option<(Option<String>, String)>) -> Element {
  let Some((label, value)) = said else {
    return Row::new().into();
  };
  let mut content = Row::new().align_items(Alignment::Center).spacing(6.0);
  if let Some(label) = label {
    content = content.child(recipe.part(TablePart::SummaryLabel).text(&label).nowrap());
  }
  content
    .child(recipe.part(TablePart::SummaryValue).text(&value).nowrap())
    .into()
}
