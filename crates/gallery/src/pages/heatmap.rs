//! The page of the heatmap: its playground, the board's Heatmap, a year of weeks that fits its frame, values under zero
//! on a range of the app's, and its props.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  node::Element,
  ui::{ChartPoint, ChartSeries, ChartSize, Heatmap},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::props_table,
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

const DAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/// The data the playground shows: what the board draws, and what tries a heatmap.
const DATA: [&str; 7] = [
  "the board's",
  "a year",
  "under zero",
  "missing values",
  "one row",
  "no data",
  "loading",
];

fn weeks(count: usize) -> Vec<String> {
  (1..=count).map(|week| format!("Week {week}")).collect()
}

/// Commits per day for `count` weeks, 0 to 8: every step of the scale.
fn commits(count: usize) -> Vec<ChartSeries> {
  DAYS
    .iter()
    .enumerate()
    .map(|(day, name)| ChartSeries::new(name, (0..count).map(move |week| ((week * 7 + day) * 37 % 9) as f64)))
    .collect()
}

/// The data of the playground's choice: the columns and the rows.
fn data(choice: usize) -> (Vec<String>, Vec<ChartSeries>) {
  match choice {
    1 => (weeks(52), commits(52)),
    2 => temperatures(),
    3 => (
      weeks(12),
      commits(12)
        .into_iter()
        .enumerate()
        .map(|(day, row)| {
          let values: Vec<f64> = row
            .values()
            .iter()
            .enumerate()
            .map(|(week, value)| if (week + day) % 5 == 0 { f64::NAN } else { *value })
            .collect();
          ChartSeries::new(row.name(), values)
        })
        .collect(),
    ),
    4 => (weeks(18), vec![commits(18).remove(0)]),
    5 => (Vec::new(), Vec::new()),
    _ => (weeks(18), commits(18)),
  }
}

/// Mean temperatures by hour of four cities, under zero and over it.
fn temperatures() -> (Vec<String>, Vec<ChartSeries>) {
  let hours: Vec<String> = (0..24).map(|hour| format!("{hour:02}:00")).collect();
  let city = |name: &str, base: f64| {
    ChartSeries::new(
      name,
      (0..24).map(move |hour| base + 6.0 * ((hour as f64 - 14.0) / 24.0 * std::f64::consts::TAU).cos()),
    )
  };
  (
    hours,
    vec![
      city("Oslo", -6.0),
      city("Kyiv", -1.0),
      city("Lisbon", 11.0),
      city("Cairo", 18.0),
    ],
  )
}

fn degrees(value: f64) -> String {
  format!("{value:.1} °C")
}

const STEPS: [(&str, usize); 3] = [("3", 3), ("5", 5), ("7", 7)];

pub struct HeatmapPage {
  frame: Frame,
  playground: Playground,
  data: Signal<usize>,
  size: Signal<usize>,
  steps: Signal<usize>,
  column_axis: Signal<bool>,
  legend: Signal<bool>,
  selected: Signal<Option<ChartPoint>>,
  motion: Signal<bool>,
}

