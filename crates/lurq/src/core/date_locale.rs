//! Everything of a date's text: the day weeks start on, the names of months and days, how a date is written and read,
//! and the names of a calendar's controls. [`DateText`] is what a calendar and a date picker ask, [`DateLocale`] the
//! ready-made answer in seven languages, and an app's own type, backed by anything, another.

use std::{
  fmt,
  sync::{Arc, OnceLock},
};

use parking_lot::RwLock;

use super::{
  Signal,
  date::{Date, DateRange, Weekday, YearMonth},
  time::{DayPeriod, HourCycle, Time},
};

/// What a calendar, a date picker and a date range picker ask of a date's text: the first day of the week, the title
/// of a month, the head of a weekday's column, what a day is called to what reads the tree, how a date and a range
/// are written in a field and how what is typed is read back, and what the controls are called.
///
/// [`DateLocale`] is the one lurq has, in seven languages, and covers most wishes with its styles and closures
/// ([`DateLocale::with_style`], [`DateLocale::with_format`]). An app implements this for what lurq does not have:
/// its own tables, `chrono`, ICU, a catalog of translations. What has a default follows from the methods without
/// one; the names of the controls are English unless it says otherwise.
///
/// ```
/// use lurq::core::{Date, DateText, Weekday};
///
/// // Months in Roman numerals: 12.XI.2026.
/// struct Roman;
///
/// const NUMERALS: [&str; 12] = ["I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X", "XI", "XII"];
///
/// impl DateText for Roman {
///   fn first_day(&self) -> Weekday {
///     Weekday::Monday
///   }
///   fn month_name(&self, month: u8) -> String {
///     NUMERALS[month as usize - 1].to_owned()
///   }
///   fn weekday_name(&self, day: Weekday) -> String {
///     format!("{day:?}")
///   }
///   fn weekday_short(&self, day: Weekday) -> String {
///     format!("{day:?}")[..2].to_owned()
///   }
///   fn format(&self, date: Date) -> String {
///     format!("{}.{}.{}", date.day(), self.month_name(date.month()), date.year())
///   }
///   fn parse(&self, text: &str) -> Option<Date> {
///     let mut parts = text.trim().split('.');
///     let day = parts.next()?.parse().ok()?;
///     let numeral = parts.next()?;
///     let month = NUMERALS.iter().position(|held| *held == numeral)? as u8 + 1;
///     Date::new(parts.next()?.parse().ok()?, month, day)
///   }
/// }
///
/// let day = Date::new(2026, 11, 12).unwrap();
/// assert_eq!(Roman.format(day), "12.XI.2026");
/// assert_eq!(Roman.parse("12.XI.2026"), Some(day));
/// assert_eq!(Roman.day_label(day), "Thursday, 12.XI.2026");
/// ```
pub trait DateText: Send + Sync {
  /// The day a week starts on: the first column of a calendar.
  fn first_day(&self) -> Weekday;

  /// The days of a week in the order a calendar's columns have them, from the first day.
  fn week(&self) -> [Weekday; 7] {
    let first = self.first_day();
    std::array::from_fn(|index| Weekday::from_monday(first as i64 + index as i64))
  }

  /// The name of a month (1 to 12) as a heading starts with it: `November`.
  fn month_name(&self, month: u8) -> String;

  /// The heading of a calendar's month: `November 2026`.
  fn month_title(&self, month: YearMonth) -> String {
    format!("{} {}", self.month_name(month.month()), month.year())
  }

  /// The name of a day of the week: what the head of its column is called to what reads the tree, and how a day's
  /// name starts. `Thursday`.
  fn weekday_name(&self, day: Weekday) -> String;

  /// What the head of a weekday's column says: `Th`.
  fn weekday_short(&self, day: Weekday) -> String;

  /// The name of a day of the week as a day of an event calendar has it before its number: `Thu`. The first three
  /// letters of its name unless the app says otherwise.
  fn weekday_medium(&self, day: Weekday) -> String {
    self.weekday_name(day).chars().take(3).collect()
  }

  /// How a field writes a date: `Nov 12, 2026`.
  fn format(&self, date: Date) -> String;

  /// How a field writes a range: both dates whole, an en dash between them.
  fn format_range(&self, range: DateRange) -> String {
    format!("{} – {}", self.format(range.start()), self.format(range.end()))
  }

  /// What a day is called to what reads the tree: `Thursday, Nov 12, 2026`.
  fn day_label(&self, date: Date) -> String {
    format!("{}, {}", self.weekday_name(date.weekday()), self.format(date))
  }

  /// The date what is typed holds, `None` where it is none yet: ISO 8601 (`2026-11-12`) unless the app reads more.
  fn parse(&self, text: &str) -> Option<Date> {
    text.trim().parse().ok()
  }

  /// The range what is typed holds: two dates as [`DateText::parse`] reads them, a dash or `to` between them.
  fn parse_range(&self, text: &str) -> Option<DateRange> {
    let (first, second) = split_range(text)?;
    Some(DateRange::new(self.parse(first)?, self.parse(second)?))
  }

  /// What the button that shows the month before is called: `Previous month`.
  fn previous_month_label(&self) -> String {
    "Previous month".to_owned()
  }

  /// What the button that shows the month after is called: `Next month`.
  fn next_month_label(&self) -> String {
    "Next month".to_owned()
  }

  /// What the button of an event calendar that shows the week before is called: `Previous week`.
  fn previous_week_label(&self) -> String {
    "Previous week".to_owned()
  }

  /// What the button of an event calendar that shows the week after is called: `Next week`.
  fn next_week_label(&self) -> String {
    "Next week".to_owned()
  }

  /// What the button of an event calendar that shows the day before is called: `Previous day`.
  fn previous_day_label(&self) -> String {
    "Previous day".to_owned()
  }

  /// What the button of an event calendar that shows the day after is called: `Next day`.
  fn next_day_label(&self) -> String {
    "Next day".to_owned()
  }

  /// What the button of an event calendar that shows today says: `Today`.
  fn today_label(&self) -> String {
    "Today".to_owned()
  }

  /// What a date picker's button and panel are called: `Choose date`.
  fn choose_date_label(&self) -> String {
    "Choose date".to_owned()
  }

  /// What a date range picker's button and panel are called: `Choose dates`.
  fn choose_dates_label(&self) -> String {
    "Choose dates".to_owned()
  }

  /// What the button that empties a picker's field is called: `Clear`.
  fn clear_label(&self) -> String {
    "Clear".to_owned()
  }

  /// How the hours are counted: from 0 to 23 unless the app says otherwise.
  fn hour_cycle(&self) -> HourCycle {
    HourCycle::H23
  }

  /// What a half of the day is called on a clock of twelve hours: `AM`, `PM`.
  fn day_period_name(&self, period: DayPeriod) -> String {
    match period {
      DayPeriod::Am => "AM".to_owned(),
      DayPeriod::Pm => "PM".to_owned(),
    }
  }

  /// How a field writes a time: `14:30`, or `2:30 PM` on a clock of twelve hours, with its seconds where `seconds`.
  fn format_time(&self, time: Time, seconds: bool) -> String {
    let separator = self.time_separator();
    let rest = if seconds {
      format!("{separator}{:02}{separator}{:02}", time.minute(), time.second())
    } else {
      format!("{separator}{:02}", time.minute())
    };
    match self.hour_cycle() {
      HourCycle::H23 => format!("{:02}{rest}", time.hour()),
      HourCycle::H12 => {
        let (hour, period) = time.hour12();
        format!("{hour}{rest} {}", self.day_period_name(period))
      }
    }
  }

