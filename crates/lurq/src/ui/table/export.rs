//! What a grid writes out of its rows: comma-separated text (CSV) or tab-separated text, a line for each row after
//! a line of the columns' titles, which a spreadsheet opens.

use std::{hash::Hash, sync::Arc};

use super::{Content, EditorKind, GridModel, TableColumn};

/// The form of the text a grid's rows are written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExportFormat {
  /// Comma-separated values, as RFC 4180 has them: a field that holds a comma, a quote or a line break is quoted, its
  /// quotes doubled, and the lines end with CR LF.
  Csv,
  /// Tab-separated values, as a spreadsheet's copy has them: a tab or a line break in a field is a space, and the
  /// lines end with LF.
  Tsv,
}

impl ExportFormat {
  /// The extension of a file of this form: `csv` or `tsv`.
  pub fn extension(self) -> &'static str {
    match self {
      Self::Csv => "csv",
      Self::Tsv => "tsv",
    }
  }

  /// The media type of this form: `text/csv` or `text/tab-separated-values`.
  pub fn media_type(self) -> &'static str {
    match self {
      Self::Csv => "text/csv",
      Self::Tsv => "text/tab-separated-values",
    }
  }

  /// `rows` written in this form, a line each.
  pub fn write(self, rows: &[Vec<String>]) -> String {
    match self {
      Self::Csv => rows
        .iter()
        .map(|row| row.iter().map(|field| csv_field(field)).collect::<Vec<_>>().join(","))
        .map(|line| line + "\r\n")
        .collect(),
      Self::Tsv => rows
        .iter()
        .map(|row| {
          row
            .iter()
            .map(|field| field.replace(['\t', '\n', '\r'], " "))
            .collect::<Vec<_>>()
            .join("\t")
        })
        .map(|line| line + "\n")
        .collect(),
    }
  }
}

/// A field of comma-separated text: quoted where it holds a comma, a quote, a line break, or a space at an end that
/// would otherwise be lost.
fn csv_field(field: &str) -> std::borrow::Cow<'_, str> {
  let quoted = field.contains([',', '"', '\n', '\r']) || field.starts_with([' ', '\t']) || field.ends_with([' ', '\t']);
  if quoted {
    format!("\"{}\"", field.replace('"', "\"\"")).into()
  } else {
    field.into()
  }
}

/// Which rows of a grid are written out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ExportScope {
  /// Every row the grid shows, filtered and sorted as it shows them, those not built or not in view too, in the
  /// columns it shows, in their order.
  Shown,
  /// The rows selected, in the columns shown; with none selected, the range of cells, its rows and its columns.
  Selected,
}

/// A grid's rows written out from its menu: what is told to [`DataGrid::on_export`](crate::ui::DataGrid::on_export).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridExport {
  pub format: ExportFormat,
  pub scope: ExportScope,
  /// The text, its first line the columns' titles.
  pub text: String,
  /// How many rows it holds, the titles' line not counted.
  pub rows: usize,
}

/// The words of a grid's export, each said otherwise:
///
/// ```
/// use lurq::ui::ExportLabels;
///
/// let labels = ExportLabels {
///   export: "Експорт".into(),
///   ..ExportLabels::default()
/// };
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportLabels {
  /// The menu of the export, in a column's menu.
  pub export: String,
  pub shown_csv: String,
  pub shown_tsv: String,
  pub selected_csv: String,
  pub selected_tsv: String,
}

impl Default for ExportLabels {
  fn default() -> Self {
    let locale = crate::ui::current_locale();
    Self {
      export: locale.data.export.to_string(),
      shown_csv: locale.data.shown_csv.to_string(),
      shown_tsv: locale.data.shown_tsv.to_string(),
      selected_csv: locale.data.selected_csv.to_string(),
      selected_tsv: locale.data.selected_tsv.to_string(),
    }
  }
}

