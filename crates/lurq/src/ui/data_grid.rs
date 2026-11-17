//! A data grid: a table whose cells are worked in. The focus is on a cell, the arrows move it from cell to cell,
//! and a cell is edited in place with `ui`'s own controls.
//!
//! It is [`Table`] in another form, not a second table: its columns are [`TableColumn`]s, with an editor each
//! that is edited ([`TableColumn::editor`]); its column tracks, the header that stays, the rows and the columns
//! it builds in view only, the selection, the sort, the pinned and the resized columns and the scroll bars that
//! keep clear of them are the table's, and so are its parts, its states and its renderer. What it adds: a cell
//! that takes the focus, its keys, the edit and what the app is told of it.

use std::{hash::Hash, sync::Arc};

use super::{
  Interaction, Length, Name, PartStyle, Recipe,
  table::{
    CellEdit, CellRange, ColumnLayout, ColumnMenuLabels, FilterLabels, GridPage, GridQuery, GridRequest, GroupKey,
    SourceLabels, SourceRow, SummaryLabels, Table, TableColumn, TableCount, TableFilters, TableSort,
  },
};
use crate::{
  app::{ctx::Ctx, theme::Density},
  core::{Signal, SignalValue},
  node::Element,
};

/// A data grid: rows of the same things in columns whose cells are worked in, a list of files with their names
/// edited, a sheet of orders with their states chosen.
///
/// ```
/// use lurq::{
///   app::{component::Component, ctx::Ctx},
///   core::Signal,
///   node::Element,
///   ui::{CellEditor, CellValue, DataGrid, TableColumn},
/// };
///
/// // In a signal, so DevTools must be able to show it where they are compiled in.
/// #[derive(Clone, lurq::DevtoolsInspectable)]
/// struct File { name: String, kind: usize, size_kb: f64, shared: bool }
///
/// fn grid(ctx: &mut Ctx, files: Signal<Vec<File>>, selected: Signal<Vec<String>>) -> Element {
///   let columns = vec![
///     TableColumn::new("name", "Name")
///       .text(|row: &File| row.name.clone())
///       .editor(CellEditor::text(|row: &File| row.name.clone())),
///     TableColumn::new("kind", "Kind")
///       .width(120.0)
///       .text(|row: &File| ["Rust", "Markdown"][row.kind].to_owned())
///       .editor(CellEditor::choice(["Rust", "Markdown"], |row: &File| row.kind)),
///     TableColumn::new("size", "Size")
///       .width(90.0)
///       .numeric()
///       .text(|row: &File| format!("{} KB", row.size_kb))
///       .editor(CellEditor::number(|row: &File| row.size_kb)),
///     TableColumn::new("shared", "Shared").width(80.0).editor(CellEditor::checkbox(|row: &File| row.shared)),
///   ];
///   let store = files.clone();
///   DataGrid::new(columns, files.get())
///     .row_key(|row: &File| row.name.clone())
///     .label("Files")
///     .selected(selected)
///     .height(320.0)
///     .on_edit(move |edit| {
///       // The app takes the edit into its rows, or refuses it: an empty name stays in the editor.
///       let mut files = store.get();
///       let Some(file) = files.iter_mut().find(|file| &file.name == edit.row) else { return false };
///       match (edit.column, &edit.value) {
///         ("name", CellValue::Text(name)) if name.is_empty() => return false,
///         ("name", CellValue::Text(name)) => file.name = name.clone(),
///         ("kind", CellValue::Choice(kind)) => file.kind = *kind,
///         ("size", CellValue::Number(size)) => file.size_kb = *size,
///         ("shared", CellValue::Checked(shared)) => file.shared = *shared,
///         _ => return false,
///       }
///       store.set(files);
///       true
///     })
///     .mount(ctx)
/// }
/// ```
///
/// **The focus is on a cell.** The grid is one stop of Tab, at the cell that last had the focus, or the first
/// cell of the first selected row. Left and Right move along the row, Up and Down to the same column of the
/// row before and after, Home and End to the row's first and last cell, Ctrl+Home and Ctrl+End to the first
/// cell of the first row and the last of the last, PageUp and PageDown by what the grid's height shows; the rows
/// not built are scrolled to, built and focused. Tab goes on to the next cell of the row, Shift+Tab to the one
/// before, and out of the grid from the row's last and first. Space selects the row (with Shift from the last
/// selected one), Enter on a cell that is not edited activates its row. A press of a cell does not select its row.
///
/// **A range of cells** ([`cell_selection`](Self::cell_selection)): a drag from a cell, Shift with the arrows or with
/// a press, and Ctrl+A select the cells between, drawn as one area with the focused cell in it, and told by the
/// rows' keys and the columns' ids. Ctrl+C copies it as tab-separated text, Ctrl+V pastes into it a cell at a time
/// through [`on_edit`](Self::on_edit), Delete clears what is edited.
///
/// **A column's menu** at the end of each header ([`column_menu`](Self::column_menu)), shown under the pointer and
/// with the focus and opened by Alt+Down too, sorts by the column, pins it, sizes it to its content, hides it and
/// shows the columns hidden; a header dragged to another place, or Ctrl+Shift+PageUp and PageDown on it, moves its
/// column. How the columns are arranged is a value the app holds and stores ([`column_layout`](Self::column_layout)).
///
/// **Group rows** ([`group_by`](Self::group_by)): the rows grouped by the values of columns, nested, each group a
/// row with its value and its count that opens and closes ([`open`](Self::open)), the rows of a closed one not
/// built.
///
/// **Summary rows** ([`TableColumn::summary`]): a row at the bottom of the grid that stays there, and one at the end
/// of each group open, with a sum, an average, a count, the least or the most the grid computes of a column of
/// numbers, or the app's text.
///
/// **The bar over it** ([`toolbar`](Self::toolbar)): the board's Table Toolbar, its search field the grid's search, a
/// tag for each filter set, Filter, the density, the columns and Export.
///
/// **Tree rows** ([`tree`](Self::tree)): each row under the row its parent key names, indented in the first column
/// with a chevron that opens and closes it, Right and Left there too. **A row's detail** ([`detail`](Self::detail)):
/// an element under an open row, as wide as the view. Both are lines of the grid's windowing.
///
/// **Export** ([`on_export`](Self::on_export)): each column's menu writes the rows shown, or those selected, out as
/// comma-separated or tab-separated text, the titles of the columns shown first, for the app to save; with no
/// handler, onto the clipboard. [`to_csv`](super::to_csv) writes any rows in the grid's columns.
///
/// **Row numbers** ([`row_numbers`](Self::row_numbers)): a column of them at the start that stays, a number pressed
/// to select its row, its header to select every row.
///
/// **A cell is edited in place** with its column's editor ([`CellEditor`]): an input for a text, a number input,
/// a select of the column's choices, or a box ticked in the cell itself. Enter or F2 on a cell, a double press of
/// it, or a letter typed on it (which the edit starts from) opens the editor; Enter commits and goes to the cell
/// under it, Tab commits and goes on to the next cell, Shift+Tab to the one before, Escape puts the cell back as
/// it was. A choice commits as it is chosen; a box turns at a press or Space. A commit tells the app the row's
/// key, the column's id and the new value ([`on_edit`](Self::on_edit)); the app that takes it changes its rows,
/// and one that refuses it (a handler returns `false`) leaves the editor open, said to be wrong. The focus moving
/// to another cell commits what was typed, and closes the editor however the app answers.
///
/// **What it builds.** Mounted ([`mount`](Self::mount)), a grid builds the rows in view and a few around them,
/// and of those the cells of the columns in view, two past each side, the pinned ones and the one that has the
/// focus: ten thousand rows of fifty columns cost what a screen of them does. Its columns are made wider or
/// narrower by a person unless a column says otherwise ([`TableColumn::resizable`]). Converted into an element,
/// it is the still form of itself: every row and every cell built, read and selected as a table's, with no cell
/// that takes the focus and nothing edited.
///
/// To what reads the tree it is a `grid`, its rows `row`s with `a11y_rowindex` and `a11y_selected`, its cells
/// `gridcell`s with `a11y_colindex` and `a11y_readonly`, its headers `columnheader`s.
pub struct DataGrid<R, K = usize> {
  table: Table<R, K>,
}

