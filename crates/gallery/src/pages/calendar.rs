//! The page of the calendar: its playground, a date and the month shown in two signals, a range over two months, the
//! days that cannot be chosen, week numbers and locales, and what the keys do.

use std::sync::Arc;

use lurq::{
  app::{component::Component, ctx::Ctx},
  components::{Column, Row},
  core::{Date, DateLocale, DateRange, DateText, Signal, Weekday, YearMonth},
  layout::Alignment,
  node::Element,
  ui::{Calendar, DateMemory, Kbd, Picked},
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

/// The months in Roman numerals.
const NUMERALS: [&str; 12] = ["I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X", "XI", "XII"];

/// A date text of the page's own, a form no locale of lurq's has: the months in Roman numerals, `12.XI.2026`, the
/// weeks from Monday, the days' names in English. What an app plugs in with `.locale(..)` or `app.set_date_locale`.
pub(crate) struct RomanDates;

impl DateText for RomanDates {
  fn first_day(&self) -> Weekday {
    Weekday::Monday
  }

  fn month_name(&self, month: u8) -> String {
    NUMERALS[(month.clamp(1, 12) - 1) as usize].to_owned()
  }

  fn weekday_name(&self, day: Weekday) -> String {
    DateLocale::english().weekday_name(day)
  }

  fn weekday_short(&self, day: Weekday) -> String {
    DateLocale::english().weekday_short(day)
  }

  fn format(&self, date: Date) -> String {
    format!("{}.{}.{}", date.day(), self.month_name(date.month()), date.year())
  }

  fn parse(&self, text: &str) -> Option<Date> {
    let mut parts = text.trim().split('.');
    let day = parts.next()?.trim().parse().ok()?;
    let numeral = parts.next()?.trim().to_ascii_uppercase();
    let month = NUMERALS.iter().position(|held| *held == numeral)? as u8 + 1;
    let year = parts.next()?.trim();
    // Four digits, as a year is read anywhere: what is still being typed is no date yet.
    if year.len() != 4 {
      return None;
    }
    Date::new(year.parse().ok()?, month, day)
  }
}

/// Saturday and Sunday.
fn weekend(date: Date) -> bool {
  date.weekday().days_from_monday() >= 5
}

/// What the playground's panel sets on a calendar of either kind.
#[derive(Clone, Copy)]
struct Settings {
  months: usize,
  first_day: Option<Weekday>,
  locale: &'static str,
  min: bool,
  blocked: Blocked,
  week_numbers: bool,
  outside_days: bool,
  disabled: bool,
  today: Date,
}

/// A calendar of a date or of a range, as the playground's panel makes it.
fn dressed<V: Picked>(
  calendar: Calendar<V>,
  settings: Settings,
  month: &Signal<YearMonth>,
  memory: &DateMemory,
) -> Calendar<V> {
  let mut calendar = calendar
    .month(month.clone())
    .months(settings.months)
    .locale(DateLocale::for_tag(settings.locale))
    .week_numbers(settings.week_numbers)
    .outside_days(settings.outside_days)
    .disabled(settings.disabled)
    .memory(memory);
  if let Some(first) = settings.first_day {
    calendar = calendar.first_day(first);
  }
  if settings.min {
    calendar = calendar.min(settings.today);
  }
  if settings.blocked == Blocked::Weekends {
    calendar = calendar.disabled_days(weekend);
  }
  calendar
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
  memory: DateMemory,
  due: Signal<Option<Date>>,
  shown: Signal<YearMonth>,
  stay: Signal<Option<DateRange>>,
  kept: [DateMemory; 7],
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
    let settings = Settings {
      months,
      first_day,
      locale,
      min,
      blocked,
      week_numbers,
      outside_days,
      disabled,
      today: Date::today(),
    };
    let forced = self.playground.forced();
    let record = self.playground.recorder();
    let shown: Element = match mode {
      Mode::Date => dressed(Calendar::new(self.date.clone()), settings, &self.month, &self.memory)
        .force(forced)
        .on_change(move |date: Date| record(format!("on_change({date})")))
        .into(),
      Mode::Range => dressed(Calendar::new(self.range.clone()), settings, &self.month, &self.memory)
        .force(forced)
        .on_change(move |range: DateRange| record(format!("on_change({range})")))
        .into(),
    };
    let detached = DateMemory::new();
    let make: Arc<dyn Fn() -> Element + Send + Sync> = {
      let (date, range, month) = (self.date.clone(), self.range.clone(), self.month.clone());
      Arc::new(move || match mode {
        Mode::Date => dressed(Calendar::new(date.clone()), settings, &month, &detached).into(),
        Mode::Range => dressed(Calendar::new(range.clone()), settings, &month, &detached).into(),
      })
    };

    let mut code = vec![
      "// value: Signal<Option<Date>>, month: Signal<YearMonth>".to_owned(),
      match mode {
        Mode::Date => "Calendar::new(value.clone())".to_owned(),
        Mode::Range => "Calendar::new(stay.clone())".to_owned(),
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
        forceable: true,
        facts,
        event: "on_change",
        code,
        tree,
        controls: vec![
          prop(
            "new",
            "Signal<Option<V>>",
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
            "impl DateText",
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
      memory: DateMemory::new(),
      due: ctx.signal(None),
      shown: ctx.signal(YearMonth::current()),
      stay: ctx.signal(None),
      kept: std::array::from_fn(|_| DateMemory::new()),
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
      Calendar::new(self.stay.clone()).months(2).memory(&self.kept[1]),
      &says_range(self.stay.get()),
    ));
    let blocked = items()
      .child(item(
        Calendar::new(Signal::new(None::<Date>))
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
    let own = items().child(item(
      Calendar::new(Signal::new(Some(today)))
        .locale(RomanDates)
        .memory(&self.kept[6]),
      "months in Roman numerals, a DateText of the page's own",
    ));
    let keyboard = KEYS
      .into_iter()
      .fold(Column::new().width(FILL).spacing(10.0), |list, (key, does)| {
        list.child(
          Row::new()
            .width(FILL)
            .align_items(Alignment::Center)
            .child(Row::new().width(200.0).child(Kbd::new(key)))
            // What the key does wraps in the room beside the key.
            .child(Column::new().flex(1.0).child(quiet(does, 13.0))),
        )
      });
    let rows = [
      (
        "value",
        "Signal<Option<Date>> / Signal<Option<DateRange>>",
        "",
        "What is chosen, which a choice sets: a date, or a range chosen with two presses.",
      ),
      (
        "controlled",
        "Option<Date> / Option<DateRange>, Fn",
        "",
        "The app's value and what it is told of a choice, which it answers with the value it renders next.",
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
        "&DateMemory",
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
        "impl DateText",
        "app's",
        "Months, days and the first day: the one given, else the app's (set_date_locale), else English.",
      ),
      ("first_day", "Weekday", "locale's", "The day the weeks start on."),
      ("week_numbers", "bool", "false", "The ISO week number before each week."),
      (
        "outside_days",
        "bool",
        "true",
        "The days of the months beside in the first and the last week. None with several months.",
      ),
      (
        "today",
        "Date",
        "system's",
        "The day marked as today, in the state Current.",
      ),
      ("disabled", "bool", "false", "Half the opacity; it takes no input."),
      (
        "label",
        "&str",
        "",
        "What the calendar is called to tooling: its panel is then a group.",
      ),
      (
        "previous_label / next_label",
        "&str",
        "English",
        "What the buttons of the heading are called: Previous month, Next month.",
      ),
      (
        "on_change",
        "Fn(Date) / Fn(DateRange)",
        "",
        "Called when a person chooses another date, or ends another range: not for what is chosen already.",
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
              "// due: Signal<Option<Date>>, shown: Signal<YearMonth>, memory: DateMemory",
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
            "Calendar::new(stay.clone()).months(2)",
            &[
              "// stay: Signal<Option<DateRange>>",
              "Calendar::new(stay.clone())",
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
          "The names of the months and the days and the first day of the week are the locale's: the one the \
           calendar is given, else the app's (app.set_date_locale), else English.",
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
          "A date text of the app's own",
          "What no locale of lurq's has is the app's own DateText: here the months are Roman numerals. It is given \
           to one calendar with .locale(..), or to the whole app with app.set_date_locale(..).",
          self.frame.demo(
            4,
            own,
            ".locale(RomanDates)",
            &[
              "struct RomanDates;",
              "",
              "impl DateText for RomanDates {",
              "  fn first_day(&self) -> Weekday { Weekday::Monday }",
              "  fn month_name(&self, month: u8) -> String { NUMERALS[month as usize - 1].to_owned() }",
              "  fn weekday_name(&self, day: Weekday) -> String { DateLocale::english().weekday_name(day) }",
              "  fn weekday_short(&self, day: Weekday) -> String { DateLocale::english().weekday_short(day) }",
              "  fn format(&self, date: Date) -> String {",
              "    format!(\"{}.{}.{}\", date.day(), self.month_name(date.month()), date.year())",
              "  }",
              "  fn parse(&self, text: &str) -> Option<Date> { /* 12.XI.2026 */ }",
              "}",
              "",
              "Calendar::new(value.clone()).locale(RomanDates)",
              "app.set_date_locale(RomanDates)",
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
