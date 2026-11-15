//! The page of the area chart: its playground, the board's Area Chart, stacked areas, and its props.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::Column,
  core::Signal,
  node::Element,
  ui::{AreaChart, ChartPoint, ChartSeries, LineCurve},
};

use super::line_chart::{BUILDS, directx, ms, wgpu};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

const MINUTES: [&str; 8] = ["0", "5", "10", "15", "20", "25", "30", "35"];
const MINUTE_LABELS: [&str; 5] = ["0 min", "10", "20", "30", "40"];

/// The data the playground shows.
const DATA: [&str; 7] = [
  "the board's",
  "three series",
  "two series",
  "under zero",
  "one value",
  "no data",
  "loading",
];

fn resident() -> ChartSeries {
  ChartSeries::new("Resident", [380.0, 520.0, 470.0, 720.0, 670.0, 920.0, 870.0, 1060.0])
}

/// A frame's three phases over seven builds.
fn phases() -> Vec<ChartSeries> {
  vec![
    ChartSeries::new("Layout", [4.0, 3.6, 3.4, 3.0, 2.6, 2.2, 2.0]),
    ChartSeries::new("Text", [3.0, 2.8, 2.8, 2.4, 2.0, 1.9, 1.8]),
    ChartSeries::new("Paint", [2.4, 2.2, 2.0, 1.8, 1.6, 1.4, 1.2]),
  ]
}

fn megabytes(value: f64) -> String {
  format!("{} MB", lurq::ui::default_value_text(value))
}

/// The data of the playground's choice: its categories, its series, the texts under the plot, and how its numbers
/// are written.
fn data(choice: usize) -> (Vec<String>, Vec<ChartSeries>, Option<Vec<String>>, fn(f64) -> String) {
  let names = |names: &[&str]| names.iter().map(|name| (*name).to_owned()).collect::<Vec<_>>();
  match choice {
    1 => (names(&BUILDS), phases(), None, ms),
    2 => (names(&BUILDS), vec![wgpu(), directx()], None, ms),
    3 => (
      names(&["Q1", "Q2", "Q3", "Q4", "Q1 '27"]),
      vec![ChartSeries::new("Balance", [120.0, -40.0, 60.0, -15.0, 90.0])],
      None,
      megabytes,
    ),
    4 => (
      names(&["0"]),
      vec![ChartSeries::new("Resident", [380.0])],
      None,
      megabytes,
    ),
    5 => (Vec::new(), Vec::new(), None, megabytes),
    _ => (
      names(&MINUTES),
      vec![resident()],
      Some(names(&MINUTE_LABELS)),
      megabytes,
    ),
  }
}

pub struct AreaChartPage {
  frame: Frame,
  playground: Playground,
  data: Signal<usize>,
  stacked: Signal<bool>,
  smooth: Signal<bool>,
  grid: Signal<bool>,
  legend: Signal<bool>,
  motion: Signal<bool>,
  hidden: Signal<Vec<String>>,
  selected: Signal<Option<ChartPoint>>,
}

impl AreaChartPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let choice = self.data.get().min(DATA.len() - 1);
    let (stacked, smooth, grid, legend) = (
      self.stacked.get(),
      self.smooth.get(),
      self.grid.get(),
      self.legend.get(),
    );
    let motion = self.motion.get();
    let (categories, series, labels, text) = data(choice);
    let make = {
      let (categories, series, labels) = (categories.clone(), series.clone(), labels.clone());
      move || {
        let mut chart = AreaChart::new(categories.clone(), series.clone())
          .title("Memory")
          .description("Megabytes resident during a session")
          .stacked(stacked)
          .curve(if smooth { LineCurve::Smooth } else { LineCurve::Straight })
          .grid(grid)
          .legend(legend)
          .loading(choice == 6)
          .motion(motion)
          .value_text(text);
        if let Some(labels) = &labels {
          chart = chart.axis_labels(labels.clone());
        }
        chart
      }
    };
    let record = self.playground.recorder();
    let (selected_to, hidden_to) = (record.clone(), record);
    let shown_chart = {
      let make = make.clone();
      let (selected, hidden) = (self.selected.clone(), self.hidden.clone());
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
      "AreaChart::new(minutes, [resident])".to_owned(),
      "    .title(\"Memory\")".to_owned(),
      "    .description(\"Megabytes resident during a session\")".to_owned(),
      "    .axis_labels([\"0 min\", \"10\", \"20\", \"30\", \"40\"])".to_owned(),
    ];
    if stacked {
      code.push("    .stacked(true)".to_owned());
    }
    if smooth {
      code.push("    .curve(LineCurve::Smooth)".to_owned());
    }
    if !grid {
      code.push("    .grid(false)".to_owned());
    }
    code.push(format!("    .legend({legend})"));
    if choice == 6 {
      code.push("    .loading(true)".to_owned());
    }
    if !motion {
      code.push("    .motion(false)".to_owned());
    }
    code.push("    .mount(ctx)".to_owned());
    let tree = vec![
      format!("root      {} · a figure · card, padded 20", self.playground.measured()),
      "├─ header  title 15/600 · description 12".to_owned(),
      "├─ plot    a canvas of 140 · areas at 0.16 under lines 2 wide".to_owned(),
      "├─ axis    its texts from the plot's start to its end".to_owned(),
      "└─ legend  a dot and a name for each series".to_owned(),
    ];
    let datas: Vec<(&str, usize)> = DATA.iter().enumerate().map(|(index, name)| (*name, index)).collect();
    let set_data = self.data.clone();
    let reset = (
      self.data.clone(),
      [self.stacked.clone(), self.smooth.clone()],
      [self.grid.clone(), self.legend.clone(), self.motion.clone()],
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
        .map(|point| format!(" · selected {} of series {}", point.index, point.series))
        .unwrap_or_default()
    );
    self.playground.card(
      ctx,
      Spec {
        title: "AreaChart",
        make: Arc::new(move || -> Element { make().into() }),
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
          divider(),
          flag("stacked", &self.stacked),
          flag("curve: smooth", &self.smooth),
          flag("grid", &self.grid),
          flag("legend", &self.legend),
          flag("motion: in, and to a change of the data", &self.motion),
        ],
        changed: [
          choice != 0,
          stacked,
          smooth,
          !grid,
          legend,
          !motion,
          !self.hidden.get().is_empty(),
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          reset.0.set(0);
          for flag in &reset.1 {
            flag.set(false);
          }
          reset.2[0].set(true);
          reset.2[1].set(false);
          reset.2[2].set(true);
          reset.3.set(Vec::new());
          reset.4.set(None);
        }),
      },
    )
  }
}