  /// The time what is typed holds, `None` where it is none yet: hours and minutes and perhaps seconds, with the
  /// locale's separator or a colon, and a half of the day by its name or `am`/`pm` after them.
  fn parse_time(&self, text: &str) -> Option<Time> {
    read_time(text, self.time_separator(), |word| {
      [DayPeriod::Am, DayPeriod::Pm]
        .into_iter()
        .find(|period| self.day_period_name(*period).eq_ignore_ascii_case(word))
    })
  }

  /// The order of a date field's segments: the day, the month and the year as the locale writes them in numbers.
  fn date_order(&self) -> DateOrder {
    DateOrder::MonthDayYear
  }

  /// What stands between a date field's segments: `/` in `11/12/2026`.
  fn date_separator(&self) -> char {
    '/'
  }

  /// What stands between a time's hours, minutes and seconds: `:`.
  fn time_separator(&self) -> char {
    ':'
  }

  /// The short name of a month, as a month picker's cell has it: `Nov`. The first three letters of its name unless
  /// the app says otherwise.
  fn month_short(&self, month: u8) -> String {
    self.month_name(month).chars().take(3).collect()
  }

  /// What a segment of a date or time field is called to what reads the tree: `Day`, `Hour`.
  fn segment_label(&self, segment: DateSegment) -> String {
    match segment {
      DateSegment::Day => "Day",
      DateSegment::Month => "Month",
      DateSegment::Year => "Year",
      DateSegment::Hour => "Hour",
      DateSegment::Minute => "Minute",
      DateSegment::Second => "Second",
      DateSegment::Period => "AM/PM",
    }
    .to_owned()
  }

  /// What an empty segment shows: `dd`, `mm`, `yyyy`, `hh`, `mm`, `ss`, and `--` for the half of the day.
  fn segment_placeholder(&self, segment: DateSegment) -> String {
    match segment {
      DateSegment::Day => "dd",
      DateSegment::Month => "mm",
      DateSegment::Year => "yyyy",
      DateSegment::Hour => "hh",
      DateSegment::Minute => "mm",
      DateSegment::Second => "ss",
      DateSegment::Period => "--",
    }
    .to_owned()
  }

  /// What a month picker's button that shows the year before is called: `Previous year`.
  fn previous_year_label(&self) -> String {
    "Previous year".to_owned()
  }

  /// What a month picker's button that shows the year after is called: `Next year`.
  fn next_year_label(&self) -> String {
    "Next year".to_owned()
  }

  /// What a year picker's button that shows the years before is called: `Previous years`.
  fn previous_years_label(&self) -> String {
    "Previous years".to_owned()
  }

  /// What a year picker's button that shows the years after is called: `Next years`.
  fn next_years_label(&self) -> String {
    "Next years".to_owned()
  }

  /// What a time picker's button and panel are called: `Choose time`.
  fn choose_time_label(&self) -> String {
    "Choose time".to_owned()
  }

  /// The heading of a year picker: `2020 – 2031`.
  fn year_range_title(&self, first: i32, last: i32) -> String {
    format!("{first} – {last}")
  }
}

/// A segment of a date field or a time field: what is typed in, stepped and read as one number (or a half of the
/// day).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DateSegment {
  Day,
  Month,
  Year,
  Hour,
  Minute,
  Second,
  /// The half of the day on a clock of twelve hours: AM or PM.
  Period,
}

impl DateSegment {
  /// Its place among the seven: what a locale's tables of segments are indexed by.
  pub fn index(self) -> usize {
    self as usize
  }
}

/// The time `text` holds: hours, minutes and perhaps seconds between `separator`s or colons, two digits for the
/// minutes and the seconds, and after them a half of the day that `period` reads (or `am`, `pm`, `a`, `p`).
pub(crate) fn read_time(text: &str, separator: char, period: impl Fn(&str) -> Option<DayPeriod>) -> Option<Time> {
  let text = text.trim();
  let split = text.find(|c: char| c.is_alphabetic()).unwrap_or(text.len());
  let (clock, words) = (text[..split].trim(), text[split..].trim());
  let half = if words.is_empty() {
    None
  } else {
    let word = words.trim_end_matches('.');
    Some(period(word).or_else(|| match word.to_ascii_lowercase().as_str() {
      "am" | "a" | "a.m" => Some(DayPeriod::Am),
      "pm" | "p" | "p.m" => Some(DayPeriod::Pm),
      _ => None,
    })?)
  };
  let parts: Vec<&str> = clock.split(|c: char| c == separator || c == ':').collect();
  if !(2..=3).contains(&parts.len()) {
    return None;
  }
  let number = |part: &str, digits: std::ops::RangeInclusive<usize>| {
    (digits.contains(&part.len()) && part.bytes().all(|byte| byte.is_ascii_digit()))
      .then(|| part.parse::<u8>().ok())
      .flatten()
  };
  let hour = number(parts[0], 1..=2)?;
  let minute = number(parts[1], 2..=2)?;
  let second = match parts.get(2) {
    Some(part) => number(part, 2..=2)?,
    None => 0,
  };
  match half {
    Some(half) => Time::from_hour12(hour, half, minute, second),
    None => Time::new(hour, minute, second),
  }
}

/// An app's own type shared: what it gives [`App::set_date_locale`](crate::app::App::set_date_locale) and a
/// component's `.locale(..)` as `Arc::new(..)`, or keeps to hand on.
impl<T: DateText + ?Sized> DateText for Arc<T> {
  fn first_day(&self) -> Weekday {
    (**self).first_day()
  }
  fn week(&self) -> [Weekday; 7] {
    (**self).week()
  }
  fn month_name(&self, month: u8) -> String {
    (**self).month_name(month)
  }
  fn month_title(&self, month: YearMonth) -> String {
    (**self).month_title(month)
  }
  fn weekday_name(&self, day: Weekday) -> String {
    (**self).weekday_name(day)
  }
  fn weekday_short(&self, day: Weekday) -> String {
    (**self).weekday_short(day)
  }
  fn weekday_medium(&self, day: Weekday) -> String {
    (**self).weekday_medium(day)
  }
  fn format(&self, date: Date) -> String {
    (**self).format(date)
  }
  fn format_range(&self, range: DateRange) -> String {
    (**self).format_range(range)
  }
  fn day_label(&self, date: Date) -> String {
    (**self).day_label(date)
  }
  fn parse(&self, text: &str) -> Option<Date> {
    (**self).parse(text)
  }
  fn parse_range(&self, text: &str) -> Option<DateRange> {
    (**self).parse_range(text)
  }
  fn previous_month_label(&self) -> String {
    (**self).previous_month_label()
  }
  fn next_month_label(&self) -> String {
    (**self).next_month_label()
  }
  fn previous_week_label(&self) -> String {
    (**self).previous_week_label()
  }
  fn next_week_label(&self) -> String {
    (**self).next_week_label()
  }
  fn previous_day_label(&self) -> String {
    (**self).previous_day_label()
  }
  fn next_day_label(&self) -> String {
    (**self).next_day_label()
  }
  fn today_label(&self) -> String {
    (**self).today_label()
  }
  fn choose_date_label(&self) -> String {
    (**self).choose_date_label()
  }
  fn choose_dates_label(&self) -> String {
    (**self).choose_dates_label()
  }
  fn clear_label(&self) -> String {
    (**self).clear_label()
  }
  fn hour_cycle(&self) -> HourCycle {
    (**self).hour_cycle()
  }
  fn day_period_name(&self, period: DayPeriod) -> String {
    (**self).day_period_name(period)
  }
  fn format_time(&self, time: Time, seconds: bool) -> String {
    (**self).format_time(time, seconds)
  }
  fn parse_time(&self, text: &str) -> Option<Time> {
    (**self).parse_time(text)
  }
  fn date_order(&self) -> DateOrder {
    (**self).date_order()
  }
  fn date_separator(&self) -> char {
    (**self).date_separator()
  }
  fn time_separator(&self) -> char {
    (**self).time_separator()
  }
  fn month_short(&self, month: u8) -> String {
    (**self).month_short(month)
  }
  fn segment_label(&self, segment: DateSegment) -> String {
    (**self).segment_label(segment)
  }
  fn segment_placeholder(&self, segment: DateSegment) -> String {
    (**self).segment_placeholder(segment)
  }
  fn previous_year_label(&self) -> String {
    (**self).previous_year_label()
  }
  fn next_year_label(&self) -> String {
    (**self).next_year_label()
  }
  fn previous_years_label(&self) -> String {
    (**self).previous_years_label()
  }
  fn next_years_label(&self) -> String {
    (**self).next_years_label()
  }
  fn choose_time_label(&self) -> String {
    (**self).choose_time_label()
  }
  fn year_range_title(&self, first: i32, last: i32) -> String {
    (**self).year_range_title(first, last)
  }
}

