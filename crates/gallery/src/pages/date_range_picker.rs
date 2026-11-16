//! The page of the date range picker: its playground, a range bound to a signal, presets beside the calendar, two
//! months, and one in a field.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::{Date, DateLocale, DateRange, Signal},
  layout::Alignment,
  node::Element,
  ui::{DateMemory, DateRangePicker, Field},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, flag, prop},
};

/// Something on a stage with what it is under it.
fn item(shown: impl Into<Element>, caption: &str) -> Column {
  Column::new()
    .max_width(FILL)
    .spacing(10.0)
    .child(Row::new().max_width(FILL).child(shown))
    .child(quiet(caption, 12.0))
}

/// What is on a stage, side by side; what does not fit goes on to the next line.
fn items() -> Row {
  Row::new().width(FILL).spacing(28.0).wrap().align_items(Alignment::End)
}

const MONTHS: [(&str, usize); 2] = [("1", 1), ("2", 2)];
const LOCALES: [(&str, &str); 4] = [("en", "en"), ("en-GB", "en-GB"), ("uk", "uk"), ("de", "de")];

/// The presets a report's period is offered with, ending today.
fn presets(picker: DateRangePicker) -> DateRangePicker {
  let today = Date::today();
  let month = today.year_month();
  picker
    .preset("Today", DateRange::day(today))
    .preset("Last 7 days", DateRange::new(today.add_days(-6), today))
    .preset("Last 30 days", DateRange::new(today.add_days(-29), today))
    .preset("This month", DateRange::new(month.first(), month.last()))
}

fn says(range: Option<DateRange>) -> String {
  match range {
    Some(range) => format!("the signal has: {range}"),
    None => "the signal has: None".to_owned(),
  }
}

pub struct DateRangePickerPage {
  frame: Frame,
  playground: Playground,
  value: Signal<Option<DateRange>>,
  months: Signal<usize>,
  locale: Signal<&'static str>,
  presets: Signal<bool>,
  clearable: Signal<bool>,
  invalid: Signal<bool>,
  disabled: Signal<bool>,
  memory: DateMemory,
  stay: Signal<Option<DateRange>>,
  period: Signal<Option<DateRange>>,
  leave: Signal<Option<DateRange>>,
  kept: [DateMemory; 3],
}

impl DateRangePickerPage {
  /// The playground: the picker as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (value, months, locale) = (self.value.get(), self.months.get(), self.locale.get());
    let (with_presets, clearable, invalid, disabled) = (
      self.presets.get(),
      self.clearable.get(),
      self.invalid.get(),
      self.disabled.get(),
    );
    let open = self.memory.is_open();
    let plain = {
      let bound = self.value.clone();
      move |memory: &DateMemory| {
        let picker = DateRangePicker::new(bound.clone())
          .placeholder("Pick dates")
          .months(months)
          .locale(DateLocale::for_tag(locale))
          .clearable(clearable)
          .invalid(invalid)
          .disabled(disabled)
          .label("Period")
          .memory(memory);
        if with_presets { presets(picker) } else { picker }
      }
    };
    let (record, record_open, record_dismiss) = (
      self.playground.recorder(),
      self.playground.recorder(),
      self.playground.recorder(),
    );
    let shown = plain(&self.memory)
      .force(self.playground.forced())
      .on_change(move |range| match range {
        Some(range) => record(format!("on_change(Some({range}))")),
        None => record("on_change(None)".to_owned()),
      })
      .on_open_change(move |open| record_open(format!("on_open_change({open})")))
      .on_dismiss(move || record_dismiss("on_dismiss()".to_owned()));
    let detached = DateMemory::new();
    let make: Arc<dyn Fn() -> Element + Send + Sync> = Arc::new(move || plain(&detached).into());

    let mut code = vec![
      "// period: Signal<Option<DateRange>>".to_owned(),
      "DateRangePicker::new(period.clone())".to_owned(),
      "    .placeholder(\"Pick dates\")".to_owned(),
    ];
    if months != 1 {
      code.push(format!("    .months({months})"));
    }
    if locale != "en" {
      code.push(format!("    .locale(DateLocale::for_tag({locale:?}))"));
    }
    if with_presets {
      code.push("    .preset(\"Today\", DateRange::day(today))".to_owned());
      code.push("    .preset(\"Last 7 days\", DateRange::new(today.add_days(-6), today))".to_owned());
      code.push("    .preset(\"Last 30 days\", DateRange::new(today.add_days(-29), today))".to_owned());
      code.push("    .preset(\"This month\", DateRange::new(month.first(), month.last()))".to_owned());
    }
    for (on, call) in [(clearable, "clearable"), (invalid, "invalid"), (disabled, "disabled")] {
      if on {
        code.push(format!("    .{call}(true)"));
      }
    }
    code.push("    .label(\"Period\")".to_owned());
    code.push("    .mount(ctx)".to_owned());

