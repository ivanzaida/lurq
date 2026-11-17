//! The page of the grid: children in the cells of columns and rows, taken in their order or named. Its
//! playground fills a grid with items as its props say; its sections are the demo's Grid page: a form whose labels
//! have a column of their own, tracks of each kind, rows that share a height, cells an item names, and items with
//! a size of their own.

use std::sync::Arc;

use lurq::{
  app::{
    component::Component,
    ctx::Ctx,
    theme::{PaletteColor, RadiusSize},
  },
  components::{Column, Grid, Rect},
  core::Signal,
  layout::{Alignment, grid::GridTrack, text_style::FontWeight},
  node::Element,
  ui::Input,
};

use super::kit::{sample, samples, tint, well};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, text},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

const ALIGNS: [(&str, Alignment); 4] = [
  ("Stretch", Alignment::Stretch),
  ("Start", Alignment::Start),
  ("Center", Alignment::Center),
  ("End", Alignment::End),
];
/// The choices of the playground's selects of an alignment: an index into [`ALIGNS`], which a signal keeps.
const ALIGN_CHOICES: [(&str, usize); 4] = [("Stretch", 0), ("Start", 1), ("Center", 2), ("End", 3)];
const COLUMNS: [(&str, i32); 6] = [("1", 1), ("2", 2), ("3", 3), ("4", 4), ("5", 5), ("6", 6)];
const ITEMS: [(&str, i32); 6] = [("0", 0), ("1", 1), ("4", 4), ("7", 7), ("12", 12), ("18", 18)];
const GAPS: [(&str, i32); 5] = [("0", 0), ("4", 4), ("10", 10), ("16", 16), ("32", 32)];

/// An item with no size of its own: it takes its cell's where the grid stretches it.
fn item(caption: &str, index: usize) -> Column {
  Column::new()
    .padding(10.0)
    .rounded(RadiusSize::Sm)
    .background(tint(index))
    .child(text(caption, 12.0, FontWeight::Medium).color(PaletteColor::Background))
}

/// The grid of the playground, as its props say.
#[derive(Clone, Copy)]
struct Flow {
  columns: i32,
  items: i32,
  gap: i32,
  align: usize,
  justify: usize,
  spans: bool,
}

impl Flow {
  fn build(self) -> Element {
    let (_, align) = ALIGNS[self.align.min(ALIGNS.len() - 1)];
    let (_, justify) = ALIGNS[self.justify.min(ALIGNS.len() - 1)];
    let mut grid = Grid::with_columns(self.columns.max(1) as usize)
      .gap(self.gap as f32)
      .align_items(align)
      .justify_items(justify)
      .width(FILL);
    for index in 0..self.items.max(0) as usize {
      let mut cell = item(&format!("item {}", index + 1), index);
      if self.spans {
        cell = match index {
          0 => cell.column_span(2),
          2 => cell.row_span(2),
          _ => cell,
        };
      }
      // Every third item is taller, so that the rows differ and what stretches shows.
      if index % 3 == 1 {
        cell = cell.child(text("a second line", 12.0, FontWeight::Normal).color(PaletteColor::Background));
      }
      grid = grid.child(cell);
    }
    well().padding(10.0).child(grid).into()
  }

  fn code(self) -> Vec<String> {
    let mut code = vec![
      format!("Grid::with_columns({})", self.columns),
      format!("    .gap({}.0)", self.gap),
    ];
    if self.align != 0 {
      code.push(format!("    .align_items(Alignment::{})", ALIGNS[self.align].0));
    }
    if self.justify != 0 {
      code.push(format!("    .justify_items(Alignment::{})", ALIGNS[self.justify].0));
    }
    if self.spans {
      code.push("    .child(item(1).column_span(2))".to_owned());
      code.push("    .child(item(2))".to_owned());
      code.push("    .child(item(3).row_span(2))".to_owned());
    } else {
      code.push("    .child(item(1))".to_owned());
      code.push("    .child(item(2))".to_owned());
    }
    code.push("    // ..".to_owned());
    code
  }
}