/// `value` as what a component or the app keeps: an `Arc` of the trait, made once.
pub(crate) fn shared_date_text(value: impl DateText + 'static) -> Arc<dyn DateText> {
  Arc::new(value)
}

/// English, shared: what a component shows where nothing says otherwise.
pub(crate) fn english_date_text() -> Arc<dyn DateText> {
  static ENGLISH: OnceLock<Arc<dyn DateText>> = OnceLock::new();
  ENGLISH.get_or_init(|| Arc::new(DateLocale::english())).clone()
}

/// The two halves of a range as it is typed: what is before and after a dash or `to`.
fn split_range(text: &str) -> Option<(&str, &str)> {
  let text = text.trim();
  [" – ", " — ", "–", "—", " - ", " to "]
    .iter()
    .find_map(|dash| text.split_once(dash))
}

/// The app's choice of a date's text, which [`App::set_date_locale`](crate::app::App::set_date_locale) makes: read so
/// that what reads it in a render is rendered again when it changes.
#[derive(Clone)]
pub(crate) struct AppDateText {
  current: Arc<RwLock<Arc<dyn DateText>>>,
  version: Signal<u64>,
}

impl AppDateText {
  pub fn new() -> Self {
    Self {
      current: Arc::new(RwLock::new(english_date_text())),
      version: Signal::new(0),
    }
  }

  /// The choice, read so that the render reading it is run again when it changes.
  pub fn get(&self) -> Arc<dyn DateText> {
    let _ = self.version.get();
    self.current.read().clone()
  }

  pub fn set(&self, text: Arc<dyn DateText>) {
    *self.current.write() = text;
    self.version.update(|version| *version = version.wrapping_add(1));
  }
}

/// How [`DateLocale`] writes a date in a field, in its locale's order. The least that covers what is asked of a date's
/// form: its numbers, the month's short or long name, and ISO 8601; anything else is
/// [`DateLocale::with_format`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DateStyle {
  /// In numbers, with the locale's separator: `12.11.2026`, `11/12/2026`.
  Numeric,
  /// The month by its short name: `Nov 12, 2026`, `12 Nov 2026`.
  Short,
  /// The month by its whole name: `November 12, 2026`, `12 November 2026`. The name is the locale's as its table
  /// has it: a language whose dates take another case of the month (`12 листопада`) gives a table or a format of its
  /// own.
  Long,
  /// ISO 8601, whatever the locale: `2026-11-12`.
  Iso,
}

/// How a date is written, an app's own: what [`DateLocale::with_format`] gives.
pub type DateFormatter = Arc<dyn Fn(Date) -> String + Send + Sync>;

/// How what is typed is read, an app's own: what [`DateLocale::with_parse`] gives.
pub type DateParser = Arc<dyn Fn(&str) -> Option<Date> + Send + Sync>;

/// What a calendar and a date picker call their controls, in a locale's words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DateLabels {
  pub previous_month: Arc<str>,
  pub next_month: Arc<str>,
  pub choose_date: Arc<str>,
  pub choose_dates: Arc<str>,
  pub clear: Arc<str>,
  pub previous_year: Arc<str>,
  pub next_year: Arc<str>,
  pub previous_years: Arc<str>,
  pub next_years: Arc<str>,
  pub choose_time: Arc<str>,
  /// What the button of an event calendar that shows today says.
  pub today: Arc<str>,
  pub previous_week: Arc<str>,
  pub next_week: Arc<str>,
  pub previous_day: Arc<str>,
  pub next_day: Arc<str>,
}

impl DateLabels {
  fn of(words: [&str; 15]) -> Self {
    let [
      previous_month,
      next_month,
      choose_date,
      choose_dates,
      clear,
      previous_year,
      next_year,
      previous_years,
      next_years,
      choose_time,
      today,
      previous_week,
      next_week,
      previous_day,
      next_day,
    ] = words.map(Arc::from);
    Self {
      previous_month,
      next_month,
      choose_date,
      choose_dates,
      clear,
      previous_year,
      next_year,
      previous_years,
      next_years,
      choose_time,
      today,
      previous_week,
      next_week,
      previous_day,
      next_day,
    }
  }

  /// English: `Previous month`, `Next month`, `Choose date`, `Choose dates`, `Clear`, `Previous year`, `Next year`,
  /// `Previous years`, `Next years`, `Choose time`, `Today`, `Previous week`, `Next week`, `Previous day`, `Next day`.
  pub fn english() -> Self {
    Self::of([
      "Previous month",
      "Next month",
      "Choose date",
      "Choose dates",
      "Clear",
      "Previous year",
      "Next year",
      "Previous years",
      "Next years",
      "Choose time",
      "Today",
      "Previous week",
      "Next week",
      "Previous day",
      "Next day",
    ])
  }
}

/// What the segments of a date field and a time field are called to what reads the tree, and what an empty one shows,
/// in a locale's words: indexed by [`DateSegment::index`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentWords {
  pub labels: [Arc<str>; 7],
  pub placeholders: [Arc<str>; 7],
}

impl SegmentWords {
  fn of(labels: [&str; 7], placeholders: [&str; 7]) -> Self {
    Self {
      labels: labels.map(Arc::from),
      placeholders: placeholders.map(Arc::from),
    }
  }

  /// English: `Day`, `Month`, `Year`, `Hour`, `Minute`, `Second`, `AM/PM`; `dd`, `mm`, `yyyy`, `hh`, `mm`, `ss`,
  /// `--`.
  pub fn english() -> Self {
    Self::of(
      ["Day", "Month", "Year", "Hour", "Minute", "Second", "AM/PM"],
      ["dd", "mm", "yyyy", "hh", "mm", "ss", "--"],
    )
  }
}

/// The order a date is written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DateOrder {
  /// `11/12/2026`, `Nov 12, 2026`.
  MonthDayYear,
  /// `12.11.2026`, `12 Nov 2026`.
  DayMonthYear,
  /// `2026-11-12`.
  YearMonthDay,
}

