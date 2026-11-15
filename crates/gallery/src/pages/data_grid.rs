//! The page of the data grid: its playground, the board's Data Grid, editing in place, a range of cells, a
//! column's menu, group rows, summary rows, row numbers, columns pinned at both ends and resized, ten thousand rows of
//! fifty columns, a grid that is read only, and its props.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::Density},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{
    Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, CellEditor, CellRange, CellValue, ColumnFilter,
    ColumnLayout, ColumnPin, DataGrid, GridPage, GridRequest, GroupKey, Pagination, SearchField, SearchMatches,
    SummaryKind, TableColumn, TableFilters, TableSort, icons,
  },
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{mono, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// What a file's build came to.
const STATUSES: [&str; 4] = ["Passing", "Building", "Modified", "Failed"];

fn badge(status: usize) -> Element {
  let variant = [
    BadgeVariant::Success,
    BadgeVariant::Info,
    BadgeVariant::Warning,
    BadgeVariant::Destructive,
  ][status % 4];
  Badge::new(STATUSES[status % 4]).variant(variant).into()
}

/// A file of a crate, as a grid of them shows it and edits it.
#[derive(Clone, Debug, PartialEq, lurq::DevtoolsInspectable)]
struct File {
  name: String,
  status: usize,
  size: f64,
  shared: bool,
  updated: &'static str,
}

const NAMES: [&str; 8] = [
  "blur.rs",
  "canvas.rs",
  "theme.rs",
  "layout.rs",
  "scroll.rs",
  "table.rs",
  "focus.rs",
  "recipe.rs",
];
const UPDATED: [&str; 4] = ["2 min ago", "just now", "1 h ago", "yesterday"];

/// The board's three files first, then as many as asked for.
fn files(count: usize) -> Vec<File> {
  (0..count)
    .map(|index| File {
      name: NAMES[index % NAMES.len()].to_owned(),
      status: [1, 0, 0, 2, 3, 0, 1, 0][index % 8],
      size: [12.0, 48.0, 9.0, 31.0, 7.0, 64.0, 22.0, 18.0][index % 8],
      shared: index % 3 == 0,
      updated: UPDATED[index % UPDATED.len()],
    })
    .collect()
}

/// The columns of a grid of files: the name, its status, its size and whether it is shared are edited, when it
/// changed is read.
fn columns(pin_name: bool) -> Vec<TableColumn<File>> {
  vec![
    TableColumn::new("name", "Name")
      .text(|row: &File| row.name.clone())
      .mono(true)
      .min_width(160.0)
      .sortable(true)
      .pin(if pin_name { ColumnPin::Start } else { ColumnPin::None })
      .editor(CellEditor::text(|row: &File| row.name.clone())),
    TableColumn::new("status", "Status")
      .width(120.0)
      .cell(|row: &File| badge(row.status))
      .editor(CellEditor::choice(STATUSES, |row: &File| row.status)),
    TableColumn::new("size", "Size")
      .width(90.0)
      .numeric()
      .text(|row: &File| format!("{} KB", row.size))
      .sort_key(|row: &File| row.size as i64)
      .editor(CellEditor::number(|row: &File| row.size)),
    TableColumn::new("shared", "Shared")
      .width(80.0)
      .editor(CellEditor::checkbox(|row: &File| row.shared)),
    TableColumn::new("updated", "Updated")
      .width(110.0)
      .muted(true)
      .text(|row: &File| row.updated.to_owned()),
  ]
}

/// Takes an edit into `rows`: whether it is taken. An empty name is refused, and stays in its editor.
fn take(rows: &Signal<Vec<File>>, name: &str, column: &str, value: &CellValue) -> bool {
  let mut files = rows.get();
  let Some(file) = files.iter_mut().find(|file| file.name == name) else {
    return false;
  };
  match (column, value) {
    ("name", CellValue::Text(text)) if text.trim().is_empty() => return false,
    ("name", CellValue::Text(text)) => file.name = text.clone(),
    ("status", CellValue::Choice(status)) => file.status = *status,
    ("size", CellValue::Number(size)) => file.size = *size,
    ("shared", CellValue::Checked(shared)) => file.shared = *shared,
    _ => return false,
  }
  rows.set(files);
  true
}

/// The rows of a page of the board's grid, in the order of `sort`.
fn sorted(mut rows: Vec<File>, sort: &[TableSort]) -> Vec<File> {
  TableSort::sort_rows(sort, &columns(false), &mut rows);
  rows
}

const SELECTIONS: [&str; 3] = ["none", "checkboxes", "by a press"];
const SUMMARIES: [(&str, Option<SummaryKind>); 6] = [
  ("none", None),
  ("sum", Some(SummaryKind::Sum)),
  ("average", Some(SummaryKind::Average)),
  ("count", Some(SummaryKind::Count)),
  ("min", Some(SummaryKind::Min)),
  ("max", Some(SummaryKind::Max)),
];
const GROUPINGS: [(&str, &[&str]); 3] = [
  ("none", &[]),
  ("status", &["status"]),
  ("status, shared", &["status", "shared"]),
];
const DENSITIES: [(&str, Density); 3] = [
  ("compact", Density::Compact),
  ("default", Density::Default),
  ("comfortable", Density::Comfortable),
];

pub struct DataGridPage {
  frame: Frame,
  playground: Playground,
  selection: Signal<usize>,
  density: Signal<usize>,
  fixed_height: Signal<bool>,
  pin_name: Signal<bool>,
  read_only: Signal<bool>,
  resizable: Signal<bool>,
  footer: Signal<bool>,
  bind_range: Signal<bool>,
  played_range: Signal<Option<CellRange<String>>>,
  column_menu: Signal<bool>,
  played_layout: Signal<ColumnLayout>,
  grouping: Signal<usize>,
  played_open: Signal<Vec<GroupKey>>,
  summary: Signal<usize>,
  row_numbers: Signal<bool>,
  played: Signal<Vec<File>>,
  played_selected: Signal<Vec<String>>,
  played_sort: Signal<Vec<TableSort>>,
  board_selected: Signal<Vec<String>>,
  board_sort: Signal<Vec<TableSort>>,
  page: Signal<usize>,
  edited: Signal<Vec<File>>,
  log: Signal<Vec<String>>,
  pinned: Signal<Vec<File>>,
  ranged: Signal<Vec<File>>,
  range: Signal<Option<CellRange<String>>>,
  menu_rows: Signal<Vec<File>>,
  menu_sort: Signal<Vec<TableSort>>,
  layout: Signal<ColumnLayout>,
  /// The arrangement of the menu's grid as stored: its text, with its sort's.
  stored_layout: Signal<Option<(String, String)>>,
  group_rows: Signal<Vec<File>>,
  groups_open: Signal<Vec<GroupKey>>,
  summed_rows: Signal<Vec<File>>,
  summed_open: Signal<Vec<GroupKey>>,
  numbered_selected: Signal<Vec<String>>,
  big: Signal<bool>,
  big_selected: Signal<Vec<usize>>,
  multi_sort: Signal<Vec<TableSort>>,
  /// The playground's filters, whether its columns filter, and whether it has a row of filters.
  played_filters: Signal<TableFilters>,
  filtering: Signal<bool>,
  filter_row: Signal<bool>,
  /// The filters of the section's grid.
  filters: Signal<TableFilters>,
  /// The search of the section's grid, and what it was told of its matches.
  query: Signal<String>,
  found: Signal<SearchMatches>,
  /// The grid over rows on demand: its sort and filters, what the app was last told, and whether the next range
  /// fails.
  asked_sort: Signal<Vec<TableSort>>,
  asked_filters: Signal<TableFilters>,
  asked: Signal<String>,
  fail_next: Arc<std::sync::atomic::AtomicBool>,
}

/// A ticket of the server the grid over rows on demand asks.
#[derive(Clone)]
struct Ticket {
  id: u32,
  title: String,
  size: u32,
  open: bool,
}

/// The server's hundred thousand tickets.
fn tickets() -> Arc<Vec<Ticket>> {
  static TICKETS: std::sync::OnceLock<Arc<Vec<Ticket>>> = std::sync::OnceLock::new();
  const WORDS: [&str; 8] = ["Layout", "Canvas", "Focus", "Scroll", "Theme", "Text", "Grid", "Menu"];
  TICKETS
    .get_or_init(|| {
      Arc::new(
        (0..100_000u32)
          .map(|id| Ticket {
            id,
            title: format!("{} issue {}", WORDS[(id as usize * 7) % 8], id),
            size: (id * 37) % 500,
            open: id % 3 != 0,
          })
          .collect(),
      )
    })
    .clone()
}

/// What the server answers a request: its tickets sorted, filtered and searched as the query asks, the range of
/// them asked for, and how many there are; or a failure where it is told to fail the next.
fn answer(request: &GridRequest, fail: &std::sync::atomic::AtomicBool) -> Result<GridPage<Ticket>, String> {
  if fail.swap(false, std::sync::atomic::Ordering::SeqCst) {
    return Err("the server is down".into());
  }
  let all = tickets();
  let search = request.query.search.to_lowercase();
  let mut rows: Vec<&Ticket> = all
    .iter()
    .filter(|ticket| {
      request.query.filters.iter().all(|(id, filter)| match (id, filter) {
        ("title", ColumnFilter::Text { text, .. }) => ticket.title.to_lowercase().contains(&text.to_lowercase()),
        ("size", ColumnFilter::Number { at_least, at_most }) => {
          at_least.is_none_or(|low| ticket.size as f64 >= low) && at_most.is_none_or(|high| ticket.size as f64 <= high)
        }
        ("open", ColumnFilter::Bool(open)) => ticket.open == *open,
        _ => true,
      }) && (search.is_empty() || ticket.title.to_lowercase().contains(&search))
    })
    .collect();
  // Sorted by the last column first, each sort keeping the order of the rows it finds equal.
  for sort in request.query.sort.iter().rev() {
    rows.sort_by(|a, b| {
      let order = match &*sort.column {
        "id" => a.id.cmp(&b.id),
        "title" => a.title.cmp(&b.title),
        "size" => a.size.cmp(&b.size),
        _ => std::cmp::Ordering::Equal,
      };
      if sort.direction == lurq::ui::SortDirection::Descending {
        order.reverse()
      } else {
        order
      }
    });
  }
  let total = rows.len();
  Ok(GridPage {
    rows: rows
      .into_iter()
      .skip(request.range.start)
      .take(request.range.len())
      .cloned()
      .collect(),
    total,
  })
}

/// The columns of the files, each filtered as it is: the name by its text, the status by its values, the size by
/// a number, whether it is shared by a yes or no.
fn filtered_columns(columns: Vec<TableColumn<File>>) -> Vec<TableColumn<File>> {
  columns
    .into_iter()
    .map(|column| match column.id() {
      "name" => column.filter_text(true),
      "status" => column.filter_values(|row: &File| STATUSES[row.status].to_owned()),
      "size" => column.filter_number(|row: &File| row.size),
      "shared" => column.filter_bool(|row: &File| row.shared),
      "updated" => column.filter_set(true),
      _ => column,
    })
    .collect()
}

/// A row of the grid of a million: its place, which every cell is made from.
#[derive(Clone)]
struct Job(u32);

/// A million rows, made once and shared by every grid that shows them.
fn million() -> Arc<Vec<Job>> {
  static ROWS: std::sync::OnceLock<Arc<Vec<Job>>> = std::sync::OnceLock::new();
  ROWS.get_or_init(|| Arc::new((0..1_000_000).map(Job).collect())).clone()
}

/// Two hundred columns of a job: its number pinned at the start, a note that wraps on every seventh, and metrics.
fn million_columns() -> Vec<TableColumn<Job>> {
  let mut columns = vec![
    TableColumn::new("job", "Job")
      .width(110.0)
      .mono(true)
      .pin(ColumnPin::Start)
      .text(|job: &Job| format!("job-{:07}", job.0)),
    TableColumn::new("note", "Note")
      .width(220.0)
      .wrap(true)
      .text(|job: &Job| {
        if job.0 % 7 == 0 {
          format!(
            "Rerun {} after the cache was cleared: the first build took the long way round",
            job.0 / 7
          )
        } else {
          "-".to_owned()
        }
      }),
  ];
  columns.extend((2..200).map(|index| {
    TableColumn::new(&format!("m{index}"), &format!("Metric {index}"))
      .width(110.0)
      .numeric()
      .text(move |job: &Job| format!("{:.1}", (job.0 as f64 * index as f64) % 997.0 / 7.0))
  }));
  columns
}

impl DataGridPage {
  /// The playground: a grid of files as the panel's props make it, its edits taken into its rows.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let selection = self.selection.get().min(SELECTIONS.len() - 1);
    let (density_name, density) = DENSITIES[self.density.get().min(DENSITIES.len() - 1)];
    let (fixed_height, pin_name, read_only, resizable, footer, bind_range) = (
      self.fixed_height.get(),
      self.pin_name.get(),
      self.read_only.get(),
      self.resizable.get(),
      self.footer.get(),
      self.bind_range.get(),
    );
    let column_menu = self.column_menu.get();
    let row_numbers = self.row_numbers.get();
    let (_, grouping) = GROUPINGS[self.grouping.get().min(GROUPINGS.len() - 1)];
    let (summary_name, summary) = SUMMARIES[self.summary.get().min(SUMMARIES.len() - 1)];
    let mut played_columns = columns(pin_name);
    if let Some(kind) = summary {
      played_columns[2] = played_columns[2].clone().summary(kind);
    }
    let (filtering, filter_row) = (self.filtering.get(), self.filter_row.get());
    if filtering {
      played_columns = filtered_columns(played_columns);
    }
    let record = self.playground.recorder();
    let rows = self.played.clone();
    let sort = self.played_sort.get();

    let (edited, selected_to, sorted_to, ranged_to, arranged_to, opened_to) = (
      record.clone(),
      record.clone(),
      record.clone(),
      record.clone(),
      record.clone(),
      record,
    );
    let mut grid = DataGrid::new(played_columns, sorted(rows.get(), &sort))
      .row_key(|row: &File| row.name.clone())
      .label("Files")
      .density(density)
      .read_only(read_only)
      .resizable(resizable)
      .column_menu(column_menu)
      .row_numbers(row_numbers)
      .filters(self.played_filters.clone())
      .filter_row(filtering && filter_row)
      .group_by(grouping.iter().copied())
      .open(self.played_open.clone())
      .on_open_change(move |key, open| opened_to(format!("on_open_change({:?}, {open})", key.values)))
      .column_layout(self.played_layout.clone())
      .on_column_layout_change(move |layout| arranged_to(format!("on_column_layout_change({})", said_layout(layout))))
      .sort(self.played_sort.clone())
      .on_sort_change(move |sort| sorted_to(format!("on_sort_change({sort:?})")))
      .on_edit({
        let rows = rows.clone();
        move |edit| {
          let taken = take(&rows, edit.row, edit.column, &edit.value);
          edited(format!(
            "on_edit({:?}, {:?}, {:?}) -> {taken}",
            edit.row, edit.column, edit.value
          ));
          taken
        }
      })
      .force(self.playground.forced());
    if selection > 0 {
      grid = grid
        .selected(self.played_selected.clone())
        .checkboxes(selection == 1)
        .on_selection_change(move |keys| selected_to(format!("on_selection_change({keys:?})")));
    }
    if fixed_height {
      grid = grid.height(240.0);
    }
    if footer {
      grid = grid.count(true);
    }
    if bind_range {
      grid = grid
        .cell_selection(self.played_range.clone())
        .on_cell_selection_change(move |range| {
          ranged_to(match range {
            Some(range) => format!(
              "on_cell_selection_change({} rows × {:?})",
              range.rows.len(),
              range.columns.iter().map(|id| id.as_ref()).collect::<Vec<_>>()
            ),
            None => "on_cell_selection_change(None)".to_owned(),
          })
        });
    }

    let mut code = vec![
      "// files: Signal<Vec<File>>, the app's rows".to_owned(),
      "DataGrid::new([".to_owned(),
      format!(
        "    TableColumn::new(\"name\", \"Name\").text(|row| row.name.clone()).mono(true).sortable(true){}",
        if pin_name { ".pin(ColumnPin::Start)" } else { "" }
      ),
      "        .editor(CellEditor::text(|row| row.name.clone())),".to_owned(),
      "    TableColumn::new(\"status\", \"Status\").width(120.0).cell(|row| badge(row.status))".to_owned(),
      "        .editor(CellEditor::choice(STATUSES, |row| row.status)),".to_owned(),
      "    TableColumn::new(\"size\", \"Size\").width(90.0).numeric().text(|row| format!(\"{} KB\", row.size))"
        .to_owned(),
      "        .editor(CellEditor::number(|row| row.size)),".to_owned(),
      "    TableColumn::new(\"shared\", \"Shared\").width(80.0).editor(CellEditor::checkbox(|row| row.shared)),"
        .to_owned(),
      "    TableColumn::new(\"updated\", \"Updated\").width(110.0).muted(true).text(|row| row.updated.into()),"
        .to_owned(),
      "], files.get())".to_owned(),
      ".row_key(|row| row.name.clone())".to_owned(),
      ".label(\"Files\")".to_owned(),
      ".sort(sort)".to_owned(),
    ];
    match selection {
      1 => code.push(".selected(selected)".to_owned()),
      2 => code.push(".selected(selected).checkboxes(false)".to_owned()),
      _ => {}
    }
    if density != Density::Default {
      code.push(format!(".density(Density::{})", capitalized(density_name)));
    }
    if read_only {
      code.push(".read_only(true)".to_owned());
    }
    if !resizable {
      code.push(".resizable(false)".to_owned());
    }
    if fixed_height {
      code.push(".height(240.0)".to_owned());
    }
    if footer {
      code.push(".count(true)".to_owned());
    }
    if bind_range {
      code.push(".cell_selection(range) // Signal<Option<CellRange<String>>>".to_owned());
    }
    code.push(".column_layout(layout) // Signal<ColumnLayout>".to_owned());
    if !grouping.is_empty() {
      code.push(format!(".group_by({grouping:?}).open(open) // Signal<Vec<GroupKey>>"));
    }
    if row_numbers {
      code.push(".row_numbers(true)".to_owned());
    }
    if summary.is_some() {
      code.push(format!(
        "// the size's column: .summary(SummaryKind::{})",
        capitalized(summary_name)
      ));
    }
    if !column_menu {
      code.push(".column_menu(false)".to_owned());
    }
    code.push(".on_edit(|edit| /* take it into files, or refuse it */ true)".to_owned());
    code.push(".mount(ctx)".to_owned());

    let tree = vec![
      format!("root        {} · a table's · grid", self.playground.measured()),
      "├─ row_number  at the start, held there · pressed to select".to_owned(),
      "├─ header   the table's · a menu_trigger and a resizer at each column's end · drop_line".to_owned(),
      "├─ group_row  a row at its level · chevron, group_label, group_count".to_owned(),
      "├─ row      not a stop · Left and Right its own".to_owned(),
      "└─ summary  at the bottom, held there · summary_label, summary_value".to_owned(),
      "   └─ cell     a gridcell · the ring inside it from the keyboard · in_range, range_top..range_end".to_owned(),
      "      └─ editor   Input, NumberInput or Select, Sm, while it is edited".to_owned(),
    ];

    let (set_selection, set_density) = (self.selection.clone(), self.density.clone());
    let indexed = |names: &[&'static str]| -> Vec<(&'static str, usize)> {
      names.iter().enumerate().map(|(index, name)| (*name, index)).collect()
    };
    let densities: Vec<(&str, usize)> = DENSITIES
      .iter()
      .enumerate()
      .map(|(index, (name, _))| (*name, index))
      .collect();
    let reset = (
      self.selection.clone(),
      self.density.clone(),
      self.fixed_height.clone(),
      self.pin_name.clone(),
      self.read_only.clone(),
      self.resizable.clone(),
      self.footer.clone(),
      self.played.clone(),
      self.played_selected.clone(),
      self.played_sort.clone(),
      self.bind_range.clone(),
      self.played_range.clone(),
      self.column_menu.clone(),
      self.played_layout.clone(),
      self.grouping.clone(),
      self.played_open.clone(),
      self.summary.clone(),
      self.row_numbers.clone(),
      self.big.clone(),
      (
        self.filtering.clone(),
        self.filter_row.clone(),
        self.played_filters.clone(),
      ),
    );
    let still = {
      let rows = rows.get();
      move || -> Element { DataGrid::new(columns(pin_name), rows.clone()).label("Files").into() }
    };
    let big = self.big.get();
    let shown = if big {
      // The same props over a million rows of 200 columns, a note on every seventh that wraps.
      let mut grid = DataGrid::new_shared(million_columns(), million())
        .label("Jobs")
        .density(density)
        .read_only(read_only)
        .resizable(resizable)
        .column_menu(column_menu)
        .row_numbers(row_numbers)
        .count(footer)
        .height(400.0)
        .force(self.playground.forced())
        .key("played-million");
      if selection > 0 {
        grid = grid.selected(self.big_selected.clone()).checkboxes(selection == 1);
      }
      code = vec![
        "// rows: Arc<Vec<Job>>, a million; columns: the job pinned, a note that wraps, 198 metrics".to_owned(),
        "DataGrid::new_shared(columns, rows)".to_owned(),
        ".label(\"Jobs\")".to_owned(),
        ".height(400.0)".to_owned(),
        ".mount(ctx)".to_owned(),
      ];
      grid.mount(ctx)
    } else {
      grid.mount(ctx)
    };
    self.playground.card(
      ctx,
      Spec {
        title: "DataGrid",
        make: Arc::new(still),
        shown: Shown::of(shown),
        fill: true,
        forceable: true,
        facts: if big {
          format!(
            "1,000,000 rows · 200 columns · {} selected",
            self.big_selected.get().len()
          )
        } else {
          format!(
            "{} rows · {} selected · {} cells in the range",
            rows.get().len(),
            self.played_selected.get().len(),
            self.played_range.get().map_or(0, |range| range.len())
          )
        },
        event: "on_edit / on_selection_change / on_cell_selection_change / on_column_layout_change / on_open_change",
        code,
        tree,
        controls: vec![
          prop(
            "selected",
            "Signal<Vec<K>>",
            self
              .playground
              .select(0, &indexed(&SELECTIONS), &selection, move |index| {
                set_selection.set(index)
              }),
          ),
          prop(
            "density",
            "Density",
            self
              .playground
              .select(1, &densities, &self.density.get(), move |index| set_density.set(index)),
          ),
          divider(),
          flag("read_only", &self.read_only),
          flag("resizable", &self.resizable),
          flag("height(240): the header stays", &self.fixed_height),
          flag("name pinned at the start", &self.pin_name),
          flag("count in a footer", &self.footer),
          flag("cell_selection(signal): told the range", &self.bind_range),
          flag("column_menu: a menu on each header", &self.column_menu),
          flag("row_numbers: a column of numbers at the start", &self.row_numbers),
          flag("a million rows by 200 columns", &self.big),
          flag("filters: each column by its kind", &self.filtering),
          flag("filter_row: a filter under each header", &self.filter_row),
          prop(
            "summary",
            "SummaryKind of the size",
            self
              .playground
              .select(3, &indexed(&SUMMARIES.map(|(name, _)| name)), &self.summary.get(), {
                let set = self.summary.clone();
                move |index| set.set(index)
              }),
          ),
          prop(
            "group_by",
            "columns' ids",
            self
              .playground
              .select(2, &indexed(&GROUPINGS.map(|(name, _)| name)), &self.grouping.get(), {
                let set = self.grouping.clone();
                move |index| set.set(index)
              }),
          ),
        ],
        changed: [
          selection != 1,
          density != Density::Default,
          read_only,
          !resizable,
          !fixed_height,
          pin_name,
          footer,
          !bind_range,
          !column_menu,
          !grouping.is_empty(),
          summary.is_some(),
          row_numbers,
          big,
          filtering,
          filter_row,
          self.played_layout.get() != ColumnLayout::default(),
          self.played.get() != files(8),
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          reset.0.set(1);
          reset.1.set(1);
          reset.2.set(true);
          reset.3.set(false);
          reset.4.set(false);
          reset.5.set(true);
          reset.6.set(false);
          reset.7.set(files(8));
          reset.8.set(Vec::new());
          reset.9.set(Vec::new());
          reset.10.set(true);
          reset.11.set(None);
          reset.12.set(true);
          reset.13.set(ColumnLayout::default());
          reset.14.set(0);
          reset.15.set(Vec::new());
          reset.16.set(0);
          reset.17.set(false);
          reset.18.set(false);
          reset.19.0.set(false);
          reset.19.1.set(false);
          reset.19.2.set(TableFilters::new());
        }),
      },
    )
  }
}

