//! The page of the angle slider: its playground, the board's dial, ranges and steps, its states, and a dial in a
//! field.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::Signal,
  layout::Alignment,
  node::Element,
  ui::{AngleSlider, Field, Interaction},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet, segmented_fill},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(9.0)
    .child(Row::new().max_width(FILL).overflow_visible().child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new()
    .width(FILL)
    .spacing(28.0)
    .wrap()
    .align_items(Alignment::End)
    .overflow_visible()
}

/// An angle as the page says a signal holds it.
fn says(angle: f64) -> String {
  format!("the signal has: {angle}")
}

/// The ranges the playground offers: a whole turn, a half turn either way of 0, and two turns.
const RANGES: [(&str, (i32, i32)); 3] = [
  ("0 – 360", (0, 360)),
  ("-180 – 180", (-180, 180)),
  ("0 – 720", (0, 720)),
];

/// The ranges as the panel's control chooses them: by their place in [`RANGES`].
const RANGE_CHOICES: [(&str, usize); 3] = [(RANGES[0].0, 0), (RANGES[1].0, 1), (RANGES[2].0, 2)];

/// The steps the playground offers.
const STEPS: [(&str, i32); 4] = [("1", 1), ("5", 5), ("15", 15), ("45", 45)];

pub struct AngleSliderPage {
  frame: Frame,
  playground: Playground,
  angle: Signal<f64>,
  range: Signal<usize>,
  step: Signal<i32>,
  wrap: Signal<bool>,
  disabled: Signal<bool>,
  board: Signal<f64>,
  half: Signal<f64>,
  fine: Signal<f64>,
  light: Signal<f64>,
}

impl AngleSliderPage {
  /// The playground: the dial as the panel's props make it, bound to the page's signal.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (range, step) = (self.range.get(), self.step.get());
    let (min, max) = RANGES[range].1;
    let (wrap, disabled) = (self.wrap.get(), self.disabled.get());
    let plain = move |angle: Signal<f64>| {
      AngleSlider::new(angle)
        .label("Rotation")
        .min(min)
        .max(max)
        .step(step)
        .wrap(wrap)
        .disabled(disabled)
    };
    let record = self.playground.recorder();
    let forced = self.playground.forced();
    let shown = {
      let (plain, angle) = (plain, self.angle.clone());
      let (change, end) = (record.clone(), record);
      Shown::built(move |_| {
        plain(angle)
          .force(forced)
          .on_change(move |angle| change(format!("on_change({angle})")))
          .on_change_end(move |angle| end(format!("on_change_end({angle})")))
          .into()
      })
    };

    let mut code = vec![
      "AngleSlider::new(angle)".to_owned(),
      "    .label(\"Rotation\")".to_owned(),
    ];
    for (on, call) in [
      ((min, max) != (0, 360), format!("    .min({min}).max({max})")),
      (step != 1, format!("    .step({step})")),
      (!wrap, "    .wrap(false)".to_owned()),
      (disabled, "    .disabled(true)".to_owned()),
    ] {
      if on {
        code.push(call);
      }
    }

    let held = self.angle.get();
    let tree = vec![
      format!(
        "root     {}   circle · background · ring 1.5 input · a slider to tooling",
        self.playground.measured()
      ),
      format!("├─ value    \"{held}°\" · mono 13 · foreground"),
      format!("└─ handle   9 · primary · 26 from the middle at {held}°"),
    ];

    let (to_range, to_step) = (self.range.clone(), self.step.clone());
    let controls = vec![
      prop(
        "min, max",
        "f64",
        segmented_fill(&RANGE_CHOICES, &range, move |range| to_range.set(range)),
      ),
      prop(
        "step",
        "f64",
        segmented_fill(&STEPS, &step, move |step| to_step.set(step)),
      ),
      divider(),
      flag("wrap", &self.wrap),
      flag("disabled", &self.disabled),
    ];
    let resets = (
      self.angle.clone(),
      self.range.clone(),
      self.step.clone(),
      self.wrap.clone(),
      self.disabled.clone(),
    );
    let make_angle = Signal::new(held);
    let make = {
      let plain = move |angle: Signal<f64>| {
        AngleSlider::new(angle)
          .label("Rotation")
          .min(min)
          .max(max)
          .step(step)
          .wrap(wrap)
          .disabled(disabled)
      };
      Arc::new(move || plain(make_angle.clone()).into())
    };
    self.playground.card(
      ctx,
      Spec {
        title: "Angle Slider",
        make,
        shown,
        fill: false,
        forceable: true,
        facts: format!("{held}° of {min} to {max}"),
        event: "on_change",
        code,
        tree,
        controls,
        changed: [(min, max) != (0, 360), step != 1, !wrap, disabled]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          resets.0.set(45.0);
          resets.1.set(0);
          resets.2.set(1);
          resets.3.set(true);
          resets.4.set(false);
        }),
      },
    )
  }
}