/// How a locale writes the dates of a calendar, and what it reads back: the first day of the week, the names of the
/// months and of the days, the form a field shows, and the names of the controls. lurq's [`DateText`].
///
/// ```
/// use lurq::core::{Date, DateLocale, Weekday};
///
/// let day = Date::new(2026, 11, 12).unwrap();
/// assert_eq!(DateLocale::for_tag("en").format(day), "Nov 12, 2026");
/// assert_eq!(DateLocale::for_tag("uk").format(day), "12.11.2026");
/// assert_eq!(DateLocale::for_tag("en-US").first_day, Weekday::Sunday);
/// assert_eq!(DateLocale::for_tag("de").parse("12.11.2026"), Some(day));
///
/// // A locale of the app's own: English names, weeks from Saturday.
/// let own = DateLocale::for_tag("en").with_first_day(Weekday::Saturday);
///
/// // Another way of writing dates is a style or a closure, not a trait of the app's.
/// use lurq::core::DateStyle;
/// assert_eq!(DateLocale::for_tag("en").with_style(DateStyle::Numeric).format(day), "11/12/2026");
/// assert_eq!(DateLocale::for_tag("en-GB").with_style(DateStyle::Long).format(day), "12 November 2026");
/// assert_eq!(DateLocale::for_tag("de").with_style(DateStyle::Iso).format(day), "2026-11-12");
/// let mine = DateLocale::english().with_format(|date| format!("{}/{}", date.day(), date.month()));
/// assert_eq!(mine.format(day), "12/11");
/// ```
///
/// lurq knows English (`en`, with weeks from Monday as the design draws them; `en-US` from Sunday; `en-GB` writing
/// the day first), Ukrainian (`uk`), German (`de`), French (`fr`), Spanish (`es`) and Polish (`pl`); any other tag is
/// English. Every field is the app's to replace.
#[derive(Clone)]
pub struct DateLocale {
  /// The tag it was made for: `en`, `uk`.
  pub tag: Arc<str>,
  /// The day a week starts on: the first column of a calendar.
  pub first_day: Weekday,
  /// The months by name, from January: what a calendar's heading says.
  pub months: [Arc<str>; 12],
  /// The months as a written date has them: `Nov`.
  pub months_short: [Arc<str>; 12],
  /// The days of the week by name, from Monday: what a day is called to what reads the tree.
  pub weekdays: [Arc<str>; 7],
  /// The days of the week as the head of a calendar's column has them, from Monday: `Mo`.
  pub weekdays_short: [Arc<str>; 7],
  /// The days of the week as a day of an event calendar has them before its number, from Monday: `Mon`.
  pub weekdays_medium: [Arc<str>; 7],
  /// The order of the day, the month and the year.
  pub order: DateOrder,
  /// How a field writes a date: in numbers, by the month's short or long name, or as ISO 8601.
  pub style: DateStyle,
  /// What is between the numbers of a date written in numbers: `.` in `12.11.2026`.
  pub separator: char,
  /// How a date is written, the app's, in place of [`Self::style`]: [`Self::with_format`].
  pub formatter: Option<DateFormatter>,
  /// How what is typed is read, the app's, in place of the locale's: [`Self::with_parse`].
  pub parser: Option<DateParser>,
  /// What a calendar and a date picker call their controls.
  pub labels: DateLabels,
  /// How the hours are counted: `14:30`, or `2:30 PM`.
  pub hour_cycle: HourCycle,
  /// What the halves of the day are called on a clock of twelve hours: `AM`, `PM`.
  pub day_periods: [Arc<str>; 2],
  /// What the segments of a date or time field are called, and what an empty one shows.
  pub segments: SegmentWords,
}

impl fmt::Debug for DateLocale {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.debug_struct("DateLocale")
      .field("tag", &self.tag)
      .field("first_day", &self.first_day)
      .field("order", &self.order)
      .field("style", &self.style)
      .field("separator", &self.separator)
      .field("formatter", &self.formatter.is_some())
      .field("parser", &self.parser.is_some())
      .finish_non_exhaustive()
  }
}

/// The same tables, form and labels, and the same closures of the app's (the same `Arc`s).
impl PartialEq for DateLocale {
  fn eq(&self, other: &Self) -> bool {
    let same = |a: &Option<DateFormatter>, b: &Option<DateFormatter>| match (a, b) {
      (None, None) => true,
      (Some(a), Some(b)) => Arc::ptr_eq(a, b),
      _ => false,
    };
    let same_parser = |a: &Option<DateParser>, b: &Option<DateParser>| match (a, b) {
      (None, None) => true,
      (Some(a), Some(b)) => Arc::ptr_eq(a, b),
      _ => false,
    };
    self.tag == other.tag
      && self.first_day == other.first_day
      && self.months == other.months
      && self.months_short == other.months_short
      && self.weekdays == other.weekdays
      && self.weekdays_short == other.weekdays_short
      && self.order == other.order
      && self.style == other.style
      && self.separator == other.separator
      && same(&self.formatter, &other.formatter)
      && same_parser(&self.parser, &other.parser)
      && self.labels == other.labels
      && self.hour_cycle == other.hour_cycle
      && self.day_periods == other.day_periods
      && self.segments == other.segments
  }
}

fn names<const N: usize>(names: [&str; N]) -> [Arc<str>; N] {
  names.map(Arc::from)
}

const ENGLISH_MONTHS: [&str; 12] = [
  "January",
  "February",
  "March",
  "April",
  "May",
  "June",
  "July",
  "August",
  "September",
  "October",
  "November",
  "December",
];