/// An arrangement of columns as a line: each column in its order, with what a person changed of it.
fn said_layout(layout: &ColumnLayout) -> String {
  layout
    .columns
    .iter()
    .map(|place| {
      let mut said = place.id.to_string();
      if place.hidden {
        said.push_str(" hidden");
      }
      if let Some(width) = place.width {
        said.push_str(&format!(" {width:.0}"));
      }
      if place.fit {
        said.push_str(" fit");
      }
      match place.pin {
        ColumnPin::Start => said.push_str(" pinned at the start"),
        ColumnPin::End => said.push_str(" pinned at the end"),
        ColumnPin::None => {}
      }
      said
    })
    .collect::<Vec<_>>()
    .join(" · ")
}

fn capitalized(name: &str) -> String {
  let mut chars = name.chars();
  chars
    .next()
    .map(|first| first.to_uppercase().chain(chars).collect())
    .unwrap_or_default()
}

impl Component for DataGridPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 4),
      selection: ctx.signal(1),
      density: ctx.signal(1),
      fixed_height: ctx.signal(true),
      pin_name: ctx.signal(false),
      read_only: ctx.signal(false),
      resizable: ctx.signal(true),
      footer: ctx.signal(false),
      bind_range: ctx.signal(true),
      played_range: ctx.signal(None),
      column_menu: ctx.signal(true),
      played_layout: ctx.signal(ColumnLayout::default()),
      grouping: ctx.signal(0),
      played_open: ctx.signal(Vec::new()),
      summary: ctx.signal(0),
      row_numbers: ctx.signal(false),
      played: ctx.signal(files(8)),
      played_selected: ctx.signal(Vec::new()),
      played_sort: ctx.signal(Vec::new()),
      board_selected: ctx.signal(vec!["blur.rs".to_owned()]),
      board_sort: ctx.signal(vec![TableSort::ascending("name")]),
      page: ctx.signal(1),
      edited: ctx.signal(files(5)),
      log: ctx.signal(Vec::new()),
      pinned: ctx.signal(files(8)),
      ranged: ctx.signal(files(8)),
      range: ctx.signal(None),
      menu_rows: ctx.signal(files(8)),
      menu_sort: ctx.signal(Vec::new()),
      layout: ctx.signal(ColumnLayout::default()),
      stored_layout: ctx.signal(None),
      group_rows: ctx.signal(files(16)),
      groups_open: ctx.signal(vec![GroupKey::new(["Passing"])]),
      summed_rows: ctx.signal(files(16)),
      summed_open: ctx.signal(vec![GroupKey::new(["Building"]), GroupKey::new(["Modified"])]),
      numbered_selected: ctx.signal(vec!["canvas.rs".to_owned()]),
      big: ctx.signal(false),
      big_selected: ctx.signal(Vec::new()),
      multi_sort: ctx.signal(vec![TableSort::ascending("status"), TableSort::descending("size")]),
      played_filters: ctx.signal(TableFilters::new()),
      filtering: ctx.signal(false),
      filter_row: ctx.signal(false),
      filters: ctx.signal(TableFilters::new().with("status", ColumnFilter::one_of(["Passing", "Building"]))),
      query: ctx.signal("rs".to_owned()),
      found: ctx.signal(SearchMatches::default()),
      asked_sort: ctx.signal(Vec::new()),
      asked_filters: ctx.signal(TableFilters::new()),
      asked: ctx.signal(String::new()),
      fail_next: Arc::new(std::sync::atomic::AtomicBool::new(false)),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    // The board's: three of a page of 24, the first selected, sorted by name, a footer with a pager.
    let board_columns = || {
      vec![
        TableColumn::new("name", "Name")
          .text(|row: &File| row.name.clone())
          .mono(true)
          .sortable(true)
          .editor(CellEditor::text(|row: &File| row.name.clone())),
        TableColumn::new("status", "Status")
          .width(110.0)
          .cell(|row: &File| badge(row.status))
          .editor(CellEditor::choice(STATUSES, |row: &File| row.status)),
        TableColumn::new("size", "Size")
          .width(70.0)
          .numeric()
          .text(|row: &File| format!("{} KB", row.size))
          .editor(CellEditor::number(|row: &File| row.size)),
      ]
    };
    let board_sort = self.board_sort.get();
    let page_rows: Vec<File> = files(24)
      .into_iter()
      .skip((self.page.get().max(1) - 1) * 3)
      .take(3)
      .collect();
    let board = DataGrid::new(board_columns(), sorted(page_rows, &board_sort))
      .row_key(|row: &File| row.name.clone())
      .label("Files")
      .sort(self.board_sort.clone())
      .selected(self.board_selected.clone())
      .total(24)
      .pagination(Pagination::for_items(self.page.clone(), 24, 3).compact(true))
      .style(lurq::ui::TablePart::Root, |s| s.width(640.0))
      .key("board")
      .mount(ctx);

    // Editing in place, each commit written under the grid.
    let (rows, log) = (self.edited.clone(), self.log.clone());
    let editing = DataGrid::new(columns(false), rows.get())
      .row_key(|row: &File| row.name.clone())
      .label("Files")
      .on_edit({
        let (rows, log) = (rows.clone(), log.clone());
        move |edit| {
          let taken = take(&rows, edit.row, edit.column, &edit.value);
          log.update(|log| {
            log.insert(
              0,
              format!(
                "{} · {} → {:?}{}",
                edit.row,
                edit.column,
                edit.value,
                if taken { "" } else { " · refused" }
              ),
            );
            log.truncate(3);
          });
          taken
        }
      })
      .key("editing")
      .mount(ctx);
    let said = self.log.get();
    let editing = Column::new()
      .spacing(10.0)
      .width(crate::kit::FILL)
      .child(editing)
      .child(quiet(
        &if said.is_empty() {
          "Edit a cell: Enter, F2, a double press or a letter. An empty name is refused.".to_owned()
        } else {
          said.join("   ")
        },
        12.0,
      ));

    // A range of cells: what it spans, by the rows' keys and the columns' ids, and the sizes in it added up.
    let ranged_rows = self.ranged.clone();
    let range_grid = DataGrid::new(columns(false), ranged_rows.get())
      .row_key(|row: &File| row.name.clone())
      .label("Files")
      .cell_selection(self.range.clone())
      .on_edit(move |edit| take(&ranged_rows, edit.row, edit.column, &edit.value))
      .key("range")
      .mount(ctx);
    let range_said = match self.range.get() {
      Some(range) => {
        let files = self.ranged.get();
        let sizes: f64 = files
          .iter()
          .filter(|file| range.rows.contains(&file.name))
          .map(|file| file.size)
          .sum();
        format!(
          "{} rows × {} columns: {} · {}{}",
          range.rows.len(),
          range.columns.len(),
          range.rows.join(", "),
          range.columns.join(", "),
          if range.columns.iter().any(|id| id.as_ref() == "size") {
            format!(" · {sizes} KB in all")
          } else {
            String::new()
          }
        )
      }
      None => "Drag from a cell, or Shift with the arrows or a press. Ctrl+C copies the range as tab-separated \
               text, Ctrl+V pastes into it, Delete clears what is edited, Ctrl+A is every cell."
        .to_owned(),
    };
    let range_grid = Column::new()
      .spacing(10.0)
      .width(crate::kit::FILL)
      .child(range_grid)
      .child(quiet(&range_said, 12.0));

    // A column's menu: the arrangement it makes, which the app could store, under the grid.
    let menu_rows = self.menu_rows.clone();
    let menu_grid = DataGrid::new(columns(false), sorted(menu_rows.get(), &self.menu_sort.get()))
      .row_key(|row: &File| row.name.clone())
      .label("Files")
      .sort(self.menu_sort.clone())
      .column_layout(self.layout.clone())
      .on_edit(move |edit| take(&menu_rows, edit.row, edit.column, &edit.value))
      .key("menu")
      .mount(ctx);
    let arrangement = self.layout.get();
    let reset_layout = self.layout.clone();
    let saved = self.stored_layout.get();
    let store = (self.layout.clone(), self.menu_sort.clone(), self.stored_layout.clone());
    let restore = store.clone();
    // The text forms, which change as the grid does: what Store keeps and Restore reads back.
    let text = format!(
      "{}\nsort: {}",
      if arrangement.columns.is_empty() {
        "(the columns as the app gives them)".to_owned()
      } else {
        arrangement.to_text()
      },
      match self.menu_sort.get() {
        sort if sort.is_empty() => "none".to_owned(),
        sort => TableSort::list_to_text(&sort),
      }
    );
    let menu_grid = Column::new()
      .spacing(10.0)
      .width(crate::kit::FILL)
      .child(menu_grid)
      .child(quiet(
        &if arrangement.columns.is_empty() {
          "Point at a header and press its ⋮, or press Alt+↓ on it. Drag a header to move its column, or press \
           Ctrl+Shift+PageUp or PageDown on it."
            .to_owned()
        } else {
          said_layout(&arrangement)
        },
        12.0,
      ))
      .child(
        Column::new()
          .width(crate::kit::FILL)
          .padding(12.0)
          .rounded(lurq::app::theme::RadiusSize::Md)
          .background(lurq::app::theme::PaletteColor::Muted)
          .child(mono(&text, 12.0).color(lurq::app::theme::PaletteColor::MutedForeground)),
      )
      .child(
        Row::new()
          .spacing(8.0)
          .align_items(Alignment::Center)
          .child(
            Button::new("Store")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm)
              .on_click(move |_| {
                let sort = TableSort::list_to_text(&store.1.get());
                store.2.set(Some((store.0.get().to_text(), sort)));
              }),
          )
          .child(
            Button::new("Restore")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm)
              .disabled(saved.is_none())
              .on_click(move |_| {
                if let Some((layout, sort)) = restore.2.get() {
                  // Read back and made valid for the columns the grid has now.
                  if let Some(layout) = ColumnLayout::parse(&layout) {
                    let ids: Vec<Arc<str>> = columns(false).iter().map(|column| Arc::from(column.id())).collect();
                    restore.0.set(layout.valid(ids.iter().map(|id| &**id)));
                  }
                  restore.1.set(TableSort::parse_list(&sort).unwrap_or_default());
                }
              }),
          )
          .child(
            Button::new("Reset columns")
              .variant(ButtonVariant::Ghost)
              .size(ButtonSize::Sm)
              .icon(icons::REFRESH)
              .disabled(arrangement.columns.is_empty())
              .on_click(move |_| reset_layout.set(ColumnLayout::default())),
          )
          .child(quiet(
            &match &saved {
              Some((layout, _)) => format!("stored: {} chars", layout.len()),
              None => "nothing stored yet".to_owned(),
            },
            12.0,
          )),
      );

    // Group rows: by the status, each group opened and closed, the rows of the closed ones not built.
    let group_rows = self.group_rows.clone();
    let grouped = DataGrid::new(columns(false), group_rows.get())
      .label("Files")
      .height(320.0)
      .group_by(["status"])
      .open(self.groups_open.clone())
      .on_edit(move |edit| {
        let name = group_rows.get().get(*edit.row).map(|file| file.name.clone());
        name.is_some_and(|name| take(&group_rows, &name, edit.column, &edit.value))
      })
      .key("groups")
      .mount(ctx);

    // Summary rows: the grid's at its bottom, and each group's at the end of its rows.
    let summed_rows = self.summed_rows.clone();
    let mut summed_columns = columns(false);
    summed_columns[0] = summed_columns[0].clone().summary(SummaryKind::Count);
    summed_columns[1] = summed_columns[1].clone().summary_text(|rows: &[&File]| {
      let failed = rows.iter().filter(|row| row.status == 3).count();
      if failed == 0 {
        "all fine".to_owned()
      } else {
        format!("{failed} failed")
      }
    });
    summed_columns[2] = summed_columns[2].clone().summary(SummaryKind::Sum);
    let summed = DataGrid::new(summed_columns, summed_rows.get())
      .label("Files")
      .height(360.0)
      .group_by(["status"])
      .open(self.summed_open.clone())
      .on_edit(move |edit| {
        let name = summed_rows.get().get(*edit.row).map(|file| file.name.clone());
        name.is_some_and(|name| take(&summed_rows, &name, edit.column, &edit.value))
      })
      .key("summary")
      .mount(ctx);

    // Row numbers: a column of them at the start, which stays, pressed to select a row, its header all of them.
    // Sorted by several columns: the status first, the rows of a status by their size, the largest first.
    let multi_columns = || {
      vec![
        TableColumn::new("status", "Status")
          .width(130.0)
          .cell(|row: &File| badge(row.status))
          .sort_key(|row: &File| row.status as i64),
        TableColumn::new("name", "Name")
          .text(|row: &File| row.name.clone())
          .mono(true)
          .sortable(true),
        TableColumn::new("size", "Size")
          .width(100.0)
          .numeric()
          .text(|row: &File| format!("{} KB", row.size))
          .sort_key(|row: &File| row.size as i64),
      ]
    };
    let multi = self.multi_sort.get();
    let mut multi_rows = files(8);
    TableSort::sort_rows(&multi, &multi_columns(), &mut multi_rows);
    let multi_grid = Column::new()
      .spacing(10.0)
      .width(crate::kit::FILL)
      .child(
        DataGrid::new(multi_columns(), multi_rows)
          .row_key(|row: &File| row.name.clone())
          .label("Files")
          .read_only(true)
          .sort(self.multi_sort.clone())
          .key("multi-sort")
          .mount(ctx),
      )
      .child(quiet(
        &format!(
          "sort: {}",
          if multi.is_empty() {
            "none".to_owned()
          } else {
            TableSort::list_to_text(&multi)
          }
        ),
        12.0,
      ));

    // Filtered: each column by its kind, from its menu and in the row under the header.
    let filters_now = self.filters.get();
    let (clear, shown_filters) = (self.filters.clone(), filters_now.clone());
    let filter_grid = Column::new()
      .spacing(10.0)
      .width(crate::kit::FILL)
      .child(
        DataGrid::new(filtered_columns(columns(false)), files(8))
          .row_key(|row: &File| row.name.clone())
          .label("Files")
          .read_only(true)
          .count(true)
          .height(360.0)
          .filters(self.filters.clone())
          .filter_row(true)
          .key("filtered")
          .mount(ctx),
      )
      .child(
        Row::new()
          .spacing(8.0)
          .align_items(Alignment::Center)
          .child(quiet(
            &format!(
              "filters: {}",
              if shown_filters.is_empty() {
                "none".to_owned()
              } else {
                shown_filters.to_text()
              }
            ),
            12.0,
          ))
          .child(
            Button::new("Clear all")
              .variant(ButtonVariant::Ghost)
              .size(ButtonSize::Sm)
              .disabled(filters_now.is_empty())
              .on_click(move |_| clear.set(TableFilters::new())),
          ),
      );

    // Searched: every match in the cells marked, F3 to the next, Ctrl+F the find bar over the grid.
    let found = self.found.get();
    let told = self.found.clone();
    let search_grid = Column::new()
      .spacing(10.0)
      .width(crate::kit::FILL)
      .child(
        SearchField::new(self.query.clone())
          .placeholder("Search the files")
          .label("Search the files"),
      )
      .child(
        DataGrid::new(columns(false), files(8))
          .row_key(|row: &File| row.name.clone())
          .label("Files")
          .read_only(true)
          .height(280.0)
          .search(self.query.clone())
          .find_bar(true)
          .on_matches_change(move |matches| told.set(matches))
          .key("searched")
          .mount(ctx),
      )
      .child(quiet(
        &match found.current {
          Some(current) => format!("on_matches_change: match {} of {}", current + 1, found.count),
          None => "on_matches_change: no match".to_owned(),
        },
        12.0,
      ));

    // Rows on demand: the server sorts, filters and searches; the grid asks for the ranges in view.
    let fail = self.fail_next.clone();
    let told = self.asked.clone();
    let failing = self.fail_next.clone();
    let source_grid = Column::new()
      .spacing(10.0)
      .width(crate::kit::FILL)
      .child(
        DataGrid::on_demand(
          vec![
            TableColumn::new("id", "Id")
              .width(90.0)
              .mono(true)
              .sortable(true)
              .text(|ticket: &Ticket| ticket.id.to_string()),
            TableColumn::new("title", "Title")
              .sortable(true)
              .filter_text(true)
              .text(|ticket: &Ticket| ticket.title.clone()),
            TableColumn::new("size", "Size")
              .width(100.0)
              .numeric()
              .sortable(true)
              .filter_number(|ticket: &Ticket| ticket.size as f64)
              .text(|ticket: &Ticket| format!("{} KB", ticket.size)),
            TableColumn::new("open", "Open")
              .width(90.0)
              .filter_bool(|ticket: &Ticket| ticket.open)
              .text(|ticket: &Ticket| if ticket.open { "open" } else { "closed" }.to_owned()),
          ],
          100_000,
          move |request: GridRequest| {
            let answered = answer(&request, &fail);
            async move { answered }
          },
        )
        .label("Tickets")
        .height(360.0)
        .count(true)
        .filter_row(true)
        .sort(self.asked_sort.clone())
        .filters(self.asked_filters.clone())
        .on_query_change(move |query| told.set(query.to_text()))
        .key("on-demand")
        .mount(ctx),
      )
      .child(
        Row::new()
          .spacing(8.0)
          .align_items(Alignment::Center)
          .child(
            Button::new("Fail the next range")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm)
              .on_click(move |_| failing.store(true, std::sync::atomic::Ordering::SeqCst)),
          )
          .child(quiet(
            &format!(
              "on_query_change: {}",
              match self.asked.get() {
                asked if asked.is_empty() => "(every row, in its order)".to_owned(),
                asked => asked,
              }
            ),
            12.0,
          )),
      );

    let numbered = DataGrid::new(columns(false), files(8))
      .row_key(|row: &File| row.name.clone())
      .label("Files")
      .height(280.0)
      .selected(self.numbered_selected.clone())
      .checkboxes(false)
      .row_numbers(true)
      .key("numbers")
      .mount(ctx);

    // Pinned at both ends: the name stays at the start, the actions at the end, while the columns between scroll.
    let pinned_rows = self.pinned.clone();
    let mut pinned_columns = columns(true);
    pinned_columns.extend((1..=4).map(|index| {
      TableColumn::new(&format!("note{index}"), &format!("Note {index}"))
        .width(150.0)
        .muted(true)
        .text(move |row: &File| format!("{} · note {index}", row.name))
    }));
    pinned_columns.push(
      TableColumn::actions("actions", |_row: &File| {
        Button::new("Open")
          .variant(ButtonVariant::Ghost)
          .size(ButtonSize::Xs)
          .icon(icons::EXTERNAL_LINK)
          .into()
      })
      .pin(ColumnPin::End),
    );
    let pinned = DataGrid::new(pinned_columns, pinned_rows.get())
      .row_key(|row: &File| row.name.clone())
      .label("Files")
      .height(280.0)
      .on_edit(move |edit| take(&pinned_rows, edit.row, edit.column, &edit.value))
      .key("pinned")
      .mount(ctx);

    // A million rows of two hundred columns: the cells in view are built, the notes that wrap measured.
    let thousands = DataGrid::new_shared(million_columns(), million())
      .label("Jobs")
      .read_only(true)
      .row_numbers(true)
      .count(true)
      .height(400.0)
      .key("million")
      .mount(ctx);

    let read_only = DataGrid::new(columns(false), files(3))
      .row_key(|row: &File| row.name.clone())
      .label("Files")
      .read_only(true)
      .key("read-only")
      .mount(ctx);

    let rows = [
      (
        "columns",
        "impl IntoIterator<Item = TableColumn<R>>",
        "",
        "A table's columns; editor(CellEditor) on one says how its cells are edited.",
      ),
      (
        "rows",
        "impl IntoIterator<Item = R>",
        "",
        "The rows, in the order the app wants them.",
      ),
      (
        "row_key",
        "impl Fn(&R) -> K",
        "the row's place",
        "What tells a row apart: the selection and an edit name rows by it.",
      ),
      (
        "on_edit",
        "impl Fn(&CellEdit<K>) -> bool",
        "",
        "Told the row's key, the column's id and the new value at each commit; false refuses it.",
      ),
      (
        "CellEditor",
        "text / number / choice / checkbox",
        "",
        "An Input, a NumberInput, a Select of the column's choices, or a box ticked in the cell.",
      ),
      (
        "read_only",
        "bool",
        "false",
        "No cell is edited; the cells still take the focus.",
      ),
      (
        "TableColumn::summary / summary_text",
        "SummaryKind / Fn(&[&R]) -> String",
        "none",
        "What a column says in the summary rows: a sum, an average, a count, the least or the most, or the app's.",
      ),
      (
        "row_numbers",
        "bool",
        "false",
        "A column of the rows' numbers at the start, which stays: pressed to select a row, its header all of them.",
      ),
      (
        "group_by / open",
        "columns' ids / Signal<Vec<GroupKey>>",
        "none / the grid's own",
        "Rows grouped by the values of columns, nested; the groups open, with open_controlled and on_open_change.",
      ),
      (
        "column_layout",
        "Signal<ColumnLayout>",
        "the grid's own",
        "How the columns are arranged: their order, which are hidden, their widths and pins; with its controlled \
         form and on_column_layout_change.",
      ),
      (
        "column_menu",
        "bool",
        "true",
        "A menu on each header: sort, pin, size to fit, hide, show the others.",
      ),
      (
        "cell_selection",
        "Signal<Option<CellRange<K>>>",
        "the grid's own",
        "The range of cells selected, by the rows' keys and the columns' ids; with its controlled form and \
         on_cell_selection_change.",
      ),
      (
        "resizable",
        "bool",
        "true",
        "Columns made wider or narrower by their edge or Ctrl and an arrow on their header.",
      ),
      (
        "selected / sort",
        "Signal<Vec<K>> / Signal<Vec<TableSort>>",
        "none",
        "As a table has them, with their controlled forms and handlers.",
      ),
      ("density / height / count / pagination", "", "", "As a table has them."),
      (
        "mount",
        "&mut Ctx",
        "",
        "The grid that works: cells that take the focus and are edited, the rows and columns in view built.",
      ),
    ];

    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Data Grid"],
        title: "Data Grid",
        description: "A table whose cells are worked in: the focus is on a cell, the arrows move it, and a cell is \
                      edited in place with the controls of the system. It is a table in another form, with its \
                      columns, its selection, its sort and its rows built in view.",
        import: Some("use lurq::ui::{CellEditor, CellValue, DataGrid, TableColumn};"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of DataGrid is a control on the right. Tab into the cells, move with the arrows, edit with \
           Enter, F2 or a letter, commit with Enter or Tab, put back with Escape. An empty name is refused.",
          playground,
        ),
        Section::new(
          "The board's data grid",
          "A column of checkboxes 12 from the name, the name in the mono type and sorted, a status as a badge, a \
           size of 12 in the muted color at its end, the selected row on the highlight, a footer that counts the \
           rows with the pager at its end.",
          self.frame.demo(
            0,
            board,
            "DataGrid::new(columns, rows)",
            &[
              "DataGrid::new([",
              "    TableColumn::new(\"name\", \"Name\").text(|row| row.name.clone()).mono(true).sortable(true)",
              "        .editor(CellEditor::text(|row| row.name.clone())),",
              "    TableColumn::new(\"status\", \"Status\").width(110.0).cell(|row| badge(row.status))",
              "        .editor(CellEditor::choice(STATUSES, |row| row.status)),",
              "    TableColumn::new(\"size\", \"Size\").width(70.0).numeric().text(|row| format!(\"{} KB\", \
               row.size))",
              "        .editor(CellEditor::number(|row| row.size)),",
              "], rows)",
              ".row_key(|row| row.name.clone())",
              ".sort(sort)",
              ".selected(selected)",
              ".total(24)",
              ".pagination(Pagination::for_items(page, 24, 3).compact(true))",
              ".mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Editing in place",
          "Each column says how its cells are edited: a text in an input, a number, a choice in a select, a box \
           ticked in the cell. The app is told each commit and takes it into its rows, or refuses it, which leaves \
           the editor open and marked wrong.",
          self.frame.demo(
            1,
            editing,
            ".on_edit(|edit| ..)",
            &[
              ".on_edit(move |edit| {",
              "    let mut files = rows.get();",
              "    let Some(file) = files.iter_mut().find(|file| &file.name == edit.row) else { return false };",
              "    match (edit.column, &edit.value) {",
              "        (\"name\", CellValue::Text(name)) if name.trim().is_empty() => return false,",
              "        (\"name\", CellValue::Text(name)) => file.name = name.clone(),",
              "        (\"status\", CellValue::Choice(status)) => file.status = *status,",
              "        (\"size\", CellValue::Number(size)) => file.size = *size,",
              "        (\"shared\", CellValue::Checked(shared)) => file.shared = *shared,",
              "        _ => return false,",
              "    }",
              "    rows.set(files);",
              "    true",
              "})",
            ],
          ),
        ),
        Section::new(
          "A range of cells",
          "A drag from a cell, Shift with the arrows or with a press select the cells between, drawn as one area \
           with the focused cell in it; Ctrl+A selects them all. The app is told the range by the rows' keys and \
           the columns' ids. Ctrl+C copies it as tab-separated text, Ctrl+V pastes into it cell by cell through \
           on_edit, Delete clears what is edited. It is apart from the rows' selection.",
          self.frame.demo(
            5,
            range_grid,
            ".cell_selection(range)",
            &[
              "// range: Signal<Option<CellRange<String>>>",
              "DataGrid::new(columns, files.get())",
              "    .row_key(|row| row.name.clone())",
              "    .cell_selection(range)",
              "    .on_edit(|edit| /* a paste and Delete come here cell by cell */ true)",
              "    .mount(ctx)",
              "// range.rows: the keys of its rows; range.columns: the ids of its columns",
            ],
          ),
        ),
        Section::new(
          "A column's menu",
          "Each header has a menu at its end, shown under the pointer and with the focus, and opened by Alt+Down \
           on the header: it sorts the rows by the column, pins it at the start or the end, sizes it to its \
           content, hides it, and shows the columns hidden. A header dragged to another place moves its column, \
           as Ctrl+Shift+PageUp and PageDown on it do. How the columns are arranged is a value the app holds and \
           stores: its text form, under the grid, changes as the grid does; Store keeps it with the sort's, Restore \
           reads them back with ColumnLayout::parse and TableSort::parse_list, made valid for the columns the grid has.",
          self.frame.demo(
            6,
            menu_grid,
            ".column_layout(layout)",
            &[
              "// layout: Signal<ColumnLayout>, sort: Signal<Vec<TableSort>>",
              "DataGrid::new(columns, files.get())",
              "    .sort(sort.clone())",
              "    .column_layout(layout.clone())",
              "    .mount(ctx)",
              "// Store: storage.set_value(\"columns\", layout.get()) and (\"sort\", sort.get())",
              "// Restore: storage.value::<ColumnLayout>(\"columns\").map(|layout| layout.valid(ids))",
            ],
          ),
        ),
        Section::new(
          "Rows on demand",
          "A server's table: the grid is told how many rows there are and asks for the ranges that come into \
           view, a hundred at a time, through the engine's futures; rows whose range has not come are skeletons, \
           and a range that failed says so with a retry (press Fail the next range, then scroll). The sort, the \
           filters and the search are the app's to apply: each request carries them as one GridQuery, whose text \
           form is written under the grid as the app is told it.",
          self.frame.demo(
            13,
            source_grid,
            "DataGrid::on_demand(columns, total, fetch)",
            &[
              "DataGrid::on_demand(columns, 100_000, |request: GridRequest| async move {",
              "    // request.query: the sort, the filters, the search; request.range: the rows asked for",
              "    server.tickets(request).await // Ok(GridPage { rows, total }) or Err(message)",
              "})",
              ".sort(sort).filters(filters).filter_row(true)",
              ".on_query_change(|query| store(query.to_text()))",
              ".mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Search",
          "A query over what the cells show: every match in a cell's text is marked as a find marks it, the \
           current one stronger. F3 and Shift+F3 in the grid go to the next match and the one before, across rows \
           not built yet, each brought into view; Ctrl+F opens the find bar over the grid, its query the grid's \
           search and its count the matches, which the app is told too. A cell whose content is an element is not \
           searched.",
          self.frame.demo(
            12,
            search_grid,
            ".search(query)",
            &[
              "// query: Signal<String>",
              "SearchField::new(query.clone())",
              "DataGrid::new(columns, files.get())",
              "    .search(query.clone())",
              "    .find_bar(true)",
              "    .on_matches_change(|matches| /* matches.count, matches.current */ ())",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Filters",
          "A column says how it is filtered: the name by its text, the status by the values ticked among those it \
           has, the size by a number, whether it is shared by a yes or no. A person filters it from the column's \
           menu, in a panel under its header, or in the row of filters (a range is written >10, <5 or 5..20); a \
           filtered header has its mark, and the count reads how many of all are shown. The filters are a value the \
           app holds and stores, as text; a summary is of the rows they let through, and the groups too.",
          self.frame.demo(
            11,
            filter_grid,
            ".filters(filters)",
            &[
              "// filters: Signal<TableFilters>",
              "TableColumn::new(\"name\", \"Name\").text(|row| row.name.clone()).filter_text(true)",
              "TableColumn::new(\"status\", \"Status\").cell(|row| badge(row.status)).filter_values(|row| status(row))",
              "TableColumn::new(\"size\", \"Size\").numeric().text(|row| kb(row)).filter_number(|row| row.size)",
              "DataGrid::new(columns, files.get())",
              "    .filters(filters.clone())",
              "    .filter_row(true)",
              "    .count(true)",
              "    .mount(ctx)",
              "// stored: filters.get().to_text(), read back with TableFilters::parse",
            ],
          ),
        ),
        Section::new(
          "Sorted by several columns",
          "A press of a header sorts the rows by its column alone: ascending, then descending, then not at all. \
           With Shift it adds the column to the sort after the others, or turns it, then takes it out; each sorted \
           header shows its place in the sort. The sort is a list the app holds, the first column first, and its \
           text form is stored as a column layout's is; TableSort::sort_rows sorts the rows by it.",
          self.frame.demo(
            10,
            multi_grid,
            ".sort(sort)",
            &[
              "// sort: Signal<Vec<TableSort>>, the first column first",
              "let mut rows = files.get();",
              "TableSort::sort_rows(&sort.get(), &columns, &mut rows);",
              "DataGrid::new(columns, rows).sort(sort.clone()).mount(ctx)",
              "// stored: TableSort::list_to_text(&sort.get()), read back with TableSort::parse_list",
            ],
          ),
        ),
        Section::new(
          "Group rows",
          "The rows grouped by their status: a group's row with its chevron, its value and how many rows it \
           holds, opened and closed by a press, Enter, Right and Left. The rows of a closed group are not built. \
           Grouping by a second column nests its groups in the first's.",
          self.frame.demo(
            7,
            grouped,
            ".group_by([\"status\"])",
            &[
              "// open: Signal<Vec<GroupKey>>, with GroupKey::new([\"Passing\"]) open",
              "DataGrid::new(columns, files.get())",
              "    .group_by([\"status\"])",
              "    .open(open)",
              "    .height(320.0)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Summary rows",
          "A summary of every row stays at the bottom of the grid, and each group open ends in one of its own: \
           the names counted, the sizes added up by the grid, the statuses said by the app.",
          self.frame.demo(
            8,
            summed,
            "TableColumn::summary(SummaryKind::Sum)",
            &[
              "TableColumn::new(\"name\", \"Name\").summary(SummaryKind::Count)",
              "TableColumn::new(\"status\", \"Status\").summary_text(|rows| format!(\"{} failed\", failed(rows)))",
              "TableColumn::new(\"size\", \"Size\").numeric().summary(SummaryKind::Sum)",
              "DataGrid::new(columns, files.get()).group_by([\"status\"]).open(open).mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Row numbers",
          "A column of the rows' numbers at the start, which stays while the columns scroll sideways. A number is \
           pressed to select its row, with Ctrl to add it, with Shift the rows from the last selected one; the \
           header's mark selects every row, or none.",
          self.frame.demo(
            9,
            numbered,
            ".row_numbers(true)",
            &["DataGrid::new(columns, files.get()).selected(selected).checkboxes(false).row_numbers(true).mount(ctx)"],
          ),
        ),
        Section::new(
          "Pinned and resized",
          "The name is pinned at the start and the actions at the end: the columns between scroll sideways under \
           them, and the bars keep clear of both. Drag the end of a header to make its column wider, or press Ctrl \
           and an arrow on it.",
          self.frame.demo(
            2,
            pinned,
            "TableColumn::pin(ColumnPin::Start)",
            &[
              "TableColumn::new(\"name\", \"Name\").pin(ColumnPin::Start)",
              "TableColumn::actions(\"actions\", |row| open_button(row)).pin(ColumnPin::End)",
              "DataGrid::new(columns, rows).height(280.0).mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "A million rows, two hundred columns",
          "A mounted grid builds the rows in view and a few around them, and of those the cells of the columns in \
           view, two past each side, the pinned ones and the focused one: a million rows cost what a screen of them \
           does. Every seventh note wraps and its row grows; the rows not built yet are taken to be a row high, and \
           what is in view stays still as they are measured. Drag the bar's thumb anywhere, or press Ctrl+End.",
          self.frame.demo(
            3,
            thousands,
            "DataGrid::new_shared(columns, rows)",
            &[
              "// rows: Arc<Vec<Job>>, a million, given again at every render without a copy",
              "// columns: the job pinned, a note that wraps, 198 metrics",
              "TableColumn::new(\"note\", \"Note\").width(220.0).wrap(true).text(|job| job.note())",
              "DataGrid::new_shared(columns, rows)",
              "    .read_only(true)",
              "    .row_numbers(true)",
              "    .count(true)",
              "    .height(400.0)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Read only",
          "The cells take the focus and the keys move it, and nothing is edited: what reads the tree is told each \
           cell is read only.",
          self.frame.demo(
            4,
            Row::new()
              .width(crate::kit::FILL)
              .align_items(Alignment::Start)
              .child(read_only),
            ".read_only(true)",
            &["DataGrid::new(columns, rows).read_only(true).mount(ctx)"],
          ),
        ),
        Section::new(
          "Props",
          "What a data grid is given beside a table's.",
          props_table(&rows, &["columns", "rows"], self.frame.room()),
        ),
      ],
    )
  }
}