impl<X: Send + Sync + 'static> DataGrid<SourceRow<X>, usize> {
  /// A grid over rows the app gives on demand, as a server's table gives them: there are `total` at first, and
  /// as rows come into view the grid asks `fetch` for their range ([`GridRequest`]), which answers now or later
  /// ([`GridPage`]: the rows and how many there are in all), through the engine's futures. Rows whose range has
  /// not come are skeletons; a range that failed says so, with a retry. Each request carries the sort, the filters
  /// and the search a person set ([`GridQuery`]): the app applies them, and another of them asks for every range
  /// again. A row is told apart by its place; it is a [`SourceRow`], its row where it has come. The cells are not
  /// edited, and the rows are not grouped or summed: those are the app's.
  pub fn on_demand<Fut>(
    columns: impl IntoIterator<Item = TableColumn<X>>,
    total: usize,
    fetch: impl Fn(GridRequest) -> Fut + Send + Sync + 'static,
  ) -> Self
  where
    Fut: std::future::Future<Output = Result<GridPage<X>, String>> + Send + 'static,
  {
    DataGrid {
      table: Table::over_source(columns, total, fetch).as_grid(),
    }
  }
}

impl<R: Send + Sync + 'static> DataGrid<R, usize> {
  /// A grid of `rows` in `columns`, each row told apart by where it is until the app gives it a key
  /// ([`row_key`](Self::row_key)).
  pub fn new(columns: impl IntoIterator<Item = TableColumn<R>>, rows: impl IntoIterator<Item = R>) -> Self {
    Self {
      table: Table::new(columns, rows).as_grid(),
    }
  }

  /// A grid of rows the app keeps shared: a list of thousands given again at every render is neither copied nor
  /// built again.
  pub fn new_shared(columns: impl IntoIterator<Item = TableColumn<R>>, rows: Arc<Vec<R>>) -> Self {
    Self {
      table: Table::new_shared(columns, rows).as_grid(),
    }
  }

  /// What tells each row apart: its id, which the selection, the handlers and an edit name it by. Given before
  /// them.
  pub fn row_key<K: Clone + Eq + Hash + Send + Sync + 'static>(
    self,
    key: impl Fn(&R) -> K + Send + Sync + 'static,
  ) -> DataGrid<R, K> {
    DataGrid {
      table: self.table.row_key(key),
    }
  }
}