impl Component for AreaChartPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      data: ctx.signal(0),
      stacked: ctx.signal(false),
      smooth: ctx.signal(false),
      grid: ctx.signal(true),
      legend: ctx.signal(false),
      motion: ctx.signal(true),
      hidden: ctx.signal(Vec::new()),
      selected: ctx.signal(None),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = AreaChart::new(MINUTES, [resident()])
      .title("Memory")
      .description("Megabytes resident during a session")
      .axis_labels(MINUTE_LABELS)
      .value_text(megabytes)
      .key("board")
      .mount(ctx);
    let stacked = AreaChart::new(BUILDS, phases())
      .title("Frame budget")
      .description("Milliseconds by phase, stacked")
      .stacked(true)
      .value_text(ms)
      .key("stacked")
      .mount(ctx);
    let rows = [
      (
        "categories / series",
        "names / impl IntoIterator<Item = ChartSeries>",
        "",
        "As a line chart's: a value for each category.",
      ),
      (
        "stacked",
        "bool",
        "false",
        "The areas one on another, the first at the bottom, each from the line of the one under it.",
      ),
      (
        "curve / axis_labels",
        "LineCurve / texts",
        "Straight / the categories",
        "As a line chart's.",
      ),
      (
        "title, description, label, size, height, grid, value_axis, legend, hidden, value_text, empty_text, loading",
        "",
        "",
        "Every chart's.",
      ),
      (
        "motion",
        "bool",
        "true",
        "The areas drawn on from their start, and each value moving to a change of the data, the fill with it, a \
         stack too.",
      ),
      (
        "on_select",
        "Fn(ChartPoint)",
        "",
        "A press at a place, or Enter or Space on the focused value.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Charts", "Area Chart"],
        title: "Area Chart",
        description: "A line chart with the fill under each line: memory over a session, or a whole and its parts \
                      stacked.",
        import: Some("use lurq::ui::{AreaChart, ChartSeries};"),
        status: Some("Preview"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of AreaChart is a control on the right, and the data too. It has the line chart's cursor, \
           tooltip and keys.",
          playground,
        ),
        Section::new(
          "The board's area chart",
          "Memory over a session: the fill under the line in the first chart color at 0.16, the line over it, an axis \
           of minutes of its own.",
          self.frame.demo(
            0,
            Column::new().width(FILL).child(board),
            "AreaChart::new(minutes, [resident]).axis_labels(..)",
            &[
              "AreaChart::new(",
              "    [\"0\", \"5\", \"10\", \"15\", \"20\", \"25\", \"30\", \"35\"],",
              "    [ChartSeries::new(\"Resident\", [380.0, 520.0, 470.0, 720.0, 670.0, 920.0, 870.0, 1060.0])],",
              ")",
              ".title(\"Memory\")",
              ".description(\"Megabytes resident during a session\")",
              ".axis_labels([\"0 min\", \"10\", \"20\", \"30\", \"40\"])",
              ".mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Stacked",
          "The phases of a frame one on another: the top line is the whole, and each band a part of it.",
          self.frame.demo(
            1,
            Column::new().width(FILL).child(stacked),
            ".stacked(true)",
            &["AreaChart::new(builds, [layout, text, paint]).stacked(true).mount(ctx)"],
          ),
        ),
        Section::new(
          "Props",
          "What an area chart is given: a line chart's, and stacked.",
          props_table(&rows, &["categories / series"], self.frame.room()),
        ),
      ],
    )
  }
}