    let tree = vec![
      format!("root             {}", self.playground.measured()),
      "├─ field          the range in one line: a combobox".to_owned(),
      "│  └─ trigger     24 × 24 px, a calendar's range".to_owned(),
      format!(
        "└─ panel          a popover 4 under the field, {}",
        if open { "open" } else { "closed" }
      ),
      if with_presets {
        "   ├─ presets     ghost buttons, a line after them".to_owned()
      } else {
        "   ├─ (presets)   where there are any".to_owned()
      },
      format!("   └─ calendar    {months} month{}", if months == 1 { "" } else { "s" }),
    ];
    let to = (
      self.value.clone(),
      self.months.clone(),
      self.locale.clone(),
      [
        self.presets.clone(),
        self.clearable.clone(),
        self.invalid.clone(),
        self.disabled.clone(),
      ],
    );
    let (set_months, set_locale) = (self.months.clone(), self.locale.clone());
    self.playground.card(
      ctx,
      Spec {
        title: "Date Range Picker",
        make,
        shown: Shown::of(shown),
        fill: false,
        forceable: true,
        facts: format!(
          "value {} · {}",
          value.map_or("None".to_owned(), |range| range.to_string()),
          if open { "open" } else { "closed" }
        ),
        event: "on_change",
        code,
        tree,
        controls: vec![
          prop(
            "months",
            "usize",
            self
              .playground
              .select(0, &MONTHS, &months, move |months| set_months.set(months)),
          ),
          prop(
            "locale",
            "impl DateText",
            self
              .playground
              .select(1, &LOCALES, &locale, move |locale| set_locale.set(locale)),
          ),
          divider(),
          flag("presets", &self.presets),
          flag("clearable", &self.clearable),
          flag("invalid", &self.invalid),
          flag("disabled", &self.disabled),
        ],
        changed: [months != 1, locale != "en", with_presets, clearable, invalid, disabled]
          .into_iter()
          .filter(|changed| *changed)
          .count(),
        reset: Arc::new(move || {
          to.0.set(None);
          to.1.set(1);
          to.2.set("en");
          for flag in &to.3 {
            flag.set(false);
          }
        }),
      },
    )
  }
}