/// `rows` in `columns`, written in `format` after a line of the columns' titles: what a column says of a row is its
/// text, or the value its editor edits where it has no text (a box `true` or `false`), or nothing for a column
/// whose cell is an element.
///
/// ```
/// use lurq::ui::{ExportFormat, TableColumn, to_delimited};
///
/// struct Crate { name: &'static str, size: u32 }
/// let columns = vec![
///   TableColumn::new("name", "Name").text(|row: &Crate| row.name.to_owned()),
///   TableColumn::new("size", "Size").text(|row: &Crate| format!("{} KB", row.size)),
/// ];
/// let rows = [Crate { name: "lurq, the engine", size: 412 }, Crate { name: "gallery", size: 96 }];
/// assert_eq!(
///   to_delimited(&columns, &rows, ExportFormat::Csv),
///   "Name,Size\r\n\"lurq, the engine\",412 KB\r\ngallery,96 KB\r\n"
/// );
/// ```
pub fn to_delimited<'a, R: 'a>(
  columns: &[TableColumn<R>],
  rows: impl IntoIterator<Item = &'a R>,
  format: ExportFormat,
) -> String {
  let mut lines = vec![
    columns
      .iter()
      .map(|column| column.title.to_string())
      .collect::<Vec<_>>(),
  ];
  lines.extend(
    rows
      .into_iter()
      .map(|row| columns.iter().map(|column| cell_text(column, row)).collect()),
  );
  format.write(&lines)
}

/// What `column` says of `row` written out: its text, or the value its editor edits, or nothing.
fn cell_text<R>(column: &TableColumn<R>, row: &R) -> String {
  if let Content::Text(text) = &column.content {
    return text(row);
  }
  match column.editor.as_ref().map(|editor| &editor.kind) {
    Some(EditorKind::Text(text)) => text(row),
    Some(EditorKind::Number(number)) => number(row).to_string(),
    Some(EditorKind::Choice(choices, chosen)) => choices
      .get(chosen(row))
      .map(|choice| choice.to_string())
      .unwrap_or_default(),
    Some(EditorKind::Checkbox(checked)) => checked(row).to_string(),
    None => String::new(),
  }
}

/// `rows` in `columns` as comma-separated text (see [`to_delimited`]).
pub fn to_csv<'a, R: 'a>(columns: &[TableColumn<R>], rows: impl IntoIterator<Item = &'a R>) -> String {
  to_delimited(columns, rows, ExportFormat::Csv)
}

/// `rows` in `columns` as tab-separated text (see [`to_delimited`]).
pub fn to_tsv<'a, R: 'a>(columns: &[TableColumn<R>], rows: impl IntoIterator<Item = &'a R>) -> String {
  to_delimited(columns, rows, ExportFormat::Tsv)
}

/// The words of a grid's bar ([`DataGrid::toolbar`](crate::ui::DataGrid::toolbar)), each said otherwise:
///
/// ```
/// use lurq::ui::GridBarLabels;
///
/// let labels = GridBarLabels {
///   search: "Шукати файли".into(),
///   ..GridBarLabels::default()
/// };
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridBarLabels {
  /// The bar's name to what reads the tree.
  pub bar: String,
  /// The search field's placeholder.
  pub search: String,
  /// The button that opens a column's filter, from a menu of the columns that filter.
  pub filter: String,
  pub columns: String,
  pub export: String,
  /// The group of the densities, and each, from the roomiest.
  pub density: String,
  pub comfortable: String,
  pub default: String,
  pub compact: String,
}

impl Default for GridBarLabels {
  fn default() -> Self {
    let locale = crate::ui::current_locale();
    Self {
      bar: locale.data.grid_tools.to_string(),
      search: locale.data.search_rows.to_string(),
      filter: locale.data.filter.to_string(),
      columns: locale.data.columns.to_string(),
      export: locale.data.export.to_string(),
      density: locale.data.density.to_string(),
      comfortable: locale.data.comfortable.to_string(),
      default: locale.data.default.to_string(),
      compact: locale.data.compact.to_string(),
    }
  }
}

