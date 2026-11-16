//! The page of the time field: its playground, a time bound to a signal and one that may be none, seconds, a clock of
//! twelve hours, a step of the minutes, and what the keys do.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::{DateLocale, HourCycle, Signal, Time},
  layout::Alignment,
  node::Element,
  ui::{DateFieldSize, DateMemory, Field, TimeField},
};

use super::date_field::{SEGMENT_KEYS, keys_list};
use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(10.0)
    .child(Row::new().max_width(FILL).child(shown))
    .child(quiet(caption, 12.0))
}

fn items() -> Row {
  Row::new().width(FILL).spacing(28.0).wrap().align_items(Alignment::End)
}

const STEPS: [(&str, u32); 4] = [("1", 1), ("5", 5), ("15", 15), ("30", 30)];
const CYCLES: [(&str, Option<HourCycle>); 3] = [
  ("Locale's", None),
  ("H23", Some(HourCycle::H23)),
  ("H12", Some(HourCycle::H12)),
];

fn time(hour: u8, minute: u8) -> Time {
  Time::new(hour, minute, 0).unwrap()
}

pub struct TimeFieldPage {
  frame: Frame,
  playground: Playground,
  value: Signal<Option<Time>>,
  step: Signal<u32>,
  cycle: Signal<Option<HourCycle>>,
  seconds: Signal<bool>,
  invalid: Signal<bool>,
  disabled: Signal<bool>,
  memory: DateMemory,
  starts: Signal<Time>,
  alarm: Signal<Option<Time>>,
  opens: Signal<Time>,
  kept: [DateMemory; 6],
}

impl TimeFieldPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (value, step, cycle) = (self.value.get(), self.step.get(), self.cycle.get());
    let (seconds, invalid, disabled) = (self.seconds.get(), self.invalid.get(), self.disabled.get());
    let plain = {
      let bound = self.value.clone();
      move |memory: &DateMemory| {
        let mut field = TimeField::optional(bound.clone())
          .step(step)
          .seconds(seconds)
          .invalid(invalid)
          .disabled(disabled)
          .label("Starts at")
          .memory(memory);
        if let Some(cycle) = cycle {
          field = field.hour_cycle(cycle);
        }
        field
      }
    };
    let record = self.playground.recorder();
    let shown = plain(&self.memory)
      .force(self.playground.forced())
      .on_change(move |time| match time {
        Some(time) => record(format!("on_change(Some({time}))")),
        None => record("on_change(None)".to_owned()),
      });
    let detached = DateMemory::new();
    let make: Arc<dyn Fn() -> Element + Send + Sync> = Arc::new(move || plain(&detached).into());
    let mut code = vec![
      "// starts: Signal<Option<Time>>".to_owned(),
      "TimeField::optional(starts.clone())".to_owned(),
    ];
    if step != 1 {
      code.push(format!("    .step({step})"));
    }
    if seconds {
      code.push("    .seconds(true)".to_owned());
    }
    if let Some(cycle) = cycle {
      code.push(format!("    .hour_cycle(HourCycle::{cycle:?})"));
    }
    for (on, call) in [(invalid, "invalid"), (disabled, "disabled")] {
      if on {
        code.push(format!("    .{call}(true)"));
      }
    }
    code.push("    .label(\"Starts at\")".to_owned());
    code.push("    .mount(ctx)".to_owned());
    let tree = vec![
      format!("root           {}", self.playground.measured()),
      format!(
        "├─ segments     hours, minutes{}{}",
        if seconds { ", seconds" } else { "" },
        if cycle == Some(HourCycle::H12) { ", AM/PM" } else { "" }
      ),
      "└─ icon         the timer's glyph, 16 px".to_owned(),
    ];
    let to = (
      self.value.clone(),
      self.step.clone(),
      self.cycle.clone(),
      [self.seconds.clone(), self.invalid.clone(), self.disabled.clone()],
    );
    let (set_step, set_cycle) = (self.step.clone(), self.cycle.clone());
    self.playground.card(
      ctx,
      Spec {
        title: "Time Field",
        make,
        shown: Shown::of(shown),
        fill: false,
        forceable: true,
        facts: format!("value {}", value.map_or("None".to_owned(), |time| time.to_string())),
        event: "on_change",
        code,
        tree,
        controls: vec![
          prop(
            "step",
            "impl Into<f64>",
            self.playground.select(0, &STEPS, &step, move |step| set_step.set(step)),
          ),
          prop(
            "hour_cycle",
            "HourCycle",
            self
              .playground
              .select(1, &CYCLES, &cycle, move |cycle| set_cycle.set(cycle)),
          ),
          divider(),
          flag("seconds", &self.seconds),
          flag("invalid", &self.invalid),
          flag("disabled", &self.disabled),
        ],
        changed: [step != 1, cycle.is_some(), seconds, invalid, disabled]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to.0.set(Some(time(14, 2)));
          to.1.set(1);
          to.2.set(None);
          for flag in &to.3 {
            flag.set(false);
          }
        }),
      },
    )
  }
}

