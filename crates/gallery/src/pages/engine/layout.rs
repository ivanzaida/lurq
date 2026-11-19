//! The page of the engine's three boxes: a row and a column, which put their children one after another along
//! their axis, and a stack, which puts them on top of each other. Its playground lays tiles out as its props say;
//! its sections are the demo's Layout page: the two axes, the five ways along the main one, the four across it,
//! growing children, a stack, and a row that wraps.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::PaletteColor},
  components::{Column, Row, Stack},
  core::Signal,
  layout::{Alignment, StackAlignment, layout_kind::Justify},
  node::{Element, dimension::Dimension},
};

use super::kit::{sample, samples, tile, tint, well};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

const DIRECTIONS: [(&str, usize); 2] = [("Row", 0), ("Column", 1)];
const JUSTIFIES: [(&str, Justify); 6] = [
  ("Start", Justify::Start),
  ("Center", Justify::Center),
  ("End", Justify::End),
  ("SpaceBetween", Justify::SpaceBetween),
  ("SpaceAround", Justify::SpaceAround),
  ("SpaceEvenly", Justify::SpaceEvenly),
];
const ALIGNS: [(&str, Alignment); 4] = [
  ("Start", Alignment::Start),
  ("Center", Alignment::Center),
  ("End", Alignment::End),
  ("Stretch", Alignment::Stretch),
];
/// The choices of the playground's selects: an index into [`JUSTIFIES`] and [`ALIGNS`], which a signal keeps.
const JUSTIFY_CHOICES: [(&str, usize); 6] = [
  ("Start", 0),
  ("Center", 1),
  ("End", 2),
  ("SpaceBetween", 3),
  ("SpaceAround", 4),
  ("SpaceEvenly", 5),
];
const ALIGN_CHOICES: [(&str, usize); 4] = [("Start", 0), ("Center", 1), ("End", 2), ("Stretch", 3)];
const SPACINGS: [(&str, i32); 4] = [("0", 0), ("8", 8), ("16", 16), ("24", 24)];
const COUNTS: [(&str, usize); 3] = [("3", 3), ("5", 5), ("12", 12)];

/// How long and how thick each tile is, in turn: they differ, so that an alignment across shows.
const LENGTHS: [f32; 3] = [64.0, 88.0, 52.0];
const THICKNESSES: [f32; 3] = [40.0, 64.0, 28.0];

/// The tiles of the playground: `count` of them, the second growing where `grow` is on. A tile has no size across
/// the axis where the children are stretched, so that it takes the box's.
fn tiles(count: usize, column: bool, stretch: bool, grow: bool) -> Vec<Element> {
  (0..count)
    .map(|index| {
      let along = Dimension::Px(LENGTHS[index % 3]);
      let across = if stretch {
        Dimension::Auto
      } else {
        Dimension::Px(THICKNESSES[index % 3])
      };
      let (width, height) = if column { (across, along) } else { (along, across) };
      let mut tile = tile(&(index + 1).to_string(), index, width, height);
      if grow && index == 1 {
        tile = tile.flex(1.0);
      }
      tile.into()
    })
    .collect()
}

pub struct LayoutPage {
  frame: Frame,
  playground: Playground,
  direction: Signal<usize>,
  /// An index into [`JUSTIFIES`].
  justify: Signal<usize>,
  /// An index into [`ALIGNS`].
  align: Signal<usize>,
  spacing: Signal<i32>,
  count: Signal<usize>,
  wrap: Signal<bool>,
  grow: Signal<bool>,
}

