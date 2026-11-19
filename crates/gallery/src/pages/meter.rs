//! The page of the meter: its playground, the board's disk, a battery whose best is its top, and meters of one tone.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::Column,
  core::Signal,
  node::Element,
  ui::{Meter, MeterPart, MeterVariant},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{props_table, segmented_fill},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// The tones of a meter without parts, as the code names them.
const VARIANTS: [(&str, MeterVariant); 5] = [
  ("Neutral", MeterVariant::Neutral),
  ("Info", MeterVariant::Info),
  ("Success", MeterVariant::Success),
  ("Warning", MeterVariant::Warning),
  ("Destructive", MeterVariant::Destructive),
];

/// The values the playground's disk may hold, in gigabytes of 128: good, a warning, bad, and past the range.
const VALUES: [(&str, f64); 5] = [("20", 20.0), ("64", 64.0), ("96", 96.0), ("120", 120.0), ("140", 140.0)];

const SEGMENTS: [(&str, usize); 4] = [("8", 8), ("12", 12), ("16", 16), ("24", 24)];

/// The board's meter: a disk running low.
fn board() -> Meter {
  Meter::new(96)
    .max(128)
    .low(64)
    .high(96)
    .label("Disk space")
    .value_text(|used| format!("{used} of 128 GB"))
    .description("Running low. Free up space soon.")
    .style(MeterPart::Root, |s| s.width(260.0))
}

#[derive(Clone, Copy)]
struct Props {
  value: f64,
  segments: usize,
  zones: bool,
  /// Which of [`VARIANTS`].
  variant: usize,
  description: bool,
  units: bool,
}

pub struct MeterPage {
  frame: Frame,
  playground: Playground,
  value: Signal<usize>,
  segments: Signal<usize>,
  zones: Signal<bool>,
  variant: Signal<usize>,
  description: Signal<bool>,
  units: Signal<bool>,
}

fn disk(props: Props) -> Meter {
  let mut meter = Meter::new(props.value)
    .max(128)
    .segments(props.segments)
    .label("Disk space")
    .style(MeterPart::Root, |s| s.width(280.0));
  if props.zones {
    meter = meter.low(64).high(96);
  } else {
    meter = meter.variant(VARIANTS[props.variant].1);
  }
  if props.units {
    meter = meter.value_text(|used| format!("{used} of 128 GB"));
  }
  if props.description {
    let words = match props.value {
      value if value < 64.0 => "Plenty of room.",
      value if value <= 96.0 => "Running low. Free up space soon.",
      _ => "Almost full. Free up space now.",
    };
    meter = meter.description(words);
  }
  meter
}

impl MeterPage {
  fn props(&self) -> Props {
    Props {
      value: VALUES[self.value.get().min(VALUES.len() - 1)].1,
      segments: SEGMENTS[self.segments.get().min(SEGMENTS.len() - 1)].1,
      zones: self.zones.get(),
      variant: self.variant.get().min(VARIANTS.len() - 1),
      description: self.description.get(),
      units: self.units.get(),
    }
  }

  fn play(&self, ctx: &mut Ctx) -> Element {
    let props = self.props();
    let make = move || -> Element { disk(props).into() };
    let mut code = vec![format!("Meter::new({})", props.value), "    .max(128)".to_owned()];
    if props.segments != 16 {
      code.push(format!("    .segments({})", props.segments));
    }
    if props.zones {
      code.push("    .low(64)".to_owned());
      code.push("    .high(96)".to_owned());
    } else {
      code.push(format!("    .variant(MeterVariant::{})", VARIANTS[props.variant].0));
    }
    code.push("    .label(\"Disk space\")".to_owned());
    if props.units {
      code.push("    .value_text(|used| format!(\"{used} of 128 GB\"))".to_owned());
    }
    if props.description {
      code.push("    .description(\"..\")".to_owned());
    }
    let tree = vec![
      format!("root        {}   8 apart", self.playground.measured()),
      "├─ header    label 13 at 500 · value mono 11 muted, at the end".to_owned(),
      format!("├─ segments  {}, 8 high, 2 apart, radius 2", props.segments),
      "└─ description  12, in the tone of a warning or bad value".to_owned(),
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
        "of 128",
        segmented(&self.value, places(&VALUES.map(|(name, _)| name))),
      ),
      prop(
        "segments",
        "usize",
        segmented(&self.segments, places(&SEGMENTS.map(|(name, _)| name))),
      ),
      divider(),
      flag("low / high (64, 96)", &self.zones),
    ];
    if !props.zones {
      controls.push(prop(
        "variant",
        "MeterVariant",
        segmented(&self.variant, places(&VARIANTS.map(|(name, _)| name))),
      ));
    }
    controls.extend([
      flag("value_text (GB)", &self.units),
      flag("description", &self.description),
    ]);
    let changed = [
      self.value.get() != 2,
      self.segments.get() != 2,
      !props.zones,
      props.variant != 0,
      !props.units,
      !props.description,
    ]
    .into_iter()
    .filter(|changed| *changed)
    .count();
    let resets = (
      [
        (self.value.clone(), 2),
        (self.segments.clone(), 2),
        (self.variant.clone(), 0),
      ],
      [self.zones.clone(), self.units.clone(), self.description.clone()],
    );
    self.playground.card(
      ctx,
      Spec {
        title: "Meter",
        make: Arc::new(make),
        shown: Shown::of(self.playground.styled(disk(props))),
        fill: false,
        forceable: false,
        facts: format!("{} of 128", props.value),
        event: "",
        code,
        tree,
        controls,
        changed,
        reset: Arc::new(move || {
          for (signal, value) in &resets.0 {
            signal.set(*value);
          }
          for signal in &resets.1 {
            signal.set(true);
          }
        }),
      },
    )
  }
}

