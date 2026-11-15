//! The page of the pie chart: its playground, the board's Pie Chart and its Donut Chart, the total in a donut's hole,
//! what the legend writes, slices hidden through the legend, and its props.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::PaletteColor},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{ChartPoint, ChartSeries, ChartSize, PieChart, PieLegendValue},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::props_table,
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

const CRATES: [&str; 4] = ["render", "layout", "text", "other"];
const PHASES: [&str; 3] = ["Layout", "Text", "Paint"];

/// The data the playground shows: what the board draws, and what tries a pie.
const DATA: [&str; 7] = [
  "the board's",
  "seven slices",
  "a sliver",
  "one slice",
  "long names",
  "all zero",
  "loading",
];

/// The data of the playground's choice: the slices' names and the series.
fn data(choice: usize) -> (Vec<String>, ChartSeries) {
  let names = |names: &[&str]| names.iter().map(|name| (*name).to_owned()).collect::<Vec<_>>();
  match choice {
    1 => (
      names(&["render", "layout", "text", "canvas", "theme", "router", "other"]),
      ChartSeries::new("Size", [3.1, 2.4, 1.9, 1.2, 0.8, 0.4, 0.2]),
    ),
    2 => (names(&["most", "a sliver"]), ChartSeries::new("Size", [9.99, 0.01])),
    3 => (names(&["render"]), ChartSeries::new("Size", [4.2])),
    4 => (
      names(&["text_pipeline_and_shaping", "canvas_mesh_tessellation", "layout"]),
      ChartSeries::new("Size", [4.0, 3.0, 1.0]),
    ),
    5 => (names(&CRATES), ChartSeries::new("Size", [0.0, 0.0, 0.0, 0.0])),
    _ => (names(&CRATES), sizes()),
  }
}

fn sizes() -> ChartSeries {
  ChartSeries::new("Size", [4.2, 2.8, 1.9, 1.1])
}

fn frame_budget() -> ChartSeries {
  ChartSeries::new("Frame", [7.2, 4.8, 4.0])
}

fn mb(value: f64) -> String {
  format!("{} MB", lurq::ui::default_value_text(value))
}

fn ms(value: f64) -> String {
  format!("{} ms", lurq::ui::default_value_text(value))
}

const LEGEND_VALUES: [(&str, usize); 3] = [("Share", 0), ("Value", 1), ("None", 2)];

fn legend_value(choice: usize) -> PieLegendValue {
  match choice {
    1 => PieLegendValue::Value,
    2 => PieLegendValue::None,
    _ => PieLegendValue::Share,
  }
}

pub struct PieChartPage {
  frame: Frame,
  playground: Playground,
  data: Signal<usize>,
  size: Signal<usize>,
  value: Signal<usize>,
  donut: Signal<bool>,
  total: Signal<bool>,
  legend: Signal<bool>,
  hidden: Signal<Vec<String>>,
  selected: Signal<Option<ChartPoint>>,
}