impl LayoutPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (direction, justify_at, align_at) = (self.direction.get(), self.justify.get(), self.align.get());
    let (justify_name, justify) = JUSTIFIES[justify_at.min(JUSTIFIES.len() - 1)];
    let (align_name, align) = ALIGNS[align_at.min(ALIGNS.len() - 1)];
    let (spacing, count, wrap, grow) = (self.spacing.get(), self.count.get(), self.wrap.get(), self.grow.get());
    let column = direction == 1;
    let stretch = align == Alignment::Stretch;
    let build = move || -> Element {
      let children = tiles(count, column, stretch, grow);
      let shown: Element = if column {
        Column::new()
          .size(FILL, FILL)
          .padding(12.0)
          .spacing(spacing as f32)
          .justify(justify)
          .align_items(align)
          .with_children(children)
          .into()
      } else {
        let mut row = Row::new()
          .size(FILL, FILL)
          .padding(12.0)
          .spacing(spacing as f32)
          .justify(justify)
          .align_items(align)
          .with_children(children);
        if wrap {
          row = row.wrap();
        }
        row.into()
      };
      well().height(240.0).child(shown).into()
    };

    let kind = if column { "Column" } else { "Row" };
    let mut code = vec![format!("{kind}::new()")];
    if spacing != 0 {
      code.push(format!("    .spacing({spacing}.0)"));
    }
    if justify != Justify::Start {
      code.push(format!("    .justify(Justify::{justify_name})"));
    }
    if align != Alignment::Start {
      code.push(format!("    .align_items(Alignment::{align_name})"));
    }
    if wrap && !column {
      code.push("    .wrap()".to_owned());
    }
    code.push(format!("    .child(Rect::new({}, {}))", LENGTHS[0], THICKNESSES[0]));
    if grow {
      code.push(format!(
        "    .child(Rect::new({}, {}).flex(1.0))",
        LENGTHS[1], THICKNESSES[1]
      ));
    } else {
      code.push(format!("    .child(Rect::new({}, {}))", LENGTHS[1], THICKNESSES[1]));
    }
    code.push("    // ..".to_owned());

    let mut tree = vec![format!("{}   {}", kind.to_lowercase(), self.playground.measured())];
    for index in 0..count.min(4) {
      let last = index + 1 == count.min(4) && count <= 4;
      tree.push(format!(
        "{} tile {}{}",
        if last { "└─" } else { "├─" },
        index + 1,
        if grow && index == 1 { "   flex 1" } else { "" }
      ));
    }
    if count > 4 {
      tree.push(format!("└─ … {} more", count - 4));
    }

    let (set_direction, set_justify, set_align, set_spacing, set_count) = (
      self.direction.clone(),
      self.justify.clone(),
      self.align.clone(),
      self.spacing.clone(),
      self.count.clone(),
    );
    let reset = {
      let (direction, justify, align, spacing, count, wrap, grow) = (
        self.direction.clone(),
        self.justify.clone(),
        self.align.clone(),
        self.spacing.clone(),
        self.count.clone(),
        self.wrap.clone(),
        self.grow.clone(),
      );
      Arc::new(move || {
        direction.set(0);
        justify.set(0);
        align.set(0);
        spacing.set(8);
        count.set(3);
        wrap.set(false);
        grow.set(false);
      })
    };
    let mut controls = vec![
      prop(
        "Row / Column",
        "the box",
        self
          .playground
          .select(0, &DIRECTIONS, &direction, move |value| set_direction.set(value)),
      ),
      prop(
        "justify",
        "Justify",
        self
          .playground
          .select(1, &JUSTIFY_CHOICES, &justify_at, move |value| set_justify.set(value)),
      ),
      prop(
        "align_items",
        "Alignment",
        self
          .playground
          .select(2, &ALIGN_CHOICES, &align_at, move |value| set_align.set(value)),
      ),
      prop(
        "spacing",
        "impl Into<SpacingValue>",
        self
          .playground
          .select(3, &SPACINGS, &spacing, move |value| set_spacing.set(value)),
      ),
      prop(
        "children",
        "usize",
        self
          .playground
          .select(4, &COUNTS, &count, move |value| set_count.set(value)),
      ),
      divider(),
      flag("flex(1.0) on the second", &self.grow),
    ];
    // A column does not wrap: the flag is there while the box is a row.
    if !column {
      controls.push(flag("wrap", &self.wrap));
    }
    self.playground.card(
      ctx,
      Spec {
        title: "Row and Column",
        make: Arc::new(build),
        shown: Shown::built(move |_| build()),
        fill: true,
        forceable: false,
        facts: format!("{count} children"),
        event: "",
        code,
        tree,
        controls,
        changed: [
          column,
          justify != Justify::Start,
          align != Alignment::Start,
          spacing != 8,
          count != 3,
          wrap,
          grow,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset,
      },
    )
  }
}

/// Three tiles in a row and the same in a column, each in a well that shows its bounds.
fn axes() -> Row {
  let row = Row::new()
    .spacing(12.0)
    .align_items(Alignment::Center)
    .padding(12.0)
    .child(tile("A", 0, 64.0, 48.0))
    .child(tile("B", 1, 64.0, 48.0))
    .child(tile("C", 2, 64.0, 48.0));
  let column = Column::new()
    .width(FILL)
    .spacing(8.0)
    .padding(12.0)
    .child(tile("A", 0, FILL, 24.0))
    .child(tile("B", 1, FILL, 24.0))
    .child(tile("C", 2, FILL, 24.0));
  samples()
    .child(sample(well().width(240.0).max_width(FILL).child(row), "Row: along x"))
    .child(sample(
      well().width(240.0).max_width(FILL).child(column),
      "Column: along y",
    ))
}