pub struct GridPage {
  frame: Frame,
  playground: Playground,
  columns: Signal<i32>,
  items: Signal<i32>,
  gap: Signal<i32>,
  /// Indexes into [`ALIGNS`].
  align: Signal<usize>,
  justify: Signal<usize>,
  spans: Signal<bool>,
  name: Signal<String>,
  email: Signal<String>,
}

impl GridPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let flow = Flow {
      columns: self.columns.get(),
      items: self.items.get(),
      gap: self.gap.get(),
      align: self.align.get(),
      justify: self.justify.get(),
      spans: self.spans.get(),
    };
    let columns = flow.columns.max(1) as usize;
    let rows = (flow.items.max(0) as usize).div_ceil(columns);
    let mut tree = vec![format!("grid   {}   {columns} columns", self.playground.measured())];
    let shown = (flow.items.max(0) as usize).min(3);
    for index in 0..shown {
      let last = index + 1 == shown && flow.items <= 3;
      let span = match (flow.spans, index) {
        (true, 0) => "   column_span 2",
        (true, 2) => "   row_span 2",
        _ => "",
      };
      tree.push(format!("{} item {}{span}", if last { "└─" } else { "├─" }, index + 1));
    }
    if flow.items > 3 {
      tree.push(format!("└─ … {} more", flow.items - 3));
    }

    let (set_columns, set_items, set_gap, set_align, set_justify) = (
      self.columns.clone(),
      self.items.clone(),
      self.gap.clone(),
      self.align.clone(),
      self.justify.clone(),
    );
    let controls = vec![
      prop(
        "with_columns",
        "usize",
        self
          .playground
          .select(0, &COLUMNS, &flow.columns, move |value| set_columns.set(value)),
      ),
      prop(
        "children",
        "",
        self
          .playground
          .select(1, &ITEMS, &flow.items, move |value| set_items.set(value)),
      ),
      prop(
        "gap",
        "impl Into<SpacingValue>",
        self
          .playground
          .select(2, &GAPS, &flow.gap, move |value| set_gap.set(value)),
      ),
      prop(
        "align_items",
        "Alignment",
        self
          .playground
          .select(3, &ALIGN_CHOICES, &flow.align, move |value| set_align.set(value)),
      ),
      prop(
        "justify_items",
        "Alignment",
        self
          .playground
          .select(4, &ALIGN_CHOICES, &flow.justify, move |value| set_justify.set(value)),
      ),
      divider(),
      flag("item 1 spans two columns, item 3 two rows", &self.spans),
    ];
    let reset = {
      let (columns, items, gap, align, justify, spans) = (
        self.columns.clone(),
        self.items.clone(),
        self.gap.clone(),
        self.align.clone(),
        self.justify.clone(),
        self.spans.clone(),
      );
      Arc::new(move || {
        columns.set(3);
        items.set(7);
        gap.set(10);
        align.set(0);
        justify.set(0);
        spans.set(true);
      })
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Grid",
        make: Arc::new(move || flow.build()),
        shown: Shown::built(move |_| flow.build()),
        fill: true,
        forceable: false,
        facts: format!("{columns} columns · {rows} rows of items"),
        event: "",
        code: flow.code(),
        tree,
        controls,
        changed: [
          flow.columns != 3,
          flow.items != 7,
          flow.gap != 10,
          flow.align != 0,
          flow.justify != 0,
          !flow.spans,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset,
      },
    )
  }

  /// Labels in a column as wide as the widest, fields in the room that is left, and a note across both.
  fn form(&self) -> Column {
    well().padding(12.0).child(
      Grid::new()
        .columns([GridTrack::Auto, GridTrack::Fr(1.0)])
        .gap(10.0)
        .align_items(Alignment::Center)
        .width(FILL)
        .child(text("Name", 13.0, FontWeight::Normal))
        .child(
          Input::new(self.name.clone())
            .placeholder("Ada Lovelace")
            .full_width(true),
        )
        .child(text("Email address", 13.0, FontWeight::Normal))
        .child(
          Input::new(self.email.clone())
            .placeholder("ada@example.com")
            .full_width(true),
        )
        .child(
          quiet(
            "The labels' column is as wide as the widest label; the fields take the rest.",
            12.0,
          )
          .width(FILL)
          .column_span(2),
        ),
    )
  }
}