impl Component for MeterPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      value: ctx.signal(2),
      segments: ctx.signal(2),
      zones: ctx.signal(true),
      variant: ctx.signal(0),
      description: ctx.signal(true),
      units: ctx.signal(true),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let battery = Meter::new(18)
      .low(20)
      .high(80)
      .optimum(100)
      .segments(10)
      .label("Battery")
      .value_text(|charge| format!("{charge}%"))
      .description("Plug in soon.")
      .style(MeterPart::Root, |s| s.width(260.0));
    let tones = Column::new()
      .width(260.0)
      .spacing(20.0)
      .child(
        Meter::new(12)
          .max(20)
          .segments(20)
          .variant(MeterVariant::Info)
          .label("Seats used"),
      )
      .child(
        Meter::new(3)
          .max(5)
          .segments(5)
          .variant(MeterVariant::Success)
          .label("Checks passed")
          .value_text(|passed| format!("{passed} of 5")),
      );

    let rows = [
      ("new", "impl Into<f64>", "", "The value."),
      (
        "min / max",
        "impl Into<f64>",
        "0 / 100",
        "The range, as a progress bar and a slider say it.",
      ),
      (
        "low / high",
        "impl Into<f64>",
        "",
        "Where the low part of the range ends and the high part starts: each filled segment is in the tone of its \
         part.",
      ),
      (
        "optimum",
        "impl Into<f64>",
        "min",
        "The best value: its part is good, the next a warning, the far one bad.",
      ),
      (
        "variant",
        "MeterVariant",
        "Neutral",
        "The tone of every filled segment of a meter with no low and no high.",
      ),
      ("segments", "usize", "16", "How many segments the range is cut into."),
      (
        "label",
        "&str",
        "",
        "What it measures, over the segments: its name to tooling.",
      ),
      (
        "description",
        "&str",
        "",
        "The line under the segments, in the tone of a value that is a warning or bad.",
      ),
      (
        "value_text",
        "Fn(f64) -> String",
        "\"96 of 128\"",
        "Writes the value as the app does.",
      ),
      (
        "style / rules",
        "MeterPart",
        "",
        "Root, Header, Label, Value, Segments, Segment, Description.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Data", "Meter"],
        title: "Meter",
        description: "How much of a known range a value takes, and whether that is good: the space a disk has \
                      used, a battery's charge, the seats of a plan.",
        import: Some("use lurq::ui::Meter;"),
        status: Some("Preview"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Meter is a control on the right, and its value too: good, a warning, bad, and past its \
           range. A meter is read, not operated: nothing on the stage is counted.",
          playground,
        ),
        Section::new(
          "Disk space",
          "The board's meter. Good under 64, a warning to 96, bad past it: each filled segment is in the tone of \
           its part of the range, and the description in the tone of the value.",
          self.frame.demo(
            0,
            board(),
            "Meter::new(96).max(128).low(64).high(96)",
            &[
              "Meter::new(96)",
              "    .max(128)",
              "    .low(64)",
              "    .high(96)",
              "    .label(\"Disk space\")",
              "    .value_text(|used| format!(\"{used} of 128 GB\"))",
              "    .description(\"Running low. Free up space soon.\")",
            ],
          ),
        ),
        Section::new(
          "The best at the top",
          "A battery's best value is its greatest: optimum(100) makes the high part good and the low part bad.",
          self.frame.demo(
            1,
            battery,
            ".optimum(100)",
            &[
              "Meter::new(18)",
              "    .low(20)",
              "    .high(80)",
              "    .optimum(100)",
              "    .segments(10)",
              "    .label(\"Battery\")",
              "    .value_text(|charge| format!(\"{charge}%\"))",
              "    .description(\"Plug in soon.\")",
            ],
          ),
        ),
        Section::new(
          "One tone",
          "Without low and high, every filled segment is in the variant's tone.",
          self.frame.demo(
            2,
            tones,
            ".variant(MeterVariant::Info)",
            &[
              "Meter::new(12).max(20).segments(20).variant(MeterVariant::Info).label(\"Seats used\")",
              "Meter::new(3).max(5).segments(5).variant(MeterVariant::Success).label(\"Checks passed\")",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a meter is given.",
          props_table(&rows, &["new"], self.frame.room()),
        ),
      ],
    )
  }
}