impl<R: Send + Sync + 'static, K: Clone + Eq + Hash + Send + Sync + 'static> DataGrid<R, K> {
  fn with(self, change: impl FnOnce(Table<R, K>) -> Table<R, K>) -> Self {
    Self {
      table: change(self.table),
    }
  }

  /// Called with each edit a person commits: the row's key, the column's id and the new value. Return whether
  /// the app takes it: one refused leaves the editor open, said to be wrong. A second handler is called after the
  /// first, and the edit is taken only where none refused it.
  pub fn on_edit(self, handler: impl Fn(&CellEdit<K>) -> bool + Send + Sync + 'static) -> Self {
    self.with(|table| table.add_on_edit(handler))
  }

  /// The range of cells selected: the signal's, which a drag, Shift with the arrows or a press, and Ctrl+A set,
  /// and a press of a cell, a move of the focus and Escape clear. Without it, the grid keeps its own.
  pub fn cell_selection(self, range: Signal<Option<CellRange<K>>>) -> Self
  where
    Option<CellRange<K>>: SignalValue,
  {
    let now = range.get();
    self.with(|table| table.cell_selection(now, Arc::new(move |next| range.set(next))))
  }

  /// The range of cells selected, as the app holds it: `on_change` is told the next after each drag, key or press
  /// that changes it, and the grid shows what the app gives it at the next render.
  pub fn cell_selection_controlled(
    self,
    range: Option<CellRange<K>>,
    on_change: impl Fn(Option<CellRange<K>>) + Send + Sync + 'static,
  ) -> Self {
    self.with(|table| table.cell_selection(range, Arc::new(on_change)))
  }

  /// Called with the range of cells selected after each drag, key or press that changes it, bound to a signal or
  /// not: not when the app sets it. A second handler is called after the first.
  pub fn on_cell_selection_change(self, handler: impl Fn(Option<&CellRange<K>>) + Send + Sync + 'static) -> Self {
    self.with(|table| table.add_on_cell_selection_change(handler))
  }

  /// How the columns are arranged: in what order, which are hidden, their widths and pins, the signal's, which a
  /// column's menu, a drag or the keys of a header and a resize set. Without it, the grid keeps its own; the
  /// columns given are as the app made them.
  pub fn column_layout(self, layout: Signal<ColumnLayout>) -> Self
  where
    ColumnLayout: SignalValue,
  {
    let now = layout.get();
    self.with(|table| table.column_layout(Some(now), Arc::new(move |next| layout.set(next))))
  }