/// A column of each kind: 90 pixels, one share and two, and as wide as its item.
fn tracks() -> Column {
  well().padding(10.0).child(
    Grid::new()
      .columns([
        GridTrack::Px(90.0),
        GridTrack::Fr(1.0),
        GridTrack::Fr(2.0),
        GridTrack::Auto,
      ])
      .gap(8.0)
      .width(FILL)
      .child(item("90 px", 0))
      .child(item("1 fr", 1))
      .child(item("2 fr", 2))
      .child(item("auto: as wide as this", 3)),
  )
}

/// Rows that share a height of 220: 30 pixels, and one share and two of what is left.
fn rows() -> Column {
  well().width(240.0).max_width(FILL).padding(10.0).child(
    Grid::new()
      .columns([GridTrack::Fr(1.0)])
      .rows([GridTrack::Px(30.0), GridTrack::Fr(1.0), GridTrack::Fr(2.0)])
      .gap(8.0)
      .width(FILL)
      .height(220.0)
      .child(item("30 px", 0))
      .child(item("1 fr", 1))
      .child(item("2 fr", 2)),
  )
}

/// Five items each in a cell it names, the others left empty.
fn named() -> Column {
  well().width(240.0).max_width(FILL).padding(10.0).child(
    Grid::with_columns(3)
      .rows([GridTrack::Px(44.0), GridTrack::Px(44.0), GridTrack::Px(44.0)])
      .gap(8.0)
      .width(FILL)
      .child(item("0, 0", 0).grid_column(0).grid_row(0))
      .child(item("2, 0", 1).grid_column(2).grid_row(0))
      .child(item("1, 1", 2).grid_column(1).grid_row(1))
      .child(item("0, 2", 3).grid_column(0).grid_row(2))
      .child(item("2, 2", 4).grid_column(2).grid_row(2)),
  )
}

/// Two items stretched to their cells, and two with a size of their own, which they keep.
fn own_size() -> Column {
  well().width(240.0).max_width(FILL).padding(10.0).child(
    Grid::with_columns(2)
      .gap(8.0)
      .width(FILL)
      .height(110.0)
      .child(Rect::new(40.0, 24.0).rounded(RadiusSize::Sm).background(tint(0)))
      .child(item("stretched", 2))
      .child(item("stretched", 3))
      .child(Rect::new(60.0, 16.0).rounded(RadiusSize::Sm).background(tint(4))),
  )
}

