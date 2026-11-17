//! The page of the sparkline: its playground, the board's frames in a stat card, sparklines of different data, and
//! one in a row of a table of the app's.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::PaletteColor},
  components::{Column, Row, Text},
  core::Signal,
  node::Element,
  ui::{Sparkline, StatCard, StatCardPart, StatCardSize},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, segmented_fill},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// The frames per second of the last minute, the board's line.
const FRAMES: [f64; 8] = [118.0, 124.0, 121.0, 133.0, 129.0, 141.0, 138.0, 144.0];

/// The data the playground's sparkline may show.
const DATA: [(&str, &[f64]); 6] = [
  ("frames", &FRAMES),
  ("negative", &[12.0, 4.0, -6.0, -2.0, 8.0, -9.0, 3.0, 10.0]),
  ("flat", &[5.0, 5.0, 5.0, 5.0]),
  ("one", &[42.0]),
  ("none", &[]),
  ("many", &[]),
];

const COLORS: [(&str, PaletteColor); 6] = [
  ("Chart1", PaletteColor::Chart1),
  ("Chart2", PaletteColor::Chart2),
  ("Chart3", PaletteColor::Chart3),
  ("Success", PaletteColor::Success),
  ("Warning", PaletteColor::Warning),
  ("Destructive", PaletteColor::Destructive),
];

/// A long series of a signal that wanders: the "many" data.
fn many() -> Vec<f64> {
  (0..240)
    .map(|index| {
      let t = index as f64 / 12.0;
      40.0 + 20.0 * t.sin() + 8.0 * (t * 3.1).cos() + (index % 7) as f64
    })
    .collect()
}

#[derive(Clone, Copy)]
struct Props {
  /// Which of [`DATA`] and [`COLORS`].
  data: usize,
  color: usize,
  wide: bool,
  range: bool,
}

pub struct SparklinePage {
  frame: Frame,
  playground: Playground,
  data: Signal<usize>,
  color: Signal<usize>,
  wide: Signal<bool>,
  range: Signal<bool>,
}

fn values(data: usize) -> Vec<f64> {
  if DATA[data].0 == "many" {
    many()
  } else {
    DATA[data].1.to_vec()
  }
}

fn line(props: Props) -> Sparkline {
  let mut sparkline = Sparkline::new(values(props.data))
    .color(COLORS[props.color].1.clone())
    .label("Frames per second, the last minute");
  if props.wide {
    sparkline = sparkline.width(200.0).height(48.0);
  }
  if props.range {
    sparkline = sparkline.min(0).max(160);
  }
  sparkline
}

impl SparklinePage {
  fn props(&self) -> Props {
    Props {
      data: self.data.get().min(DATA.len() - 1),
      color: self.color.get().min(COLORS.len() - 1),
      wide: self.wide.get(),
      range: self.range.get(),
    }
  }

  fn play(&self, ctx: &mut Ctx) -> Element {
    let props = self.props();
    let make = move || -> Element { line(props).into() };
    let mut code = vec![format!("Sparkline::new(values)   // {}", DATA[props.data].0)];
    code.push(format!("    .color(PaletteColor::{})", COLORS[props.color].0));
    code.push("    .label(\"Frames per second, the last minute\")".to_owned());
    if props.wide {
      code.push("    .width(200.0)".to_owned());
      code.push("    .height(48.0)".to_owned());
    }
    if props.range {
      code.push("    .min(0)".to_owned());
      code.push("    .max(160)".to_owned());
    }
    let tree = vec![
      format!("root        {}   a canvas", self.playground.measured()),
      "├─ line      1.75, round joins and ends".to_owned(),
      "├─ baseline  1, the border's color, where values are on both sides of 0".to_owned(),
      "└─ point     4, the one value of a single one".to_owned(),
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
        "values",
        "data",
        segmented(&self.data, places(&DATA.map(|(name, _)| name))),
      ),
      divider(),
      prop(
        "color",
        "PaletteColor",
        segmented(&self.color, places(&COLORS.map(|(name, _)| name))),
      ),
      flag("width / height (200 × 48)", &self.wide),
      flag("min / max (0 to 160)", &self.range),
    ];
    let changed = [props.data != 0, props.color != 3, props.wide, props.range]
      .into_iter()
      .filter(|changed| *changed)
      .count();
    let resets = (
      self.data.clone(),
      self.color.clone(),
      self.wide.clone(),
      self.range.clone(),
    );
    self.playground.card(
      ctx,
      Spec {
        title: "Sparkline",
        make: Arc::new(make),
        shown: Shown::of(self.playground.styled(line(props))),
        fill: false,
        forceable: false,
        facts: format!("{} values", values(props.data).len()),
        event: "",
        code,
        tree,
        controls,
        changed,
        reset: Arc::new(move || {
          resets.0.set(0);
          resets.1.set(3);
          resets.2.set(false);
          resets.3.set(false);
        }),
      },
    )
  }
}

