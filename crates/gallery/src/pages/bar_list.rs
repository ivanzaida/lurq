//! The page of the bar list: its playground, the board's slowest tests, rows that are pressed, and the rows without
//! their box in a card of the app's.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::PaletteColor},
  components::Column,
  core::Signal,
  node::Element,
  ui::{BarList, BarListItem, BarListPart, Card, Length},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, segmented_fill},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// The chart colors a list may be drawn in, as the code names them.
const COLORS: [(&str, PaletteColor); 5] = [
  ("Chart1", PaletteColor::Chart1),
  ("Chart2", PaletteColor::Chart2),
  ("Chart3", PaletteColor::Chart3),
  ("Chart4", PaletteColor::Chart4),
  ("Chart5", PaletteColor::Chart5),
];

/// How many rows the playground's list has.
const COUNTS: [(&str, usize); 4] = [("none", 0), ("3", 3), ("5", 5), ("8", 8)];

/// The tests of a run and their seconds, slowest first.
const TESTS: [(&str, f64); 8] = [
  ("text_pipeline", 9.2),
  ("canvas_mesh", 7.0),
  ("layout_flex", 4.7),
  ("theme_roles", 2.5),
  ("focus_groups", 1.8),
  ("glyph_atlas", 1.1),
  ("scroll_state", 0.6),
  ("signals", 0.2),
];

/// The board's bar list: the slowest tests of a run.
fn board() -> BarList {
  BarList::new(TESTS.iter().take(4).copied())
    .title("Slowest tests")
    .description("Seconds")
    .style(BarListPart::Root, |s| s.width(300.0).max_width(Length::Fill))
}

/// What the playground's panel says of the list.
#[derive(Clone, Copy)]
struct Props {
  count: usize,
  /// Which of [`COLORS`].
  color: usize,
  long: bool,
  negative: bool,
  header: bool,
  framed: bool,
  units: bool,
  select: bool,
}

pub struct BarListPage {
  frame: Frame,
  playground: Playground,
  /// Which of [`COUNTS`] and [`COLORS`] are chosen.
  count: Signal<usize>,
  color: Signal<usize>,
  long: Signal<bool>,
  negative: Signal<bool>,
  header: Signal<bool>,
  framed: Signal<bool>,
  units: Signal<bool>,
  select: Signal<bool>,
}

/// The playground's list, as `props` make it, telling `record` which row was pressed.
fn tests(props: Props, record: Option<Arc<dyn Fn(String) + Send + Sync>>) -> BarList {
  let rows = TESTS
    .iter()
    .take(props.count)
    .enumerate()
    .map(|(index, (name, seconds))| {
      let name = if props.long && index == 1 {
        "canvas_mesh_tessellates_every_path_of_the_board"
      } else {
        name
      };
      let seconds = if props.negative && index + 1 == props.count {
        -1.5
      } else {
        *seconds
      };
      BarListItem::new(name, seconds)
    });
  let mut list = BarList::new(rows)
    .color(COLORS[props.color].1.clone())
    .framed(props.framed)
    .style(BarListPart::Root, |s| s.width(320.0));
  if props.header {
    list = list.title("Slowest tests").description("Seconds");
  } else {
    list = list.label("Slowest tests");
  }
  if props.units {
    list = list.value_text(|seconds| format!("{seconds:.1} s"));
  }
  if props.select {
    list = list.on_select(move |name| {
      if let Some(record) = &record {
        record(format!("on_select(\"{name}\")"));
      }
    });
  }
  list
}

impl BarListPage {
  fn props(&self) -> Props {
    Props {
      count: COUNTS[self.count.get().min(COUNTS.len() - 1)].1,
      color: self.color.get().min(COLORS.len() - 1),
      long: self.long.get(),
      negative: self.negative.get(),
      header: self.header.get(),
      framed: self.framed.get(),
      units: self.units.get(),
      select: self.select.get(),
    }
  }

  /// The playground: the slowest tests as the panel's data and props make them.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let props = self.props();
    let record: Arc<dyn Fn(String) + Send + Sync> = Arc::new(self.playground.recorder());
    let shown = tests(props, Some(record)).force(if props.select { self.playground.forced() } else { None });
    let make = move || -> Element { tests(props, None).into() };