impl HeatmapPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let choice = self.data.get().min(DATA.len() - 1);
    let motion = self.motion.get();
    let (column_axis, legend) = (self.column_axis.get(), self.legend.get());
    let steps = self.steps.get();
    let size = if self.size.get() == 1 {
      ChartSize::Md
    } else {
      ChartSize::Sm
    };
    let (columns, rows) = data(choice);
    let record = self.playground.recorder();
    let selected = self.selected.clone();
    let make = {
      let (columns, rows) = (columns.clone(), rows.clone());
      move || {
        let mut chart = Heatmap::new(columns.clone(), rows.clone())
          .title(if choice == 2 { "Temperature" } else { "Commits" })
          .description(if choice == 2 {
            "Mean by hour, four cities"
          } else {
            "Per day, by week"
          })
          .size(size)
          .steps(steps)
          .column_axis(column_axis)
          .legend(legend)
          .loading(choice == 6)
          .motion(motion);
        if choice == 2 {
          chart = chart
            .range(-12.0, 24.0)
            .value_text(degrees)
            .less_text("Colder")
            .more_text("Warmer");
        }
        chart
      }
    };
    let shown_chart = {
      let stage = self.playground.styled(make());
      move |ctx: &mut Ctx| {
        let (selected, record) = (selected.clone(), record.clone());
        stage
          .on_select(move |point| {
            selected.set(Some(point));
            record(format!("on_select(series {}, index {})", point.series, point.index));
          })
          .key("played")
          .mount(ctx)
      }
    };
    let mut code = vec![
      "Heatmap::new(weeks, days)".to_owned(),
      "    .title(\"Commits\")".to_owned(),
      "    .description(\"Per day, by week\")".to_owned(),
    ];
    if size == ChartSize::Md {
      code.push("    .size(ChartSize::Md)".to_owned());
    }
    if steps != 5 {
      code.push(format!("    .steps({steps})"));
    }
    if choice == 2 {
      code.push("    .range(-12.0, 24.0)".to_owned());
      code.push("    .less_text(\"Colder\").more_text(\"Warmer\")".to_owned());
    }
    if column_axis {
      code.push("    .column_axis(true)".to_owned());
    }
    if !legend {
      code.push("    .legend(false)".to_owned());
    }
    if choice == 6 {
      code.push("    .loading(true)".to_owned());
    }
    if !motion {
      code.push("    .motion(false)".to_owned());
    }
    code.push("    .on_select(|point| ..)".to_owned());
    code.push("    .mount(ctx)".to_owned());
    let tree = vec![
      format!("root      {} · a figure · card, padded 20", self.playground.measured()),
      "├─ header  title 15/600 · description 12".to_owned(),
      "├─ plot    a canvas · the rows' names, mono 10 · cells of 14, 4 apart".to_owned(),
      "└─ scale   Less · a step of each color, 12 · More".to_owned(),
    ];
    let (set_data, set_size, set_steps) = (self.data.clone(), self.size.clone(), self.steps.clone());
    let datas: Vec<(&str, usize)> = DATA.iter().enumerate().map(|(index, name)| (*name, index)).collect();
    let sizes = [("sm", 0_usize), ("md", 1)];
    let reset = (
      self.data.clone(),
      self.size.clone(),
      self.steps.clone(),
      self.column_axis.clone(),
      self.legend.clone(),
      self.selected.clone(),
      self.motion.clone(),
    );
    let facts = format!(
      "{} rows · {} columns{}",
      rows.len(),
      columns.len(),
      self
        .selected
        .get()
        .map(|point| format!(" · selected {}, {}", point.series, point.index))
        .unwrap_or_default()
    );
    let still = {
      let make = make.clone();
      move || -> Element { make().into() }
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Heatmap",
        make: Arc::new(still),
        shown: Shown::built(shown_chart),
        fill: true,
        forceable: true,
        facts,
        event: "on_select",
        code,
        tree,
        controls: vec![
          prop(
            "data",
            "columns, rows",
            self
              .playground
              .select(0, &datas, &choice, move |index| set_data.set(index)),
          ),
          prop(
            "size",
            "ChartSize",
            self
              .playground
              .select(1, &sizes, &self.size.get().min(1), move |index| set_size.set(index)),
          ),
          prop(
            "steps",
            "usize",
            self
              .playground
              .select(2, &STEPS, &steps, move |index| set_steps.set(index)),
          ),
          divider(),
          flag("column_axis", &self.column_axis),
          flag("legend: the scale", &self.legend),
          flag("motion: in, and to a change of the data", &self.motion),
        ],
        changed: [
          choice != 0,
          self.size.get() != 0,
          steps != 5,
          column_axis,
          !legend,
          !motion,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          reset.0.set(0);
          reset.1.set(0);
          reset.2.set(5);
          reset.3.set(false);
          reset.4.set(true);
          reset.5.set(None);
          reset.6.set(true);
        }),
      },
    )
  }
}