impl Component for SparklinePage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      data: ctx.signal(0),
      color: ctx.signal(3),
      wide: ctx.signal(false),
      range: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let board = StatCard::new("Frames per second", "144")
      .size(StatCardSize::Sm)
      .media(
        Sparkline::new(FRAMES)
          .color(PaletteColor::Success)
          .label("Frames per second, the last minute"),
      )
      .style(StatCardPart::Root, |s| s.width(200.0));
    // What does not fit beside the others goes on to the next line.
    let shapes = Row::new()
      .width(FILL)
      .spacing(24.0)
      .wrap()
      .child(caption(
        Sparkline::new([12.0, 4.0, -6.0, -2.0, 8.0, -9.0, 3.0, 10.0]),
        "about 0",
      ))
      .child(caption(Sparkline::new([5.0, 5.0, 5.0, 5.0]), "flat"))
      .child(caption(Sparkline::new([42.0]), "one value"))
      .child(caption(Sparkline::new(Vec::<f64>::new()), "none"))
      .child(caption(
        Sparkline::new(many()).color(PaletteColor::Chart2).width(160.0),
        "240 values",
      ));
    let builds = Column::new()
      .spacing(10.0)
      .child(build_row(
        "main",
        &[4.1, 4.3, 4.0, 4.6, 4.4, 4.2, 4.0],
        PaletteColor::Success,
      ))
      .child(build_row(
        "release",
        &[6.2, 6.8, 7.4, 7.1, 8.0, 8.6, 9.2],
        PaletteColor::Destructive,
      ))
      .child(build_row(
        "docs",
        &[1.2, 1.1, 1.3, 1.2, 1.2, 1.1, 1.2],
        PaletteColor::Chart1,
      ));

    let rows = [
      (
        "new",
        "IntoIterator<Item = impl Into<f64>>",
        "",
        "The values, the first at the start.",
      ),
      (
        "min / max",
        "impl Into<f64>",
        "the values'",
        "The values at the bottom and the top: the least and the greatest unless given.",
      ),
      (
        "color",
        "impl Into<BackgroundColor>",
        "Chart1",
        "The role of the line: a chart color, or a tone where the series is good or bad.",
      ),
      ("width / height", "impl Into<Length>", "80 / 28", "Its size."),
      ("label", "&str", "", "What it shows, to tooling."),
      (
        "summary_text",
        "Fn(&[f64]) -> String",
        "\"8 values from 118 to 144, the last 144\"",
        "What its values are, to tooling.",
      ),
      (
        "empty_text",
        "&str",
        "No data",
        "What a sparkline of no values says to tooling.",
      ),
      ("style / rules", "SparklinePart", "", "Root, Line, Baseline, Point."),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Charts", "Sparkline"],
        title: "Sparkline",
        description: "A series as a line with no axes, small enough to stand beside a number: the frames of the \
                      last minute, the builds of the week.",
        import: Some("use lurq::ui::Sparkline;"),
        status: Some("Preview"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Sparkline is a control on the right, and its data too: values on both sides of 0, all the \
           same, one, none, and 240 of them. A sparkline is a picture, read and not operated.",
          playground,
        ),
        Section::new(
          "Frames per second",
          "The board's sparkline, beside the number it is the history of: a stat card of the small size holds it \
           at the end of its value's row.",
          self.frame.demo(
            0,
            board,
            "StatCard::new(..).media(Sparkline::new(values))",
            &[
              "StatCard::new(\"Frames per second\", \"144\")",
              "    .size(StatCardSize::Sm)",
              "    .media(",
              "        Sparkline::new([118.0, 124.0, 121.0, 133.0, 129.0, 141.0, 138.0, 144.0])",
              "            .color(PaletteColor::Success)",
              "            .label(\"Frames per second, the last minute\"),",
              "    )",
            ],
          ),
        ),
        Section::new(
          "Its data",
          "Values on both sides of 0 have the line at 0 under them; all the same, a line across the middle; one \
           value, a dot; none, nothing. A long series keeps every value.",
          self.frame.demo(
            1,
            shapes,
            "Sparkline::new(values)",
            &[
              "Sparkline::new([12.0, 4.0, -6.0, -2.0, 8.0, -9.0, 3.0, 10.0])",
              "Sparkline::new([5.0, 5.0, 5.0, 5.0])",
              "Sparkline::new([42.0])",
              "Sparkline::new(Vec::<f64>::new())",
              "Sparkline::new(series).color(PaletteColor::Chart2).width(160.0)",
            ],
          ),
        ),
        Section::new(
          "In rows of the app's",
          "A sparkline in each row of a list of builds, its color a tone the app chooses: the release build is \
           getting slower.",
          self.frame.demo(
            2,
            builds,
            "Row::new().child(name).child(Sparkline::new(minutes))",
            &[
              "Row::new()",
              "    .child(Text::new(\"release\"))",
              "    .child(Sparkline::new(minutes).color(PaletteColor::Destructive).label(\"release, minutes\"))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a sparkline is given.",
          props_table(&rows, &["new"], self.frame.room()),
        ),
      ],
    )
  }
}

/// A sparkline over a word that says what it shows.
fn caption(sparkline: Sparkline, words: &str) -> Column {
  Column::new()
    .spacing(6.0)
    .child(sparkline.label(words))
    .child(crate::kit::mono(words, 11.0))
}

/// A row of a build's name and the minutes of its last runs.
fn build_row(name: &str, minutes: &[f64], color: PaletteColor) -> Row {
  Row::new().spacing(16.0).child(Text::new(name).width(80.0)).child(
    Sparkline::new(minutes.to_vec())
      .color(color)
      .label(&format!("{name}, minutes")),
  )
}
