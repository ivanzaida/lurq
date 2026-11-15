//! The page of the gauge: its playground, the board's processor, gauges of other ranges and their parts.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx, theme::PaletteColor},
  components::Row,
  core::Signal,
  node::Element,
  ui::Gauge,
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{props_table, segmented_fill},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

const VALUES: [(&str, f64); 6] = [
  ("0", 0.0),
  ("25", 25.0),
  ("70", 70.0),
  ("90", 90.0),
  ("100", 100.0),
  ("130", 130.0),
];

const COLORS: [(&str, PaletteColor); 3] = [
  ("Chart1", PaletteColor::Chart1),
  ("Chart2", PaletteColor::Chart2),
  ("Chart4", PaletteColor::Chart4),
];

#[derive(Clone, Copy)]
struct Props {
  value: usize,
  zones: bool,
  color: usize,
  degrees: bool,
}

pub struct GaugePage {
  frame: Frame,
  playground: Playground,
  value: Signal<usize>,
  zones: Signal<bool>,
  color: Signal<usize>,
  degrees: Signal<bool>,
}

fn gauge(props: Props) -> Gauge {
  let mut gauge = Gauge::new(VALUES[props.value].1).label("CPU");
  if props.zones {
    gauge = gauge.low(60).high(85);
  } else {
    gauge = gauge.color(COLORS[props.color].1.clone());
  }
  if props.degrees {
    gauge = gauge.label("GPU").value_text(|degrees| format!("{degrees}°"));
  }
  gauge
}

impl GaugePage {
  fn props(&self) -> Props {
    Props {
      value: self.value.get().min(VALUES.len() - 1),
      zones: self.zones.get(),
      color: self.color.get().min(COLORS.len() - 1),
      degrees: self.degrees.get(),
    }
  }

