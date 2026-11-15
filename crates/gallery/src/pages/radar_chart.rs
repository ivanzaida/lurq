//! The page of the radar chart: its playground, the board's Radar Chart, three series with the rings' numbers, and its
//! props.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  node::Element,
  ui::{ChartPoint, ChartSeries, ChartSize, RadarChart},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::props_table,
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

const AREAS: [&str; 6] = ["Draw", "Text", "Layout", "Memory", "Startup", "Input"];

/// The data the playground shows: what the board draws, and what tries a radar.
const DATA: [&str; 7] = [
  "the board's",
  "three series",
  "under zero",
  "three axes",
  "long names",
  "no data",
  "loading",
];

/// The data of the playground's choice: the axes and the series.
fn data(choice: usize) -> (Vec<String>, Vec<ChartSeries>) {
  let names = |names: &[&str]| names.iter().map(|name| (*name).to_owned()).collect::<Vec<_>>();
  match choice {
    1 => (names(&AREAS), three()),
    2 => (
      names(&["Q1", "Q2", "Q3", "Q4", "Q5"]),
      vec![ChartSeries::new("Change", [12.0, -8.0, 5.0, -3.0, 9.0])],
    ),
    3 => (
      names(&["Draw", "Text", "Layout"]),
      vec![ChartSeries::new("wgpu", [9.0, 6.0, 7.5])],
    ),
    4 => (
      names(&[
        "text_pipeline",
        "canvas_mesh",
        "layout_flex",
        "theme_roles",
        "input_routing",
      ]),
      vec![ChartSeries::new("Score", [80.0, 64.0, 72.0, 55.0, 90.0])],
    ),
    5 => (Vec::new(), Vec::new()),
    _ => (names(&AREAS), renderers()),
  }
}

fn renderers() -> Vec<ChartSeries> {
  vec![
    ChartSeries::new("wgpu", [92.0, 70.0, 64.0, 58.0, 75.0, 66.0]),
    ChartSeries::new("DX12", [68.0, 76.0, 70.0, 80.0, 62.0, 86.0]),
  ]
}

fn three() -> Vec<ChartSeries> {
  let mut series = renderers();
  series.push(ChartSeries::new("Software", [40.0, 58.0, 66.0, 90.0, 85.0, 52.0]));
  series
}

pub struct RadarChartPage {
  frame: Frame,
  playground: Playground,
  data: Signal<usize>,
  size: Signal<usize>,
  value_axis: Signal<bool>,
  legend: Signal<bool>,
  hidden: Signal<Vec<String>>,
  selected: Signal<Option<ChartPoint>>,
}

impl RadarChartPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let choice = self.data.get().min(DATA.len() - 1);
    let (value_axis, legend) = (self.value_axis.get(), self.legend.get());
    let size = if self.size.get() == 1 {
      ChartSize::Md
    } else {
      ChartSize::Sm
    };
    let (axes, series) = data(choice);
    let record = self.playground.recorder();
    let (selected_to, hidden_to) = (record.clone(), record);
    let (selected, hidden) = (self.selected.clone(), self.hidden.clone());
    let make = {
      let (axes, series) = (axes.clone(), series.clone());
      move || {
        let mut chart = RadarChart::new(axes.clone(), series.clone())
          .title("Renderer profile")
          .description("Relative score per area")
          .size(size)
          .value_axis(value_axis)
          .loading(choice == 6);
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
      "RadarChart::new(axes, [wgpu, dx12])".to_owned(),
      "    .title(\"Renderer profile\")".to_owned(),
      "    .description(\"Relative score per area\")".to_owned(),
    ];
    if size == ChartSize::Md {
      code.push("    .size(ChartSize::Md)".to_owned());
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
      format!("root      {} · a figure · card, padded 20", self.playground.measured()),
      "├─ header  title 15/600 · description 12".to_owned(),
      "├─ plot    a square canvas of 150 in the middle · one stop of Tab".to_owned(),
      "│          the axes' names round it, mono 11".to_owned(),
      "└─ legend  a dot and a name for each series".to_owned(),
    ];
    let (set_data, set_size) = (self.data.clone(), self.size.clone());
    let datas: Vec<(&str, usize)> = DATA.iter().enumerate().map(|(index, name)| (*name, index)).collect();
    let sizes = [("sm", 0_usize), ("md", 1)];
    let reset = (
      self.data.clone(),
      self.size.clone(),
      self.value_axis.clone(),
      self.legend.clone(),
      self.hidden.clone(),
      self.selected.clone(),
    );
    let facts = format!(
      "{} axes · {} series{}",
      axes.len(),
      series.len(),
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
        title: "RadarChart",
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
            "axes, series",
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
          flag("value_axis: the rings' numbers", &self.value_axis),
          flag("legend", &self.legend),
        ],
        changed: [
          choice != 0,
          self.size.get() != 0,
          value_axis,
          legend,
          !self.hidden.get().is_empty(),
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          reset.0.set(0);
          reset.1.set(0);
          reset.2.set(false);
          reset.3.set(false);
          reset.4.set(Vec::new());
          reset.5.set(None);
        }),
      },
    )
  }
}