/// What a filter of a column lets through, in a few words: what the bar's tag of the filter says after the column's
/// title ("Status: Passing, Failed", "Size: 10 to 20").
pub(super) fn filter_said(filter: &super::ColumnFilter) -> String {
  use super::{ColumnFilter, TextMatch};
  let locale = crate::ui::current_locale();
  let words = &locale.data;
  let fill = crate::core::fill;
  let range = |from: Option<String>, to: Option<String>| match (from, to) {
    (Some(from), Some(to)) if from == to => from,
    (Some(from), Some(to)) => fill(&words.from_to, &[("from", &from), ("to", &to)]),
    (Some(from), None) => fill(&words.from_only, &[("from", &from)]),
    (None, Some(to)) => fill(&words.to_only, &[("to", &to)]),
    (None, None) => String::new(),
  };
  match filter {
    ColumnFilter::Text { how, text } => match how {
      TextMatch::Contains => text.clone(),
      TextMatch::StartsWith => format!("{text}…"),
      TextMatch::Equals => format!("= {text}"),
    },
    ColumnFilter::Number { at_least, at_most } => {
      range(at_least.map(|n| n.to_string()), at_most.map(|n| n.to_string()))
    }
    ColumnFilter::Date { from, to } => range(from.map(|day| day.to_string()), to.map(|day| day.to_string())),
    ColumnFilter::Set(values) => values.join(", "),
    ColumnFilter::Bool(value) => if *value { &locale.common.yes } else { &locale.common.no }.to_string(),
  }
}

/// Where a column's menu writes the grid's rows out from: whether anything is selected to write, and the words.
#[derive(Clone)]
pub(super) struct ExportLink {
  pub selected: bool,
  pub labels: Arc<ExportLabels>,
}

impl<R: Send + Sync + 'static, K: Clone + Eq + Hash + Send + Sync + 'static> GridModel<R, K> {
  /// The rows of `scope` in the columns shown written in `format` after their titles, and how many there are: every
  /// row shown, or those selected (with none, the range of cells). A cell says its column's text, or the value its
  /// editor edits.
  pub(super) fn export_rows(&self, scope: ExportScope, format: ExportFormat) -> (String, usize) {
    let every: Vec<usize> = (0..self.titles.len()).collect();
    let (ranks, columns): (Vec<usize>, Vec<usize>) = match scope {
      ExportScope::Shown => ((0..self.keys.len()).collect(), every),
      ExportScope::Selected => {
        let mut ranks: Vec<usize> = self
          .model
          .selected
          .iter()
          .filter_map(|key| self.keys.rank_of(key))
          .collect();
        ranks.sort_unstable();
        ranks.dedup();
        match self.span {
          Some(span) if ranks.is_empty() => {
            let ((top, bottom), (start, end)) = (span.rows(), span.columns());
            let last = self.keys.len().saturating_sub(1);
            (
              (top..=bottom.min(last)).collect(),
              (start..=end.min(every.len().saturating_sub(1))).collect(),
            )
          }
          _ => (ranks, every),
        }
      }
    };
    let mut lines = vec![
      columns
        .iter()
        .map(|column| self.titles[*column].to_string())
        .collect::<Vec<_>>(),
    ];
    lines.extend(
      ranks
        .iter()
        .map(|rank| columns.iter().map(|column| self.export_cell(*rank, *column)).collect()),
    );
    (format.write(&lines), ranks.len())
  }

  /// What the cell at `rank` and `column` says written out: its column's text, as it is shown, or the value its
  /// editor edits.
  fn export_cell(&self, rank: usize, column: usize) -> String {
    match (self.row(rank), self.texts.get(column).and_then(Option::as_ref)) {
      (Some(values), Some(text)) => text(values),
      _ => self.cell_text(rank, column),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::ExportFormat;

  fn rows(rows: &[&[&str]]) -> Vec<Vec<String>> {
    rows
      .iter()
      .map(|row| row.iter().map(|field| field.to_string()).collect())
      .collect()
  }

  #[test]
  fn a_field_with_a_comma_a_quote_a_line_break_or_an_end_space_is_quoted() {
    let text = ExportFormat::Csv.write(&rows(&[
      &["plain", "a, b", "say \"hi\""],
      &["two\nlines", " lead", "trail "],
    ]));
    assert_eq!(
      text,
      "plain,\"a, b\",\"say \"\"hi\"\"\"\r\n\"two\nlines\",\" lead\",\"trail \"\r\n"
    );
  }

  #[test]
  fn tab_separated_fields_lose_their_tabs_and_line_breaks() {
    let text = ExportFormat::Tsv.write(&rows(&[&["a\tb", "c\nd", "e"]]));
    assert_eq!(text, "a b\tc d\te\n");
  }
}