impl PieChartPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let choice = self.data.get().min(DATA.len() - 1);
    let (donut, total, legend) = (self.donut.get(), self.total.get(), self.legend.get());
    let size = match self.size.get() {
      1 => Some(ChartSize::Sm),
      2 => Some(ChartSize::Md),
      _ => None,
    };
    let value = self.value.get().min(2);
    let (names, series) = data(choice);
    let record = self.playground.recorder();
    let (selected_to, hidden_to) = (record.clone(), record);
    let (selected, hidden) = (self.selected.clone(), self.hidden.clone());
    let make = {
      let (names, series) = (names.clone(), series.clone());
      move || {
        let mut chart = PieChart::new(names.clone(), series.clone())
          .title("Crates by size")
          .description("Share of compiled code")
          .donut(donut)
          .legend(legend)
          .legend_value(legend_value(value))
          .loading(choice == 6)
          .value_text(mb);
        if let Some(size) = size {
          chart = chart.size(size);
        }
        if total {
          chart = chart.total_text("in all");
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
      "PieChart::new(names, ChartSeries::new(\"Size\", values))".to_owned(),
      "    .title(\"Crates by size\")".to_owned(),
      "    .description(\"Share of compiled code\")".to_owned(),
      "    .value_text(|mb| format!(\"{mb} MB\"))".to_owned(),
    ];
    if donut {
      code.push("    .donut(true)".to_owned());
    }
    if total {
      code.push("    .total_text(\"in all\")".to_owned());
    }
    match size {
      Some(ChartSize::Sm) => code.push("    .size(ChartSize::Sm)".to_owned()),
      Some(ChartSize::Md) => code.push("    .size(ChartSize::Md)".to_owned()),
      None => {}
    }
    if value != 0 {
      code.push(format!("    .legend_value(PieLegendValue::{})", LEGEND_VALUES[value].0));
    }
    if !legend {
      code.push("    .legend(false)".to_owned());
    }
    if choice == 6 {
      code.push("    .loading(true)".to_owned());
    }
    code.push("    .hidden(hidden)".to_owned());
    code.push("    .on_select(|point| ..)".to_owned());
    code.push("    .mount(ctx)".to_owned());
    let tree = vec![
      format!("root      {} · a figure · card", self.playground.measured()),
      "├─ header  title · description".to_owned(),
      "└─ body    the pie beside its legend, 18 apart (20 in Md)".to_owned(),
      "   ├─ plot    a canvas of 96 · one stop of Tab".to_owned(),
      "   └─ legend  a dot, a name and a share for each slice".to_owned(),
    ];
    let (set_data, set_size, set_value) = (self.data.clone(), self.size.clone(), self.value.clone());
    let datas: Vec<(&str, usize)> = DATA.iter().enumerate().map(|(index, name)| (*name, index)).collect();
    let sizes = [("the board's", 0_usize), ("sm", 1), ("md", 2)];
    let reset = (
      self.data.clone(),
      self.size.clone(),
      self.value.clone(),
      self.donut.clone(),
      self.total.clone(),
      self.legend.clone(),
      self.hidden.clone(),
      self.selected.clone(),
    );
    let facts = format!(
      "{} slices{}",
      names.len(),
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
        title: "PieChart",
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
            "names, series",
            self
              .playground
              .select(0, &datas, &choice, move |index| set_data.set(index)),
          ),
          prop(
            "size",
            "ChartSize",
            self
              .playground
              .select(1, &sizes, &self.size.get().min(2), move |index| set_size.set(index)),
          ),
          prop(
            "legend_value",
            "PieLegendValue",
            self
              .playground
              .select(2, &LEGEND_VALUES, &value, move |index| set_value.set(index)),
          ),
          divider(),
          flag("donut", &self.donut),
          flag("total_text: \"in all\"", &self.total),
          flag("legend", &self.legend),
        ],
        changed: [
          choice != 0,
          self.size.get() != 0,
          value != 0,
          donut,
          total,
          !legend,
          !self.hidden.get().is_empty(),
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          reset.0.set(0);
          reset.1.set(0);
          reset.2.set(0);
          reset.3.set(false);
          reset.4.set(false);
          reset.5.set(true);
          reset.6.set(Vec::new());
          reset.7.set(None);
        }),
      },
    )
  }
}