impl Component for RadarChartPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      data: ctx.signal(0),
      size: ctx.signal(0),
      value_axis: ctx.signal(false),
      legend: ctx.signal(false),
      hidden: ctx.signal(Vec::new()),
      selected: ctx.signal(None),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = RadarChart::new(AREAS, renderers())
      .title("Renderer profile")
      .description("Relative score per area")
      .key("board")
      .mount(ctx);
    let rings = RadarChart::new(AREAS, three())
      .title("Renderer profile")
      .description("Three backends, the rings' numbers written")
      .size(ChartSize::Md)
      .value_axis(true)
      .key("rings")
      .mount(ctx);

    let rows = [
      (
        "axes",
        "impl IntoIterator<Item = impl AsRef<str>>",
        "",
        "The axes' names, the first at the top and the others clockwise.",
      ),
      (
        "series",
        "impl IntoIterator<Item = ChartSeries>",
        "",
        "ChartSeries::new(name, values): a value for each axis; .color(..) over the theme's chart color.",
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
        "Sm",
        "Sm: padded 20, a square of 150. Md: padded 24, a square of 180.",
      ),
      ("height", "impl Into<Length>", "the size's", "The square's side."),
      (
        "value_axis",
        "bool",
        "false",
        "The rings' numbers, beside the first axis.",
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
        "Every number it writes: the tooltip's, the rings', what tooling reads.",
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
        "A press of a point, or Enter or Space on the focused one: ChartPoint { series, index: axis }.",
      ),
      (
        "mount",
        "&mut Ctx",
        "",
        "The series and the axis under the pointer or the focus picked out with their tooltip; Left and Right round \
         the axes, Up and Down between series.",
      ),
    ];

    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Charts", "Radar Chart"],
        title: "Radar Chart",
        description: "A value for each of several axes round a center, a polygon for each series: the profile of a \
                      renderer, the strengths of two backends side by side.",
        import: Some("use lurq::ui::{ChartSeries, RadarChart};"),
        status: Some("Preview"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of RadarChart is a control on the right, and the data too. Point at an axis for every series' \
           value on it, or Tab into the plot: Left and Right go round the axes, Up and Down between the series.",
          playground,
        ),
        Section::new(
          "The board's radar",
          "Six axes, three rings at round numbers, two series filled and edged in their colors at 0.2, the legend \
           under the plot.",
          self.frame.demo(
            0,
            Row::new()
              .width(crate::kit::FILL)
              .child(Column::new().width(270.0).child(board)),
            "RadarChart::new(areas, [wgpu, dx12])",
            &[
              "RadarChart::new(",
              "    [\"Draw\", \"Text\", \"Layout\", \"Memory\", \"Startup\", \"Input\"],",
              "    [",
              "        ChartSeries::new(\"wgpu\", [92.0, 70.0, 64.0, 58.0, 75.0, 66.0]),",
              "        ChartSeries::new(\"DX12\", [68.0, 76.0, 70.0, 80.0, 62.0, 86.0]),",
              "    ],",
              ")",
              ".title(\"Renderer profile\")",
              ".description(\"Relative score per area\")",
              ".mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Three series, the rings' numbers",
          "A third series takes the third chart color; the rings' numbers stand beside the first axis.",
          self.frame.demo(
            1,
            Row::new()
              .width(crate::kit::FILL)
              .child(Column::new().width(340.0).child(rings)),
            ".size(ChartSize::Md).value_axis(true)",
            &["RadarChart::new(areas, [wgpu, dx12, software]).size(ChartSize::Md).value_axis(true).mount(ctx)"],
          ),
        ),
        Section::new(
          "Props",
          "What a radar chart is given; every chart has the same words for the same things.",
          props_table(&rows, &["axes", "series"], self.frame.room()),
        ),
      ],
    )
  }
}