    let mut code = vec![format!(
      "BarList::new([(\"text_pipeline\", 9.2), (\"canvas_mesh\", 7.0), ..])   // {} rows",
      props.count
    )];
    if props.header {
      code.push("    .title(\"Slowest tests\")".to_owned());
      code.push("    .description(\"Seconds\")".to_owned());
    } else {
      code.push("    .label(\"Slowest tests\")".to_owned());
    }
    if props.color != 0 {
      code.push(format!(
        "    .color(PaletteColor::{})",
        COLORS[self.color.get().min(4)].0
      ));
    }
    if !props.framed {
      code.push("    .framed(false)".to_owned());
    }
    if props.units {
      code.push("    .value_text(|seconds| format!(\"{seconds:.1} s\"))".to_owned());
    }
    if props.select {
      code.push("    .on_select(|name| /* open the test */)".to_owned());
    }

    let tree = vec![
      format!(
        "root        {}   {}",
        self.playground.measured(),
        if props.framed { "the card, 20 in" } else { "no box" }
      ),
      "├─ header   title 15 at 600, description 12 muted, 3 apart".to_owned(),
      "└─ rows     16 apart".to_owned(),
      "   └─ row     name 92 mono 11 · track 26 · value mono 11, 10 apart".to_owned(),
      "      └─ bar    14, radius sm, the scale's share of its track".to_owned(),
    ];
    let segmented = |signal: &Signal<usize>, names: Vec<(&'static str, usize)>| {
      let choose = signal.clone();
      segmented_fill(&names, &signal.get(), move |index| choose.set(index))
    };
    let places = |names: &[&'static str]| -> Vec<(&'static str, usize)> {
      names
        .iter()
        .copied()
        .enumerate()
        .map(|(index, name)| (name, index))
        .collect()
    };
    let controls = vec![
      prop(
        "rows",
        "data",
        segmented(&self.count, places(&COUNTS.map(|(name, _)| name))),
      ),
      flag("a long name", &self.long),
      flag("a negative value", &self.negative),
      divider(),
      prop(
        "color",
        "PaletteColor",
        segmented(&self.color, places(&COLORS.map(|(name, _)| name))),
      ),
      flag("title / description", &self.header),
      flag("framed", &self.framed),
      flag("value_text (seconds)", &self.units),
      flag("on_select", &self.select),
    ];
    let changed = [
      self.count.get() != 2,
      self.color.get() != 0,
      props.long,
      props.negative,
      !props.header,
      !props.framed,
      props.units,
      props.select,
    ]
    .into_iter()
    .filter(|changed| *changed)
    .count();
    let resets = (
      [self.count.clone(), self.color.clone()],
      [
        (self.long.clone(), false),
        (self.negative.clone(), false),
        (self.header.clone(), true),
        (self.framed.clone(), true),
        (self.units.clone(), false),
        (self.select.clone(), false),
      ],
    );
    self.playground.card(
      ctx,
      Spec {
        title: "Bar List",
        make: Arc::new(make),
        shown: Shown::of(self.playground.styled(shown)),
        fill: false,
        forceable: props.select,
        facts: format!("{} rows", props.count),
        event: if props.select { "on_select" } else { "" },
        code,
        tree,
        controls,
        changed,
        reset: Arc::new(move || {
          resets.0[0].set(2);
          resets.0[1].set(0);
          for (signal, value) in &resets.1 {
            signal.set(*value);
          }
        }),
      },
    )
  }
}

