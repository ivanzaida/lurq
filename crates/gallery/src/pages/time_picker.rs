//! The page of the time picker: its playground, a time bound to a signal, the step of the minutes, a clock of twelve
//! hours, the panel drawn in place, and one in a field.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::{DateLocale, Signal, Time},
  layout::Alignment,
  node::Element,
  ui::{DateMemory, Field, Kbd, TimePicker},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .spacing(10.0)
    .child(Row::new().child(shown))
    .child(quiet(caption, 12.0))
}

fn items() -> Row {
  Row::new()
    .width(FILL)
    .spacing(28.0)
    .wrap()
    .align_items(Alignment::Start)
}

const STEPS: [(&str, u32); 4] = [("5", 5), ("10", 10), ("15", 15), ("30", 30)];
const LOCALES: [(&str, &str); 4] = [("en", "en"), ("en-US", "en-US"), ("uk", "uk"), ("de", "de")];

const KEYS: [(&str, &str); 5] = [
  ("Down", "In the field, opens the panel with the focus on the hour held."),
  (
    "Enter",
    "In the field, writes what is typed as the locale writes times.",
  ),
  (
    "Up / Down",
    "In a column, the item before or after; Tab goes to the next column.",
  ),
  (
    "Enter / Space",
    "Chooses the item: an hour keeps the panel open, a minute closes it.",
  ),
  ("Escape", "Closes the panel and gives the focus back to the field."),
];

fn time(hour: u8, minute: u8) -> Time {
  Time::new(hour, minute, 0).unwrap()
}

pub struct TimePickerPage {
  frame: Frame,
  playground: Playground,
  value: Signal<Time>,
  step: Signal<u32>,
  locale: Signal<&'static str>,
  invalid: Signal<bool>,
  disabled: Signal<bool>,
  memory: DateMemory,
  meeting: Signal<Time>,
  slot: Signal<Time>,
  reminder: Signal<Option<Time>>,
  kept: [DateMemory; 4],
}

impl TimePickerPage {
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (value, step, locale) = (self.value.get(), self.step.get(), self.locale.get());
    let (invalid, disabled) = (self.invalid.get(), self.disabled.get());
    let open = self.memory.is_open();
    let plain = {
      let bound = self.value.clone();
      move |memory: &DateMemory| {
        TimePicker::new(bound.clone())
          .step(step)
          .locale(DateLocale::for_tag(locale))
          .invalid(invalid)
          .disabled(disabled)
          .label("Meeting")
          .memory(memory)
      }
    };
    let (record, record_open, record_dismiss) = (
      self.playground.recorder(),
      self.playground.recorder(),
      self.playground.recorder(),
    );
    let shown = plain(&self.memory)
      .force(self.playground.forced())
      .on_change(move |time| record(format!("on_change({time})")))
      .on_open_change(move |open| record_open(format!("on_open_change({open})")))
      .on_dismiss(move || record_dismiss("on_dismiss()".to_owned()));
    let detached = DateMemory::new();
    let make: Arc<dyn Fn() -> Element + Send + Sync> = Arc::new(move || plain(&detached).into());
    let mut code = vec![
      "// at: Signal<Time>".to_owned(),
      "TimePicker::new(at.clone())".to_owned(),
    ];
    if step != 15 {
      code.push(format!("    .step({step})"));
    }
    if locale != "en" {
      code.push(format!("    .locale(DateLocale::for_tag({locale:?}))"));
    }
    for (on, call) in [(invalid, "invalid"), (disabled, "disabled")] {
      if on {
        code.push(format!("    .{call}(true)"));
      }
    }
    code.push("    .label(\"Meeting\")".to_owned());
    code.push("    .mount(ctx)".to_owned());
    let tree = vec![
      format!("root           {}", self.playground.measured()),
      "├─ field        the input with the timer's glyph: a combobox".to_owned(),
      format!(
        "└─ panel        a popover 4 under it, {}",
        if open { "open" } else { "closed" }
      ),
      format!("   └─ columns   hours, and minutes {step} apart, 74 × 30 items"),
    ];
    let to = (
      self.value.clone(),
      self.step.clone(),
      self.locale.clone(),
      [self.invalid.clone(), self.disabled.clone()],
    );
    let (set_step, set_locale) = (self.step.clone(), self.locale.clone());
    self.playground.card(
      ctx,
      Spec {
        title: "Time Picker",
        make,
        shown: Shown::of(shown),
        fill: false,
        forceable: true,
        facts: format!("value {value} · {}", if open { "open" } else { "closed" }),
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
            "locale",
            "impl DateText",
            self
              .playground
              .select(1, &LOCALES, &locale, move |locale| set_locale.set(locale)),
          ),
          divider(),
          flag("invalid", &self.invalid),
          flag("disabled", &self.disabled),
        ],
        changed: [step != 15, locale != "en", invalid, disabled]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to.0.set(time(9, 30));
          to.1.set(15);
          to.2.set("en");
          for flag in &to.3 {
            flag.set(false);
          }
        }),
      },
    )
  }
}

