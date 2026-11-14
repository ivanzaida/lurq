//! The page of the data grid: its playground, the board's Data Grid, editing in place, columns pinned at both
//! ends and resized, ten thousand rows of fifty columns, a grid that is read only, and its props.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::Density},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{
    Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, CellEditor, CellValue, ColumnPin, DataGrid, Pagination,
    TableColumn, TableSort, icons,
  },
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{props_table, quiet},
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
fn sorted(mut rows: Vec<File>, sort: &Option<TableSort>) -> Vec<File> {
  if let Some(sort) = sort {
    sort.sort(&columns(false), &mut rows);
  }
  rows
}

const SELECTIONS: [&str; 3] = ["none", "checkboxes", "by a press"];
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
  played: Signal<Vec<File>>,
  played_selected: Signal<Vec<String>>,
  played_sort: Signal<Option<TableSort>>,
  board_selected: Signal<Vec<String>>,
  board_sort: Signal<Option<TableSort>>,
  page: Signal<usize>,
  edited: Signal<Vec<File>>,
  log: Signal<Vec<String>>,
  pinned: Signal<Vec<File>>,
  many: Arc<Vec<File>>,
}

impl DataGridPage {
  /// The playground: a grid of files as the panel's props make it, its edits taken into its rows.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let selection = self.selection.get().min(SELECTIONS.len() - 1);
    let (density_name, density) = DENSITIES[self.density.get().min(DENSITIES.len() - 1)];
    let (fixed_height, pin_name, read_only, resizable, footer) = (
      self.fixed_height.get(),
      self.pin_name.get(),
      self.read_only.get(),
      self.resizable.get(),
      self.footer.get(),
    );
    let record = self.playground.recorder();
    let rows = self.played.clone();
    let sort = self.played_sort.get();

    let (edited, selected_to, sorted_to) = (record.clone(), record.clone(), record);
    let mut grid = DataGrid::new(columns(pin_name), sorted(rows.get(), &sort))
      .row_key(|row: &File| row.name.clone())
      .label("Files")
      .density(density)
      .read_only(read_only)
      .resizable(resizable)
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
    code.push(".on_edit(|edit| /* take it into files, or refuse it */ true)".to_owned());
    code.push(".mount(ctx)".to_owned());

    let tree = vec![
      format!("root        {} · a table's · grid", self.playground.measured()),
      "├─ header   the table's · a resizer at each column's end".to_owned(),
      "└─ row      not a stop · Left and Right its own".to_owned(),
      "   └─ cell     a gridcell · the ring inside it from the keyboard".to_owned(),
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
    );
    let still = {
      let rows = rows.get();
      move || -> Element { DataGrid::new(columns(pin_name), rows.clone()).label("Files").into() }
    };
    let shown = grid.mount(ctx);
    self.playground.card(
      ctx,
      Spec {
        title: "DataGrid",
        make: Arc::new(still),
        shown: Shown::of(shown),
        fill: true,
        forceable: true,
        facts: format!(
          "{} rows · {} selected",
          rows.get().len(),
          self.played_selected.get().len()
        ),
        event: "on_edit / on_selection_change",
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
        ],
        changed: [
          selection != 1,
          density != Density::Default,
          read_only,
          !resizable,
          !fixed_height,
          pin_name,
          footer,
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
          reset.9.set(None);
        }),
      },
    )
  }
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
      playground: Playground::new(ctx, 2),
      selection: ctx.signal(1),
      density: ctx.signal(1),
      fixed_height: ctx.signal(true),
      pin_name: ctx.signal(false),
      read_only: ctx.signal(false),
      resizable: ctx.signal(true),
      footer: ctx.signal(false),
      played: ctx.signal(files(8)),
      played_selected: ctx.signal(Vec::new()),
      played_sort: ctx.signal(None),
      board_selected: ctx.signal(vec!["blur.rs".to_owned()]),
      board_sort: ctx.signal(Some(TableSort::ascending("name"))),
      page: ctx.signal(1),
      edited: ctx.signal(files(5)),
      log: ctx.signal(Vec::new()),
      pinned: ctx.signal(files(8)),
      many: Arc::new(
        (0..10_000)
          .map(|index| File {
            name: format!("file_{index:05}.rs"),
            ..files(8)[index % 8].clone()
          })
          .collect(),
      ),
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

    // Ten thousand rows of fifty columns: the cells in view are built.
    let mut wide_columns = vec![
      TableColumn::new("name", "Name")
        .width(160.0)
        .mono(true)
        .pin(ColumnPin::Start)
        .text(|row: &File| row.name.clone()),
    ];
    wide_columns.extend((1..50).map(|index| {
      TableColumn::new(&format!("m{index}"), &format!("Metric {index}"))
        .width(110.0)
        .numeric()
        .text(move |row: &File| format!("{:.1}", row.size * index as f64 / 7.0))
    }));
    let thousands = DataGrid::new_shared(wide_columns, self.many.clone())
      .label("Metrics")
      .read_only(true)
      .count(true)
      .height(360.0)
      .key("thousands")
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
        "resizable",
        "bool",
        "true",
        "Columns made wider or narrower by their edge or Ctrl and an arrow on their header.",
      ),
      (
        "selected / sort",
        "Signal<Vec<K>> / Signal<Option<TableSort>>",
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
          "Ten thousand rows, fifty columns",
          "A mounted grid builds the rows in view and a few around them, and of those the cells of the columns in \
           view, two past each side, the pinned ones and the focused one. Ctrl+End goes to the last cell.",
          self.frame.demo(
            3,
            thousands,
            "DataGrid::new_shared(columns, rows)",
            &[
              "// rows: Arc<Vec<File>>, ten thousand; columns: a name pinned and 49 metrics",
              "DataGrid::new_shared(columns, rows)",
              "    .read_only(true)",
              "    .count(true)",
              "    .height(360.0)",
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