impl Component for AngleSliderPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 0),
      angle: ctx.signal(45.0),
      range: ctx.signal(0),
      step: ctx.signal(1),
      wrap: ctx.signal(true),
      disabled: ctx.signal(false),
      board: ctx.signal(45.0),
      half: ctx.signal(-30.0),
      fine: ctx.signal(22.5),
      light: ctx.signal(135.0),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let playground = self.play(ctx);
    let board = items().child(item(
      AngleSlider::new(self.board.clone()).label("Rotation"),
      &says(self.board.get()),
    ));
    let ranges = items()
      .child(item(
        AngleSlider::new(self.half.clone())
          .label("Tilt")
          .min(-180)
          .max(180)
          .step(15)
          .wrap(false),
        &format!("-180 to 180, steps of 15, held at its ends: {}", self.half.get()),
      ))
      .child(item(
        AngleSlider::new(self.fine.clone()).label("Hue shift").step(0.5),
        &format!("steps of a half: {}", self.fine.get()),
      ));
    let states = items()
      .child(item(
        AngleSlider::controlled(45.0, |_| {}).force(Some(Interaction::FocusVisible)),
        "focus-visible",
      ))
      .child(item(AngleSlider::controlled(45.0, |_| {}).disabled(true), "disabled"));
    let field = items().child(item(
      Column::new().width(240.0).max_width(FILL).child(
        Field::new("Direction of the light", AngleSlider::new(self.light.clone()).step(5))
          .description("0 is from the top, clockwise."),
      ),
      &says(self.light.get()),
    ));
    let rows = [
      (
        "new",
        "Signal<f64>",
        "",
        "The angle the signal holds, in degrees; a drag, a press and the keys set it.",
      ),
      (
        "controlled",
        "f64, impl Fn(f64)",
        "",
        "In place of a signal: the app holds the angle and is told what a person makes it.",
      ),
      (
        "on_change",
        "impl Fn(f64)",
        "",
        "Each change as it is made: a drag as it moves, a press, a key. After the signal is set; not when the app \
         sets it.",
      ),
      (
        "on_change_end",
        "impl Fn(f64)",
        "",
        "The angle when the drag is let go or the key's step made, where it is another than it was as it began.",
      ),
      (
        "min, max",
        "impl Into<f64>",
        "0, 360",
        "The range. More than a turn is turned round more than once.",
      ),
      (
        "step",
        "impl Into<f64>",
        "1",
        "What an arrow key moves it by and what a drag snaps to.",
      ),
      (
        "page_step",
        "impl Into<f64>",
        "15",
        "What Page Up and Page Down move it by, in whole steps.",
      ),
      (
        "wrap",
        "bool",
        "true",
        "Where the range is a whole turn, it goes round: a step past 359 is 0. Off, or another range, it stops at \
         its ends.",
      ),
      (
        "disabled",
        "bool",
        "false",
        "Dimmed; it takes no input and is no stop of Tab.",
      ),
      (
        "label",
        "&str",
        "\"Angle\"",
        "What it is called to tooling. A field gives it its label.",
      ),
      (
        "value_text",
        "Fn(f64) -> String",
        "\"45°\"",
        "What it says the angle is, on the dial and to tooling, with the decimals of the step.",
      ),
    ];
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Angle Slider"],
        title: "Angle Slider",
        description: "A dial that sets an angle in degrees, by a drag round it, a press on it or the keys: the \
                      rotation of a layer, the direction of a gradient or of a light.",
        import: Some("use lurq::ui::AngleSlider;"),
        status: Some("New"),
        since: Some("0.63"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of AngleSlider is a control on the right. The dial on the stage is live: a press sets the \
           angle under it and a drag goes on round; the arrow keys, Page Up and Page Down, Home and End do once it \
           has the focus.",
          playground,
        ),
        Section::new(
          "Angle slider",
          "The board's: 64 across, the angle in its middle and the dot on its ring at it, 0 at the top and growing \
           clockwise. The page holds the angle in a signal. It is one stop of Tab, and to a screen reader a slider \
           that says its angle.",
          self.frame.demo(
            0,
            board,
            "AngleSlider::new(angle).label(\"Rotation\")",
            &["// angle: Signal<f64>", "AngleSlider::new(angle).label(\"Rotation\")"],
          ),
        ),
        Section::new(
          "Ranges and steps",
          "A range of less than a turn stops at its ends, and a press past them goes to the nearer end. A step of a \
           half says its decimals.",
          self.frame.demo(
            1,
            ranges,
            ".min(-180).max(180).step(15).wrap(false)",
            &[
              "AngleSlider::new(tilt).min(-180).max(180).step(15).wrap(false)",
              "AngleSlider::new(shift).step(0.5)",
            ],
          ),
        ),
        Section::new(
          "States",
          "The board draws the dial at rest. The ring of the focus is around the circle, and a disabled dial is \
           dimmed.",
          self.frame.demo(
            2,
            states,
            ".disabled(true)",
            &[
              "AngleSlider::new(angle).force(Some(Interaction::FocusVisible))",
              "AngleSlider::new(angle).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "In a field",
          "A field names the dial and says what it is for.",
          self.frame.demo(
            3,
            field,
            "Field::new(\"Direction of the light\", AngleSlider::new(light).step(5))",
            &[
              "// light: Signal<f64>",
              "Field::new(\"Direction of the light\", AngleSlider::new(light).step(5))",
              "    .description(\"0 is from the top, clockwise.\")",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What an angle slider is given.",
          props_table(&rows, &["new"], self.frame.room()),
        ),
      ],
    )
  }
}