/// A row for each way of putting room along the main axis.
fn justified() -> Column {
  let mut rows = Column::new().width(FILL).spacing(10.0);
  for (name, justify) in JUSTIFIES {
    rows = rows.child(sample(
      well().child(
        Row::new()
          .width(FILL)
          .height(40.0)
          .padding_horizontal(8.0)
          .align_items(Alignment::Center)
          .justify(justify)
          .child(tile("", 0, 48.0, 24.0))
          .child(tile("", 1, 48.0, 24.0))
          .child(tile("", 2, 48.0, 24.0)),
      ),
      &format!("Justify::{name}"),
    ));
  }
  rows
}

/// A row of tiles of three heights in each alignment across.
fn aligned() -> Row {
  let mut all = samples();
  for (name, align) in ALIGNS {
    let stretch = align == Alignment::Stretch;
    let block = |index: usize, height: f32| {
      tile(
        "",
        index,
        28.0,
        if stretch {
          Dimension::Auto
        } else {
          Dimension::Px(height)
        },
      )
    };
    all = all.child(sample(
      well().width(150.0).child(
        Row::new()
          .height(110.0)
          .padding(8.0)
          .spacing(6.0)
          .align_items(align)
          .child(block(0, 36.0))
          .child(block(1, 72.0))
          .child(block(2, 24.0)),
      ),
      &format!("Alignment::{name}"),
    ));
  }
  all
}

/// Three children that share the room of a row by their factors: 1, 2 and 1.
fn grown() -> Column {
  let share = |label: &str, index: usize, factor: f32| {
    Row::new()
      .height(56.0)
      .flex(factor)
      .min_width(0.0)
      .align_items(Alignment::Center)
      .justify(Justify::Center)
      .rounded(lurq::app::theme::RadiusSize::Sm)
      .background(tint(index))
      .child(quiet(label, 12.0).color(PaletteColor::Background).nowrap())
  };
  // The labels are short: a quarter of a phone's row has room for little more than the factor.
  sample(
    well().padding(12.0).child(
      Row::new()
        .width(FILL)
        .spacing(4.0)
        .child(share("flex 1", 0, 1.0))
        .child(share("flex 2", 1, 2.0))
        .child(share("flex 1", 2, 1.0)),
    ),
    "a quarter, a half and a quarter of the row",
  )
}

/// Three squares on top of each other, in the middle of a stack.
fn stacked() -> Column {
  well().child(
    Stack::new()
      .width(FILL)
      .height(220.0)
      .stack_align(StackAlignment::Center)
      .child(tile("", 0, 180.0, 180.0))
      .child(tile("", 1, 130.0, 130.0))
      .child(tile("3", 2, 80.0, 80.0)),
  )
}

/// Twelve tiles in a row that wraps: what does not fit goes on to the next line.
fn wrapped() -> Column {
  well().child(
    Row::new()
      .width(FILL)
      .padding(12.0)
      .spacing(10.0)
      .wrap()
      .with_children((0..12).map(|index| tile(&(index + 1).to_string(), index, 72.0, 40.0))),
  )
}