  /// How the columns are arranged, as the app holds it: `on_change` is told the whole of the next at each change,
  /// and the grid shows what the app gives it at the next render.
  pub fn column_layout_controlled(
    self,
    layout: ColumnLayout,
    on_change: impl Fn(ColumnLayout) + Send + Sync + 'static,
  ) -> Self {
    self.with(|table| table.column_layout(Some(layout), Arc::new(on_change)))
  }

  /// Called with the whole of the arrangement of the columns at each change, bound to a signal or not: to store
  /// it. A second handler is called after the first.
  pub fn on_column_layout_change(self, handler: impl Fn(&ColumnLayout) + Send + Sync + 'static) -> Self {
    self.with(|table| table.add_on_column_layout_change(handler))
  }

  /// Whether each header has a menu: sort, pin, size to fit, hide the column and show the others. On unless
  /// told.
  pub fn column_menu(self, menu: bool) -> Self {
    self.with(|table| table.set_column_menu(menu))
  }

  /// The rows as a tree: `parent` gives the key of the row each is under ([`row_key`](Self::row_key)), or `None`
  /// for a row at the top. Each row comes after the row it is under, the rows under one in the order given (the
  /// sort's), indented 18 a level in the first column, where a chevron opens and closes the rows under it: a press
  /// of it, or Right and Left on the first cell, Left on a row that is closed going to the row it is under. A row
  /// a filter keeps keeps the rows it is under; a row whose parent is not among the rows is at the top. The rows of
  /// a closed row are not built. A grid grouped by columns is no tree.
  pub fn tree(self, parent: impl Fn(&R) -> Option<K> + Send + Sync + 'static) -> Self {
    self.with(|table| table.set_tree(parent))
  }

  /// The rows of the tree that are open, by their keys: those the signal has. Without it, the grid keeps its own,
  /// every row closed at first.
  pub fn expanded(self, open: Signal<Vec<K>>) -> Self
  where
    Vec<K>: SignalValue,
  {
    let now = open.get();
    self.with(|table| table.set_expanded(now, Arc::new(move |next| open.set(next))))
  }

  /// The rows of the tree that are open, as the app holds them: `on_change` is told the rows open after each press
  /// or key that opens or closes one.
  pub fn expanded_controlled(
    self,
    open: impl IntoIterator<Item = K>,
    on_change: impl Fn(Vec<K>) + Send + Sync + 'static,
  ) -> Self {
    let now = open.into_iter().collect();
    self.with(|table| table.set_expanded(now, Arc::new(on_change)))
  }

  /// Called with each row of the tree a press or a key opens (`true`) or closes.
  pub fn on_expanded_change(self, handler: impl Fn(&K, bool) + Send + Sync + 'static) -> Self {
    self.with(|table| table.add_on_expanded_change(handler))
  }

  /// What a row shows under it while it is open: a panel as wide as the grid's view, which stays in it while the
  /// columns scroll sideways, as high as what it holds. Where the rows are no tree, the first cell's chevron opens
  /// and closes it, and Right and Left on that cell do. Its line is measured as it is laid out, as a row that wraps
  /// is; what is in it is reached with Tab, and the arrows go from row to row past it.
  pub fn detail(self, detail: impl Fn(&R) -> Element + Send + Sync + 'static) -> Self {
    self.with(|table| table.set_detail(detail))
  }

  /// The rows whose detail is open, by their keys: those the signal has. Without it, the grid keeps its own, every
  /// detail closed at first.
  pub fn details(self, open: Signal<Vec<K>>) -> Self
  where
    Vec<K>: SignalValue,
  {
    let now = open.get();
    self.with(|table| table.set_details_open(now, Arc::new(move |next| open.set(next))))
  }

  /// The rows whose detail is open, as the app holds them: `on_change` is told them after each press or key that
  /// opens or closes one.
  pub fn details_controlled(
    self,
    open: impl IntoIterator<Item = K>,
    on_change: impl Fn(Vec<K>) + Send + Sync + 'static,
  ) -> Self {
    let now = open.into_iter().collect();
    self.with(|table| table.set_details_open(now, Arc::new(on_change)))
  }

