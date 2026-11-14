//! The page of the calendar: its playground, a date and the month shown in two signals, a range over two months, the
//! days that cannot be chosen, week numbers and locales, and what the keys do.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::{Date, DateLocale, DateRange, Signal, Weekday, YearMonth},
  layout::Alignment,
  node::Element,
  ui::{Calendar, CalendarMemory, Kbd},
};

use crate::{
  doc::{Frame, Header, Section},
  kit::{FILL, props_table, quiet},
  playground::{Playground, Shown, Spec, divider, flag, prop},
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
  Row::new()
    .width(FILL)
    .spacing(28.0)
    .wrap()
    .align_items(Alignment::Start)
}

/// What the calendar chooses: a date, or a range.
#[derive(Clone, Copy, PartialEq, Eq, lurq::DevtoolsInspectable)]
enum Mode {
  Date,
  Range,
}

const MODES: [(&str, Mode); 2] = [("Date", Mode::Date), ("Range", Mode::Range)];
const MONTHS: [(&str, usize); 2] = [("1", 1), ("2", 2)];
const FIRST_DAYS: [(&str, Option<Weekday>); 4] = [
  ("Locale's", None),
  ("Monday", Some(Weekday::Monday)),
  ("Sunday", Some(Weekday::Sunday)),
  ("Saturday", Some(Weekday::Saturday)),
];
const LOCALES: [(&str, &str); 5] = [
  ("en", "en"),
  ("en-US", "en-US"),
  ("en-GB", "en-GB"),
  ("uk", "uk"),
  ("de", "de"),
];

/// What cannot be chosen besides the range: nothing, or weekends.
#[derive(Clone, Copy, PartialEq, Eq, lurq::DevtoolsInspectable)]
enum Blocked {
  None,
  Weekends,
}

const BLOCKED: [(&str, Blocked); 2] = [("None", Blocked::None), ("Weekends", Blocked::Weekends)];

/// The least date the playground offers: none, or today.
const MINS: [(&str, bool); 2] = [("None", false), ("Today", true)];

/// What each key does on a day of a calendar.
const KEYS: [(&str, &str); 9] = [
  (
    "Tab",
    "Into the days at the chosen one, today or the first of the month; Tab again leaves them.",
  ),
  (
    "Left / Right",
    "The day before or after, into the month beside, which is then shown.",
  ),
  ("Up / Down", "The same day a week before or after."),
  ("Home / End", "The first or the last day of the week."),
  (
    "Page Up / Down",
    "The same day a month before or after: the last of a shorter month.",
  ),
  ("Shift + Page Up / Down", "The same day a year before or after."),
  ("Enter / Space", "Chooses the day, or starts or ends a range."),
  ("Escape", "In a date picker, closes the calendar."),
  (
    "Disabled day",
    "Takes the focus and says it cannot be chosen: the keys pass no day by, and Enter does nothing on it.",
  ),
];

/// Saturday and Sunday.
fn weekend(date: Date) -> bool {
  date.weekday().days_from_monday() >= 5
}

fn says(date: Option<Date>) -> String {
  match date {
    Some(date) => format!("the signal has: {date}"),
    None => "the signal has: None".to_owned(),
  }
}

fn says_range(range: Option<DateRange>) -> String {
  match range {
    Some(range) => format!("the signal has: {range}"),
    None => "the signal has: None".to_owned(),
  }
}

pub struct CalendarPage {
  frame: Frame,
  playground: Playground,
  mode: Signal<Mode>,
  date: Signal<Option<Date>>,
  range: Signal<Option<DateRange>>,
  month: Signal<YearMonth>,
  months: Signal<usize>,
  first_day: Signal<Option<Weekday>>,
  locale: Signal<&'static str>,
  min: Signal<bool>,
  blocked: Signal<Blocked>,
  week_numbers: Signal<bool>,
  outside_days: Signal<bool>,
  disabled: Signal<bool>,
  memory: CalendarMemory,
  due: Signal<Option<Date>>,
  shown: Signal<YearMonth>,
  stay: Signal<Option<DateRange>>,
  kept: [CalendarMemory; 6],
}