impl Component for LayoutPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 5),
      direction: ctx.signal(0),
      justify: ctx.signal(0),
      align: ctx.signal(0),
      spacing: ctx.signal(8),
      count: ctx.signal(3),
      wrap: ctx.signal(false),
      grow: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let rows = [
      (
        "spacing",
        "impl Into<SpacingValue>",
        "0",
        "The room between one child and the next, along the axis. A number of pixels, or a step of the theme's \
         spacing.",
      ),
      (
        "justify",
        "Justify",
        "Start",
        "Where the room left along the axis goes: before, after, around or between the children.",
      ),
      (
        "align_items",
        "Alignment",
        "Start",
        "Where a child stands across the axis. Stretch gives a child with no size across it the box's.",
      ),
      (
        "wrap",
        "()",
        "",
        "A row's children that do not fit go on to the next line, with the spacing between the lines too.",
      ),
      (
        "flex",
        "f32",
        "0",
        "On a child: its share of the room left along the axis, beside the others' shares. A child that grows \
         with a min_width of 0 also gives way when there is too little.",
      ),
      (
        "stack_align",
        "StackAlignment",
        "Center",
        "On a stack: where its children stand, one of nine points of its box.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Engine", "Layout"],
        title: "Layout",
        description: "Row, Column and Stack: the boxes everything in lurq is laid out in. A row and a column put \
                      their children one after another along their axis; a stack puts them on top of each other.",
        import: Some("use lurq::components::{Column, Row, Stack};"),
        status: Some("Stable"),
        since: None,
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "A row or a column of tiles of three lengths, in a box of a fixed height whose bounds are dashed. Choose \
           where the room goes along the axis and where the tiles stand across it, and let the second grow.",
          playground,
        ),
        Section::new(
          "Row and Column",
          "A row lays its children out from left to right, a column from top to bottom. A child as wide as FILL in \
           a column is as wide as the column.",
          self.frame.demo(
            0,
            axes(),
            "Row::new().spacing(12.0)",
            &[
              "Row::new()",
              "    .spacing(12.0)",
              "    .align_items(Alignment::Center)",
              "    .child(Rect::new(64.0, 48.0))",
              "    .child(Rect::new(64.0, 48.0))",
              "    .child(Rect::new(64.0, 48.0))",
              "",
              "Column::new()",
              "    .width(Dimension::Pct(100.0))",
              "    .spacing(8.0)",
              "    .child(Rect::new(Dimension::Pct(100.0), 24.0))",
              "    // ..",
            ],
          ),
        ),
        Section::new(
          "Along the axis",
          "What a row does with the room its children leave: puts it after them, before them, on both sides, \
           between them, around each, or evenly.",
          self.frame.demo(
            1,
            justified(),
            ".justify(Justify::SpaceBetween)",
            &[
              "Row::new()",
              "    .width(Dimension::Pct(100.0))",
              "    .height(40.0)",
              "    .align_items(Alignment::Center)",
              "    .justify(Justify::SpaceBetween)",
              "    .child(Rect::new(48.0, 24.0))",
              "    .child(Rect::new(48.0, 24.0))",
              "    .child(Rect::new(48.0, 24.0))",
            ],
          ),
        ),
        Section::new(
          "Across the axis",
          "Tiles of three heights at the top of the row, in its middle, at its foot, and stretched to its height: \
           a child stretched has no height of its own.",
          self.frame.demo(
            2,
            aligned(),
            ".align_items(Alignment::Center)",
            &[
              "Row::new()",
              "    .height(110.0)",
              "    .spacing(6.0)",
              "    .align_items(Alignment::Center)",
              "    .child(Rect::new(28.0, 36.0))",
              "    .child(Rect::new(28.0, 72.0))",
              "    .child(Rect::new(28.0, 24.0))",
              "",
              "// Stretched: a child with no height takes the row's.",
              "Row::new().align_items(Alignment::Stretch).child(Rect::new(28.0, Dimension::Auto))",
            ],
          ),
        ),
        Section::new(
          "Children that grow",
          "Children with a flex factor share the room left in the row by their factors: 1, 2 and 1 are a quarter, \
           a half and a quarter of it.",
          self.frame.demo(
            3,
            grown(),
            ".flex(2.0)",
            &[
              "Row::new()",
              "    .width(Dimension::Pct(100.0))",
              "    .spacing(4.0)",
              "    .child(Row::new().height(56.0).flex(1.0))",
              "    .child(Row::new().height(56.0).flex(2.0))",
              "    .child(Row::new().height(56.0).flex(1.0))",
            ],
          ),
        ),
        Section::new(
          "Stack",
          "A stack lays its children over each other, the last on top, each where the stack's alignment puts it: \
           here, in its middle. The Position page has the other eight points and children placed by hand.",
          self.frame.demo(
            4,
            stacked(),
            "Stack::new().stack_align(StackAlignment::Center)",
            &[
              "Stack::new()",
              "    .width(Dimension::Pct(100.0))",
              "    .height(220.0)",
              "    .stack_align(StackAlignment::Center)",
              "    .child(Rect::new(180.0, 180.0))",
              "    .child(Rect::new(130.0, 130.0))",
              "    .child(Rect::new(80.0, 80.0))",
            ],
          ),
        ),
        Section::new(
          "A row that wraps",
          "Twelve tiles in a row that wraps: what does not fit on a line goes on to the next. Make the window \
           narrower and the lines get more.",
          self.frame.demo(
            5,
            wrapped(),
            ".wrap()",
            &[
              "Row::new()",
              "    .width(Dimension::Pct(100.0))",
              "    .spacing(10.0)",
              "    .wrap()",
              "    .with_children((0..12).map(|_| Rect::new(72.0, 40.0)))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a row, a column and a stack are given, and what a child of one is.",
          props_table(&rows, &[], self.frame.room()),
        ),
      ],
    )
  }
}