impl Component for HeatmapPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 3),
      data: ctx.signal(0),
      size: ctx.signal(0),
      steps: ctx.signal(5),
      column_axis: ctx.signal(false),
      legend: ctx.signal(true),
      selected: ctx.signal(None),
      motion: ctx.signal(true),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = Heatmap::new(weeks(18), commits(18))
      .title("Commits")
      .description("Per day, last 18 weeks")
      .key("board")
      .mount(ctx);
    let year = Heatmap::new(weeks(52), commits(52))
      .title("Commits")
      .description("Per day, the last year")
      .column_axis(true)
      .key("year")
      .mount(ctx);
    let (hours, cities) = temperatures();
    let warm = Heatmap::new(hours, cities)
      .title("Temperature")
      .description("Mean by hour, four cities")
      .range(-12.0, 24.0)
      .steps(7)
      .value_text(degrees)
      .less_text("Colder")
      .more_text("Warmer")
      .key("warm")
      .mount(ctx);

    let rows = [
      (
        "columns",
        "impl IntoIterator<Item = impl AsRef<str>>",
        "",
        "The columns' names, in their order: the tooltip's title.",
      ),
      (
        "rows",
        "impl IntoIterator<Item = ChartSeries>",
        "",
        "ChartSeries::new(name, values): a row, a value for each column; a value that is not a number has no cell.",
      ),
      (
        "steps",
        "usize",
        "5",
        "The scale's steps: Muted at the least, Chart1 at its share of opacity for the others.",
      ),
      (
        "range",
        "(f64, f64)",
        "zero to the most",
        "The values the scale runs between; at or under the least is the lowest step.",
      ),
      (
        "cell_size",
        "f32",
        "14",
        "A cell's side; smaller, its gaps with it, where the grid is wider than the frame, the grid as high as they are.",
      ),
      (
        "column_axis",
        "bool",
        "false",
        "The columns' names under the grid, as many as have room.",
      ),
      (
        "less_text / more_text",
        "&str",
        "\"Less\" / \"More\"",
        "The scale's words at its ends.",
      ),
      (
        "missing_text",
        "&str",
        "\"No value\"",
        "What the tooltip and tooling say of a cell with no value.",
      ),
      (
        "title / description / label",
        "&str",
        "none",
        "The header, and the name to tooling (the title's).",
      ),
      ("size", "ChartSize", "Sm", "Sm: padded 20. Md: padded 24."),
      (
        "height",
        "impl Into<Length>",
        "the rows'",
        "The grid's height: the cells as tall as the rows can be in it.",
      ),
      ("legend", "bool", "true", "The scale under the grid."),
      (
        "value_text",
        "Fn(f64) -> String",
        "whole or two places",
        "Every value it writes: the tooltip's, what tooling reads.",
      ),
      (
        "empty_text / loading",
        "&str / bool",
        "\"No data\" / false",
        "No data, and the skeleton while it comes.",
      ),
      (
        "motion",
        "bool",
        "true",
        "Cells fading in, and each going to its new step's color on a change of the data; off where motion is reduced, still while it is resized.",
      ),
      (
        "on_select",
        "Fn(ChartPoint)",
        "",
        "A press of a cell, or Enter or Space on the focused one: ChartPoint { series: row, index: column }.",
      ),
      (
        "mount",
        "&mut Ctx",
        "",
        "The cell under the pointer or the focus ringed with its tooltip; the arrows from cell to cell.",
      ),
    ];

    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Charts", "Heatmap"],
        title: "Heatmap",
        description: "A grid of cells, a row for each series and a column for each category, each colored by its \
                      value's step on a scale: commits per day, temperature by hour.",
        import: Some("use lurq::ui::{ChartSeries, Heatmap};"),
        status: Some("Preview"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Heatmap is a control on the right, and the data too. Point at a cell for its tooltip, or Tab \
           into the grid and go from cell to cell with the arrows.",
          playground,
        ),
        Section::new(
          "The board's heatmap",
          "Seven days by eighteen weeks, cells of 14 four apart, every other day named, the scale under the grid.",
          self.frame.demo(
            0,
            Row::new()
              .width(crate::kit::FILL)
              .child(Column::new().width(420.0).max_width(crate::kit::FILL).child(board)),
            "Heatmap::new(weeks, days)",
            &[
              "Heatmap::new(weeks, [\"Mon\", \"Tue\", \"Wed\", \"Thu\", \"Fri\", \"Sat\", \"Sun\"].map(|day| {",
              "    ChartSeries::new(day, commits_on(day))",
              "}))",
              ".title(\"Commits\")",
              ".description(\"Per day, last 18 weeks\")",
              ".mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "A year in its frame",
          "Fifty-two weeks: where the grid is wider than its frame its cells are smaller, its gaps with them, and the \
           grid as high as they are; the weeks' names written where they have room.",
          self.frame.demo(
            1,
            Column::new().width(crate::kit::FILL).child(year),
            ".column_axis(true)",
            &["Heatmap::new(weeks, days).column_axis(true).mount(ctx)"],
          ),
        ),
        Section::new(
          "Under zero, on the app's range",
          "Temperatures from -12 to 24 on seven steps, the scale's words the app's.",
          self.frame.demo(
            2,
            Column::new().width(crate::kit::FILL).child(warm),
            ".range(-12.0, 24.0).steps(7)",
            &[
              "Heatmap::new(hours, cities)",
              "    .range(-12.0, 24.0)",
              "    .steps(7)",
              "    .value_text(|t| format!(\"{t:.1} °C\"))",
              "    .less_text(\"Colder\").more_text(\"Warmer\")",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a heatmap is given; every chart has the same words for the same things.",
          props_table(&rows, &["columns", "rows"], self.frame.room()),
        ),
      ],
    )
  }
}
