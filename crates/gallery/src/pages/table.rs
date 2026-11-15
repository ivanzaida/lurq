//! The page of the table: its playground, the board's table, rows selected with a footer, sorting, a header
//! that stays in a height, ten thousand rows, the empty and the loading forms, the bar over a table, and its
//! states.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::Density},
  components::{Column, Row},
  core::{ElementRef, Signal},
  layout::Alignment,
  node::Element,
  ui::{
    Avatar, AvatarSize, Badge, BadgeVariant, Button, ButtonPart, ButtonSize, ButtonVariant, Choice, ColumnPin,
    EmptyState, EmptyStateSize, InputSize, Interaction, Length, Menu, MenuItem, Pagination, SearchField,
    SearchFieldPart, SortDirection, Table, TableColumn, TablePart, TableSort, Tag, TagPart, TagVariant, ToggleGroup,
    ToggleGroupPart, Toolbar, ToolbarItem, ToolbarPart, icons,
  },
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{props_table, quiet},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Status {
  Passing,
  Building,
  Modified,
  Failed,
}

impl Status {
  fn badge(self) -> Element {
    let (label, variant) = match self {
      Self::Passing => ("Passing", BadgeVariant::Success),
      Self::Building => ("Building", BadgeVariant::Info),
      Self::Modified => ("Modified", BadgeVariant::Warning),
      Self::Failed => ("Failed", BadgeVariant::Destructive),
    };
    Badge::new(label).variant(variant).into()
  }
}

/// A crate of a workspace, as a table of them shows it.
#[derive(Clone, Debug, PartialEq)]
struct Krate {
  name: &'static str,
  status: Status,
  owner: &'static str,
  size: u32,
  updated: &'static str,
}

const NAMES: [&str; 12] = [
  "crates/lurq",
  "crates/lurq_macros",
  "crates/mobile_demo",
  "crates/gallery",
  "crates/lurq_devtools",
  "crates/lurq_mcp",
  "crates/render_bench",
  "crates/icon_font",
  "crates/lurq_router",
  "crates/lurq_form",
  "crates/svg_loader",
  "crates/text_shaping",
];
const OWNERS: [&str; 4] = ["Ivan Zaida", "Mira Holt", "Ada Kerr", "Tom Rios"];
const UPDATED: [&str; 6] = [
  "2 min ago",
  "just now",
  "1 h ago",
  "yesterday",
  "3 days ago",
  "last week",
];
const STATUSES: [Status; 4] = [Status::Passing, Status::Building, Status::Failed, Status::Modified];

/// The `index`th crate: the board's three first, then as many as asked for.
fn krate(index: usize) -> Krate {
  Krate {
    name: NAMES[index % NAMES.len()],
    status: STATUSES[index % STATUSES.len()],
    owner: OWNERS[index % OWNERS.len()],
    size: ((index * 37 + 11) % 120) as u32 + 4,
    updated: UPDATED[index % UPDATED.len()],
  }
}

fn crates(count: usize) -> Vec<Krate> {
  (0..count).map(krate).collect()
}

/// The rows of the table of ten thousand: a name of its own each.
fn many() -> Arc<Vec<Krate>> {
  Arc::new(
    (0..10_000)
      .map(|index| Krate {
        name: Box::leak(format!("crates/generated_{index:05}").into_boxed_str()),
        ..krate(index)
      })
      .collect(),
  )
}

/// The columns of the playground's table: a name that may stay, its status, its owner, its size and when it
/// changed, and a button at the end that is told to `record`.
fn columns(sticky: bool, record: Option<Arc<dyn Fn(String) + Send + Sync>>) -> Vec<TableColumn<Krate>> {
  let mut columns = vec![
    TableColumn::new("name", "Name")
      .text(|row: &Krate| row.name.to_owned())
      .mono(true)
      .min_width(170.0)
      .pin(if sticky { ColumnPin::Start } else { ColumnPin::None })
      .sortable(true),
    TableColumn::new("status", "Status")
      .width(110.0)
      .cell(|row: &Krate| row.status.badge())
      .sort_key(|row: &Krate| row.status),
    TableColumn::new("owner", "Owner").fit().cell(|row: &Krate| {
      Row::new()
        .spacing(8.0)
        .align_items(Alignment::Center)
        .child(Avatar::new(row.owner).size(AvatarSize::Xs))
        .child(quiet(row.owner, 13.0))
        .into()
    }),
    TableColumn::new("size", "Size")
      .width(70.0)
      .numeric()
      .text(|row: &Krate| format!("{} KB", row.size))
      .sort_key(|row: &Krate| row.size),
    TableColumn::new("updated", "Updated")
      .width(100.0)
      .muted(true)
      .text(|row: &Krate| row.updated.to_owned()),
  ];
  if let Some(record) = record {
    columns.push(TableColumn::actions("actions", move |row: &Krate| {
      let (record, name) = (record.clone(), row.name);
      Button::new("Open")
        .variant(ButtonVariant::Ghost)
        .size(ButtonSize::Xs)
        .on_click(move |_| record(format!("Open {name:?}")))
        .into()
    }));
  }
  columns
}