  /// Called with each row whose detail a press or a key opens (`true`) or closes.
  pub fn on_detail_change(self, handler: impl Fn(&K, bool) + Send + Sync + 'static) -> Self {
    self.with(|table| table.add_on_detail_change(handler))
  }

  /// Groups the rows by the values of these columns, by their ids: a group's row for each value of the first,
  /// with its value and how many rows it holds, and in it a group for each value of the next, then the rows. The
  /// groups are in the order their first rows are, the rows of each in theirs. A column grouped by may be hidden.
  pub fn group_by<S: AsRef<str>>(self, columns: impl IntoIterator<Item = S>) -> Self {
    let columns = columns.into_iter().map(|id| Arc::from(id.as_ref())).collect();
    self.with(|table| table.set_group_by(columns))
  }

  /// The groups that are open: those the signal has. A press of a group's row, Enter or Space on it, Right and
  /// Left open and close it. The rows of a closed group are not built. Without it, the grid keeps its own, every
  /// group closed at first.
  pub fn open(self, open: Signal<Vec<GroupKey>>) -> Self
  where
    Vec<GroupKey>: SignalValue,
  {
    let now = open.get();
    self.with(|table| table.set_open_groups(now, Arc::new(move |next| open.set(next))))
  }

  /// The groups that are open, as the app holds them: `on_change` is told the groups open after each press or key
  /// that opens or closes one.
  pub fn open_controlled(
    self,
    open: impl IntoIterator<Item = GroupKey>,
    on_change: impl Fn(Vec<GroupKey>) + Send + Sync + 'static,
  ) -> Self {
    let now = open.into_iter().collect();
    self.with(|table| table.set_open_groups(now, Arc::new(on_change)))
  }

  /// Called with each group a press or a key opens (`true`) or closes, bound to a signal or not. A second handler
  /// is called after the first.
  pub fn on_open_change(self, handler: impl Fn(&GroupKey, bool) + Send + Sync + 'static) -> Self {
    self.with(|table| table.add_on_open_change(handler))
  }

  /// The words of each column's menu: English unless told ([`ColumnMenuLabels`]).
  pub fn column_menu_labels(self, labels: ColumnMenuLabels) -> Self {
    self.with(|table| table.set_column_menu_labels(labels))
  }

  /// The name of a column's menu trigger to what reads the tree, given the column's title: "Name column menu"
  /// unless told.
  pub fn menu_trigger_label(self, label: impl Fn(&str) -> String + Send + Sync + 'static) -> Self {
    self.with(|table| table.set_menu_trigger_label(label))
  }

  /// The name of a group's row to what reads the tree, given the title of the column grouped by, the group's value
  /// and how many rows it holds: "Status: Passing, 3 rows" unless told.
  pub fn group_label(self, label: impl Fn(&str, &str, usize) -> String + Send + Sync + 'static) -> Self {
    self.with(|table| table.set_group_label(label))
  }

  /// The words of the summary rows: what a sum, an average, a count, the least and the most say before their
  /// value, and what a summary row is called. English unless told ([`SummaryLabels`]).
  pub fn summary_labels(self, labels: SummaryLabels) -> Self {
    self.with(|table| table.set_summary_labels(labels))
  }

  /// The mark of the header of the rows' numbers: "#" unless told. What it is called is the select-all label's.
  pub fn row_numbers_label(self, label: &str) -> Self {
    self.with(|table| table.set_row_numbers_label(label))
  }

  /// Whether the grid has a column of its rows' numbers at its start, which stays there while the rows scroll
  /// sideways: a number is pressed to select its row (with Ctrl adds it, with Shift the rows from the last
  /// selected one), its header to select every row, or none where all are.
  pub fn row_numbers(self, numbers: bool) -> Self {
    self.with(|table| table.set_row_numbers(numbers))
  }

  /// Whether no cell is edited: the cells take the focus and are read, and say so to what reads the tree.
  pub fn read_only(self, read_only: bool) -> Self {
    self.with(|table| table.set_read_only(read_only))
  }

  /// What the rows are sorted by: see [`Table::sort`].
  pub fn sort(self, sort: Signal<Vec<TableSort>>) -> Self
  where
    Vec<TableSort>: SignalValue,
  {
    self.with(|table| table.sort(sort))
  }

  /// What the rows are sorted by, as the app holds it: see [`Table::sort_controlled`].
  pub fn sort_controlled(
    self,
    sort: Vec<TableSort>,
    on_change: impl Fn(Vec<TableSort>) + Send + Sync + 'static,
  ) -> Self {
    self.with(|table| table.sort_controlled(sort, on_change))
  }

