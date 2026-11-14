//! The page of the slider: its playground, the slider bound to a signal, its
//! states, one with marks under it, the vertical one, the range slider and
//! what the keys do.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::{Alignment, layout_kind::Justify},
  node::Element,
  ui::{Interaction, Kbd, Length, RangeSlider, Slider, SliderOrientation, SliderPart},
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
/// What each key does to a focused slider, and to the focused thumb of a range slider.
const KEYS: [(&str, &str); 8] = [
  ("Right  Up", "One step toward the greatest value, across or up."),
  ("Left  Down", "One step toward the least."),
  (
    "Page Up",
    "A tenth of the range toward the greatest, in whole steps and at least one: as a browser's range input pages.",
  ),
  ("Page Down", "A tenth of the range toward the least."),
  ("Home", "To the least value; a range's high thumb stops at the low one."),
  (
    "End",
    "To the greatest value; a range's low thumb stops at the high one.",
  ),
  (
    "Tab",
    "To the next stop. A range slider is two: its low thumb, then its high one.",
  ),
  (
    "Shift+Tab",
    "Back: a range slider's high thumb first, then its low one.",
  ),
];

/// The ways the playground's track can run.
const ORIENTATIONS: [(&str, SliderOrientation); 2] = [
  ("Horizontal", SliderOrientation::Horizontal),
  ("Vertical", SliderOrientation::Vertical),
];

pub struct SliderPage {
  frame: Frame,
  playground: Playground,
  value: Signal<i32>,
  label: Signal<String>,
  min: Signal<i32>,
  max: Signal<i32>,
  step: Signal<i32>,
  orientation: Signal<SliderOrientation>,
  disabled: Signal<bool>,
  volume: Signal<i32>,
  opacity: Signal<f32>,
  marked: Signal<i32>,
  levels: [Signal<i32>; 3],
  price: Signal<(i32, i32)>,
  hours: Signal<(i32, i32)>,
}

impl SliderPage {
  /// The playground: the slider as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (value, label) = (self.value.get(), self.label.get());
    let (min, max, step, disabled) = (self.min.get(), self.max.get(), self.step.get(), self.disabled.get());
    let orientation = self.orientation.get();
    let vertical = orientation == SliderOrientation::Vertical;
    let plain = {
      let (bound, label) = (self.value.clone(), label.clone());
      move || {
        Slider::new(bound.clone())
          .min(min)
          .max(max)
          .step(step)
          .orientation(orientation)
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
    if vertical {
      code.push("    .orientation(SliderOrientation::Vertical)".to_owned());
    }
    code.push(format!("    .label({label:?})"));
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }

    let span = (max - min).max(1) as f32;
    let share = ((value - min) as f32 / span * 100.0).clamp(0.0, 100.0).round();
    let track = if vertical { "4 × 140 px" } else { "240 × 4 px" };
    let tree = vec![
      format!("root       {}", self.playground.measured()),
      format!("├─ track   {track}     secondary"),
      format!("├─ range   {share}% of the track   primary"),
      format!("└─ thumb   16 × 16 px     at {value}"),
    ];

    let (to_value, to_label) = (self.value.clone(), self.label.clone());
    let (to_min, to_max, to_step, to_orientation, to_disabled) = (
      self.min.clone(),
      self.max.clone(),
      self.step.clone(),
      self.orientation.clone(),
      self.disabled.clone(),
    );
    let (set_min, set_max, set_step, set_orientation) = (
      self.min.clone(),
      self.max.clone(),
      self.step.clone(),
      self.orientation.clone(),
    );
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
          prop(
            "orientation",
            "SliderOrientation",
            self
              .playground
              .select(3, &ORIENTATIONS, &orientation, move |orientation| {
                set_orientation.set(orientation)
              }),
          ),
          divider(),
          flag("disabled", &self.disabled),
        ],
        changed: [min != 0, max != 100, step != 1, vertical, disabled]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to_value.set(40);
          to_label.set("Volume".to_owned());
          to_min.set(0);
          to_max.set(100);
          to_step.set(1);
          to_orientation.set(SliderOrientation::Horizontal);
          to_disabled.set(false);
        }),
      },
    )
  }
}