/// The board's table: a name in the mono type, a status, when it changed.
fn board_columns() -> Vec<TableColumn<Krate>> {
  vec![
    TableColumn::new("name", "Name")
      .text(|row: &Krate| row.name.to_owned())
      .mono(true),
    TableColumn::new("status", "Status")
      .width(120.0)
      .cell(|row: &Krate| row.status.badge()),
    TableColumn::new("updated", "Updated")
      .width(120.0)
      .muted(true)
      .text(|row: &Krate| row.updated.to_owned()),
  ]
}

/// A filter that is set, as the bar draws it: 26 high, its text 12, with its remove button.
fn tag(text: &str) -> Tag {
  Tag::new(text)
    .variant(TagVariant::Info)
    .on_remove(|| {})
    .style(TagPart::Root, |s| s.height(26.0))
    .style(TagPart::Label, |s| s.font_size(12.0))
}

/// The board's columns of its tables of 420 (Table/Empty, Table/Loading): the name in the room left, the status
/// and when it changed 90 each.
fn narrow_columns() -> Vec<TableColumn<Krate>> {
  vec![
    TableColumn::new("name", "Name")
      .text(|row: &Krate| row.name.to_owned())
      .mono(true),
    TableColumn::new("status", "Status")
      .width(90.0)
      .cell(|row: &Krate| row.status.badge()),
    TableColumn::new("updated", "Updated")
      .width(90.0)
      .muted(true)
      .text(|row: &Krate| row.updated.to_owned()),
  ]
}

/// The rows of `rows` in the order `sort` asks for.
fn sorted(columns: &[TableColumn<Krate>], mut rows: Vec<Krate>, sort: &[TableSort]) -> Vec<Krate> {
  TableSort::sort_rows(sort, columns, &mut rows);
  rows
}