  /// Called with the new sort at each press of a sortable header: see [`Table::on_sort_change`].
  pub fn on_sort_change(self, handler: impl Fn(&[TableSort]) + Send + Sync + 'static) -> Self {
    self.with(|table| table.on_sort_change(handler))
  }

  /// The rows that are selected: see [`Table::selected`].
  pub fn selected(self, selected: Signal<Vec<K>>) -> Self
  where
    Vec<K>: SignalValue,
  {
    self.with(|table| table.selected(selected))
  }

  /// The rows that are selected, as the app holds them: see [`Table::selected_controlled`].
  pub fn selected_controlled(
    self,
    selected: impl IntoIterator<Item = K>,
    on_change: impl Fn(Vec<K>) + Send + Sync + 'static,
  ) -> Self {
    self.with(|table| table.selected_controlled(selected, on_change))
  }

  /// Called with the whole new selection: see [`Table::on_selection_change`].
  pub fn on_selection_change(self, handler: impl Fn(&[K]) + Send + Sync + 'static) -> Self {
    self.with(|table| table.on_selection_change(handler))
  }

  /// Whether a grid whose rows are selected has the column of checkboxes: see [`Table::checkboxes`].
  pub fn checkboxes(self, checkboxes: bool) -> Self {
    self.with(|table| table.checkboxes(checkboxes))
  }

  /// Called with the row a press of it is on: see [`Table::on_row_click`].
  pub fn on_row_click(self, handler: impl Fn(&K) + Send + Sync + 'static) -> Self {
    self.with(|table| table.on_row_click(handler))
  }

  /// Called with the row Enter on a cell that is not edited, or a double press of one, activates.
  pub fn on_row_activate(self, handler: impl Fn(&K) + Send + Sync + 'static) -> Self {
    self.with(|table| table.on_row_activate(handler))
  }

  /// Whether a person makes the columns wider or narrower, each that says nothing of its own: on unless told.
  pub fn resizable(self, resizable: bool) -> Self {
    self.with(|table| table.resizable(resizable))
  }

  /// Called with a column's id and width while a person resizes it: see [`Table::on_column_resize`].
  pub fn on_column_resize(self, handler: impl Fn(&str, f32) + Send + Sync + 'static) -> Self {
    self.with(|table| table.on_column_resize(handler))
  }

  /// Called with a column's id and width when a person has resized it: see [`Table::on_column_resize_end`].
  pub fn on_column_resize_end(self, handler: impl Fn(&str, f32) + Send + Sync + 'static) -> Self {
    self.with(|table| table.on_column_resize_end(handler))
  }

  /// The name of a column's resizer, given the column's title: see [`Table::resize_label`].
  pub fn resize_label(self, label: impl Fn(&str) -> String + Send + Sync + 'static) -> Self {
    self.with(|table| table.resize_label(label))
  }

  /// How much room a row takes, whatever the theme is at.
  pub fn density(self, density: impl Into<Density>) -> Self {
    self.with(|table| table.density(density))
  }

  /// The grid's height: the header stays and the rows scroll under it.
  pub fn height(self, height: impl Into<Length>) -> Self {
    self.with(|table| table.height(height))
  }

  /// The most the grid is high: as high as its rows up to it.
  pub fn max_height(self, height: impl Into<Length>) -> Self {
    self.with(|table| table.max_height(height))
  }

  /// The title of the empty state under the header where there are no rows: "No rows" unless told.
  pub fn empty_text(self, text: &str) -> Self {
    self.with(|table| table.empty_text(text))
  }

  /// What stands under the header where there are no rows, in place of the empty state.
  pub fn empty(self, empty: impl Into<Element>) -> Self {
    self.with(|table| table.empty(empty))
  }

  /// Whether the rows are on their way: rows of skeletons stand where they will be.
  pub fn loading(self, loading: bool) -> Self {
    self.with(|table| table.loading(loading))
  }

  /// How many rows of skeletons stand while the grid is loading: 4 unless it is said.
  pub fn loading_rows(self, rows: usize) -> Self {
    self.with(|table| table.loading_rows(rows))
  }

  /// A footer under the rows with their count and this pagination at its end.
  pub fn pagination(self, pagination: impl Into<Element>) -> Self {
    self.with(|table| table.pagination(pagination))
  }

