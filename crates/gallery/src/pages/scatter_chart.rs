//! The page of the scatter chart: its playground, the board's Scatter Chart with its trend, several series with their
//! legend, and its props.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::PaletteColor},
  components::{Column, Row},
  core::Signal,
  node::Element,
  ui::{ChartPoint, ChartSeries, ScatterChart},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// The data the playground shows.
const DATA: [&str; 6] = [
  "the board's",
  "three series",
  "under zero",
  "one point",
  "no data",
  "loading",
];

fn scenes() -> ChartSeries {
  ChartSeries::points(
    "Scenes",
    [
      (550.0, 3.3),
      (1050.0, 5.1),
      (1600.0, 4.3),
      (2150.0, 6.6),
      (2700.0, 6.1),
      (3200.0, 8.4),
      (3900.0, 7.6),
      (4300.0, 9.9),
      (5000.0, 11.0),
      (5500.0, 10.4),
      (6000.0, 12.7),
      (6600.0, 13.4),
      (7300.0, 15.5),
    ],
  )
}

/// What stands apart from the scenes, in the third chart color as the board draws it.
fn outliers() -> ChartSeries {
  ChartSeries::points(
    "Outlier",
    [(1850.0, 7.6), (3700.0, 10.3), (6150.0, 14.2), (7100.0, 16.3)],
  )
  .color(PaletteColor::Chart3)
}

fn thousands(value: f64) -> String {
  if value == 0.0 {
    "0".to_owned()
  } else if value.abs() >= 1000.0 {
    format!("{}k", lurq::ui::default_value_text(value / 1000.0))
  } else {
    lurq::ui::default_value_text(value)
  }
}

fn ms(value: f64) -> String {
  format!("{} ms", lurq::ui::default_value_text(value))
}

fn data(choice: usize) -> Vec<ChartSeries> {
  match choice {
    1 => vec![
      scenes(),
      outliers(),
      ChartSeries::points("Mobile", [(800.0, 9.0), (2400.0, 13.5), (4100.0, 18.2), (5800.0, 22.0)]),
    ],
    2 => vec![ChartSeries::points(
      "Drift",
      [
        (-3000.0, -4.2),
        (-1500.0, -1.0),
        (500.0, 0.8),
        (2000.0, 3.6),
        (3500.0, 2.1),
      ],
    )],
    3 => vec![ChartSeries::points("Scenes", [(4300.0, 9.9)])],
    4 => Vec::new(),
    _ => vec![scenes(), outliers()],
  }
}

pub struct ScatterChartPage {
  frame: Frame,
  playground: Playground,
  data: Signal<usize>,
  trend: Signal<bool>,
  legend: Signal<bool>,
  grid: Signal<bool>,
  hidden: Signal<Vec<String>>,
  selected: Signal<Option<ChartPoint>>,
}

impl ScatterChartPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let choice = self.data.get().min(DATA.len() - 1);
    let (trend, legend, grid) = (self.trend.get(), self.legend.get(), self.grid.get());
    let series = data(choice);
    let make = {
      let series = series.clone();
      move || {
        ScatterChart::new(series.clone())
          .title("Nodes vs frame time")
          .description("One dot per scene")
          .trend(trend)
          .legend(legend)
          .grid(grid)
          .loading(choice == 5)
          .x_text(thousands)
          .value_text(ms)
      }
    };
    let record = self.playground.recorder();
    let (selected_to, hidden_to) = (record.clone(), record);
    let shown_chart = {
      let make = make.clone();
      let (selected, hidden) = (self.selected.clone(), self.hidden.clone());
      move |ctx: &mut Ctx| {
        let (selected, selected_to, hidden_to) = (selected.clone(), selected_to.clone(), hidden_to.clone());
        Row::new()
          .width(FILL)
          .child(
            Column::new().width(320.0).child(
              make()
                .hidden(hidden.clone())
                .on_hidden_change(move |hidden| hidden_to(format!("on_hidden_change({hidden:?})")))
                .on_select(move |point| {
                  selected.set(Some(point));
                  selected_to(format!("on_select(series {}, index {})", point.series, point.index));
                })
                .key("played")
                .mount(ctx),
            ),
          )
          .into()
      }
    };
    let mut code = vec![
      "ScatterChart::new([scenes, outliers])".to_owned(),
      "    .title(\"Nodes vs frame time\")".to_owned(),
      "    .description(\"One dot per scene\")".to_owned(),
      "    .x_text(|nodes| format!(\"{}k\", nodes / 1000.0))".to_owned(),
      "    .value_text(|ms| format!(\"{ms} ms\"))".to_owned(),
    ];
    if trend {
      code.push("    .trend(true)".to_owned());
    }
    if legend {
      code.push("    .legend(true)".to_owned());
    }
    if !grid {
      code.push("    .grid(false)".to_owned());
    }
    if choice == 5 {
      code.push("    .loading(true)".to_owned());
    }
    code.push("    .mount(ctx)".to_owned());
    let tree = vec![
      format!("root      {} · a figure · card, padded 20", self.playground.measured()),
      "├─ header  title 15/600 · description 12".to_owned(),
      "├─ plot    a canvas of 140 · dots of 8 at 0.85 · one stop of Tab".to_owned(),
      "└─ axis    the numbers across, mono 11, from the plot's start to its end".to_owned(),
    ];
    let datas: Vec<(&str, usize)> = DATA.iter().enumerate().map(|(index, name)| (*name, index)).collect();
    let set_data = self.data.clone();
    let reset = (
      self.data.clone(),
      self.trend.clone(),
      self.legend.clone(),
      self.grid.clone(),
      self.hidden.clone(),
      self.selected.clone(),
    );
    let facts = format!(
      "{} series · {} points{}",
      series.len(),
      series.iter().map(ChartSeries::len).sum::<usize>(),
      self
        .selected
        .get()
        .map(|point| format!(" · selected {} of series {}", point.index, point.series))
        .unwrap_or_default()
    );
    self.playground.card(
      ctx,
      Spec {
        title: "ScatterChart",
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
            "series of points",
            self
              .playground
              .select(0, &datas, &choice, move |index| set_data.set(index)),
          ),
          divider(),
          flag("trend", &self.trend),
          flag("legend", &self.legend),
          flag("grid", &self.grid),
        ],
        changed: [choice != 0, !trend, legend, !grid, !self.hidden.get().is_empty()]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          reset.0.set(0);
          reset.1.set(true);
          reset.2.set(false);
          reset.3.set(true);
          reset.4.set(Vec::new());
          reset.5.set(None);
        }),
      },
    )
  }
}