  fn play(&self, ctx: &mut Ctx) -> Element {
    let props = self.props();
    let make = move || -> Element { gauge(props).into() };
    let mut code = vec![format!("Gauge::new({})", VALUES[props.value].0)];
    code.push(format!("    .label(\"{}\")", if props.degrees { "GPU" } else { "CPU" }));
    if props.zones {
      code.push("    .low(60)".to_owned());
      code.push("    .high(85)".to_owned());
    } else if props.color != 0 {
      code.push(format!("    .color(PaletteColor::{})", COLORS[props.color].0));
    }
    if props.degrees {
      code.push("    .value_text(|degrees| format!(\"{degrees}°\"))".to_owned());
    }
    let tree = vec![
      format!("root        {}   140 wide", self.playground.measured()),
      "├─ arc       140 × 72, a canvas: track 15.4 secondary, value's share chart 1".to_owned(),
      "│  └─ reading  20 at 600, its foot 10 over the ring's".to_owned(),
      "└─ range     bound mono 11 muted · label 12 at 500 · bound, 4 under".to_owned(),
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
    let mut controls = vec![
      prop(
        "value",
        "of 100",
        segmented(&self.value, places(&VALUES.map(|(name, _)| name))),
      ),
      divider(),
      flag("low / high (60, 85)", &self.zones),
    ];
    if !props.zones {
      controls.push(prop(
        "color",
        "PaletteColor",
        segmented(&self.color, places(&COLORS.map(|(name, _)| name))),
      ));
    }
    controls.push(flag("value_text (degrees)", &self.degrees));
    let changed = [props.value != 2, props.zones, props.color != 0, props.degrees]
      .into_iter()
      .filter(|changed| *changed)
      .count();
    let resets = (
      self.value.clone(),
      self.zones.clone(),
      self.color.clone(),
      self.degrees.clone(),
    );
    self.playground.card(
      ctx,
      Spec {
        title: "Gauge",
        make: Arc::new(make),
        shown: Shown::of(gauge(props)),
        fill: false,
        forceable: false,
        facts: format!("{} of 100", VALUES[props.value].0),
        event: "",
        code,
        tree,
        controls,
        changed,
        reset: Arc::new(move || {
          resets.0.set(2);
          resets.1.set(false);
          resets.2.set(0);
          resets.3.set(false);
        }),
      },
    )
  }
}

impl Component for GaugePage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      value: ctx.signal(2),
      zones: ctx.signal(false),
      color: ctx.signal(0),
      degrees: ctx.signal(false),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let parts = Row::new()
      .spacing(32.0)
      .child(Gauge::new(42).low(60).high(85).label("Load"))
      .child(Gauge::new(74).low(60).high(85).label("Load"))
      .child(Gauge::new(93).low(60).high(85).label("Load"));
    let ranges = Row::new()
      .spacing(32.0)
      .child(
        Gauge::new(86)
          .min(20)
          .max(100)
          .label("GPU")
          .value_text(|degrees| format!("{degrees}°")),
      )
      .child(
        Gauge::new(312)
          .max(1000)
          .label("Link")
          .color(PaletteColor::Chart2)
          .value_text(|megabits| format!("{megabits}"))
          .bound_text(|megabits| format!("{megabits} Mb")),
      );
    let rows = [
      ("new", "impl Into<f64>", "", "The value."),
      (
        "min / max",
        "impl Into<f64>",
        "0 / 100",
        "The range, from the ring's start to its end.",
      ),
      (
        "low / high / optimum",
        "impl Into<f64>",
        "",
        "The parts of the range as a meter's: the value's share is in the tone of its part.",
      ),
      (
        "color",
        "PaletteColor",
        "Chart1",
        "The role of the value's share of a gauge with no parts.",
      ),
      (
        "label",
        "&str",
        "",
        "What it measures, under the ring between its bounds: its name to tooling.",
      ),
      (
        "value_text",
        "Fn(f64) -> String",
        "\"70%\"",
        "Writes the reading: the value's share of the range in percent unless given.",
      ),
      (
        "bound_text",
        "Fn(f64) -> String",
        "\"0\", \"100\"",
        "Writes the bounds under the ring.",
      ),
      (
        "style / rules",
        "GaugePart",
        "",
        "Root, Arc, Track, Value, Reading, Range, Label, Bound.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Gauge"],
        title: "Gauge",
        description: "A value of a known range as the share of a half ring, its reading in it: the load of a \
                      processor, the speed of a link, the heat of a card.",
        import: Some("use lurq::ui::Gauge;"),
        status: Some("Preview"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Gauge is a control on the right, and its value too, past its range at 130. A gauge is \
           read, not operated: nothing on the stage is counted.",
          playground,
        ),
        Section::new(
          "CPU",
          "The board's gauge: 70% of the half ring from its start, the reading in it, the range and the label \
           under it.",
          self.frame.demo(
            0,
            Gauge::new(70).label("CPU"),
            "Gauge::new(70).label(\"CPU\")",
            &["Gauge::new(70).label(\"CPU\")"],
          ),
        ),
        Section::new(
          "Parts of the range",
          "With low and high the value's share is in the tone of its part, as a meter's segment is: good, a \
           warning, bad.",
          self.frame.demo(
            1,
            parts,
            ".low(60).high(85)",
            &[
              "Gauge::new(42).low(60).high(85).label(\"Load\")",
              "Gauge::new(74).low(60).high(85).label(\"Load\")",
              "Gauge::new(93).low(60).high(85).label(\"Load\")",
            ],
          ),
        ),
        Section::new(
          "Other ranges",
          "A range of the app's, its reading and its bounds written as the app writes them.",
          self.frame.demo(
            2,
            ranges,
            ".min(..).max(..).value_text(..)",
            &[
              "Gauge::new(86).min(20).max(100).label(\"GPU\").value_text(|degrees| format!(\"{degrees}°\"))",
              "Gauge::new(312).max(1000).label(\"Link\").bound_text(|megabits| format!(\"{megabits} Mb\"))",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a gauge is given.",
          props_table(&rows, &["new"], self.frame.room()),
        ),
      ],
    )
  }
}