impl Component for GridPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 5),
      columns: ctx.signal(3),
      items: ctx.signal(7),
      gap: ctx.signal(10),
      align: ctx.signal(0),
      justify: ctx.signal(0),
      spans: ctx.signal(true),
      name: ctx.signal(String::new()),
      email: ctx.signal(String::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let props = [
      (
        "Grid::with_columns",
        "usize",
        "",
        "A grid of so many columns of one share each.",
      ),
      (
        "columns / rows",
        "impl IntoIterator<Item = GridTrack>",
        "",
        "The tracks, each Px, Fr (a share of the room the others leave) or Auto (as large as its largest item). \
         Rows past the last named are added as the items need them.",
      ),
      (
        "gap",
        "impl Into<SpacingValue>",
        "0",
        "The room between two columns and between two rows; column_gap and row_gap say one of them.",
      ),
      (
        "align_items",
        "Alignment",
        "Stretch",
        "How an item sits in its cell from top to bottom.",
      ),
      (
        "justify_items",
        "Alignment",
        "Stretch",
        "How an item sits in its cell from side to side.",
      ),
      (
        "column_span / row_span",
        "usize",
        "1",
        "On an item: how many columns or rows it takes.",
      ),
      (
        "grid_column / grid_row",
        "usize",
        "",
        "On an item: the cell it starts in, counted from 0. Without them it takes the next cell with room.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Grid"],
        title: "Grid",
        description: "A grid lays its children out in the cells of columns and rows, as CSS grid does: they take \
                      the cells in their order, or name the one they start in.",
        import: Some("use lurq::{components::Grid, layout::grid::GridTrack};"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Items take the cells in their order. Rows are as tall as their tallest item and are added as the items \
           need them; make the window narrower and the columns share less width. Every third item has a second \
           line, so that how the others sit in their rows shows.",
          playground,
        ),
        Section::new(
          "A form",
          "A column as wide as the widest label and one that takes the rest. Type in a field: a field that grows \
           does not move the labels' column.",
          self.frame.demo(
            0,
            self.form(),
            ".columns([GridTrack::Auto, GridTrack::Fr(1.0)])",
            &[
              "Grid::new()",
              "    .columns([GridTrack::Auto, GridTrack::Fr(1.0)])",
              "    .gap(10.0)",
              "    .align_items(Alignment::Center)",
              "    .child(Text::new(\"Name\"))",
              "    .child(Input::new(name).placeholder(\"Ada Lovelace\"))",
              "    .child(Text::new(\"Email address\"))",
              "    .child(Input::new(email).placeholder(\"ada@example.com\"))",
              "    .child(Text::new(\"The labels' column …\").column_span(2))",
            ],
          ),
        ),
        Section::new(
          "Kinds of track",
          "A column of 90 pixels, two that share what is left one to two, and one as wide as its item.",
          self.frame.demo(
            1,
            tracks(),
            "GridTrack::Fr(2.0)",
            &[
              "Grid::new()",
              "    .columns([GridTrack::Px(90.0), GridTrack::Fr(1.0), GridTrack::Fr(2.0), GridTrack::Auto])",
              "    .gap(8.0)",
              "    .child(item(\"90 px\"))",
              "    .child(item(\"1 fr\"))",
              "    .child(item(\"2 fr\"))",
              "    .child(item(\"auto: as wide as this\"))",
            ],
          ),
        ),
        Section::new(
          "Rows that share a height",
          "A grid told its height: a row of 30 pixels, and two that share what is left one to two.",
          self.frame.demo(
            2,
            rows(),
            ".rows([GridTrack::Px(30.0), GridTrack::Fr(1.0), GridTrack::Fr(2.0)])",
            &[
              "Grid::new()",
              "    .columns([GridTrack::Fr(1.0)])",
              "    .rows([GridTrack::Px(30.0), GridTrack::Fr(1.0), GridTrack::Fr(2.0)])",
              "    .height(220.0)",
              "    .child(item(\"30 px\"))",
              "    .child(item(\"1 fr\"))",
              "    .child(item(\"2 fr\"))",
            ],
          ),
        ),
        Section::new(
          "Cells an item names",
          "Each item names its column and its row; the cells no item names stay empty.",
          self.frame.demo(
            3,
            named(),
            ".grid_column(2).grid_row(0)",
            &[
              "Grid::with_columns(3)",
              "    .rows([GridTrack::Px(44.0), GridTrack::Px(44.0), GridTrack::Px(44.0)])",
              "    .gap(8.0)",
              "    .child(item(\"0, 0\").grid_column(0).grid_row(0))",
              "    .child(item(\"2, 0\").grid_column(2).grid_row(0))",
              "    .child(item(\"1, 1\").grid_column(1).grid_row(1))",
              "    // ..",
            ],
          ),
        ),
        Section::new(
          "Items with a size of their own",
          "An item with no size is stretched to its cell; one with a size keeps it, at the start of its cell.",
          self.frame.demo(
            4,
            samples().child(sample(own_size(), "Grid::with_columns(2).height(110.0)")),
            "Rect::new(40.0, 24.0)",
            &[
              "Grid::with_columns(2)",
              "    .gap(8.0)",
              "    .height(110.0)",
              "    .child(Rect::new(40.0, 24.0))",
              "    .child(item(\"stretched\"))",
              "    .child(item(\"stretched\"))",
              "    .child(Rect::new(60.0, 16.0))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a grid is given, and what an item in it is.",
          props_table(&props, &[], self.frame.room()),
        ),
      ],
    )
  }
}