impl DateLocale {
  /// English, as the design writes it: weeks from Monday, `Nov 12, 2026`.
  pub fn english() -> Self {
    Self {
      tag: Arc::from("en"),
      first_day: Weekday::Monday,
      months: names(ENGLISH_MONTHS),
      months_short: names([
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
      ]),
      weekdays: names([
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
        "Sunday",
      ]),
      weekdays_short: names(["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"]),
      weekdays_medium: names(["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]),
      order: DateOrder::MonthDayYear,
      style: DateStyle::Short,
      separator: '/',
      formatter: None,
      parser: None,
      labels: DateLabels::english(),
      hour_cycle: HourCycle::H23,
      day_periods: names(["AM", "PM"]),
      segments: SegmentWords::english(),
    }
  }

  /// The locale of a tag (`en`, `en-US`, `uk-UA`): by the whole tag where lurq knows it, else by its language, else
  /// English.
  pub fn for_tag(tag: &str) -> Self {
    let tag = tag.trim().replace('_', "-").to_ascii_lowercase();
    let language = tag.split('-').next().unwrap_or("");
    let mut locale = match (tag.as_str(), language) {
      ("en-us" | "en-ca" | "en-ph", _) => Self {
        first_day: Weekday::Sunday,
        hour_cycle: HourCycle::H12,
        ..Self::english()
      },
      ("en-gb" | "en-ie" | "en-au" | "en-nz" | "en-in" | "en-za", _) => Self {
        order: DateOrder::DayMonthYear,
        ..Self::english()
      },
      (_, "uk") => Self {
        months: names([
          "січень",
          "лютий",
          "березень",
          "квітень",
          "травень",
          "червень",
          "липень",
          "серпень",
          "вересень",
          "жовтень",
          "листопад",
          "грудень",
        ]),
        months_short: names([
          "січ", "лют", "бер", "кві", "тра", "чер", "лип", "сер", "вер", "жов", "лис", "гру",
        ]),
        weekdays: names([
          "понеділок",
          "вівторок",
          "середа",
          "четвер",
          "пʼятниця",
          "субота",
          "неділя",
        ]),
        weekdays_short: names(["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Нд"]),
        weekdays_medium: names(["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Нд"]),
        order: DateOrder::DayMonthYear,
        style: DateStyle::Numeric,
        separator: '.',
        labels: DateLabels::of([
          "Попередній місяць",
          "Наступний місяць",
          "Виберіть дату",
          "Виберіть дати",
          "Очистити",
          "Попередній рік",
          "Наступний рік",
          "Попередні роки",
          "Наступні роки",
          "Виберіть час",
          "Сьогодні",
          "Попередній тиждень",
          "Наступний тиждень",
          "Попередній день",
          "Наступний день",
        ]),
        segments: SegmentWords::of(
          ["День", "Місяць", "Рік", "Година", "Хвилина", "Секунда", "Частина доби"],
          ["дд", "мм", "рррр", "гг", "хх", "сс", "--"],
        ),
        ..Self::english()
      },
      (_, "de") => Self {
        months: names([
          "Januar",
          "Februar",
          "März",
          "April",
          "Mai",
          "Juni",
          "Juli",
          "August",
          "September",
          "Oktober",
          "November",
          "Dezember",
        ]),
        months_short: names([
          "Jan", "Feb", "Mär", "Apr", "Mai", "Jun", "Jul", "Aug", "Sep", "Okt", "Nov", "Dez",
        ]),
        weekdays: names([
          "Montag",
          "Dienstag",
          "Mittwoch",
          "Donnerstag",
          "Freitag",
          "Samstag",
          "Sonntag",
        ]),
        weekdays_short: names(["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"]),
        weekdays_medium: names(["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"]),
        order: DateOrder::DayMonthYear,
        style: DateStyle::Numeric,
        separator: '.',
        labels: DateLabels::of([
          "Vorheriger Monat",
          "Nächster Monat",
          "Datum wählen",
          "Daten wählen",
          "Löschen",
          "Vorheriges Jahr",
          "Nächstes Jahr",
          "Vorherige Jahre",
          "Nächste Jahre",
          "Uhrzeit wählen",
          "Heute",
          "Vorherige Woche",
          "Nächste Woche",
          "Vorheriger Tag",
          "Nächster Tag",
        ]),
        segments: SegmentWords::of(
          ["Tag", "Monat", "Jahr", "Stunde", "Minute", "Sekunde", "AM/PM"],
          ["TT", "MM", "JJJJ", "hh", "mm", "ss", "--"],
        ),
        ..Self::english()
      },
      (_, "fr") => Self {
        months: names([
          "janvier",
          "février",
          "mars",
          "avril",
          "mai",
          "juin",
          "juillet",
          "août",
          "septembre",
          "octobre",
          "novembre",
          "décembre",
        ]),
        months_short: names([
          "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc.",
        ]),
        weekdays: names(["lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche"]),
        weekdays_short: names(["Lu", "Ma", "Me", "Je", "Ve", "Sa", "Di"]),
        weekdays_medium: names(["Lun", "Mar", "Mer", "Jeu", "Ven", "Sam", "Dim"]),
        order: DateOrder::DayMonthYear,
        style: DateStyle::Numeric,
        separator: '/',
        labels: DateLabels::of([
          "Mois précédent",
          "Mois suivant",
          "Choisir une date",
          "Choisir des dates",
          "Effacer",
          "Année précédente",
          "Année suivante",
          "Années précédentes",
          "Années suivantes",
          "Choisir une heure",
          "Aujourd’hui",
          "Semaine précédente",
          "Semaine suivante",
          "Jour précédent",
          "Jour suivant",
        ]),
        segments: SegmentWords::of(
          ["Jour", "Mois", "Année", "Heure", "Minute", "Seconde", "AM/PM"],
          ["jj", "mm", "aaaa", "hh", "mm", "ss", "--"],
        ),
        ..Self::english()
      },
      (_, "es") => Self {
        months: names([
          "enero",
          "febrero",
          "marzo",
          "abril",
          "mayo",
          "junio",
          "julio",
          "agosto",
          "septiembre",
          "octubre",
          "noviembre",
          "diciembre",
        ]),
        months_short: names([
          "ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sept", "oct", "nov", "dic",
        ]),
        weekdays: names(["lunes", "martes", "miércoles", "jueves", "viernes", "sábado", "domingo"]),
        weekdays_short: names(["Lu", "Ma", "Mi", "Ju", "Vi", "Sá", "Do"]),
        weekdays_medium: names(["Lun", "Mar", "Mié", "Jue", "Vie", "Sáb", "Dom"]),
        order: DateOrder::DayMonthYear,
        style: DateStyle::Numeric,
        separator: '/',
        labels: DateLabels::of([
          "Mes anterior",
          "Mes siguiente",
          "Elegir fecha",
          "Elegir fechas",
          "Borrar",
          "Año anterior",
          "Año siguiente",
          "Años anteriores",
          "Años siguientes",
          "Elegir hora",
          "Hoy",
          "Semana anterior",
          "Semana siguiente",
          "Día anterior",
          "Día siguiente",
        ]),
        segments: SegmentWords::of(
          ["Día", "Mes", "Año", "Hora", "Minuto", "Segundo", "AM/PM"],
          ["dd", "mm", "aaaa", "hh", "mm", "ss", "--"],
        ),
        ..Self::english()
      },
      (_, "pl") => Self {
        months: names([
          "styczeń",
          "luty",
          "marzec",
          "kwiecień",
          "maj",
          "czerwiec",
          "lipiec",
          "sierpień",
          "wrzesień",
          "październik",
          "listopad",
          "grudzień",
        ]),
        months_short: names([
          "sty", "lut", "mar", "kwi", "maj", "cze", "lip", "sie", "wrz", "paź", "lis", "gru",
        ]),
        weekdays: names([
          "poniedziałek",
          "wtorek",
          "środa",
          "czwartek",
          "piątek",
          "sobota",
          "niedziela",
        ]),
        weekdays_short: names(["Pn", "Wt", "Śr", "Cz", "Pt", "So", "Nd"]),
        weekdays_medium: names(["Pon", "Wt", "Śr", "Czw", "Pt", "Sob", "Nd"]),
        order: DateOrder::DayMonthYear,
        style: DateStyle::Numeric,
        separator: '.',
        labels: DateLabels::of([
          "Poprzedni miesiąc",
          "Następny miesiąc",
          "Wybierz datę",
          "Wybierz daty",
          "Wyczyść",
          "Poprzedni rok",
          "Następny rok",
          "Poprzednie lata",
          "Następne lata",
          "Wybierz godzinę",
          "Dzisiaj",
          "Poprzedni tydzień",
          "Następny tydzień",
          "Poprzedni dzień",
          "Następny dzień",
        ]),
        segments: SegmentWords::of(
          ["Dzień", "Miesiąc", "Rok", "Godzina", "Minuta", "Sekunda", "AM/PM"],
          ["dd", "mm", "rrrr", "gg", "mm", "ss", "--"],
        ),
        ..Self::english()
      },
      _ => Self::english(),
    };
    locale.tag = Arc::from(tag.as_str());
    locale
  }

  /// It, with weeks from `first`.
  pub fn with_first_day(mut self, first: Weekday) -> Self {
    self.first_day = first;
    self
  }

  /// It, writing dates in `style`, in its order: [`DateStyle::Numeric`] `12.11.2026`, [`DateStyle::Short`] `Nov 12,
  /// 2026`, [`DateStyle::Long`] `12 November 2026`, [`DateStyle::Iso`] `2026-11-12`. What is typed is read in any of
  /// them.
  pub fn with_style(mut self, style: DateStyle) -> Self {
    self.style = style;
    self
  }

  /// It, with `separator` between the numbers of a date written in numbers: `12-11-2026`.
  pub fn with_separator(mut self, separator: char) -> Self {
    self.separator = separator;
    self
  }

  /// It, writing dates the app's way: a form no style is. A range is written as two whole dates. Give
  /// [`Self::with_parse`] too where what it writes is not read back by the locale: a field reads what it shows.
  pub fn with_format(mut self, format: impl Fn(Date) -> String + Send + Sync + 'static) -> Self {
    self.formatter = Some(Arc::new(format));
    self
  }

  /// It, reading what is typed the app's way, in place of the locale's forms and ISO 8601: `None` for what is no date,
  /// or not yet one.
  pub fn with_parse(mut self, parse: impl Fn(&str) -> Option<Date> + Send + Sync + 'static) -> Self {
    self.parser = Some(Arc::new(parse));
    self
  }

  /// It, calling a calendar's and a date picker's controls by `labels`.
  pub fn with_labels(mut self, labels: DateLabels) -> Self {
    self.labels = labels;
    self
  }

  /// The days of a week in the order a calendar's columns have them, from the first day.
  pub fn week(&self) -> [Weekday; 7] {
    std::array::from_fn(|index| Weekday::from_monday(self.first_day as i64 + index as i64))
  }

  /// The name of a month, as a heading starts with it: `November`, `Листопад`.
  pub fn month_name(&self, month: u8) -> String {
    capitalized(&self.months[(month.clamp(1, 12) - 1) as usize])
  }

  /// The heading of a calendar's month: `November 2026`.
  pub fn month_title(&self, month: YearMonth) -> String {
    format!("{} {}", self.month_name(month.month()), month.year())
  }

  /// The name of a month in a written date, by the style: `Nov`, or `November`. `None` where the style writes numbers.
  fn written_month(&self, month: u8) -> Option<&str> {
    let index = (month.clamp(1, 12) - 1) as usize;
    match self.style {
      DateStyle::Short => Some(&self.months_short[index]),
      DateStyle::Long => Some(&self.months[index]),
      DateStyle::Numeric | DateStyle::Iso => None,
    }
  }

  /// The form of a date a date picker's field shows: `Nov 12, 2026`, `12.11.2026`, by its style, or the app's.
  pub fn format(&self, date: Date) -> String {
    if let Some(format) = &self.formatter {
      return format(date);
    }
    let (day, month, year) = (date.day(), date.month(), date.year());
    let separator = self.separator;
    if self.style == DateStyle::Iso {
      return date.to_string();
    }
    match (self.written_month(month), self.order) {
      (None, DateOrder::DayMonthYear) => format!("{day:02}{separator}{month:02}{separator}{year:04}"),
      (None, DateOrder::MonthDayYear) => format!("{month:02}{separator}{day:02}{separator}{year:04}"),
      (None, DateOrder::YearMonthDay) => format!("{year:04}{separator}{month:02}{separator}{day:02}"),
      (Some(name), DateOrder::MonthDayYear) => format!("{name} {day}, {year}"),
      (Some(name), DateOrder::DayMonthYear) => format!("{day} {name} {year}"),
      (Some(name), DateOrder::YearMonthDay) => format!("{year} {name} {day}"),
    }
  }

  /// The form of a range: `Nov 9 – Nov 15, 2026`, the year once where both ends are in it and the month is written by
  /// name; else both ends whole, between them an en dash.
  pub fn format_range(&self, range: DateRange) -> String {
    let (start, end) = (range.start(), range.end());
    let named = self.formatter.is_none() && self.written_month(1).is_some();
    if named && start.year() == end.year() {
      let name = |date: Date| self.written_month(date.month()).unwrap_or_default();
      match self.order {
        DateOrder::MonthDayYear => {
          return format!("{} {} – {}", name(start), start.day(), self.format(end));
        }
        DateOrder::DayMonthYear => {
          return format!("{} {} – {}", start.day(), name(start), self.format(end));
        }
        DateOrder::YearMonthDay => {}
      }
    }
    format!("{} – {}", self.format(start), self.format(end))
  }

  /// What a day is called to what reads the tree: its weekday and its short form, `Thursday, Nov 12, 2026`.
  pub fn day_label(&self, date: Date) -> String {
    format!(
      "{}, {}",
      capitalized(&self.weekdays[date.weekday().days_from_monday() as usize]),
      self.format(date)
    )
  }

  /// The date `text` holds, written in the short form, in numbers in the locale's order with any separator, or as
  /// ISO 8601 (`2026-11-12`). A month may be written by its name or the start of it, in the locale's words or in
  /// English. The year has four digits: what is still being typed (`12.11.20`) is no date yet.
  pub fn parse(&self, text: &str) -> Option<Date> {
    if let Some(parse) = &self.parser {
      return parse(text);
    }
    let text = text.trim();
    if let Ok(date) = text.parse::<Date>() {
      return Some(date);
    }
    // The words and the numbers of the text, in order; anything else is between them.
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut digits = false;
    for character in text.chars() {
      let kind = if character.is_ascii_digit() {
        Some(true)
      } else if character.is_alphabetic() {
        Some(false)
      } else {
        None
      };
      match kind {
        Some(kind) if !current.is_empty() && kind == digits => current.push(character),
        Some(kind) => {
          if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
          }
          digits = kind;
          current.push(character);
        }
        None => {
          if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
          }
        }
      }
    }
    if !current.is_empty() {
      tokens.push(current);
    }
    if tokens.len() != 3 {
      return None;
    }
    let number = |token: &str| -> Option<i64> {
      token
        .bytes()
        .all(|byte| byte.is_ascii_digit())
        .then(|| token.parse().ok())
        .flatten()
    };
    let year_of = |token: &str| (token.len() == 4).then(|| number(token)).flatten();
    let named: Vec<usize> = (0..3).filter(|&index| number(&tokens[index]).is_none()).collect();
    let (year, month, day) = match named.as_slice() {
      // A month by name: of the two numbers, the one of four digits is the year.
      [at] => {
        let month = self.month_of(&tokens[*at])?;
        let numbers: Vec<&String> = (0..3).filter(|index| index != at).map(|index| &tokens[index]).collect();
        let (year, day) = match (year_of(numbers[0]), year_of(numbers[1])) {
          (Some(year), None) => (year, number(numbers[1])?),
          (None, Some(year)) => (year, number(numbers[0])?),
          _ => return None,
        };
        (year, month as i64, day)
      }
      [] => {
        let [a, b, c] = [&tokens[0], &tokens[1], &tokens[2]];
        if a.len() == 4 {
          (year_of(a)?, number(b)?, number(c)?)
        } else {
          match self.order {
            DateOrder::MonthDayYear => (year_of(c)?, number(a)?, number(b)?),
            DateOrder::DayMonthYear => (year_of(c)?, number(b)?, number(a)?),
            DateOrder::YearMonthDay => (year_of(a)?, number(b)?, number(c)?),
          }
        }
      }
      _ => return None,
    };
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
      return None;
    }
    Date::new(year as i32, month as u8, day as u8)
  }

  /// The range `text` holds: two dates, as [`DateLocale::parse`] reads them, between them a dash or `to`. The year of
  /// the first may be left out where it is the second's: `Nov 9 – Nov 15, 2026`.
  pub fn parse_range(&self, text: &str) -> Option<DateRange> {
    let (first, second) = split_range(text)?;
    let end = self.parse(second)?;
    let start = self.parse(first).or_else(|| {
      let written = match self.order {
        DateOrder::MonthDayYear => format!("{}, {}", first.trim(), end.year()),
        _ => format!("{} {}", first.trim(), end.year()),
      };
      self.parse(&written)
    })?;
    Some(DateRange::new(start, end))
  }

  /// The month (1 to 12) a word names: the start of one of the locale's names of months, or of the English ones, of
  /// three letters or more, or the whole of a shorter name.
  fn month_of(&self, word: &str) -> Option<u8> {
    let word = word.to_lowercase();
    let long: Vec<String> = self.months.iter().map(|name| name.to_lowercase()).collect();
    let short: Vec<String> = self
      .months_short
      .iter()
      .map(|name| name.trim_end_matches('.').to_lowercase())
      .collect();
    let english: Vec<String> = ENGLISH_MONTHS.iter().map(|name| name.to_lowercase()).collect();
    let matches = |names: &[String]| {
      names.iter().position(|name| {
        *name == word || (word.chars().count() >= 3 && (name.starts_with(&word) || word.starts_with(name.as_str())))
      })
    };
    matches(&short)
      .or_else(|| matches(&long))
      .or_else(|| matches(&english))
      .map(|index| index as u8 + 1)
  }
}