impl CalendarPage {
  /// The playground: the calendar as the panel's props make it.
  fn play(&self, ctx: &mut Ctx) -> Element {
    let (mode, months, first_day, locale) = (
      self.mode.get(),
      self.months.get(),
      self.first_day.get(),
      self.locale.get(),
    );
    let (min, blocked, week_numbers, outside_days, disabled) = (
      self.min.get(),
      self.blocked.get(),
      self.week_numbers.get(),
      self.outside_days.get(),
      self.disabled.get(),
    );
    let (date, range, month) = (self.date.get(), self.range.get(), self.month.get());
    let today = Date::today();
    let plain = {
      let (date, range, month) = (self.date.clone(), self.range.clone(), self.month.clone());
      move |memory: &CalendarMemory| {
        let mut calendar = match mode {
          Mode::Date => Calendar::new(date.clone()),
          Mode::Range => Calendar::range(range.clone()),
        }
        .month(month.clone())
        .months(months)
        .locale(DateLocale::for_tag(locale))
        .week_numbers(week_numbers)
        .outside_days(outside_days)
        .disabled(disabled)
        .memory(memory);
        if let Some(first) = first_day {
          calendar = calendar.first_day(first);
        }
        if min {
          calendar = calendar.min(today);
        }
        if blocked == Blocked::Weekends {
          calendar = calendar.disabled_days(weekend);
        }
        calendar
      }
    };
    let (record, record_range) = (self.playground.recorder(), self.playground.recorder());
    let shown = plain(&self.memory)
      .on_change(move |date| record(format!("on_change({date})")))
      .on_range_change(move |range| record_range(format!("on_range_change({range})")));
    let detached = CalendarMemory::new();
    let make: Arc<dyn Fn() -> Element + Send + Sync> = Arc::new(move || plain(&detached).into());

    let mut code = vec![
      "// value: Signal<Option<Date>>, month: Signal<YearMonth>".to_owned(),
      match mode {
        Mode::Date => "Calendar::new(value.clone())".to_owned(),
        Mode::Range => "Calendar::range(stay.clone())".to_owned(),
      },
      "    .month(month.clone())".to_owned(),
    ];
    if months != 1 {
      code.push(format!("    .months({months})"));
    }
    if locale != "en" {
      code.push(format!("    .locale(DateLocale::for_tag({locale:?}))"));
    }
    if let Some(first) = first_day {
      code.push(format!("    .first_day(Weekday::{first:?})"));
    }
    if min {
      code.push("    .min(Date::today())".to_owned());
    }
    if blocked == Blocked::Weekends {
      code.push("    .disabled_days(|date| date.weekday().days_from_monday() >= 5)".to_owned());
    }
    if week_numbers {
      code.push("    .week_numbers(true)".to_owned());
    }
    if !outside_days {
      code.push("    .outside_days(false)".to_owned());
    }
    if disabled {
      code.push("    .disabled(true)".to_owned());
    }
    code.push("    .mount(ctx)".to_owned());

    let tree = vec![
      format!("root           {}   padding 12", self.playground.measured()),
      format!(
        "├─ months       {} heading{}, 28 buttons at the ends",
        months,
        if months == 1 { "" } else { "s" }
      ),
      "└─ months       the days, one stop of Tab".to_owned(),
      format!(
        "   └─ grid      {}weekdays over 6 weeks, rows 8 apart",
        if week_numbers { "week numbers, " } else { "" }
      ),
      "      └─ day    32 × 32 px, radius 6".to_owned(),
    ];
    let facts = match mode {
      Mode::Date => format!(
        "value {} · month {month}",
        date.map_or("None".to_owned(), |date| date.to_string())
      ),
      Mode::Range => format!(
        "value {} · month {month}",
        range.map_or("None".to_owned(), |range| range.to_string())
      ),
    };

    let to = (
      self.mode.clone(),
      self.months.clone(),
      self.first_day.clone(),
      self.locale.clone(),
      self.min.clone(),
      self.blocked.clone(),
      [self.week_numbers.clone(), self.disabled.clone()],
      self.outside_days.clone(),
    );
    let (set_mode, set_months, set_first, set_locale, set_min, set_blocked) = (
      self.mode.clone(),
      self.months.clone(),
      self.first_day.clone(),
      self.locale.clone(),
      self.min.clone(),
      self.blocked.clone(),
    );
    self.playground.card(
      ctx,
      Spec {
        title: "Calendar",
        make,
        shown: Shown::of(shown),
        fill: false,
        forceable: false,
        facts,
        event: match mode {
          Mode::Date => "on_change",
          Mode::Range => "on_range_change",
        },
        code,
        tree,
        controls: vec![
          prop(
            "new / range",
            "Signal<Option<Date>> / Signal<Option<DateRange>>",
            self.playground.select(0, &MODES, &mode, move |mode| set_mode.set(mode)),
          ),
          prop(
            "months",
            "usize",
            self
              .playground
              .select(1, &MONTHS, &months, move |months| set_months.set(months)),
          ),
          prop(
            "locale",
            "DateLocale",
            self
              .playground
              .select(2, &LOCALES, &locale, move |locale| set_locale.set(locale)),
          ),
          prop(
            "first_day",
            "Weekday",
            self
              .playground
              .select(3, &FIRST_DAYS, &first_day, move |first| set_first.set(first)),
          ),
          prop(
            "min",
            "Date",
            self.playground.select(4, &MINS, &min, move |min| set_min.set(min)),
          ),
          prop(
            "disabled_days",
            "Fn(Date) -> bool",
            self
              .playground
              .select(5, &BLOCKED, &blocked, move |blocked| set_blocked.set(blocked)),
          ),
          divider(),
          flag("week_numbers", &self.week_numbers),
          flag("outside_days", &self.outside_days),
          flag("disabled", &self.disabled),
        ],
        changed: [
          mode != Mode::Date,
          months != 1,
          locale != "en",
          first_day.is_some(),
          min,
          blocked != Blocked::None,
          week_numbers,
          !outside_days,
          disabled,
        ]
        .into_iter()
        .filter(|changed| *changed)
        .count(),
        reset: Arc::new(move || {
          to.0.set(Mode::Date);
          to.1.set(1);
          to.2.set(None);
          to.3.set("en");
          to.4.set(false);
          to.5.set(Blocked::None);
          for flag in &to.6 {
            flag.set(false);
          }
          to.7.set(true);
        }),
      },
    )
  }
}

