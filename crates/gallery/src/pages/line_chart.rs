//! The page of the line chart: its playground, the board's Line Chart, a smooth line with a gap, ten thousand values,
//! and its props.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::Column,
  core::Signal,
  node::Element,
  ui::{ChartPoint, ChartSeries, ChartSize, LineChart, LineCurve},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

pub(crate) const BUILDS: [&str; 7] = ["0.54", "0.55", "0.56", "0.57", "0.58", "0.59", "0.60"];

/// The data the playground shows: what the board draws, and what tries a chart.
const DATA: [&str; 8] = [
  "the board's",
  "three series",
  "a gap",
  "under zero",
  "one value",
  "ten thousand values",
  "no data",
  "loading",
];

pub(crate) fn wgpu() -> ChartSeries {
  ChartSeries::new("wgpu", [10.4, 9.6, 9.9, 8.2, 8.6, 6.9, 6.3])
}

pub(crate) fn directx() -> ChartSeries {
  ChartSeries::new("DirectX 12", [11.6, 11.2, 10.3, 10.6, 9.4, 9.4, 8.6])
}

pub(crate) fn ms(value: f64) -> String {
  format!("{} ms", lurq::ui::default_value_text(value))
}

/// Ten thousand samples of a signal, and their names.
pub(crate) fn signal() -> (Vec<String>, Vec<ChartSeries>) {
  (
    (0..10_000).map(|index| index.to_string()).collect(),
    vec![ChartSeries::new(
      "Signal",
      (0..10_000).map(|index| ((index as f64) / 300.0).sin() * 40.0 + ((index as f64) / 37.0).sin() * 8.0 + 60.0),
    )],
  )
}

/// The data of the playground's choice: its categories and its series.
fn data(choice: usize) -> (Vec<String>, Vec<ChartSeries>) {
  let names = |names: &[&str]| names.iter().map(|name| (*name).to_owned()).collect::<Vec<_>>();
  match choice {
    1 => (
      names(&BUILDS),
      vec![
        wgpu(),
        directx(),
        ChartSeries::new("Vulkan", [9.8, 9.9, 9.1, 8.9, 8.0, 7.7, 7.1]),
      ],
    ),
    2 => (
      names(&BUILDS),
      vec![ChartSeries::new("wgpu", [10.4, 9.6, f64::NAN, f64::NAN, 8.6, 6.9, 6.3])],
    ),
    3 => (
      names(&["Q1", "Q2", "Q3", "Q4", "Q1 '27"]),
      vec![ChartSeries::new("Change", [12.0, -8.5, 5.0, -2.5, 9.0])],
    ),
    4 => (names(&["0.60"]), vec![ChartSeries::new("Frame time", [6.3])]),
    5 => signal(),
    6 => (Vec::new(), Vec::new()),
    _ => (names(&BUILDS), vec![wgpu(), directx()]),
  }
}

pub struct LineChartPage {
  frame: Frame,
  playground: Playground,
  data: Signal<usize>,
  curve: Signal<usize>,
  size: Signal<usize>,
  grid: Signal<bool>,
  value_axis: Signal<bool>,
  legend: Signal<bool>,
  motion: Signal<bool>,
  hidden: Signal<Vec<String>>,
  selected: Signal<Option<ChartPoint>>,
}