  /// Whether the grid has a footer that counts its rows.
  pub fn count(self, count: bool) -> Self {
    self.with(|table| table.count(count))
  }

  /// How many rows there are in all, where the grid shows some of them.
  pub fn total(self, total: usize) -> Self {
    self.with(|table| table.total(total))
  }

  /// What the footer's count says: see [`Table::count_text`].
  pub fn count_text(self, text: impl Fn(TableCount) -> String + Send + Sync + 'static) -> Self {
    self.with(|table| table.count_text(text))
  }

  /// What the rows are filtered by: see [`Table::filters`]. A person sets the filter of a column that filters
  /// from its menu, and, with [`filter_row`](Self::filter_row), in the row under the header.
  pub fn filters(self, filters: Signal<TableFilters>) -> Self
  where
    TableFilters: SignalValue,
  {
    self.with(|table| table.filters(filters))
  }

  /// What the rows are filtered by, as the app holds it: see [`Table::filters_controlled`].
  pub fn filters_controlled(
    self,
    filters: TableFilters,
    on_change: impl Fn(TableFilters) + Send + Sync + 'static,
  ) -> Self {
    self.with(|table| table.filters_controlled(filters, on_change))
  }

  /// Called with the next filters each time a person changes them: see [`Table::on_filters_change`].
  pub fn on_filters_change(self, handler: impl Fn(&TableFilters) + Send + Sync + 'static) -> Self {
    self.with(|table| table.on_filters_change(handler))
  }

  /// What is searched for in the cells, every match marked: see [`Table::search`].
  pub fn search(self, query: Signal<String>) -> Self
  where
    String: SignalValue,
  {
    self.with(|table| table.search(query))
  }

  /// What is searched for, as the app holds it: see [`Table::search_controlled`].
  pub fn search_controlled(self, query: &str, on_change: impl Fn(String) + Send + Sync + 'static) -> Self {
    self.with(|table| table.search_controlled(query, on_change))
  }

  /// Called with what is searched for as the find bar's query changes: see [`Table::on_search_change`].
  pub fn on_search_change(self, handler: impl Fn(&str) + Send + Sync + 'static) -> Self {
    self.with(|table| table.on_search_change(handler))
  }

  /// How the query matches: see [`Table::search_options`].
  pub fn search_options(self, options: super::FindOptions) -> Self {
    self.with(|table| table.search_options(options))
  }

  /// Called with how many matches there are and which is current: see [`Table::on_matches_change`].
  pub fn on_matches_change(self, handler: impl Fn(super::SearchMatches) + Send + Sync + 'static) -> Self {
    self.with(|table| table.on_matches_change(handler))
  }

  /// Whether each column's menu has Export, which writes the rows shown or selected out as comma-separated or
  /// tab-separated text, the titles of the columns shown first: on unless told.
  pub fn export(self, export: bool) -> Self {
    self.with(|table| table.export(export))
  }

  /// Told the text of each export from the grid's menu ([`GridExport`](super::GridExport)): its form, which rows,
  /// the text and how many rows it holds. The app saves it, or sends it; with no handler the text goes on the
  /// clipboard.
  pub fn on_export(self, handler: impl Fn(&super::GridExport) + Send + Sync + 'static) -> Self {
    self.with(|table| table.on_export(handler))
  }

  /// The words of the export in the grid's menu.
  pub fn export_labels(self, labels: super::ExportLabels) -> Self {
    self.with(|table| table.export_labels(labels))
  }

  /// The bar of the grid's tools over it, as the board's Table Toolbar draws it, built from a
  /// [`Toolbar`](super::Toolbar): a search field that is the grid's search (the app's, or the grid's own), a tag
  /// for each filter set whose × takes it off, Filter (a menu of the columns that filter, each opening its filter),
  /// and at the end the density, which the grid takes once a person chooses one, the columns to show, and Export.
  /// What does not fit goes into the bar's menu, the search field last. Off unless told.
  pub fn toolbar(self, toolbar: bool) -> Self {
    self.with(|table| table.set_toolbar(toolbar))
  }

  /// The words of the grid's bar.
  pub fn toolbar_labels(self, labels: super::GridBarLabels) -> Self {
    self.with(|table| table.set_toolbar_labels(labels))
  }