impl Default for DateLocale {
  fn default() -> Self {
    Self::english()
  }
}

impl DateText for DateLocale {
  fn first_day(&self) -> Weekday {
    self.first_day
  }

  fn week(&self) -> [Weekday; 7] {
    DateLocale::week(self)
  }

  fn month_name(&self, month: u8) -> String {
    DateLocale::month_name(self, month)
  }

  fn month_title(&self, month: YearMonth) -> String {
    DateLocale::month_title(self, month)
  }

  fn weekday_name(&self, day: Weekday) -> String {
    capitalized(&self.weekdays[day.days_from_monday() as usize])
  }

  fn weekday_short(&self, day: Weekday) -> String {
    self.weekdays_short[day.days_from_monday() as usize].to_string()
  }

  fn weekday_medium(&self, day: Weekday) -> String {
    self.weekdays_medium[day.days_from_monday() as usize].to_string()
  }

  fn format(&self, date: Date) -> String {
    DateLocale::format(self, date)
  }

  fn format_range(&self, range: DateRange) -> String {
    DateLocale::format_range(self, range)
  }

  fn day_label(&self, date: Date) -> String {
    DateLocale::day_label(self, date)
  }

  fn parse(&self, text: &str) -> Option<Date> {
    DateLocale::parse(self, text)
  }