impl Component for BarListPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      count: ctx.signal(2),
      color: ctx.signal(0),
      long: ctx.signal(false),
      negative: ctx.signal(false),
      header: ctx.signal(true),
      framed: ctx.signal(true),
      units: ctx.signal(false),
      select: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let folders = BarList::new([
      BarListItem::new("target/", 27.4),
      BarListItem::new("node_modules/", 3.1),
      BarListItem::new("pencil/", 1.6),
      BarListItem::new("docs/", 0.4),
    ])
    .title("Largest folders")
    .description("Gigabytes on disk")
    .value_text(|gigabytes| format!("{gigabytes:.1} GB"))
    .color(PaletteColor::Chart2)
    .on_select(|_| {})
    .style(BarListPart::Root, |s| s.width(320.0).max_width(Length::Fill));
    // The card is as wide as the design draws it, and no wider than the stage.
    let in_card = Column::new().width(340.0).max_width(FILL).child(
      Card::new()
        .title("Pages")
        .description("Visits this week")
        .child(
          BarList::new([
            ("/timeline", 1840),
            ("/table", 1325),
            ("/button", 960),
            ("/bar-list", 412),
          ])
          .label("Pages visited most")
          .framed(false),
        )
        .width(Length::Fill),
    );

    let rows = [
      (
        "new",
        "IntoIterator<Item = impl Into<BarListItem>>",
        "",
        "The rows in the app's order: BarListItem::new(name, value) or (name, value). item and items add more.",
      ),
      (
        "title / description",
        "&str",
        "",
        "Over the rows: what they are, and the line under it.",
      ),
      ("label", "&str", "the title", "What the list is called, to tooling."),
      (
        "max",
        "impl Into<f64>",
        "rounded up",
        "The value a bar fills its track at: the largest value rounded up to 1, 2, 2.5 or 5 of a power of ten.",
      ),
      (
        "color",
        "impl Into<BackgroundColor>",
        "Chart1",
        "The role of the bars; BarListItem::color for one row's.",
      ),
      (
        "value_text",
        "Fn(f64) -> String",
        "the decimals needed",
        "Writes a row's value as the app does.",
      ),
      ("empty_text", "&str", "No data", "What a list with no rows says."),
      (
        "framed",
        "bool",
        "true",
        "The board's card; off, the rows stand on what holds them.",
      ),
      (
        "on_select",
        "Fn(&str)",
        "",
        "Told the row's key or name: every row is pressed, the rows one stop of Tab with the arrows between them.",
      ),
      (
        "BarListItem",
        "on_select, disabled, key, id, color",
        "",
        "A row's own handler, called first; a row that takes nothing; what on_select is told in place of its name.",
      ),
      (
        "style / rules",
        "BarListPart",
        "",
        "Root, Header, Title, Description, Rows, Row, Name, Track, Bar, Value, Empty.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Charts", "Bar List"],
        title: "Bar List",
        description: "Named values as bars of one scale: the slowest tests of a run, the largest folders, the \
                      pages visited most.",
        import: Some("use lurq::ui::{BarList, BarListItem};"),
        status: Some("Preview"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of BarList is a control on the right, and the data too: no rows, a long name, a negative \
           value. A bar list is read, not operated: with on_select its rows are pressed, and each press is counted.",
          playground,
        ),
        Section::new(
          "Slowest tests",
          "The board's bar list. The bars share a scale from 0 to the largest value rounded up, 10 here: 9.2 is \
           92% of its track. The values are written with the decimals the most precise of them needs.",
          self.frame.demo(
            0,
            board(),
            "BarList::new([(name, value), ..])",
            &[
              "BarList::new([",
              "    (\"text_pipeline\", 9.2),",
              "    (\"canvas_mesh\", 7.0),",
              "    (\"layout_flex\", 4.7),",
              "    (\"theme_roles\", 2.5),",
              "])",
              ".title(\"Slowest tests\")",
              ".description(\"Seconds\")",
            ],
          ),
        ),
        Section::new(
          "Rows that are pressed",
          "With on_select every row is pressed and the list is told its name. The rows are one stop of Tab: the \
           arrows, Home and End move between them, a letter to the next name it starts.",
          self.frame.demo(
            1,
            folders,
            ".on_select(|name| ..)",
            &[
              "BarList::new([BarListItem::new(\"target/\", 27.4), ..])",
              "    .title(\"Largest folders\")",
              "    .description(\"Gigabytes on disk\")",
              "    .value_text(|gigabytes| format!(\"{gigabytes:.1} GB\"))",
              "    .color(PaletteColor::Chart2)",
              "    .on_select(|folder| /* open it */)",
            ],
          ),
        ),
        Section::new(
          "In a card of the app's",
          "framed(false) leaves the box out: the rows stand on what holds them.",
          self.frame.demo(
            2,
            in_card,
            ".framed(false)",
            &[
              "Card::new()",
              "    .title(\"Pages\")",
              "    .description(\"Visits this week\")",
              "    .child(BarList::new(pages).label(\"Pages visited most\").framed(false))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a bar list and its rows are given.",
          props_table(&rows, &["new"], self.frame.room()),
        ),
      ],
    )
  }
}