impl LineChartPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let choice = self.data.get().min(DATA.len() - 1);
    let curve = if self.curve.get() == 1 {
      LineCurve::Smooth
    } else {
      LineCurve::Straight
    };
    let size = if self.size.get() == 1 {
      ChartSize::Md
    } else {
      ChartSize::Sm
    };
    let (grid, value_axis, legend) = (self.grid.get(), self.value_axis.get(), self.legend.get());
    let motion = self.motion.get();
    let (categories, series) = data(choice);
    let make = {
      let (categories, series) = (categories.clone(), series.clone());
      move || {
        LineChart::new(categories.clone(), series.clone())
          .title("Frame time")
          .description("Milliseconds per frame, last 7 builds")
          .size(size)
          .curve(curve)
          .grid(grid)
          .value_axis(value_axis)
          .legend(legend)
          .loading(choice == 7)
          .motion(motion)
          .value_text(ms)
      }
    };
    let record = self.playground.recorder();
    let (selected_to, hidden_to) = (record.clone(), record);
    let shown_chart = {
      let stage = self.playground.styled(make());
      let (selected, hidden) = (self.selected.clone(), self.hidden.clone());
      move |ctx: &mut Ctx| {
        let (selected, selected_to, hidden_to) = (selected.clone(), selected_to.clone(), hidden_to.clone());
        stage
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
      "LineChart::new(builds, [wgpu, directx])".to_owned(),
      "    .title(\"Frame time\")".to_owned(),
      "    .description(\"Milliseconds per frame, last 7 builds\")".to_owned(),
      "    .value_text(|ms| format!(\"{ms} ms\"))".to_owned(),
    ];
    if size == ChartSize::Md {
      code.push("    .size(ChartSize::Md)".to_owned());
    }
    if curve == LineCurve::Smooth {
      code.push("    .curve(LineCurve::Smooth)".to_owned());
    }
    if !grid {
      code.push("    .grid(false)".to_owned());
    }
    if value_axis {
      code.push("    .value_axis(true)".to_owned());
    }
    code.push(format!("    .legend({legend})"));
    if choice == 7 {
      code.push("    .loading(true)".to_owned());
    }
    if !motion {
      code.push("    .motion(false)".to_owned());
    }
    code.push("    .hidden(hidden)".to_owned());
    code.push("    .on_select(|point| ..)".to_owned());
    code.push("    .mount(ctx)".to_owned());
    let tree = vec![
      format!("root      {} · a figure · card, padded 20", self.playground.measured()),
      "├─ header  title 15/600 · description 12".to_owned(),
      "├─ plot    a canvas of 140 · lines 2 wide · one stop of Tab".to_owned(),
      "├─ axis    the categories, mono 11, from the plot's start to its end".to_owned(),
      "└─ legend  a dot and a name for each series".to_owned(),
    ];
    let datas: Vec<(&str, usize)> = DATA.iter().enumerate().map(|(index, name)| (*name, index)).collect();
    let curves = [("Straight", 0_usize), ("Smooth", 1)];
    let sizes = [("sm", 0_usize), ("md", 1)];
    let (set_data, set_curve, set_size) = (self.data.clone(), self.curve.clone(), self.size.clone());
    let reset = (
      self.data.clone(),
      self.curve.clone(),
      self.size.clone(),
      [self.grid.clone(), self.legend.clone(), self.motion.clone()],
      self.value_axis.clone(),
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
        title: "LineChart",
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
          prop(
            "curve",
            "LineCurve",
            self
              .playground
              .select(1, &curves, &self.curve.get().min(1), move |index| set_curve.set(index)),
          ),
          prop(
            "size",
            "ChartSize",
            self
              .playground
              .select(2, &sizes, &self.size.get().min(1), move |index| set_size.set(index)),
          ),
          divider(),
          flag("grid", &self.grid),
          flag("value_axis", &self.value_axis),
          flag("legend", &self.legend),
          flag("motion: in, and to a change of the data", &self.motion),
        ],
        changed: [
          choice != 0,
          self.curve.get() != 0,
          self.size.get() != 0,
          !grid,
          value_axis,
          !legend,
          !motion,
          !self.hidden.get().is_empty(),
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          reset.0.set(0);
          reset.1.set(0);
          reset.2.set(0);
          for flag in &reset.3 {
            flag.set(true);
          }
          reset.4.set(false);
          reset.5.set(Vec::new());
          reset.6.set(None);
        }),
      },
    )
  }
}

