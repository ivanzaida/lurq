//! The page of the bar chart: its playground, the board's Bar Chart and its stacked form, series side by side,
//! values under zero on a grid, the legend and the tooltip on their own, and its props.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::PaletteColor},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{
    BarChart, ChartLegend, ChartLegendItem, ChartLegendShape, ChartPoint, ChartSeries, ChartSize, ChartTooltip,
    ChartTooltipRow,
  },
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{props_table, quiet},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

const RELEASES: [&str; 6] = ["0.55", "0.56", "0.57", "0.58", "0.59", "0.60"];
const BUILDS: [&str; 5] = ["0.56", "0.57", "0.58", "0.59", "0.60"];

/// The data the playground shows: what the board draws, and what tries a chart.
const DATA: [&str; 7] = [
  "the board's",
  "three series",
  "under zero",
  "one value",
  "long names",
  "no data",
  "loading",
];

/// The data of the playground's choice: its categories and its series.
fn data(choice: usize) -> (Vec<String>, Vec<ChartSeries>) {
  let names = |names: &[&str]| names.iter().map(|name| (*name).to_owned()).collect::<Vec<_>>();
  match choice {
    1 => (names(&BUILDS), phases()),
    2 => (
      names(&["Q1", "Q2", "Q3", "Q4"]),
      vec![ChartSeries::new("Change", [12.0, -8.5, 5.0, -2.5])],
    ),
    3 => (names(&["0.60"]), vec![ChartSeries::new("Frame time", [5.2])]),
    4 => (
      names(&[
        "text_pipeline",
        "canvas_mesh_tessellation",
        "layout_flex",
        "theme_roles_resolved",
      ]),
      vec![ChartSeries::new("Seconds", [9.2, 7.0, 4.7, 2.5])],
    ),
    5 => (Vec::new(), Vec::new()),
    _ => (names(&RELEASES), vec![frame_time()]),
  }
}

fn frame_time() -> ChartSeries {
  ChartSeries::new("Frame time", [11.8, 13.2, 9.6, 8.4, 7.0, 5.2])
}

/// The board's three phases of a frame, for five builds.
fn phases() -> Vec<ChartSeries> {
  vec![
    ChartSeries::new("Layout", [4.0, 3.4, 3.0, 2.6, 2.0]),
    ChartSeries::new("Text", [3.0, 2.8, 2.2, 2.0, 1.8]),
    ChartSeries::new("Paint", [2.4, 2.0, 1.8, 1.4, 1.2]),
  ]
}

fn ms(value: f64) -> String {
  format!("{} ms", lurq::ui::default_value_text(value))
}

pub struct BarChartPage {
  frame: Frame,
  playground: Playground,
  data: Signal<usize>,
  size: Signal<usize>,
  stacked: Signal<bool>,
  current: Signal<bool>,
  grid: Signal<bool>,
  value_axis: Signal<bool>,
  legend: Signal<bool>,
  hidden: Signal<Vec<String>>,
  selected: Signal<Option<ChartPoint>>,
}