impl Component for ScatterChartPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 1),
      data: ctx.signal(0),
      trend: ctx.signal(true),
      legend: ctx.signal(false),
      grid: ctx.signal(true),
      hidden: ctx.signal(Vec::new()),
      selected: ctx.signal(None),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = ScatterChart::new([scenes(), outliers()])
      .title("Nodes vs frame time")
      .description("One dot per scene")
      .trend(true)
      .x_text(thousands)
      .value_text(ms)
      .key("board")
      .mount(ctx);
    let several = ScatterChart::new(data(1))
      .title("Nodes vs frame time")
      .description("Desktop, its outliers, and mobile")
      .legend(true)
      .x_text(thousands)
      .value_text(ms)
      .key("several")
      .mount(ctx);
    let rows = [
      (
        "series",
        "impl IntoIterator<Item = ChartSeries>",
        "",
        "ChartSeries::points(name, [(x, y)]): each series' points, in any order; the arrows go through them by x.",
      ),
      (
        "trend",
        "bool",
        "false",
        "The straight line that fits the first series shown best, from its least x to its most.",
      ),
      (
        "x_text",
        "Fn(f64) -> String",
        "value_text",
        "How a number across is written: under the plot and as a tooltip's title.",
      ),
      (
        "legend",
        "bool",
        "false",
        "The legend under the axis, as the board draws none.",
      ),
      (
        "title, description, label, size, height, grid, value_axis, hidden, value_text, empty_text, loading",
        "",
        "",
        "Every chart's.",
      ),
      (
        "on_select",
        "Fn(ChartPoint)",
        "",
        "A press of the point nearest the pointer, or Enter or Space on the focused one.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Charts", "Scatter Chart"],
        title: "Scatter Chart",
        description: "Points by an x and a y on two value axes: how the frame time of a scene grows with its nodes, \
                      and what stands apart from the trend.",
        import: Some("use lurq::ui::{ChartSeries, ScatterChart};"),
        status: Some("Preview"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of ScatterChart is a control on the right, and the data too. Point near a dot for its ring and \
           tooltip, or Tab into the plot and go along a series with Left and Right.",
          playground,
        ),
        Section::new(
          "The board's scatter chart",
          "Thirteen scenes and four outliers, dots of 8 at 0.85, the numbers of the axis across under the plot, and the \
           line of the scenes' trend over them.",
          self.frame.demo(
            0,
            Row::new().width(FILL).child(Column::new().width(300.0).child(board)),
            "ScatterChart::new([scenes, outliers]).trend(true)",
            &[
              "ScatterChart::new([",
              "    ChartSeries::points(\"Scenes\", [(550.0, 3.3), (1050.0, 5.1), ..]),",
              "    ChartSeries::points(\"Outlier\", [(1850.0, 7.6), ..]).color(PaletteColor::Chart3),",
              "])",
              ".title(\"Nodes vs frame time\")",
              ".description(\"One dot per scene\")",
              ".trend(true)",
              ".x_text(|nodes| format!(\"{}k\", nodes / 1000.0))",
              ".mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Several series",
          "Each series takes the next chart color; the legend, off as the board draws it, names them when told.",
          self.frame.demo(
            1,
            Column::new().width(FILL).child(several),
            ".legend(true)",
            &["ScatterChart::new([desktop, outliers, mobile]).legend(true).mount(ctx)"],
          ),
        ),
        Section::new(
          "Props",
          "What a scatter chart is given.",
          props_table(&rows, &["series"], self.frame.room()),
        ),
      ],
    )
  }
}