  /// Whether Ctrl+F opens a find bar over the grid: see [`Table::find_bar`].
  pub fn find_bar(self, find_bar: bool) -> Self {
    self.with(|table| table.find_bar(find_bar))
  }

  /// What the grid's find bar is made of besides its query and matches: see [`Table::find_bar_with`].
  pub fn find_bar_with(self, with: impl Fn(super::FindBar) -> super::FindBar + Send + Sync + 'static) -> Self {
    self.with(|table| table.find_bar_with(with))
  }

  /// Called with what the rows are asked by each time a person changes it: see [`Table::on_query_change`].
  pub fn on_query_change(self, handler: impl Fn(&GridQuery) + Send + Sync + 'static) -> Self {
    self.with(|table| table.on_query_change(handler))
  }

  /// The words of rows the app gives on demand: see [`Table::source_labels`].
  pub fn source_labels(self, labels: SourceLabels) -> Self {
    self.with(|table| table.source_labels(labels))
  }

  /// Whether the grid has a row of filters under its header: see [`Table::filter_row`].
  pub fn filter_row(self, filter_row: bool) -> Self {
    self.with(|table| table.filter_row(filter_row))
  }

  /// Every word of filtering, in place of the English: see [`Table::filter_labels`].
  pub fn filter_labels(self, labels: FilterLabels) -> Self {
    self.with(|table| table.filter_labels(labels))
  }

  /// The name of the header's checkbox: "Select all" unless told.
  pub fn select_all_label(self, label: &str) -> Self {
    self.with(|table| table.select_all_label(label))
  }

  /// The name of a row's checkbox: "Select row" unless told.
  pub fn select_row_label(self, label: &str) -> Self {
    self.with(|table| table.select_row_label(label))
  }

  /// What the grid is called, to what reads the tree. It draws nothing.
  pub fn label(self, label: &str) -> Self {
    self.with(|table| table.label(label))
  }

  /// Whether nothing in the grid takes input.
  pub fn disabled(self, disabled: bool) -> Self {
    self.with(|table| table.disabled(disabled))
  }

  /// Puts the grid in a state the theme's recipe for a table names, or takes it out.
  pub fn state(self, state: impl Into<Name>, on: bool) -> Self {
    self.with(|table| table.state(state, on))
  }

  /// Styles one part of the grid over what the theme gives a table: its parts are a table's
  /// ([`TablePart`](super::TablePart)).
  pub fn style(self, part: impl Into<Name>, style: impl FnOnce(PartStyle) -> PartStyle) -> Self {
    self.with(|table| table.style(part, style))
  }

  /// Lays rules over the theme's recipe for this grid.
  pub fn rules(self, rules: &Recipe) -> Self {
    self.with(|table| table.rules(rules))
  }

  /// Draws every row as it is under the pointer, held down or focused, with nothing pointing at it.
  pub fn force(self, interaction: Option<Interaction>) -> Self {
    self.with(|table| table.force(interaction))
  }

  /// The id of the box; a row's is the grid's with `-row-` and its place.
  pub fn id(self, id: impl Into<Arc<str>>) -> Self {
    self.with(|table| table.id(id))
  }

  pub fn key(self, key: impl Into<Arc<str>>) -> Self {
    self.with(|table| table.key(key))
  }

  /// Builds the still form of the grid in `theme`: every row and every cell, read and selected as a table's.
  pub fn build(self, theme: &crate::app::theme::Theme) -> Element {
    self.table.build(theme)
  }

  /// The grid mounted where it is declared: its cells take the focus and are edited, and it builds the rows and
  /// the columns in view and a few around them.
  pub fn mount(self, ctx: &mut Ctx) -> Element {
    self.table.mount(ctx)
  }
}

impl<R: Send + Sync + 'static, K: Clone + Eq + Hash + Send + Sync + 'static> crate::ui::Styled for DataGrid<R, K> {
  type Component = super::Table<R, K>;

  fn rules(self, rules: &crate::ui::Recipe) -> Self {
    Self::rules(self, rules)
  }

  fn state(self, state: impl Into<crate::ui::Name>, on: bool) -> Self {
    Self::state(self, state, on)
  }
}

impl<R: Send + Sync + 'static, K: Clone + Eq + Hash + Send + Sync + 'static> From<DataGrid<R, K>> for Element {
  fn from(grid: DataGrid<R, K>) -> Self {
    grid.table.into()
  }
}