impl Component for TimeFieldPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      value: ctx.signal(Some(time(14, 2))),
      step: ctx.signal(1),
      cycle: ctx.signal(None),
      seconds: ctx.signal(false),
      invalid: ctx.signal(false),
      disabled: ctx.signal(false),
      memory: DateMemory::new(),
      starts: ctx.signal(time(9, 30)),
      alarm: ctx.signal(None),
      opens: ctx.signal(time(8, 0)),
      kept: std::array::from_fn(|_| DateMemory::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let says = |time: Option<Time>| {
      format!(
        "the signal has: {}",
        time.map_or("None".to_owned(), |time| time.to_string())
      )
    };
    let bound = items()
      .child(item(
        TimeField::new(self.starts.clone())
          .step(15)
          .label("Starts at")
          .memory(&self.kept[0]),
        &format!("{}, a step of 15", says(Some(self.starts.get()))),
      ))
      .child(item(
        TimeField::optional(self.alarm.clone())
          .seconds(true)
          .label("Lap")
          .memory(&self.kept[1]),
        &format!("{}, with seconds", says(self.alarm.get())),
      ));
    let clocks = items()
      .child(item(
        TimeField::new(Signal::new(time(14, 30)))
          .locale(DateLocale::for_tag("en-US"))
          .memory(&self.kept[2]),
        "en-US: a clock of twelve hours",
      ))
      .child(item(
        TimeField::new(Signal::new(time(14, 30)))
          .hour_cycle(HourCycle::H12)
          .locale(DateLocale::for_tag("en-GB"))
          .memory(&self.kept[3]),
        "hour_cycle(H12)",
      ));
    let fields = items().child(
      Field::new(
        "Opens at",
        TimeField::new(self.opens.clone())
          .min(time(6, 0))
          .max(time(12, 0))
          .memory(&self.kept[4]),
      )
      .description("Between 6:00 and 12:00."),
    );
    let states = items()
      .child(item(
        TimeField::new(Signal::new(time(9, 0)))
          .size(DateFieldSize::Lg)
          .memory(&self.kept[5]),
        "Lg",
      ))
      .child(item(TimeField::new(Signal::new(time(9, 0))).disabled(true), "disabled"));
    let rows = [
      (
        "new / optional",
        "Signal<Time> / Signal<Option<Time>>",
        "",
        "The time the field holds, which a time typed whole sets; optional holds none while a segment is empty.",
      ),
      (
        "controlled / controlled_optional",
        "Time / Option<Time>, Fn",
        "",
        "The app's time and what it is told of a time typed.",
      ),
      (
        "step",
        "impl Into<f64>",
        "1",
        "The minutes Up and Down step the minutes by.",
      ),
      (
        "seconds",
        "bool",
        "false",
        "A segment for the seconds. Without it the value's seconds are kept.",
      ),
      (
        "hour_cycle",
        "HourCycle",
        "locale's",
        "From 0 to 23, or from 1 to 12 with a segment for AM and PM.",
      ),
      (
        "min / max",
        "Time",
        "None",
        "The least and the greatest time that is the field's.",
      ),
      (
        "memory",
        "&DateMemory",
        "",
        "What is half typed, kept by the app; mount(ctx) keeps its own.",
      ),
      (
        "locale, icon, size, judgments, read_only, disabled, full_width, label, name",
        "",
        "",
        "As a date field's.",
      ),
      (
        "on_change",
        "Fn(Time) / Fn(Option<Time>)",
        "",
        "Called when a person changes the time.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Time Field"],
        title: "Time Field",
        description: "A time typed in by its segments, the hours and the minutes, each stepped by the arrows: the \
                      time of a meeting, an alarm, the opening hours of a shop.",
        import: Some("use lurq::ui::TimeField;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of TimeField is a control on the right. Press a segment and type, or step it with the arrows.",
          playground,
        ),
        Section::new(
          "Time field",
          "The minutes stepped a quarter at a time, and a time with seconds that may be none.",
          self.frame.demo(
            0,
            bound,
            "TimeField::new(starts.clone()).step(15)",
            &[
              "TimeField::new(starts.clone()).step(15).label(\"Starts at\").mount(ctx)",
              "TimeField::optional(lap.clone()).seconds(true).label(\"Lap\").mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "A clock of twelve hours",
          "The locale's clock, or the field's own: the last segment is then AM or PM, which A, P and the arrows set.",
          self.frame.demo(
            1,
            clocks,
            ".hour_cycle(HourCycle::H12)",
            &[
              "TimeField::new(value.clone()).locale(DateLocale::for_tag(\"en-US\"))",
              "TimeField::new(value.clone()).hour_cycle(HourCycle::H12)",
            ],
          ),
        ),
        Section::new(
          "In a field",
          "A time out of the range is no value: what is typed stays, and the time held does not change.",
          self.frame.demo(
            2,
            fields,
            ".min(..).max(..)",
            &[
              "Field::new(",
              "    \"Opens at\",",
              "    TimeField::new(opens.clone())",
              "        .min(Time::new(6, 0, 0).unwrap())",
              "        .max(Time::new(12, 0, 0).unwrap())",
              "        .memory(&memory),",
              ")",
            ],
          ),
        ),
        Section::new(
          "Sizes and states",
          "An input's sizes, and disabled.",
          self.frame.demo(
            3,
            states,
            ".size(DateFieldSize::Lg)",
            &[
              "TimeField::new(value.clone()).size(DateFieldSize::Lg)",
              "TimeField::new(value.clone()).disabled(true)",
            ],
          ),
        ),
        Section::new("Keyboard", "Each segment is a spin button.", keys_list(&SEGMENT_KEYS)),
        Section::new(
          "Props",
          "What a time field is given.",
          props_table(&rows, &["new / optional"], self.frame.room()),
        ),
      ],
    )
  }
}