  fn parse_range(&self, text: &str) -> Option<DateRange> {
    DateLocale::parse_range(self, text)
  }

  fn previous_month_label(&self) -> String {
    self.labels.previous_month.to_string()
  }

  fn next_month_label(&self) -> String {
    self.labels.next_month.to_string()
  }

  fn previous_week_label(&self) -> String {
    self.labels.previous_week.to_string()
  }

  fn next_week_label(&self) -> String {
    self.labels.next_week.to_string()
  }

  fn previous_day_label(&self) -> String {
    self.labels.previous_day.to_string()
  }

  fn next_day_label(&self) -> String {
    self.labels.next_day.to_string()
  }

  fn today_label(&self) -> String {
    self.labels.today.to_string()
  }

  fn choose_date_label(&self) -> String {
    self.labels.choose_date.to_string()
  }

  fn choose_dates_label(&self) -> String {
    self.labels.choose_dates.to_string()
  }

  fn clear_label(&self) -> String {
    self.labels.clear.to_string()
  }

  fn hour_cycle(&self) -> HourCycle {
    self.hour_cycle
  }

  fn day_period_name(&self, period: DayPeriod) -> String {
    self.day_periods[period as usize].to_string()
  }

  fn date_order(&self) -> DateOrder {
    self.order
  }

  fn date_separator(&self) -> char {
    self.separator
  }

  fn month_short(&self, month: u8) -> String {
    capitalized(self.months_short[(month.clamp(1, 12) - 1) as usize].trim_end_matches('.'))
  }

  fn segment_label(&self, segment: DateSegment) -> String {
    self.segments.labels[segment.index()].to_string()
  }

  fn segment_placeholder(&self, segment: DateSegment) -> String {
    self.segments.placeholders[segment.index()].to_string()
  }

  fn previous_year_label(&self) -> String {
    self.labels.previous_year.to_string()
  }

  fn next_year_label(&self) -> String {
    self.labels.next_year.to_string()
  }

  fn previous_years_label(&self) -> String {
    self.labels.previous_years.to_string()
  }

  fn next_years_label(&self) -> String {
    self.labels.next_years.to_string()
  }

  fn choose_time_label(&self) -> String {
    self.labels.choose_time.to_string()
  }
}