impl Component for SliderPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let playground = Playground::new(ctx, 4);
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
      orientation: ctx.signal(SliderOrientation::Horizontal),
      disabled: ctx.signal(false),
      volume: ctx.signal(40),
      opacity: ctx.signal(0.5),
      marked: ctx.signal(50),
      levels: [ctx.signal(70), ctx.signal(45), ctx.signal(20)],
      price: ctx.signal((20, 80)),
      hours: ctx.signal((9, 18)),
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
    // What the keys do, a row each: the key as it is on a keyboard, and what it does.
    let keyboard = KEYS
      .into_iter()
      .fold(Column::new().width(FILL).spacing(10.0), |list, (key, does)| {
        let chord = key
          .split("  ")
          .fold(Row::new().width(140.0).spacing(6.0), |chord, key| {
            chord.child(Kbd::shortcut(key))
          });
        list.child(
          Row::new()
            .width(FILL)
            .align_items(Alignment::Center)
            .child(chord)
            .child(quiet(does, 13.0)),
        )
      });
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
    // The board's vertical slider, three of them side by side as a mixer's channels; the last one shorter.
    let channels = ["Bass", "Mid", "Treble"]
      .into_iter()
      .zip(&self.levels)
      .fold(Row::new().spacing(24.0), |row, (name, level)| {
        let slider = Slider::new(level.clone())
          .orientation(SliderOrientation::Vertical)
          .label(name);
        let slider = if name == "Treble" { slider.length(100.0) } else { slider };
        row.child(item(slider, &format!("{name} {}", level.get())))
      })
      .align_items(Alignment::End);
    let upright = items().child(channels);
    let (low, high) = self.price.get();
    let (open, close) = self.hours.get();
    let ranges = items()
      .child(item(
        RangeSlider::new(self.price.clone()).step(5).label("Price"),
        &format!("the signal has: ({low}, {high})"),
      ))
      .child(item(
        RangeSlider::new(self.hours.clone()).max(24).label("Opening hours"),
        &format!("open {open}:00 to {close}:00"),
      ))
      .child(item(
        RangeSlider::new(Signal::new((20, 60))).label("Disabled").disabled(true),
        "disabled",
      ));
    let rows = [
      (
        "value",
        "Signal<i32> / Signal<f32>",
        "",
        "What the slider is bound to: Slider::new for whole numbers, Slider::new_f32 for fractions. A drag, a press \
         on the track and the arrow keys set it. A RangeSlider is bound to a Signal<(i32, i32)> or a \
         Signal<(f32, f32)>, its low value and its high one.",
      ),
      ("min", "impl Into<f64>", "0", "The least value: the start of the track."),
      (
        "max",
        "impl Into<f64>",
        "100, or 1",
        "The greatest value: the end of the track. 100 for whole numbers, 1 for fractions.",
      ),
      (
        "orientation",
        "SliderOrientation",
        "Horizontal",
        "Which way the track runs. Vertical is 16 wide and 140 high, the least value at the bottom; the up and down \
         arrows move it.",
      ),
      (
        "length",
        "f32",
        "240, or 140",
        "How long the slider is along its track: its width across, its height up.",
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
           the track, or Tab to it and use the arrow keys, Page Up and Page Down, Home and End.",
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
          "Vertical",
          "A slider whose track runs up, its least value at the bottom: a level, a mixer's channel. Its length is \
           its height, here 100 for the last.",
          self.frame.demo(
            3,
            upright,
            ".orientation(SliderOrientation::Vertical)",
            &[
              "// bass, mid, treble: Signal<i32>",
              "Row::new()",
              "    .spacing(24.0)",
              "    .align_items(Alignment::End)",
              "    .child(",
              "        Slider::new(bass.clone())",
              "            .orientation(SliderOrientation::Vertical)",
              "            .label(\"Bass\"),",
              "    )",
              "    .child(",
              "        Slider::new(mid.clone())",
              "            .orientation(SliderOrientation::Vertical)",
              "            .label(\"Mid\"),",
              "    )",
              "    .child(",
              "        Slider::new(treble.clone())",
              "            .orientation(SliderOrientation::Vertical)",
              "            .length(100.0)",
              "            .label(\"Treble\"),",
              "    )",
            ],
          ),
        ),
        Section::new(
          "Range slider",
          "Two values on one track, the range between them filled. A press on the track moves the nearer thumb, a \
           drag the thumb it began on, and neither passes the other. Each thumb is a stop of Tab, the low one first.",
          self.frame.demo(
            4,
            ranges,
            "RangeSlider::new(price.clone())",
            &[
              "// price, hours: Signal<(i32, i32)>",
              "RangeSlider::new(price.clone()).step(5).label(\"Price\")",
              "",
              "RangeSlider::new(hours.clone()).max(24).label(\"Opening hours\")",
              "",
              "RangeSlider::new(range.clone()).label(\"Disabled\").disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Keyboard",
          "What the keys do to the slider Tab is on: the same across and up, and for each thumb of a range slider, \
           which never passes the other. A disabled slider is not a stop and takes no key.",
          keyboard,
        ),
        Section::new(
          "Props",
          "What a slider is given; a range slider is given the same, bound to two values.",
          props_table(&rows, &["value"], self.frame.room()),
        ),
      ],
    )
  }
}