const SELECTIONS: [&str; 3] = ["none", "with checkboxes", "by a press of a row"];
const DENSITIES: [(&str, Density); 3] = [
  ("compact", Density::Compact),
  ("default", Density::Default),
  ("comfortable", Density::Comfortable),
];
const FORMS: [&str; 3] = ["rows", "empty", "loading"];

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(9.0)
    .child(Row::new().overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

pub struct TablePage {
  frame: Frame,
  playground: Playground,
  sortable: Signal<bool>,
  selection: Signal<usize>,
  density: Signal<usize>,
  fixed_height: Signal<bool>,
  form: Signal<usize>,
  sticky: Signal<bool>,
  footer: Signal<bool>,
  played_sort: Signal<Vec<TableSort>>,
  played_selected: Signal<Vec<&'static str>>,
  grid_selected: Signal<Vec<&'static str>>,
  grid_sort: Signal<Vec<TableSort>>,
  page: Signal<usize>,
  sort: Signal<Vec<TableSort>>,
  tall_selected: Signal<Vec<&'static str>>,
  board_selected: Signal<Vec<&'static str>>,
  /// The Columns menu of the bar: its anchor, whether it is open, and whether the status and the time show.
  columns_menu: (ElementRef, Signal<bool>),
  shown_columns: Signal<(bool, bool)>,
  many: Arc<Vec<Krate>>,
  many_selected: Signal<Vec<&'static str>>,
  search: Signal<String>,
  bar_density: Signal<u8>,
}

impl TablePage {
  /// The playground: a table of crates as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (sortable, selection, sticky, footer) = (
      self.sortable.get(),
      self.selection.get().min(SELECTIONS.len() - 1),
      self.sticky.get(),
      self.footer.get(),
    );
    let (density_name, density) = DENSITIES[self.density.get().min(DENSITIES.len() - 1)];
    let (fixed_height, form) = (self.fixed_height.get(), self.form.get().min(FORMS.len() - 1));
    // With no height of its own the table is as high as its rows: five of the loosest, with the footer, keep
    // within the stage's 382.
    let rows = if fixed_height { 8 } else { 5 };
    let (sort, selected) = (self.played_sort.get(), self.played_selected.get());
    let record = self.playground.recorder();
    let recorded: Arc<dyn Fn(String) + Send + Sync> = Arc::new(record.clone());

    let build = {
      let sort = sort.clone();
      let selected = selected.clone();
      move |record: Option<Arc<dyn Fn(String) + Send + Sync>>| {
        let mut columns = columns(sticky, record);
        if !sortable {
          columns = columns.into_iter().map(|column| column.sortable(false)).collect();
        }
        let rows = match form {
          1 => Vec::new(),
          _ => sorted(&columns, crates(rows), &sort),
        };
        let mut table = Table::new(columns, rows)
          .row_key(|row: &Krate| row.name)
          .label("Crates")
          .density(density)
          .loading(form == 2);
        if fixed_height {
          table = table.height(240.0);
        }
        if footer {
          table = table.count(true).total(24);
        }
        (table, sort.clone(), selected.clone())
      }
    };

    let (set_sort, set_selected) = (self.played_sort.clone(), self.played_selected.clone());
    let (sorted_to, changed, clicked, activated) = (record.clone(), record.clone(), record.clone(), record);
    let (table, sort_now, selected_now) = build(Some(recorded));
    let mut shown = table
      .force(self.playground.forced())
      .sort_controlled(sort_now.clone(), move |next| {
        sorted_to(format!("on_sort_change({next:?})"));
        set_sort.set(next);
      })
      .on_row_click(move |key| clicked(format!("on_row_click({key:?})")))
      .on_row_activate(move |key| activated(format!("on_row_activate({key:?})")));
    if selection > 0 {
      shown = shown
        .selected_controlled(selected_now.clone(), move |next| {
          changed(format!("on_selection_change({next:?})"));
          set_selected.set(next);
        })
        .checkboxes(selection == 1);
    }

    let make = {
      let build = build.clone();
      move || {
        let (table, sort, selected) = build(None);
        let mut table = table.sort_controlled(sort, |_| {});
        if selection > 0 {
          table = table.selected_controlled(selected, |_| {}).checkboxes(selection == 1);
        }
        table.into()
      }
    };

    let mut code = vec![
      "// sort: Signal<Vec<TableSort>>".to_owned(),
      if selection > 0 {
        "// selected: Signal<Vec<&str>>".to_owned()
      } else {
        "// rows: Vec<Crate>, in the order of `sort`".to_owned()
      },
      "Table::new([".to_owned(),
      format!(
        "    TableColumn::new(\"name\", \"Name\").text(|row| row.name.into()).mono(true).min_width(170.0){}{},",
        if sticky { ".pin(ColumnPin::Start)" } else { "" },
        if sortable { ".sortable(true)" } else { "" }
      ),
      format!(
        "    TableColumn::new(\"status\", \"Status\").width(110.0).cell(|row| row.status.badge()){},",
        if sortable { ".sort_key(|row| row.status)" } else { "" }
      ),
      "    TableColumn::new(\"owner\", \"Owner\").fit().cell(|row| /* avatar and name */),".to_owned(),
      format!(
        "    TableColumn::new(\"size\", \"Size\").width(70.0).numeric().text(|row| format!(\"{{}} KB\", row.size)){},",
        if sortable { ".sort_key(|row| row.size)" } else { "" }
      ),
      "    TableColumn::new(\"updated\", \"Updated\").width(100.0).muted(true).text(|row| row.updated.into()),"
        .to_owned(),
      "    TableColumn::actions(\"actions\", |row| Button::new(\"Open\").variant(ButtonVariant::Ghost).into()),"
        .to_owned(),
      format!("], {})", if form == 1 { "Vec::new()" } else { "rows" }),
      ".row_key(|row| row.name)".to_owned(),
      ".label(\"Crates\")".to_owned(),
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
    if fixed_height {
      code.push(".height(240.0)".to_owned());
    }
    if form == 2 {
      code.push(".loading(true)".to_owned());
    }
    if footer {
      code.push(".count(true).total(24)".to_owned());
    }
    code.push(".on_row_activate(|name| /* open the crate */)".to_owned());

    let row_height = match density {
      Density::Compact => 36,
      Density::Default => 44,
      _ => 52,
    };
    let tree = vec![
      format!("root        {} · radius lg · edge 1", self.playground.measured()),
      "├─ scroll   both ways · fills its width".to_owned(),
      format!("│  ├─ header      {} high · muted · sticks to the top", row_height - 4),
      "│  │  └─ header_cell  13/500 · sort_icon 14 · 4 apart".to_owned(),
      format!(
        "│  └─ row         {row_height} high · padding 16 · {} between columns",
        if selection == 1 { 12 } else { 16 }
      ),
      format!("│     ├─ select    16{}", if selection == 1 { "" } else { "   (none)" }),
      "│     └─ cell      13 · mono, muted, numeric by column".to_owned(),
      format!("└─ footer    44 · count{}", if footer { "" } else { "   (none)" }),
    ];

    let (to_sortable, to_selection, to_density, to_height) = (
      self.sortable.clone(),
      self.selection.clone(),
      self.density.clone(),
      self.fixed_height.clone(),
    );
    let (to_form, to_sticky, to_footer, to_sort, to_selected) = (
      self.form.clone(),
      self.sticky.clone(),
      self.footer.clone(),
      self.played_sort.clone(),
      self.played_selected.clone(),
    );
    let (set_selection, set_density, set_form) = (self.selection.clone(), self.density.clone(), self.form.clone());
    let indexed = |names: &[&'static str]| -> Vec<(&'static str, usize)> {
      names.iter().enumerate().map(|(index, name)| (*name, index)).collect()
    };
    let densities: Vec<(&str, usize)> = DENSITIES
      .iter()
      .enumerate()
      .map(|(index, (name, _))| (*name, index))
      .collect();
    let shown = shown.mount(ctx);
    self.playground.card(
      ctx,
      Spec {
        title: "Table",
        make: Arc::new(make),
        shown: Shown::of(shown),
        fill: true,
        forceable: true,
        facts: format!(
          "{} selected{}",
          selected.len(),
          sort
            .iter()
            .map(|sort| format!(
              " · {} {}",
              sort.column,
              if sort.direction == SortDirection::Ascending {
                "↑"
              } else {
                "↓"
              }
            ))
            .collect::<String>()
        ),
        event: "on_sort_change / on_selection_change",
        code,
        tree,
        controls: vec![
          flag("sortable", &self.sortable),
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
          flag("height(240): the header stays", &self.fixed_height),
          flag("sticky first column", &self.sticky),
          flag("count in a footer", &self.footer),
          prop(
            "rows",
            "empty / loading",
            self
              .playground
              .select(2, &indexed(&FORMS), &form, move |index| set_form.set(index)),
          ),
        ],
        changed: [
          !sortable,
          selection != 1,
          density != Density::Default,
          !fixed_height,
          sticky,
          footer,
          form != 0,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to_sortable.set(true);
          to_selection.set(1);
          to_density.set(1);
          to_height.set(true);
          to_form.set(0);
          to_sticky.set(false);
          to_footer.set(false);
          to_sort.set(Vec::new());
          to_selected.set(Vec::new());
        }),
      },
    )
  }

  /// The Columns button of the bar and its menu, which shows or hides the status and the time.
  fn columns_button(&self) -> Element {
    let (anchor, open) = self.columns_menu.clone();
    let (status, updated) = self.shown_columns.get();
    let (to_status, to_updated) = (self.shown_columns.clone(), self.shown_columns.clone());
    let menu = Menu::new(open.clone(), anchor.clone())
      .item(
        MenuItem::new("Status")
          .checked(status)
          .on_select(move || to_status.update(|shown| shown.0 = !shown.0)),
      )
      .item(
        MenuItem::new("Updated")
          .checked(updated)
          .on_select(move || to_updated.update(|shown| shown.1 = !shown.1)),
      );
    Row::new()
      .overflow_visible()
      .ref_element(anchor)
      .child(
        Button::new("Columns")
          .trailing_icon(icons::CHEVRON_DOWN)
          .variant(ButtonVariant::Outline)
          // The board's is 30 high, padded 14 before its label and 10 after its chevron.
          .style(ButtonPart::Root, |s| s.height(30.0).padding_start(14.0).padding_end(10.0))
          .on_click(move |_| open.update(|open| *open = !*open)),
      )
      .child(menu)
      .into()
  }

  /// The board's Table Toolbar over a table whose density it sets: a search field, the filters that are set,
  /// Filter, and at the end the density, the columns and Export.
  fn with_bar(&self, ctx: &mut Ctx) -> Element {
    let density = match self.bar_density.get() {
      4 => Density::Compact,
      2 => Density::Comfortable,
      _ => Density::Default,
    };
    let bar = Toolbar::new()
      .label("Crates")
      .overflow(true)
      .style(ToolbarPart::Root, |s| {
        s.width(Length::Fill)
          .height(52.0)
          .padding_x(12.0)
          .gap(8.0)
          .radius(lurq::app::theme::RadiusSize::Lg)
      })
      .item(
        ToolbarItem::element(
          SearchField::new(self.search.clone())
            .placeholder("Filter crates")
            .size(InputSize::Sm)
            .style(SearchFieldPart::Root, |s| s.width(220.0)),
        )
        .stays(true),
      )
      .item(ToolbarItem::element(tag("Status: Passing")))
      .item(ToolbarItem::element(tag("Kind: Rust")))
      .item(
        ToolbarItem::from(
          Button::new("Filter")
            .icon(icons::PLUS)
            .variant(ButtonVariant::Outline)
            .size(ButtonSize::Xs)
            // The board's is 26 high, its icon and its text 12, padded 8.
            .style(ButtonPart::Root, |s| s.height(26.0).padding_x(8.0).gap(4.0))
            .style(ButtonPart::Icon, |s| s.square(12.0))
            .style(ButtonPart::Label, |s| s.font_size(12.0)),
        )
        .as_given(true),
      )
      .item(Toolbar::spacer())
      .item(
        ToggleGroup::new(
          self.bar_density.clone(),
          [
            Choice::new(2u8, "Comfortable").icon(icons::ROWS_2).icon_only(true),
            Choice::new(3u8, "Default").icon(icons::ROWS_3).icon_only(true),
            Choice::new(4u8, "Compact").icon(icons::ROWS_4).icon_only(true),
          ],
        )
        // The board's cells are 30 with icons of 14.
        .style(ToggleGroupPart::Item, |s| s.square(30.0)),
      )
      .item(ToolbarItem::element(self.columns_button()))
      .item(
        ToolbarItem::from(
          Button::new("Export")
            .icon(icons::DOWNLOAD)
            .variant(ButtonVariant::Outline)
            .style(ButtonPart::Root, |s| s.height(30.0).padding_x(10.0)),
        )
        .as_given(true),
      )
      .mount(ctx);
    let query = self.search.get().to_lowercase();
    let rows: Vec<Krate> = crates(6)
      .into_iter()
      .filter(|row| row.name.contains(query.as_str()))
      .collect();
    let search = self.search.clone();
    let (status, updated) = self.shown_columns.get();
    let shown: Vec<TableColumn<Krate>> = board_columns()
      .into_iter()
      .filter(|column| match column.id() {
        "status" => status,
        "updated" => updated,
        _ => true,
      })
      .collect();
    let table = Table::new(shown, rows)
      .row_key(|row: &Krate| row.name)
      .density(density)
      .empty(
        EmptyState::new(&format!("No crates match “{query}”"))
          .description("Check the spelling or clear the filter.")
          .icon(icons::SEARCH_CLOSE)
          .size(EmptyStateSize::Sm)
          .action(
            Button::new("Clear filter")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm)
              .on_click(move |_| search.set(String::new())),
          ),
      );
    Column::new().spacing(12.0).child(bar).child(table).into()
  }
}