impl BarChartPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let choice = self.data.get().min(DATA.len() - 1);
    let (stacked, current, grid, value_axis, legend) = (
      self.stacked.get(),
      self.current.get(),
      self.grid.get(),
      self.value_axis.get(),
      self.legend.get(),
    );
    let size = if self.size.get() == 0 {
      ChartSize::Sm
    } else {
      ChartSize::Md
    };
    let (categories, series) = data(choice);
    let record = self.playground.recorder();
    let (selected_to, hidden_to) = (record.clone(), record);
    let (selected, hidden) = (self.selected.clone(), self.hidden.clone());
    let make = {
      let (categories, series) = (categories.clone(), series.clone());
      move || {
        let mut chart = BarChart::new(categories.clone(), series.clone())
          .title("Frame time")
          .description("Median per build, last six releases")
          .size(size)
          .stacked(stacked)
          .grid(grid)
          .value_axis(value_axis)
          .loading(choice == 6)
          .value_text(ms);
        if current {
          chart = chart.current(categories.len().saturating_sub(1));
        }
        if legend {
          chart = chart.legend(true);
        }
        chart
      }
    };
    let shown_chart = {
      let make = make.clone();
      let (selected, hidden) = (selected.clone(), hidden.clone());
      move |ctx: &mut Ctx| {
        let (selected, selected_to, hidden_to) = (selected.clone(), selected_to.clone(), hidden_to.clone());
        make()
          .hidden(hidden.clone())
          .on_hidden_change(move |hidden| hidden_to(format!("on_hidden_change({hidden:?})")))
          .on_select(move |point| {
            selected.set(Some(point));
            selected_to(format!("on_select(series {}, index {})", point.series, point.index));
          })
          .key("played")
          .mount(ctx)
      }
    };
    let mut code = vec![
      "BarChart::new(categories, [ChartSeries::new(\"Frame time\", values)])".to_owned(),
      "    .title(\"Frame time\")".to_owned(),
      "    .description(\"Median per build, last six releases\")".to_owned(),
      "    .value_text(|ms| format!(\"{ms} ms\"))".to_owned(),
    ];
    if size == ChartSize::Sm {
      code.push("    .size(ChartSize::Sm)".to_owned());
    }
    if stacked {
      code.push("    .stacked(true)".to_owned());
    }
    if current {
      code.push("    .current(last)".to_owned());
    }
    if grid {
      code.push("    .grid(true)".to_owned());
    }
    if value_axis {
      code.push("    .value_axis(true)".to_owned());
    }
    if legend {
      code.push("    .legend(true)".to_owned());
    }
    if choice == 6 {
      code.push("    .loading(true)".to_owned());
    }
    code.push("    .hidden(hidden)".to_owned());
    code.push("    .on_select(|point| ..)".to_owned());
    code.push("    .mount(ctx)".to_owned());
    let tree = vec![
      format!("root      {} · a figure · card, padded 24", self.playground.measured()),
      "├─ header  title 16/600 · description 13".to_owned(),
      "├─ plot    a canvas of 160 · bars 12 apart · one stop of Tab".to_owned(),
      "├─ axis    the categories, mono 11, each under its bar".to_owned(),
      "└─ legend  a dot and a name for each series".to_owned(),
    ];
    let (set_data, set_size) = (self.data.clone(), self.size.clone());
    let datas: Vec<(&str, usize)> = DATA.iter().enumerate().map(|(index, name)| (*name, index)).collect();
    let sizes = [("sm", 0_usize), ("md", 1)];
    let reset = (
      self.data.clone(),
      self.size.clone(),
      self.stacked.clone(),
      self.current.clone(),
      self.grid.clone(),
      self.value_axis.clone(),
      self.legend.clone(),
      self.hidden.clone(),
      self.selected.clone(),
    );
    let facts = format!(
      "{} categories · {} series{}",
      categories.len(),
      series.len(),
      self
        .selected
        .get()
        .map(|point| format!(" · selected {}", point.index))
        .unwrap_or_default()
    );
    let still = {
      let make = make.clone();
      move || -> Element { make().into() }
    };
    self.playground.card(
      ctx,
      Spec {
        title: "BarChart",
        make: Arc::new(still),
        shown: Shown::built(shown_chart),
        fill: true,
        forceable: true,
        facts,
        event: "on_select / on_hidden_change",
        code,
        tree,
        controls: vec![
          prop(
            "data",
            "categories, series",
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
          divider(),
          flag("stacked", &self.stacked),
          flag("current: the last category", &self.current),
          flag("grid", &self.grid),
          flag("value_axis", &self.value_axis),
          flag("legend", &self.legend),
        ],
        changed: [
          choice != 0,
          self.size.get() != 1,
          stacked,
          !current,
          grid,
          value_axis,
          legend,
          !self.hidden.get().is_empty(),
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          reset.0.set(0);
          reset.1.set(1);
          reset.2.set(false);
          reset.3.set(true);
          reset.4.set(false);
          reset.5.set(false);
          reset.6.set(false);
          reset.7.set(Vec::new());
          reset.8.set(None);
        }),
      },
    )
  }
}