impl Component for CalendarPage {
  type Props = ();

  fn create(ctx: &mut Ctx) -> Self {
    let playground = Playground::new(ctx, 6);
    let month = ctx.signal(YearMonth::current());
    // The month shown is told apart from the value: each has its line in the Events tab.
    let record = playground.recorder();
    ctx.watch(&month, move |month| record(format!("month = {month}")));
    Self {
      frame: Frame::new(ctx),
      playground,
      mode: ctx.signal(Mode::Date),
      date: ctx.signal(Some(Date::today())),
      range: ctx.signal(None),
      month,
      months: ctx.signal(1),
      first_day: ctx.signal(None),
      locale: ctx.signal("en"),
      min: ctx.signal(false),
      blocked: ctx.signal(Blocked::None),
      week_numbers: ctx.signal(false),
      outside_days: ctx.signal(true),
      disabled: ctx.signal(false),
      memory: CalendarMemory::new(),
      due: ctx.signal(None),
      shown: ctx.signal(YearMonth::current()),
      stay: ctx.signal(None),
      kept: std::array::from_fn(|_| CalendarMemory::new()),
    }
  }

  fn render(&self, ctx: &mut Ctx) -> impl Into<Element> {
    let today = Date::today();
    let bound = items().child(item(
      Calendar::new(self.due.clone())
        .month(self.shown.clone())
        .memory(&self.kept[0]),
      &format!("{} · the month shown: {}", says(self.due.get()), self.shown.get()),
    ));
    let range = items().child(item(
      Calendar::range(self.stay.clone()).months(2).memory(&self.kept[1]),
      &says_range(self.stay.get()),
    ));
    let blocked = items()
      .child(item(
        Calendar::new(Signal::new(None))
          .min(today)
          .max(today.add_months(2))
          .disabled_days(weekend)
          .memory(&self.kept[2]),
        "from today, for two months, no weekends",
      ))
      .child(item(Calendar::new(Signal::new(Some(today))).disabled(true), "disabled"));
    let locales = items()
      .child(item(
        Calendar::new(Signal::new(Some(today)))
          .locale(DateLocale::for_tag("en-US"))
          .week_numbers(true)
          .memory(&self.kept[3]),
        "en-US: from Sunday, with week numbers",
      ))
      .child(item(
        Calendar::new(Signal::new(Some(today)))
          .locale(DateLocale::for_tag("uk"))
          .memory(&self.kept[4]),
        "uk",
      ))
      .child(item(
        Calendar::new(Signal::new(Some(today)))
          .locale(DateLocale::for_tag("de"))
          .outside_days(false)
          .memory(&self.kept[5]),
        "de, without the days of the months beside",
      ));
    let keyboard = KEYS
      .into_iter()
      .fold(Column::new().width(FILL).spacing(10.0), |list, (key, does)| {
        list.child(
          Row::new()
            .width(FILL)
            .align_items(Alignment::Center)
            .child(Row::new().width(200.0).child(Kbd::new(key)))
            .child(quiet(does, 13.0)),
        )
      });
    let rows = [
      (
        "value",
        "Signal<Option<Date>> / Signal<Option<DateRange>>",
        "",
        "What is chosen: Calendar::new for a date, Calendar::range for a range chosen with two presses.",
      ),
      (
        "month",
        "Signal<YearMonth>",
        "kept",
        "The month shown, the first where there are several: not the value. The calendar keeps its own where none \
         is given.",
      ),
      (
        "memory",
        "&CalendarMemory",
        "",
        "What the calendar keeps between renders, kept by the app: the month, a range's first press. mount(ctx) \
         keeps one of its own.",
      ),
      (
        "months",
        "usize",
        "1",
        "How many months side by side, each with its heading.",
      ),
      (
        "min / max",
        "Date",
        "None",
        "The least and the greatest date that can be chosen.",
      ),
      (
        "disabled_days",
        "Fn(Date) -> bool",
        "",
        "Which days cannot be chosen besides those out of the range.",
      ),
      (
        "locale",
        "DateLocale",
        "app's",
        "The names of months and days and the first day of the week: the app's i18n locale, else English.",
      ),
      ("first_day", "Weekday", "locale's", "The day the weeks start on."),
      ("week_numbers", "bool", "false", "The ISO week number before each week."),
      (
        "outside_days",
        "bool",
        "true",
        "The days of the months beside in the first and the last week. None with several months.",
      ),
      ("today", "Date", "system's", "The day marked as today."),
      ("disabled", "bool", "false", "Half the opacity; it takes no input."),
      ("on_change", "Fn(Date)", "", "Called when a press chooses a date."),
      (
        "on_range_change",
        "Fn(DateRange)",
        "",
        "Called when a second press ends a range.",
      ),
    ];
    let playground = self.play(ctx);
    self.frame.page(
      ctx,
      Header {
        breadcrumb: &["Components", "Inputs", "Calendar"],
        title: "Calendar",
        description: "A month of days a person chooses a date from, or a range of dates: a due date, a day to book, \
                      the week of a report.",
        import: Some("use lurq::ui::Calendar;"),
        status: Some("Stable"),
        since: Some("0.62"),
        source: file!(),
      },
      vec![
        Section::new(
          "Playground",
          "Every prop of Calendar is a control on the right. The calendar on the stage is live: press a day, the \
           buttons of its heading, or Tab into its days and use the arrow keys, Home, End and Page Up and Down. The \
           Events tab tells the value and the month shown apart.",
          playground,
        ),
        Section::new(
          "Calendar",
          "The page holds the date and the month shown in two signals: the buttons and the keys change the month, a \
           press changes the date.",
          self.frame.demo(
            0,
            bound,
            "Calendar::new(due.clone()).month(shown.clone())",
            &[
              "// due: Signal<Option<Date>>, shown: Signal<YearMonth>, memory: CalendarMemory",
              "Calendar::new(due.clone())",
              "    .month(shown.clone())",
              "    .memory(&memory)",
            ],
          ),
        ),
        Section::new(
          "Range",
          "Two presses: the first starts the range, the pointer and the keys show where it would end, the second \
           ends it, before the first or after it. Its ends are chosen days and the days between are one band.",
          self.frame.demo(
            1,
            range,
            "Calendar::range(stay.clone()).months(2)",
            &[
              "// stay: Signal<Option<DateRange>>",
              "Calendar::range(stay.clone())",
              "    .months(2)",
              "    .mount(ctx)",
            ],
          ),
        ),
        Section::new(
          "Days that cannot be chosen",
          "Out of the range or turned down by the app, a day is muted and takes no press. The keys still reach it, \
           and it says it cannot be chosen.",
          self.frame.demo(
            2,
            blocked,
            ".disabled_days(..)",
            &[
              "let today = Date::today();",
              "Calendar::new(value.clone())",
              "    .min(today)",
              "    .max(today.add_months(2))",
              "    .disabled_days(|date| date.weekday().days_from_monday() >= 5)",
              "    .mount(ctx)",
              "",
              "Calendar::new(value.clone()).disabled(true)",
            ],
          ),
        ),
        Section::new(
          "Locales",
          "The names of the months and the days and the first day of the week are the locale's: the app's i18n \
           locale for a mounted calendar, or the one it is given.",
          self.frame.demo(
            3,
            locales,
            ".locale(DateLocale::for_tag(\"uk\"))",
            &[
              "Calendar::new(value.clone())",
              "    .locale(DateLocale::for_tag(\"en-US\"))",
              "    .week_numbers(true)",
              "",
              "Calendar::new(value.clone()).locale(DateLocale::for_tag(\"uk\"))",
              "",
              "Calendar::new(value.clone())",
              "    .locale(DateLocale::for_tag(\"de\"))",
              "    .outside_days(false)",
            ],
          ),
        ),
        Section::new(
          "Keyboard",
          "The days are a grid: the keys move by days, weeks and months.",
          keyboard,
        ),
        Section::new(
          "Props",
          "What a calendar is given.",
          props_table(&rows, &["value"], self.frame.room()),
        ),
      ],
    )
  }
}