/// `word` with its first letter in capitals.
fn capitalized(word: &str) -> String {
  let mut characters = word.chars();
  match characters.next() {
    Some(first) => first.to_uppercase().chain(characters).collect(),
    None => String::new(),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_style_writes_what_is_asked_of_a_dates_form() {
    let day = Date::new(2026, 11, 12).unwrap();
    let style = |tag: &str, style: DateStyle| DateLocale::for_tag(tag).with_style(style).format(day);
    assert_eq!(style("de", DateStyle::Numeric), "12.11.2026");
    assert_eq!(style("en", DateStyle::Numeric), "11/12/2026");
    assert_eq!(style("en", DateStyle::Short), "Nov 12, 2026");
    assert_eq!(style("en-GB", DateStyle::Long), "12 November 2026");
    assert_eq!(style("en", DateStyle::Long), "November 12, 2026");
    assert_eq!(style("uk", DateStyle::Iso), "2026-11-12");
    assert_eq!(
      DateLocale::for_tag("en-GB")
        .with_style(DateStyle::Numeric)
        .with_separator('-')
        .format(day),
      "12-11-2026"
    );
    // Whatever the style, each of the forms is read back.
    let long = DateLocale::for_tag("en-GB").with_style(DateStyle::Long);
    for text in ["12 November 2026", "12/11/2026", "2026-11-12", "12 Nov 2026"] {
      assert_eq!(long.parse(text), Some(day), "{text}");
    }
    let week = DateRange::new(day, day.add_days(3));
    assert_eq!(long.format_range(week), "12 November – 15 November 2026");
  }

  #[test]
  fn an_apps_own_format_and_parse_are_the_locales() {
    let day = Date::new(2026, 11, 12).unwrap();
    let own = DateLocale::for_tag("uk")
      .with_format(|date| format!("{}·{}·{}", date.day(), date.month(), date.year()))
      .with_parse(|text| {
        let mut parts = text.split('·').map(|part| part.trim().parse::<i32>().ok());
        let (day, month, year) = (parts.next()??, parts.next()??, parts.next()??);
        Date::new(year, month as u8, day as u8)
      });
    assert_eq!(own.format(day), "12·11·2026");
    assert_eq!(own.parse("12·11·2026"), Some(day));
    assert_eq!(own.parse("12.11.2026"), None, "the app's reading alone");
    assert_eq!(
      own.format_range(DateRange::new(day, day.add_days(1))),
      "12·11·2026 – 13·11·2026"
    );
    // The rest is the locale's.
    assert_eq!(own.month_title(day.year_month()), "Листопад 2026");
    assert_eq!(DateText::previous_month_label(&own), "Попередній місяць");
    assert_eq!(DateText::clear_label(&DateLocale::english()), "Clear");
  }

  #[test]
  fn a_locale_writes_and_reads_a_time() {
    let time = Time::new(14, 5, 9).unwrap();
    let en = DateLocale::english();
    assert_eq!(en.format_time(time, false), "14:05");
    assert_eq!(en.format_time(time, true), "14:05:09");
    let us = DateLocale::for_tag("en-US");
    assert_eq!(us.format_time(time, false), "2:05 PM");
    assert_eq!(us.format_time(Time::new(0, 30, 0).unwrap(), false), "12:30 AM");
    for (text, read) in [
      ("14:05", Some(time.without_seconds())),
      ("2:05 pm", Some(time.without_seconds())),
      ("2:05 PM", Some(time.without_seconds())),
      ("14:05:09", Some(time)),
      ("9:30", Time::new(9, 30, 0)),
      ("12:00 am", Time::new(0, 0, 0)),
      ("14:5", None),
      ("25:00", None),
      ("2:05 xm", None),
    ] {
      assert_eq!(us.parse_time(text), read, "{text}");
    }
    assert_eq!(
      DateText::segment_placeholder(&DateLocale::for_tag("de"), DateSegment::Year),
      "JJJJ"
    );
    assert_eq!(
      DateText::segment_label(&DateLocale::for_tag("uk"), DateSegment::Month),
      "Місяць"
    );
    assert_eq!(DateText::month_short(&DateLocale::for_tag("uk"), 11), "Лис");
    assert_eq!(
      DateText::date_order(&DateLocale::for_tag("en-GB")),
      DateOrder::DayMonthYear
    );
    assert_eq!(
      DateText::choose_time_label(&DateLocale::for_tag("pl")),
      "Wybierz godzinę"
    );
  }

  #[test]
  fn the_traits_defaults_follow_from_its_other_answers() {
    struct Plain;
    impl DateText for Plain {
      fn first_day(&self) -> Weekday {
        Weekday::Sunday
      }
      fn month_name(&self, month: u8) -> String {
        format!("M{month}")
      }
      fn weekday_name(&self, day: Weekday) -> String {
        format!("{day:?}")
      }
      fn weekday_short(&self, day: Weekday) -> String {
        format!("{day:?}")[..1].to_owned()
      }
      fn format(&self, date: Date) -> String {
        date.to_string()
      }
    }
    let day = Date::new(2026, 11, 12).unwrap();
    assert_eq!(Plain.week()[0], Weekday::Sunday);
    assert_eq!(Plain.month_title(day.year_month()), "M11 2026");
    assert_eq!(Plain.day_label(day), "Thursday, 2026-11-12");
    assert_eq!(Plain.parse(" 2026-11-12 "), Some(day));
    assert_eq!(
      Plain.parse_range("2026-11-12 – 2026-11-15"),
      Some(DateRange::new(day, day.add_days(3)))
    );
    assert_eq!(Plain.next_month_label(), "Next month");
    let time = Time::new(9, 5, 0).unwrap();
    assert_eq!(Plain.format_time(time, false), "09:05");
    assert_eq!(Plain.parse_time("9:05"), Some(time));
    assert_eq!(Plain.month_short(11), "M11");
    assert_eq!(Plain.segment_placeholder(DateSegment::Year), "yyyy");
    assert_eq!(Plain.year_range_title(2020, 2031), "2020 – 2031");
    // Shared, it answers as it does.
    let shared: Arc<dyn DateText> = Arc::new(Plain);
    assert_eq!(shared.month_title(day.year_month()), "M11 2026");
  }

  fn date(year: i32, month: u8, day: u8) -> Date {
    Date::new(year, month, day).unwrap()
  }

  #[test]
  fn a_tag_is_its_locale_or_its_language_or_english() {
    assert_eq!(DateLocale::for_tag("en").first_day, Weekday::Monday);
    assert_eq!(DateLocale::for_tag("en_US").first_day, Weekday::Sunday);
    assert_eq!(DateLocale::for_tag("en-GB").order, DateOrder::DayMonthYear);
    assert_eq!(DateLocale::for_tag("uk-UA").weekdays_short[0].as_ref(), "Пн");
    assert_eq!(DateLocale::for_tag("xx").months[10].as_ref(), "November");
    assert_eq!(
      DateLocale::for_tag("en").week()[0],
      Weekday::Monday,
      "the first column is the first day"
    );
    assert_eq!(DateLocale::for_tag("en-US").week()[6], Weekday::Saturday);
  }

  #[test]
  fn a_date_is_written_in_the_short_form() {
    let day = date(2026, 11, 2);
    assert_eq!(DateLocale::for_tag("en").format(day), "Nov 2, 2026");
    assert_eq!(DateLocale::for_tag("en-GB").format(day), "2 Nov 2026");
    assert_eq!(DateLocale::for_tag("uk").format(day), "02.11.2026");
    assert_eq!(DateLocale::for_tag("fr").format(day), "02/11/2026");
    assert_eq!(DateLocale::for_tag("uk").month_title(day.year_month()), "Листопад 2026");
    assert_eq!(DateLocale::for_tag("en").day_label(day), "Monday, Nov 2, 2026");
  }

  #[test]
  fn a_range_writes_the_year_once_where_it_can() {
    let en = DateLocale::for_tag("en");
    let week = DateRange::new(date(2026, 11, 9), date(2026, 11, 15));
    assert_eq!(en.format_range(week), "Nov 9 – Nov 15, 2026");
    let across = DateRange::new(date(2026, 12, 28), date(2027, 1, 3));
    assert_eq!(en.format_range(across), "Dec 28, 2026 – Jan 3, 2027");
    assert_eq!(DateLocale::for_tag("de").format_range(week), "09.11.2026 – 15.11.2026");
    assert_eq!(en.parse_range("Nov 9 – Nov 15, 2026"), Some(week));
    assert_eq!(en.parse_range("2026-11-15 to 2026-11-09"), Some(week));
    assert_eq!(
      DateLocale::for_tag("en-GB").parse_range("9 Nov – 15 Nov 2026"),
      Some(week)
    );
    assert_eq!(en.parse_range("Nov 9"), None);
  }

  #[test]
  fn what_is_written_is_read_back() {
    let day = date(2026, 11, 12);
    for tag in ["en", "en-US", "en-GB", "uk", "de", "fr", "es", "pl"] {
      let locale = DateLocale::for_tag(tag);
      assert_eq!(locale.parse(&locale.format(day)), Some(day), "{tag}");
    }
  }

  #[test]
  fn a_date_is_read_in_more_than_one_form() {
    let day = date(2026, 11, 12);
    let en = DateLocale::for_tag("en");
    assert_eq!(en.parse("2026-11-12"), Some(day));
    assert_eq!(en.parse("11/12/2026"), Some(day));
    assert_eq!(en.parse("november 12 2026"), Some(day));
    assert_eq!(en.parse("12 Nov 2026"), Some(day));
    assert_eq!(DateLocale::for_tag("uk").parse("12 листопада 2026"), Some(day));
    assert_eq!(DateLocale::for_tag("de").parse("12.11.2026"), Some(day));
    assert_eq!(
      DateLocale::for_tag("uk").parse("12 Nov 2026"),
      Some(day),
      "English names are read anywhere"
    );
  }

  #[test]
  fn what_is_half_typed_or_wrong_is_no_date() {
    let en = DateLocale::for_tag("en");
    assert_eq!(en.parse(""), None);
    assert_eq!(en.parse("Nov 12, 20"), None);
    assert_eq!(en.parse("Nov 31, 2026"), None);
    assert_eq!(en.parse("13/12/2026"), None);
    assert_eq!(en.parse("Nov 12 2026 3"), None);
    assert_eq!(en.parse("Nob 12, 2026"), None);
  }

  #[test]
  fn the_words_of_an_event_calendar_are_the_locales() {
    let en = DateLocale::english();
    assert_eq!(DateText::weekday_medium(&en, Weekday::Thursday), "Thu");
    assert_eq!(DateText::today_label(&en), "Today");
    assert_eq!(DateText::previous_week_label(&en), "Previous week");
    assert_eq!(DateText::next_day_label(&en), "Next day");
    let uk = DateLocale::for_tag("uk");
    assert_eq!(DateText::weekday_medium(&uk, Weekday::Thursday), "Чт");
    assert_eq!(DateText::today_label(&uk), "Сьогодні");
    assert_eq!(DateText::previous_week_label(&uk), "Попередній тиждень");
    assert_eq!(DateText::next_week_label(&uk), "Наступний тиждень");
    assert_eq!(DateText::previous_day_label(&uk), "Попередній день");
    assert_eq!(DateText::next_day_label(&uk), "Наступний день");
    // Every language lurq has says them in its own words, and shared it says the same.
    for tag in ["de", "fr", "es", "pl"] {
      let locale = DateLocale::for_tag(tag);
      assert_ne!(DateText::today_label(&locale), "Today", "{tag}");
      assert_ne!(DateText::next_week_label(&locale), "Next week", "{tag}");
      let shared: Arc<dyn DateText> = Arc::new(locale.clone());
      assert_eq!(shared.today_label(), DateText::today_label(&locale), "{tag}");
      assert_eq!(
        shared.weekday_medium(Weekday::Monday),
        DateText::weekday_medium(&locale, Weekday::Monday),
        "{tag}"
      );
    }
  }
}