impl Component for BarChartPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      data: ctx.signal(0),
      size: ctx.signal(1),
      stacked: ctx.signal(false),
      current: ctx.signal(true),
      grid: ctx.signal(false),
      value_axis: ctx.signal(false),
      legend: ctx.signal(false),
      hidden: ctx.signal(Vec::new()),
      selected: ctx.signal(None),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = BarChart::new(RELEASES, [frame_time()])
      .title("Frame time")
      .description("Median per build, last six releases")
      .current(5)
      .value_text(ms)
      .key("board")
      .mount(ctx);
    let side = BarChart::new(BUILDS, phases())
      .title("Frame budget")
      .description("Milliseconds by phase")
      .value_text(ms)
      .key("side")
      .mount(ctx);
    let stacked = BarChart::new(BUILDS, phases())
      .title("Frame budget")
      .description("Milliseconds by phase")
      .size(ChartSize::Sm)
      .stacked(true)
      .value_text(ms)
      .key("stacked")
      .mount(ctx);
    let under = BarChart::new(
      ["Q1", "Q2", "Q3", "Q4", "Q1 '27"],
      [ChartSeries::new("Change", [12.0, -8.5, 5.0, -2.5, 9.0])],
    )
    .title("Frame time against the last release")
    .description("Milliseconds saved, under zero where a release was slower")
    .grid(true)
    .value_axis(true)
    .value_text(|change| format!("{change:+} ms"))
    .key("under")
    .mount(ctx);
    let pieces = Column::new()
      .spacing(24.0)
      .align_items(Alignment::Start)
      .child(
        ChartTooltip::new("Build 0.59")
          .row(ChartTooltipRow::new(PaletteColor::Chart1, "wgpu", "6.9 ms"))
          .row(ChartTooltipRow::new(PaletteColor::Chart2, "DirectX 12", "9.4 ms")),
      )
      .child(ChartLegend::new([
        ChartLegendItem::new("Layout", PaletteColor::Chart1),
        ChartLegendItem::new("Text", PaletteColor::Chart2),
        ChartLegendItem::new("Paint", PaletteColor::Chart3),
        ChartLegendItem::new("Other", PaletteColor::Chart4),
      ]))
      .child(
        ChartLegend::new([
          ChartLegendItem::new("wgpu", PaletteColor::Chart1),
          ChartLegendItem::new("DirectX 12", PaletteColor::Chart2),
        ])
        .shape(ChartLegendShape::Dot),
      );

    let rows = [
      (
        "categories",
        "impl IntoIterator<Item = impl AsRef<str>>",
        "",
        "The names under the bars, in their order.",
      ),
      (
        "series",
        "impl IntoIterator<Item = ChartSeries>",
        "",
        "ChartSeries::new(name, values): a value for each category; .color(..) over the theme's chart color.",
      ),
      (
        "title / description / label",
        "&str",
        "none",
        "The header, and the name to tooling (the title's).",
      ),
      (
        "size",
        "ChartSize",
        "Md",
        "Md: padded 24, a plot of 160. Sm: padded 20, a plot of 140.",
      ),
      ("height", "impl Into<Length>", "the size's", "The plot's height."),
      (
        "stacked",
        "bool",
        "false",
        "The series one on another in each category, the first at the bottom.",
      ),
      (
        "current",
        "usize",
        "none",
        "The current category: the others in the input's color.",
      ),
      (
        "grid / value_axis",
        "bool",
        "false",
        "Lines across the plot at its ticks, and their numbers.",
      ),
      ("legend", "bool", "more than one series", "The legend under the plot."),
      (
        "hidden",
        "Signal<Vec<String>>",
        "none",
        "The series hidden; a press of the legend's item hides or shows one. hidden_controlled, on_hidden_change.",
      ),
      (
        "value_text",
        "Fn(f64) -> String",
        "whole or two places",
        "Every number it writes.",
      ),
      (
        "empty_text / loading",
        "&str / bool",
        "\"No data\" / false",
        "No data, and the skeleton while it comes.",
      ),
      (
        "on_select",
        "Fn(ChartPoint)",
        "",
        "A press of a bar, or Enter or Space on the focused one.",
      ),
      (
        "mount",
        "&mut Ctx",
        "",
        "The tooltip under the pointer and the focus, the arrows between bars and series; converted it is still.",
      ),
    ];

    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Charts", "Bar Chart"],
        title: "Bar Chart",
        description: "A value for each category, as a bar from zero: the frame time of each release, the budget of a \
                      frame by phase. Drawn on a canvas, again only when its data, its size or its theme changes.",
        import: Some("use lurq::ui::{BarChart, ChartSeries};"),
        status: Some("Preview"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of BarChart is a control on the right, and the data too. Point at a bar for its tooltip, or Tab \
           into the plot and go between bars with the arrows; a press or Enter selects one.",
          playground,
        ),
        Section::new(
          "The board's bar chart",
          "Six releases, the current one in the first chart color and the others in the input's, 12 between bars, the \
           names under them in the mono type.",
          self.frame.demo(
            0,
            Column::new().width(crate::kit::FILL).child(board),
            "BarChart::new(releases, [frame_time]).current(5)",
            &[
              "BarChart::new(",
              "    [\"0.55\", \"0.56\", \"0.57\", \"0.58\", \"0.59\", \"0.60\"],",
              "    [ChartSeries::new(\"Frame time\", [11.8, 13.2, 9.6, 8.4, 7.0, 5.2])],",
              ")",
              ".title(\"Frame time\")",
              ".description(\"Median per build, last six releases\")",
              ".current(5)",
              ".value_text(|ms| format!(\"{ms} ms\"))",
              ".mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Series side by side",
          "Each series takes the next chart color; a category's bars stand 4 apart, and the legend under the plot \
           names them.",
          self.frame.demo(
            1,
            Column::new().width(crate::kit::FILL).child(side),
            "BarChart::new(builds, phases)",
            &["BarChart::new(builds, [layout, text, paint]).title(\"Frame budget\").mount(ctx)"],
          ),
        ),
        Section::new(
          "Stacked",
          "The board's Stacked Bar Chart: the phases one on another, the first at the bottom, a pixel of the card \
           between them, the top of each stack rounded.",
          self.frame.demo(
            2,
            Row::new()
              .width(crate::kit::FILL)
              .child(Column::new().width(320.0).child(stacked)),
            ".stacked(true)",
            &["BarChart::new(builds, [layout, text, paint]).size(ChartSize::Sm).stacked(true).mount(ctx)"],
          ),
        ),
        Section::new(
          "Under zero, on a grid",
          "A value under zero goes down from it; the grid is a line at each tick, and the ticks' numbers are written \
           over their lines.",
          self.frame.demo(
            3,
            Column::new().width(crate::kit::FILL).child(under),
            ".grid(true).value_axis(true)",
            &["BarChart::new(quarters, [change]).grid(true).value_axis(true).value_text(|v| format!(\"{v:+} ms\"))"],
          ),
        ),
        Section::new(
          "The tooltip and the legend",
          "Every chart's, as the board draws them, and on their own: the tooltip's title over a row for each series, \
           the legend's square keys, and the dots a chart's own legend has.",
          self.frame.demo(
            4,
            Column::new()
              .width(crate::kit::FILL)
              .child(pieces)
              .child(quiet("ChartTooltip · ChartLegend · ChartLegend::shape(Dot)", 12.0)),
            "ChartTooltip::new(title).row(..)",
            &[
              "ChartTooltip::new(\"Build 0.59\")",
              "    .row(ChartTooltipRow::new(PaletteColor::Chart1, \"wgpu\", \"6.9 ms\"))",
              "ChartLegend::new([ChartLegendItem::new(\"Layout\", PaletteColor::Chart1), ..])",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a bar chart is given; every chart has the same words for the same things.",
          props_table(&rows, &["categories", "series"], self.frame.room()),
        ),
      ],
    )
  }
}