impl Component for TimePickerPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      value: ctx.signal(time(9, 30)),
      step: ctx.signal(15),
      locale: ctx.signal("en"),
      invalid: ctx.signal(false),
      disabled: ctx.signal(false),
      memory: DateMemory::new(),
      meeting: ctx.signal(time(9, 30)),
      slot: ctx.signal(time(14, 0)),
      reminder: ctx.signal(None),
      kept: std::array::from_fn(|_| DateMemory::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let bound = items()
      .child(item(
        TimePicker::new(self.meeting.clone())
          .label("Meeting")
          .memory(&self.kept[0]),
        &format!("the signal has: {}", self.meeting.get()),
      ))
      .child(item(
        TimePicker::new(Signal::new(time(14, 30)))
          .locale(DateLocale::for_tag("en-US"))
          .memory(&self.kept[1]),
        "en-US: a clock of twelve hours",
      ));
    let inline = items().child(item(
      TimePicker::new(self.slot.clone())
        .step(30)
        .min(time(13, 0))
        .max(time(16, 0))
        .inline(true)
        .memory(&self.kept[2]),
      &format!("the panel in place: {}, from 13:00 to 16:00", self.slot.get()),
    ));
    let fields = items().child(
      Field::new(
        "Remind me at",
        TimePicker::optional(self.reminder.clone())
          .placeholder("hh:mm")
          .memory(&self.kept[3]),
      )
      .description("Empty for no reminder."),
    );
    let keyboard = KEYS
      .into_iter()
      .fold(Column::new().width(FILL).spacing(10.0), |list, (key, does)| {
        list.child(
          Row::new()
            .width(FILL)
            .align_items(Alignment::Center)
            .child(Row::new().width(200.0).child(Kbd::new(key)))
            .child(Column::new().flex(1.0).child(quiet(does, 13.0))),
        )
      });
    let rows = [
      (
        "new / optional",
        "Signal<Time> / Signal<Option<Time>>",
        "",
        "The time the field holds: typed, or chosen from the panel; optional holds none while the field is empty.",
      ),
      (
        "controlled / controlled_optional",
        "Time / Option<Time>, Fn",
        "",
        "The app's time and what it is told of an edit or a choice.",
      ),
      (
        "step",
        "impl Into<f64>",
        "15",
        "The minutes the panel offers apart, as the board draws them; any minute may be typed.",
      ),
      (
        "hour_cycle",
        "HourCycle",
        "locale's",
        "From 0 to 23, or from 1 to 12 with a column of AM and PM.",
      ),
      (
        "min / max",
        "Time",
        "None",
        "The least and the greatest time that can be typed or chosen.",
      ),
      (
        "memory",
        "&DateMemory",
        "",
        "Whether the panel is open, kept by the app; mount(ctx) keeps its own.",
      ),
      (
        "open, placement, offset, collision, inline",
        "",
        "",
        "Whether the panel is open, and where it opens, as a date picker's.",
      ),
      (
        "trigger_label / panel_label",
        "&str",
        "locale's",
        "What the button and the panel are called to tooling: Choose time.",
      ),
      (
        "placeholder, locale, size, judgments, read_only, disabled, full_width, label, name",
        "",
        "",
        "As a date picker's.",
      ),
      (
        "on_change / on_input / on_submit",
        "Fn(V) / Fn(&TextInputEvent) / Fn(V)",
        "",
        "Told of another time, of an edit before it is made, and of Enter.",
      ),
      (
        "on_open_change / on_dismiss",
        "Fn(bool) / Fn()",
        "",
        "Told of the panel opening and closing, and of Escape or a press outside closing it.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Time Picker"],
        title: "Time Picker",
        description: "A field that holds a time, typed in it or chosen from columns of hours and minutes that open \
                      under it: the time of a meeting, a reminder, a delivery slot.",
        import: Some("use lurq::ui::TimePicker;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of TimePicker is a control on the right. Open the panel from the glyph or with Down, or type a \
           time.",
          playground,
        ),
        Section::new(
          "Time picker",
          "A time bound to a signal, and one on a clock of twelve hours, whose panel has a column of AM and PM.",
          self.frame.demo(
            0,
            bound,
            "TimePicker::new(meeting.clone())",
            &[
              "TimePicker::new(meeting.clone()).label(\"Meeting\").mount(ctx)",
              "TimePicker::new(at.clone()).locale(DateLocale::for_tag(\"en-US\")).mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "The panel in place",
          "inline(true) draws the panel under the field, open or not: here half hours from 13:00 to 16:00, as a slot \
           of a booking.",
          self.frame.demo(
            1,
            inline,
            ".inline(true)",
            &[
              "TimePicker::new(slot.clone())",
              "    .step(30)",
              "    .min(Time::new(13, 0, 0).unwrap())",
              "    .max(Time::new(16, 0, 0).unwrap())",
              "    .inline(true)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "In a field",
          "A time that may be none: an emptied field holds none.",
          self.frame.demo(
            2,
            fields,
            "Field::new(\"Remind me at\", TimePicker::optional(reminder.clone()).memory(&memory))",
            &[
              "Field::new(",
              "    \"Remind me at\",",
              "    TimePicker::optional(reminder.clone()).placeholder(\"hh:mm\").memory(&memory),",
              ")",
            ],
          ),
        ),
        Section::new(
          "Keyboard",
          "The field is a combobox; the panel's columns are listboxes.",
          keyboard,
        ),
        Section::new(
          "Props",
          "What a time picker is given.",
          props_table(&rows, &["new / optional"], self.frame.room()),
        ),
      ],
    )
  }
}