fn capitalized(name: &str) -> String {
  let mut chars = name.chars();
  chars
    .next()
    .map(|first| first.to_uppercase().chain(chars).collect())
    .unwrap_or_default()
}

impl Component for TablePage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 3),
      sortable: ctx.signal(true),
      selection: ctx.signal(1),
      density: ctx.signal(1),
      fixed_height: ctx.signal(true),
      form: ctx.signal(0),
      sticky: ctx.signal(false),
      footer: ctx.signal(false),
      played_sort: ctx.signal(Vec::new()),
      played_selected: ctx.signal(Vec::new()),
      grid_selected: ctx.signal(vec!["crates/lurq"]),
      grid_sort: ctx.signal(vec![TableSort::ascending("name")]),
      page: ctx.signal(1),
      sort: ctx.signal(Vec::new()),
      tall_selected: ctx.signal(Vec::new()),
      board_selected: ctx.signal(vec!["crates/lurq_macros"]),
      columns_menu: (ElementRef::new(), ctx.signal(false)),
      shown_columns: ctx.signal((true, true)),
      many: many(),
      many_selected: ctx.signal(Vec::new()),
      search: ctx.signal(String::new()),
      bar_density: ctx.signal(3),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = Table::new(board_columns(), crates(3))
      .row_key(|row: &Krate| row.name)
      .label("Crates")
      .selected(self.board_selected.clone())
      .checkboxes(false)
      .style(TablePart::Root, |s| s.width(640.0));

    let grid_columns = vec![
      TableColumn::new("name", "Name")
        .text(|row: &Krate| row.name.to_owned())
        .mono(true)
        .sortable(true),
      TableColumn::new("status", "Status")
        .width(110.0)
        .cell(|row: &Krate| row.status.badge()),
      TableColumn::new("size", "Size")
        .width(70.0)
        .numeric()
        .text(|row: &Krate| format!("{} KB", row.size))
        .sort_key(|row: &Krate| row.size),
    ];
    let page = self.page.get();
    let grid_sort = self.grid_sort.get();
    let page_rows: Vec<Krate> = (0..3).map(|index| krate((page - 1) * 3 + index)).collect();
    let grid = Table::new(grid_columns.clone(), sorted(&grid_columns, page_rows, &grid_sort))
      .row_key(|row: &Krate| row.name)
      .label("Files")
      .sort(self.grid_sort.clone())
      .selected(self.grid_selected.clone())
      .total(24)
      .pagination(Pagination::for_items(self.page.clone(), 24, 3).compact(true))
      .style(TablePart::Root, |s| s.width(640.0));

    let sort = self.sort.get();
    let sort_columns = columns(false, None);
    let sorting = Table::new(sort_columns.clone(), sorted(&sort_columns, crates(6), &sort))
      .row_key(|row: &Krate| row.name)
      .label("Crates")
      .sort(self.sort.clone());

    let tall = Table::new(columns(true, None), crates(40))
      .row_key(|row: &Krate| row.name)
      .label("Crates")
      .selected(self.tall_selected.clone())
      .height(280.0)
      .style(TablePart::Root, |s| s.width(560.0))
      .mount(ctx);

    let thousands = Table::new_shared(columns(false, None), self.many.clone())
      .row_key(|row: &Krate| row.name)
      .label("Generated crates")
      .selected(self.many_selected.clone())
      .count(true)
      .height(360.0)
      .mount(ctx);

    let empty = Table::new(narrow_columns(), Vec::new())
      .row_key(|row: &Krate| row.name)
      .empty(
        EmptyState::new("No crates match “blurr”")
          .description("Check the spelling or clear the filter.")
          .icon(icons::SEARCH_CLOSE)
          .size(EmptyStateSize::Sm)
          .action(
            Button::new("Clear filter")
              .variant(ButtonVariant::Outline)
              .size(ButtonSize::Sm),
          ),
      )
      .style(TablePart::Root, |s| s.width(420.0));
    let loading = Table::new(narrow_columns(), crates(3))
      .row_key(|row: &Krate| row.name)
      .loading(true)
      .style(TablePart::Root, |s| s.width(420.0));
    let forms = Row::new()
      .spacing(28.0)
      .wrap()
      .align_items(Alignment::Start)
      .child(item(empty, "empty: the table's empty state under the header"))
      .child(item(loading, "loading: skeletons where the rows will be"));

    let state = |forced: Option<Interaction>| {
      Table::new(narrow_columns(), crates(2))
        .row_key(|row: &Krate| row.name)
        .selected_controlled(["crates/lurq_macros"], |_| {})
        .checkboxes(false)
        .force(forced)
        .style(TablePart::Root, |s| s.width(420.0))
    };
    let states = Row::new()
      .spacing(28.0)
      .wrap()
      .align_items(Alignment::Start)
      .child(item(state(None), "at rest, and a row selected"))
      .child(item(state(Some(Interaction::Hovered)), "under the pointer"))
      .child(item(
        state(Some(Interaction::FocusVisible)),
        "focused from the keyboard",
      ));

    let rows = [
      (
        "columns",
        "impl IntoIterator<Item = TableColumn<R>>",
        "",
        "The columns: `TableColumn::new(id, title)` with `text` or `cell`, `width`, `share`, `fit`, `min_width`, \
         `max_width`, `align`, `numeric`, `mono`, `muted`, `sortable`, `sort_by`, `sort_key`, `sticky`, \
         `controls`; `TableColumn::actions(id, cell)` for a row's buttons.",
      ),
      (
        "rows",
        "impl IntoIterator<Item = R>",
        "",
        "The rows, in the order the app wants them. `new_shared` takes an `Arc<Vec<R>>` kept from render to render.",
      ),
      (
        "row_key",
        "impl Fn(&R) -> K",
        "the row's place",
        "What tells a row apart: the selection and the handlers name rows by it.",
      ),
      (
        "sort",
        "Signal<Vec<TableSort>>",
        "none",
        "What the rows are sorted by: a press of a sortable header sets it to ascending, descending, none. \
         `sort_controlled(sort, on_change)` is the same held by the app; `TableSort::sort` sorts rows by a column.",
      ),
      (
        "selected",
        "Signal<Vec<K>>",
        "none",
        "The rows selected, with a column of checkboxes. `selected_controlled(keys, on_change)` is the same held by the app.",
      ),
      (
        "checkboxes",
        "bool",
        "true",
        "Whether a table whose rows are selected has the column of checkboxes.",
      ),
      (
        "density",
        "Density",
        "the theme's",
        "Rows of 36 compact, 44, 52 comfortable.",
      ),
      (
        "height / max_height",
        "Length",
        "its rows'",
        "The table's height: the header stays and the rows scroll under it.",
      ),
      (
        "empty",
        "impl Into<Element>",
        "No rows",
        "What stands under the header where there are no rows; empty_text(..) says it in other words.",
      ),
      (
        "on_sort_change / on_selection_change",
        "impl Fn(&[TableSort]) / impl Fn(&[K])",
        "",
        "Told the new sort, and the whole new selection where it changed, bound or not.",
      ),
      (
        "select_all_label / select_row_label / count_text",
        "&str / impl Fn(usize, Option<usize>) -> String",
        "English",
        "The words the table writes itself: its checkboxes' names, the footer's count.",
      ),
      (
        "loading",
        "bool",
        "false",
        "Rows of skeletons under the header; `loading_rows(n)` how many.",
      ),
      (
        "pagination / count / total",
        "Pagination / bool / usize",
        "none",
        "A footer that counts the rows and what is selected, with the pagination at its end.",
      ),
      (
        "on_row_click / on_row_activate",
        "impl Fn(&K)",
        "",
        "Told of a press of a row, and of a double press or Enter.",
      ),
      (
        "mount",
        "&mut Ctx",
        "",
        "Builds the rows in view and a few around them, keeps the measured widths, and takes PageUp and PageDown.",
      ),
      (
        "label / disabled",
        "&str / bool",
        "",
        "What the table is called to what reads the tree; no row takes input.",
      ),
      (
        "style / rules",
        "TablePart",
        "",
        "Root, Scroll, Header, HeaderCell, SortIcon, Row, Cell, Select, Empty, Footer, Count.",
      ),
    ];
    let playground = self.play(ctx);
    let with_bar = self.with_bar(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Table"],
        title: "Table",
        description: "Rows of the same things in columns: crates and their status, files and their sizes. The \
                      columns of the header and of every row are the same widths whatever is in a cell.",
        import: Some("use lurq::ui::{Table, TableColumn, TableSort};"),
        status: Some("Preview"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Table is a control on the right. The table on the stage is live: a press of a header \
           sorts, a press of a row selects it, with Ctrl adds it, with Shift a range, and its button is its own.",
          playground,
        ),
        Section::new(
          "The board's table",
          "A name in the mono type, a status as a badge, when it changed in the muted color. The header is 40 \
           high on the muted fill, a row 44 with a line under it but the last.",
          self.frame.demo(
            0,
            board,
            "Table::new(columns, rows)",
            &[
              "Table::new([",
              "    TableColumn::new(\"name\", \"Name\").text(|row| row.name.into()).mono(true),",
              "    TableColumn::new(\"status\", \"Status\").width(120.0).cell(|row| row.status.badge()),",
              "    TableColumn::new(\"updated\", \"Updated\").width(120.0).muted(true).text(|row| row.updated.into()),",
              "], rows)",
              ".row_key(|row| row.name)",
              ".label(\"Crates\")",
              "// A row selected by a press of it, on the highlight, as the board draws crates/lurq_macros.",
              ".selected(selected)",
              ".checkboxes(false)",
            ],
          ),
        ),
        Section::new(
          "Selecting rows",
          "With a selection a table has a column of checkboxes. The header's selects every row shown, or none, \
           and is mixed while some are. Up and Down move through the rows, Space selects the focused one, Enter \
           activates it, Ctrl+A selects them all. The footer counts them, with the pagination at its end.",
          self.frame.demo(
            1,
            grid,
            ".selected(selected).pagination(..)",
            &[
              "// selected: Signal<Vec<&str>>, page: Signal<usize>, sort: Signal<Vec<TableSort>>",
              "Table::new(columns, rows_of_the_page)",
              "    .row_key(|row| row.name)",
              "    .sort(sort)",
              "    .selected(selected)",
              "    .total(24)",
              "    .pagination(Pagination::for_items(page, 24, 3).compact(true))",
            ],
          ),
        ),
        Section::new(
          "Sorting",
          "A press of a sortable header sorts the rows by it alone, ascending, then descending, then not at all; \
           with Shift it adds its column to the sort after the others, and each sorted header shows its place. \
           The app gives the rows in that order: `TableSort::sort_rows` sorts them by each column's text or key.",
          self.frame.demo(
            2,
            sorting,
            ".sort(sort)",
            &[
              "// sort: Signal<Vec<TableSort>>, the first column first",
              "let mut rows = rows;",
              "TableSort::sort_rows(&sort.get(), &columns, &mut rows);",
              "Table::new(columns, rows).sort(sort)",
            ],
          ),
        ),
        Section::new(
          "A header that stays",
          "In a height of its own the rows scroll under the header, and a table wider than its box scrolls \
           sideways with it; the sticky first column stays at the start.",
          self.frame.demo(
            3,
            tall,
            ".height(280.0)",
            &[
              "Table::new(columns_with_a_sticky_name, forty_crates)",
              "    .selected(selected)",
              "    .height(280.0)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Ten thousand rows",
          "A mounted table builds the rows in view and a few around them: ten thousand cost what a screen of \
           them does. Home and End reach the first and the last, PageUp and PageDown go a page at a time.",
          self.frame.demo(
            4,
            thousands,
            "Table::new_shared(columns, rows).mount(ctx)",
            &[
              "// rows: Arc<Vec<Crate>> of 10 000, kept from render to render",
              "Table::new_shared(columns, rows.clone())",
              "    .row_key(|row| row.name)",
              "    .selected(selected)",
              "    .count(true)",
              "    .height(360.0)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Empty and loading",
          "Where there are no rows an empty state stands under the header; while they are on their way, rows of \
           skeletons. The header stays in both.",
          self.frame.demo(
            5,
            forms,
            ".empty(..) / .loading(true)",
            &[
              "Table::new(columns, Vec::new()).empty(",
              "    EmptyState::new(\"No crates match “blurr”\")",
              "        .description(\"Check the spelling or clear the filter.\")",
              "        .icon(icons::SEARCH_CLOSE)",
              "        .size(EmptyStateSize::Sm)",
              "        .action(Button::new(\"Clear filter\").variant(ButtonVariant::Outline)),",
              ")",
              "",
              "Table::new(columns, rows).loading(true)",
            ],
          ),
        ),
        Section::new(
          "With its bar",
          "The board's Table Toolbar is a `Toolbar`: the search filters the rows, and the density sets the \
           table's.",
          self.frame.demo(
            6,
            with_bar,
            "Toolbar::new() over Table::new(..).density(density)",
            &[
              "// query: Signal<String>, density: Signal<u8>",
              "Toolbar::new()",
              "    .item(ToolbarItem::element(SearchField::new(query).placeholder(\"Filter crates\")).stays(true))",
              "    .item(ToolbarItem::element(Tag::new(\"Status: Passing\").variant(TagVariant::Info)))",
              "    .item(ToolbarItem::element(Tag::new(\"Kind: Rust\").variant(TagVariant::Info)))",
              "    .item(ToolbarItem::from(Button::new(\"Filter\").icon(icons::PLUS)).as_given(true))",
              "    .item(Toolbar::spacer())",
              "    .item(ToggleGroup::new(density, [/* rows-2, rows-3, rows-4 */]))",
              "    .item(ToolbarItem::element(columns_menu_button))",
              "    .item(ToolbarItem::from(Button::new(\"Export\").icon(icons::DOWNLOAD)).as_given(true))",
              "    .mount(ctx)",
              "Table::new(columns, filtered).density(Density::Compact)",
            ],
          ),
        ),
        Section::new(
          "States",
          "The board draws a row at rest and a selected one. Under the pointer and held down a row is veiled, and \
           the focus from the keyboard is a ring inside the row.",
          self.frame.demo(
            7,
            states,
            ".force(Some(Interaction::FocusVisible))",
            &[
              "Table::new(columns, rows)",
              "    .selected_controlled([\"crates/lurq_macros\"], |_| {})",
              "    .checkboxes(false)",
              "",
              "// Drawn as under the pointer or focused, with nothing pointing at it: for a gallery, a test.",
              "    .force(Some(Interaction::FocusVisible))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a table is given.",
          props_table(&rows, &["columns", "rows"], self.frame.room()),
        ),
      ],
    )
  }
}
