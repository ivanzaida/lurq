//! The page of the slider: its playground, the slider bound to a signal, its
//! states, and one with marks under it.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify},
  node::Element,
  ui::{Interaction, Length, Slider, SliderPart},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Spec, divider, field, flag, prop},
};

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(10.0)
    .child(Row::new().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new().width(FILL).spacing(28.0).wrap().align_items(Alignment::End)
}

/// The least values, the greatest and the steps the playground offers.
const MINS: [(&str, i32); 3] = [("0", 0), ("10", 10), ("-50", -50)];
const MAXES: [(&str, i32); 3] = [("100", 100), ("50", 50), ("10", 10)];
const STEPS: [(&str, i32); 4] = [("1", 1), ("5", 5), ("10", 10), ("25", 25)];

pub struct SliderPage {
  frame: Frame,
  playground: Playground,
  value: Signal<i32>,
  label: Signal<String>,
  min: Signal<i32>,
  max: Signal<i32>,
  step: Signal<i32>,
  disabled: Signal<bool>,
  volume: Signal<i32>,
  opacity: Signal<f32>,
  marked: Signal<i32>,
}

impl SliderPage {
  /// The playground: the slider as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (value, label) = (self.value.get(), self.label.get());
    let (min, max, step, disabled) = (self.min.get(), self.max.get(), self.step.get(), self.disabled.get());
    let plain = {
      let (bound, label) = (self.value.clone(), label.clone());
      move || {
        Slider::new(bound.clone())
          .min(min)
          .max(max)
          .step(step)
          .label(&label)
          .disabled(disabled)
      }
    };
    let slider = plain().force(self.playground.forced());

    let mut code = vec![
      "// volume: Signal<i32>".to_owned(),
      "Slider::new(volume.clone())".to_owned(),
    ];
    if min != 0 {
      code.push(format!("    .min({min})"));
    }
    if max != 100 {
      code.push(format!("    .max({max})"));
    }
    if step != 1 {
      code.push(format!("    .step({step})"));
    }
    code.push(format!("    .label({label:?})"));
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }

    let span = (max - min).max(1) as f32;
    let share = ((value - min) as f32 / span * 100.0).clamp(0.0, 100.0).round();
    let tree = vec![
      format!("root       {}", self.playground.measured()),
      "├─ track   240 × 4 px     secondary".to_owned(),
      format!("├─ range   {share}% of the track   primary"),
      format!("└─ thumb   16 × 16 px     at {value}"),
    ];

    let (to_value, to_label) = (self.value.clone(), self.label.clone());
    let (to_min, to_max, to_step, to_disabled) = (
      self.min.clone(),
      self.max.clone(),
      self.step.clone(),
      self.disabled.clone(),
    );
    let (set_min, set_max, set_step) = (self.min.clone(), self.max.clone(), self.step.clone());
    self.playground.card(
      ctx,
      Spec {
        title: "Slider",
        make: Arc::new(move || plain().into()),
        shown: slider.into(),
        fill: false,
        forceable: true,
        facts: format!("value {value}"),
        event: "value",
        code,
        tree,
        controls: vec![
          prop("label", "&str", field(&self.label)),
          prop(
            "min",
            "impl Into<f64>",
            self.playground.select(0, &MINS, &min, move |min| set_min.set(min)),
          ),
          prop(
            "max",
            "impl Into<f64>",
            self.playground.select(1, &MAXES, &max, move |max| set_max.set(max)),
          ),
          prop(
            "step",
            "impl Into<f64>",
            self.playground.select(2, &STEPS, &step, move |step| set_step.set(step)),
          ),
          divider(),
          flag("disabled", &self.disabled),
        ],
        changed: [min != 0, max != 100, step != 1, disabled]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to_value.set(40);
          to_label.set("Volume".to_owned());
          to_min.set(0);
          to_max.set(100);
          to_step.set(1);
          to_disabled.set(false);
        }),
      },
    )
  }
}