impl Component for DateRangePickerPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    Self {
      frame: Frame::new(ctx),
      playground: Playground::new(ctx, 2),
      value: ctx.signal(None),
      months: ctx.signal(1),
      locale: ctx.signal("en"),
      presets: ctx.signal(false),
      clearable: ctx.signal(false),
      invalid: ctx.signal(false),
      disabled: ctx.signal(false),
      memory: DateMemory::new(),
      stay: ctx.signal(None),
      period: ctx.signal(None),
      leave: ctx.signal(None),
      kept: std::array::from_fn(|_| DateMemory::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let bound = items().child(item(
      DateRangePicker::new(self.stay.clone())
        .placeholder("Check in – check out")
        .min(Date::today())
        .months(2)
        .memory(&self.kept[0]),
      &says(self.stay.get()),
    ));
    let preset = items().child(item(
      presets(DateRangePicker::new(self.period.clone()).placeholder("Pick dates")).memory(&self.kept[1]),
      &says(self.period.get()),
    ));
    let fields = items().child(
      Field::new(
        "Leave",
        DateRangePicker::new(self.leave.clone())
          .placeholder("Pick dates")
          .disabled_days(|date| date.weekday().days_from_monday() >= 5)
          .clearable(true)
          .memory(&self.kept[2]),
      )
      .description("Working days only."),
    );
    let rows = [
      (
        "value",
        "Signal<Option<DateRange>>",
        "",
        "The range the field holds: typed in it, chosen from its calendar with two presses, or a preset.",
      ),
      (
        "memory",
        "&DateMemory",
        "",
        "What the picker keeps between renders, kept by the app: whether it is open, the month, a first press. \
         mount(ctx) keeps one of its own.",
      ),
      (
        "months",
        "usize",
        "1",
        "How many months the calendar shows side by side.",
      ),
      (
        "controlled",
        "Option<DateRange>, Fn(Option<DateRange>)",
        "",
        "The app's range and what it is told of an edit or a choice, which it answers with the range it renders next.",
      ),
      (
        "preset / presets",
        "(&str, DateRange) / [(&str, DateRange)]",
        "",
        "Ranges offered beside the calendar under a label; a press chooses one and closes the calendar.",
      ),
      (
        "format / parse",
        "Fn(DateRange) -> String / Fn(&str) -> Option<DateRange>",
        "locale's",
        "How the field writes the range and reads what is typed: `Nov 9 – Nov 15, 2026`.",
      ),
      ("placeholder", "&str", "", "What the field says while it is empty."),
      (
        "min / max",
        "Date",
        "None",
        "The least and the greatest date either end may be.",
      ),
      (
        "disabled_days",
        "Fn(Date) -> bool",
        "",
        "Which days cannot be an end of the range.",
      ),
      (
        "clearable",
        "bool",
        "false",
        "A button at the end of the field that empties it.",
      ),
      (
        "size, judgments, read_only, disabled, label",
        "",
        "",
        "As a date picker's.",
      ),
      (
        "placement / offset / collision / inline",
        "Placement / SpacingValue / CollisionStrategy / bool",
        "BottomStart / 4 / FlipThenClamp / false",
        "Where the calendar opens, as a date picker's does, or in place under the field.",
      ),
      (
        "trigger_label / clear_label / panel_label",
        "&str",
        "English",
        "What the button, the cross and the panel are called to tooling: Choose dates, Clear, Choose dates.",
      ),
      (
        "on_change",
        "Fn(Option<DateRange>)",
        "",
        "Called when a person changes the range, after a choice closed the calendar: not for the range it holds.",
      ),
      (
        "on_input / on_submit",
        "Fn(&TextInputEvent) / Fn(Option<DateRange>)",
        "",
        "Called before an edit of the text, which it may refuse, and on Enter with the range it holds.",
      ),
      (
        "on_open_change / on_dismiss",
        "Fn(bool) / Fn()",
        "",
        "Called when the calendar opens or closes, and when Escape or a press outside closed it.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Date Range Picker"],
        title: "Date Range Picker",
        description: "A field that holds a range of dates, typed or chosen from a calendar under it, with ranges \
                      offered beside it: the dates of a stay, the period of a report.",
        import: Some("use lurq::ui::DateRangePicker;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of DateRangePicker is a control on the right. The picker on the stage is live: open its \
           calendar, press the first day and then the last, or type a range.",
          playground,
        ),
        Section::new(
          "Date range picker",
          "The page holds the range in a signal. The first press in the calendar starts it, the pointer shows where \
           it would end, and the second ends it and closes the calendar.",
          self.frame.demo(
            0,
            bound,
            "DateRangePicker::new(stay.clone()).months(2)",
            &[
              "// stay: Signal<Option<DateRange>>",
              "DateRangePicker::new(stay.clone())",
              "    .placeholder(\"Check in – check out\")",
              "    .min(Date::today())",
              "    .months(2)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Presets",
          "Ranges offered beside the calendar: a press chooses one and closes it. The one chosen is on the \
           secondary fill.",
          self.frame.demo(
            1,
            preset,
            ".preset(\"Last 7 days\", ..)",
            &[
              "let today = Date::today();",
              "let month = today.year_month();",
              "DateRangePicker::new(period.clone())",
              "    .placeholder(\"Pick dates\")",
              "    .preset(\"Today\", DateRange::day(today))",
              "    .preset(\"Last 7 days\", DateRange::new(today.add_days(-6), today))",
              "    .preset(\"Last 30 days\", DateRange::new(today.add_days(-29), today))",
              "    .preset(\"This month\", DateRange::new(month.first(), month.last()))",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "In a field",
          "A field names it and judges it; what it keeps is in a DateMemory the page made once.",
          self.frame.demo(
            2,
            fields,
            "Field::new(\"Leave\", DateRangePicker::new(leave.clone()).memory(&memory))",
            &[
              "Field::new(",
              "    \"Leave\",",
              "    DateRangePicker::new(leave.clone())",
              "        .disabled_days(|date| date.weekday().days_from_monday() >= 5)",
              "        .clearable(true)",
              "        .memory(&memory),",
              ")",
              ".description(\"Working days only.\")",
            ],
          ),
        ),
        Section::new(
          "Props",
          "What a date range picker is given.",
          props_table(&rows, &["value"], self.frame.room()),
        ),
      ],
    )
  }
}