impl Component for PieChartPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 3),
      data: ctx.signal(0),
      size: ctx.signal(0),
      value: ctx.signal(0),
      donut: ctx.signal(false),
      total: ctx.signal(false),
      legend: ctx.signal(true),
      hidden: ctx.signal(Vec::new()),
      selected: ctx.signal(None),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let card = |chart: Element| Column::new().width(280.0).child(chart);
    let pie = PieChart::new(CRATES, sizes())
      .title("Crates by size")
      .description("Share of compiled code")
      .value_text(mb)
      .key("pie")
      .mount(ctx);
    let donut = PieChart::new(PHASES, frame_budget())
      .donut(true)
      .title("Frame budget")
      .description("Share of a 16 ms frame")
      .value_text(ms)
      .key("donut")
      .mount(ctx);
    let total = PieChart::new(PHASES, frame_budget())
      .donut(true)
      .title("Frame budget")
      .description("Milliseconds by phase")
      .total_text("a frame")
      .legend_value(PieLegendValue::Value)
      .value_text(ms)
      .key("total")
      .mount(ctx);
    let many = PieChart::new(
      ["render", "layout", "text", "canvas", "theme", "router", "other"],
      ChartSeries::new("Size", [3.1, 2.4, 1.9, 1.2, 0.8, 0.4, 0.2]),
    )
    .colors([
      PaletteColor::Chart1,
      PaletteColor::Chart2,
      PaletteColor::Chart3,
      PaletteColor::Chart4,
      PaletteColor::Chart5,
      PaletteColor::MutedForeground,
      PaletteColor::Border,
    ])
    .title("Crates by size")
    .description("Seven crates, the last two in the muted roles")
    .value_text(mb)
    .key("many")
    .mount(ctx);

    let rows = [
      (
        "names",
        "impl IntoIterator<Item = impl AsRef<str>>",
        "",
        "The slices' names, in their order, from the top and clockwise.",
      ),
      (
        "series",
        "ChartSeries",
        "",
        "ChartSeries::new(name, values): a value for each slice; the name titles the tooltip.",
      ),
      (
        "donut",
        "bool",
        "false",
        "A ring with a hole of 0.66 of its radius: the board's Donut Chart, Md unless told.",
      ),
      (
        "total_text",
        "&str",
        "none",
        "The words a donut writes in its hole under the total of the slices shown.",
      ),
      (
        "colors",
        "impl IntoIterator<Item = impl Into<BackgroundColor>>",
        "Chart1 to Chart5",
        "Each slice's color, in place of the theme's chart colors.",
      ),
      (
        "legend_value",
        "PieLegendValue",
        "Share",
        "What each item of the legend writes after the name: the share, the value, or nothing.",
      ),
      (
        "share_text",
        "Fn(f64) -> String",
        "a whole percent",
        "How a share, from 0 to 1, is written: 42%, <1% for a sliver.",
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
        "Sm, Md for a donut",
        "Sm: padded 20, the legend 18 away. Md: padded 24, the legend 20 away, its names 13.",
      ),
      ("height", "impl Into<Length>", "96", "The pie's side."),
      ("legend", "bool", "true", "The legend beside the pie."),
      (
        "hidden",
        "Signal<Vec<String>>",
        "none",
        "The slices hidden, by name; a press of the legend's item hides or shows one, and the rest share the whole.",
      ),
      (
        "value_text",
        "Fn(f64) -> String",
        "whole or two places",
        "Every value it writes: the tooltip's, the total, the legend's where it writes values.",
      ),
      (
        "empty_text / loading",
        "&str / bool",
        "\"No data\" / false",
        "No data (or all zero), and the skeleton while it comes.",
      ),
      (
        "on_select",
        "Fn(ChartPoint)",
        "",
        "A press of a slice, or Enter or Space on the focused one: ChartPoint { series: 0, index }.",
      ),
      (
        "mount",
        "&mut Ctx",
        "",
        "The slice under the pointer or the focus picked out with its tooltip, the arrows round the slices.",
      ),
    ];

    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Charts", "Pie Chart"],
        title: "Pie Chart",
        description: "The parts of a whole as slices of a disc, or of a ring: the crates by size, the phases of a \
                      frame. The legend beside it names each slice and writes its share.",
        import: Some("use lurq::ui::{ChartSeries, PieChart};"),
        status: Some("Preview"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of PieChart is a control on the right, and the data too. Point at a slice for its tooltip, or \
           Tab into the pie and go round the slices with the arrows; a press of the legend hides a slice.",
          playground,
        ),
        Section::new(
          "The board's pie",
          "Four crates from the top and clockwise, a line of the card between two slices, the legend beside the pie \
           with each crate's share.",
          self.frame.demo(
            0,
            Row::new().width(crate::kit::FILL).child(card(pie)),
            "PieChart::new(crates, sizes)",
            &[
              "PieChart::new(",
              "    [\"render\", \"layout\", \"text\", \"other\"],",
              "    ChartSeries::new(\"Size\", [4.2, 2.8, 1.9, 1.1]),",
              ")",
              ".title(\"Crates by size\")",
              ".description(\"Share of compiled code\")",
              ".value_text(|mb| format!(\"{mb} MB\"))",
              ".mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "The donut",
          "The same chart with a hole of 0.66 of its radius, roomier, as the board's Donut Chart.",
          self.frame.demo(
            1,
            Row::new().width(crate::kit::FILL).child(card(donut)),
            ".donut(true)",
            &["PieChart::new([\"Layout\", \"Text\", \"Paint\"], frame).donut(true).mount(ctx)"],
          ),
        ),
        Section::new(
          "A total in the hole",
          "Where the app gives its words, the donut writes the total of the slices shown over them; the legend can \
           write each value in place of its share.",
          self.frame.demo(
            2,
            Row::new()
              .width(crate::kit::FILL)
              .align_items(Alignment::Start)
              .child(card(total)),
            ".total_text(\"a frame\").legend_value(PieLegendValue::Value)",
            &[
              "PieChart::new(phases, frame)",
              "    .donut(true)",
              "    .total_text(\"a frame\")",
              "    .legend_value(PieLegendValue::Value)",
              "    .value_text(|ms| format!(\"{ms} ms\"))",
            ],
          ),
        ),
        Section::new(
          "More slices than chart colors",
          "The five chart colors go round again past the fifth slice; give the slices their colors where that would \
           confuse them.",
          self.frame.demo(
            3,
            Row::new().width(crate::kit::FILL).child(card(many)),
            ".colors([..])",
            &["PieChart::new(seven, sizes).colors([Chart1, Chart2, Chart3, Chart4, Chart5, MutedForeground, Border])"],
          ),
        ),
        Section::new(
          "Props",
          "What a pie chart is given; every chart has the same words for the same things.",
          props_table(&rows, &["names", "series"], self.frame.room()),
        ),
      ],
    )
  }
}