impl Component for SliderPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let playground = Playground::new(ctx, 3);
    let value = ctx.signal(40);
    let record = playground.recorder();
    ctx.watch(&value, move |value| record(format!("value = {value}")));
    Self {
      frame: Frame::new(ctx),
      playground,
      value,
      label: ctx.signal("Volume".to_owned()),
      min: ctx.signal(0),
      max: ctx.signal(100),
      step: ctx.signal(1),
      disabled: ctx.signal(false),
      volume: ctx.signal(40),
      opacity: ctx.signal(0.5),
      marked: ctx.signal(50),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let live = items()
      .child(item(
        Slider::new(self.volume.clone()).label("Volume"),
        &format!("the signal has: {}", self.volume.get()),
      ))
      .child(item(
        Slider::new_f32(self.opacity.clone()).step(0.05).label("Opacity"),
        &format!("the signal has: {:.2}", self.opacity.get()),
      ));
    let states = items()
      .child(item(Slider::new(Signal::new(0)).label("Empty"), "at the least"))
      .child(item(Slider::new(Signal::new(40)).label("Rest"), "at 40"))
      .child(item(
        Slider::new(Signal::new(40))
          .label("Focused")
          .force(Some(Interaction::FocusVisible)),
        "focus-visible",
      ))
      .child(item(
        Slider::new(Signal::new(40)).label("Disabled").disabled(true),
        "disabled",
      ));
    // The board's slider with marks: the slider as wide as what holds it, the marks under it from end to end.
    let marks = ["0", "25", "50", "75", "100"]
      .into_iter()
      .fold(Row::new().width(FILL).justify(Justify::SpaceBetween), |row, mark| {
        row.child(quiet(mark, 11.0))
      });
    let marked = items().child(item(
      Column::new()
        .width(240.0)
        .spacing(6.0)
        .child(
          Slider::new(self.marked.clone())
            .step(25)
            .label("Quality")
            .style(SliderPart::Root, |s| s.width(Length::Fill)),
        )
        .child(marks),
      &format!("the signal has: {}", self.marked.get()),
    ));
    let rows = [
      (
        "value",
        "Signal<i32> / Signal<f32>",
        "",
        "What the slider is bound to: Slider::new for whole numbers, Slider::new_f32 for fractions. A drag, a press \
         on the track and the arrow keys set it.",
      ),
      ("min", "impl Into<f64>", "0", "The least value: the start of the track."),
      (
        "max",
        "impl Into<f64>",
        "100, or 1",
        "The greatest value: the end of the track. 100 for whole numbers, 1 for fractions.",
      ),
      (
        "step",
        "impl Into<f64>",
        "1, or 0.01",
        "How far apart the values are, from the least: what an arrow key moves it by and a drag snaps to.",
      ),
      (
        "label",
        "&str",
        "",
        "What the slider is called, to tooling that reads the tree. The field it is in shows it.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Half the opacity; it takes no input, and Tab passes it by.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Slider"],
        title: "Slider",
        description: "A value picked along a track, between a least and a greatest: volume, the size of a brush, a \
                      price no higher than.",
        import: Some("use lurq::ui::Slider;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Slider is a control on the right. The slider on the stage is live: drag its thumb, press \
           the track, or Tab to it and use the arrow keys.",
          playground,
        ),
        Section::new(
          "Slider",
          "The page holds each value in a signal the slider is bound to. A fraction is a Signal<f32>, here in \
           steps of a twentieth.",
          self.frame.demo(
            0,
            live,
            "Slider::new(volume.clone())",
            &[
              "// volume: Signal<i32>, opacity: Signal<f32>",
              "Slider::new(volume.clone()).label(\"Volume\")",
              "",
              "Slider::new_f32(opacity.clone())",
              "    .step(0.05)",
              "    .label(\"Opacity\")",
            ],
          ),
        ),
        Section::new(
          "States",
          "The track is the secondary color, filled in the primary color up to the thumb. The ring of the focus is \
           the thumb's, and a disabled slider is dimmed.",
          self.frame.demo(
            1,
            states,
            ".disabled(true)",
            &[
              "Slider::new(empty.clone()).label(\"Empty\")",
              "Slider::new(rest.clone()).label(\"Rest\")",
              "",
              "// Drawn focused from the keyboard with nothing focused: for a gallery, a test.",
              "Slider::new(rest.clone()).force(Some(Interaction::FocusVisible))",
              "",
              "Slider::new(rest.clone()).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "With marks",
          "Steps of 25 with the values marked under the track: the slider as wide as the column it is in, and a \
           row of text from one end to the other.",
          self.frame.demo(
            2,
            marked,
            ".step(25)",
            &[
              "// quality: Signal<i32>",
              "Column::new()",
              "    .width(240.0)",
              "    .spacing(6.0)",
              "    .child(",
              "        Slider::new(quality.clone())",
              "            .step(25)",
              "            .label(\"Quality\")",
              "            .style(SliderPart::Root, |s| s.width(Length::Fill)),",
              "    )",
              "    .child(",
              "        Row::new()",
              "            .width(Dimension::Pct(100.0))",
              "            .justify(Justify::SpaceBetween)",
              "            .child(Text::new(\"0\"))",
              "            .child(Text::new(\"25\"))",
              "            .child(Text::new(\"50\"))",
              "            .child(Text::new(\"75\"))",
              "            .child(Text::new(\"100\")),",
              "    )",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a slider is given.",
          props_table(&rows, &["value"], self.frame.room()),
        ),
      ],
    )
  }
}