impl Component for LineChartPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 3),
      data: ctx.signal(0),
      curve: ctx.signal(0),
      size: ctx.signal(0),
      grid: ctx.signal(true),
      value_axis: ctx.signal(false),
      legend: ctx.signal(true),
      motion: ctx.signal(true),
      hidden: ctx.signal(Vec::new()),
      selected: ctx.signal(None),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = LineChart::new(BUILDS, [wgpu(), directx()])
      .title("Frame time")
      .description("Milliseconds per frame, last 7 builds")
      .value_text(ms)
      .key("board")
      .mount(ctx);
    let smooth = LineChart::new(
      BUILDS,
      [ChartSeries::new("wgpu", [10.4, 9.6, f64::NAN, 8.2, 8.6, 6.9, 6.3])],
    )
    .title("Frame time")
    .description("A smooth line; 0.56 was never measured")
    .curve(LineCurve::Smooth)
    .value_text(ms)
    .key("smooth")
    .mount(ctx);
    let (names, samples) = signal();
    let many = LineChart::new(names, samples)
      .title("Signal")
      .description("Ten thousand samples: the pointer finds its sample at once")
      .key("many")
      .mount(ctx);
    let rows = [
      (
        "categories",
        "impl IntoIterator<Item = impl AsRef<str>>",
        "",
        "The names of the places along the plot, in their order: the first at its start and the last at its end.",
      ),
      (
        "series",
        "impl IntoIterator<Item = ChartSeries>",
        "",
        "ChartSeries::new(name, values): a value for each category; a value that is not a number leaves a gap.",
      ),
      (
        "curve",
        "LineCurve",
        "Straight",
        "Straight, as the board draws it, or Smooth: a curve that never goes past the values beside it.",
      ),
      (
        "axis_labels",
        "impl IntoIterator<Item = impl AsRef<str>>",
        "the categories",
        "The texts under the plot, spread from its start to its end; of many categories eight are written.",
      ),
      (
        "title / description / label",
        "&str",
        "none",
        "The header, and the name to tooling (the title's).",
      ),
      (
        "size / height",
        "ChartSize / impl Into<Length>",
        "Sm",
        "Sm: padded 20, a plot of 140, as the board draws it. Md: padded 24, a plot of 160.",
      ),
      (
        "grid / value_axis",
        "bool",
        "true / false",
        "The lines across the plot at its ticks, and their numbers.",
      ),
      ("legend", "bool", "more than one series", "The legend under the axis."),
      (
        "hidden",
        "Signal<Vec<String>>",
        "none",
        "The series hidden; a press of the legend's item hides or shows one.",
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
        "motion",
        "bool",
        "true",
        "The line drawn on from its start, and each value moving to a change of the data; off where motion is \
         reduced, still while it is resized.",
      ),
      (
        "on_select",
        "Fn(ChartPoint)",
        "",
        "A press at a place, the series nearest the pointer, or Enter or Space on the focused value.",
      ),
      (
        "mount",
        "&mut Ctx",
        "",
        "The cursor, the marks and the tooltip under the pointer and the focus, the arrows; converted it is still.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Charts", "Line Chart"],
        title: "Line Chart",
        description: "A value for each category joined by a line, of one series or several: the frame time of each \
                      build on two backends. Drawn on a canvas, again only when its data, its size or its theme \
                      changes.",
        import: Some("use lurq::ui::{ChartSeries, LineChart};"),
        status: Some("Preview"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of LineChart is a control on the right, and the data too. Point at the plot for the cursor and \
           the tooltip of every series there, or Tab into it and go along with Left and Right and between the series \
           with Up and Down.",
          playground,
        ),
        Section::new(
          "The board's line chart",
          "Two backends over seven builds: lines 2 wide in the first two chart colors, the first over the second; at \
           the place under the pointer a line across the plot, a mark on each line and the tooltip beside it.",
          self.frame.demo(
            0,
            Column::new().width(FILL).child(board),
            "LineChart::new(builds, [wgpu, directx])",
            &[
              "LineChart::new(",
              "    [\"0.54\", \"0.55\", \"0.56\", \"0.57\", \"0.58\", \"0.59\", \"0.60\"],",
              "    [",
              "        ChartSeries::new(\"wgpu\", [10.4, 9.6, 9.9, 8.2, 8.6, 6.9, 6.3]),",
              "        ChartSeries::new(\"DirectX 12\", [11.6, 11.2, 10.3, 10.6, 9.4, 9.4, 8.6]),",
              "    ],",
              ")",
              ".title(\"Frame time\")",
              ".description(\"Milliseconds per frame, last 7 builds\")",
              ".value_text(|ms| format!(\"{ms} ms\"))",
              ".mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Smooth, with a gap",
          "A smooth curve goes through every value and never past the ones beside it; a value that is not a number \
           leaves a gap.",
          self.frame.demo(
            1,
            Column::new().width(FILL).child(smooth),
            ".curve(LineCurve::Smooth)",
            &["LineChart::new(builds, [ChartSeries::new(\"wgpu\", [10.4, 9.6, f64::NAN, 8.2, ..])]).curve(LineCurve::Smooth)"],
          ),
        ),
        Section::new(
          "Ten thousand values",
          "Drawn as one line with no mark of its own for each value; the pointer finds its value at once, and the axis \
           writes eight of the names.",
          self.frame.demo(
            2,
            Column::new().width(FILL).child(many),
            "LineChart::new(names, [samples])",
            &["LineChart::new(names, [ChartSeries::new(\"Signal\", samples)]).title(\"Signal\").mount(ctx)"],
          ),
        ),
        Section::new(
          "Props",
          "What a line chart is given; every chart has the same words for the same things.",
          props_table(&rows, &["categories", "series"], self.frame.room()),
        ),
      ],
    )
  }
}
