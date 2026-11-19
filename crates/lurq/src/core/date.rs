//! A day of the calendar, with the arithmetic a calendar needs: the days of a month, the weekday of a date, adding
//! days and months, comparing, the ISO week.
//!
//! lurq keeps its own small type rather than a dependency: a calendar needs a year, a month and a day and nothing of
//! times or zones, and an app that keeps its dates in `chrono` or `time` converts at the edge with
//! [`Date::from_unix_days`] and [`Date::unix_days`] (or the year, the month and the day).

use std::{fmt, str::FromStr};

/// A day of the week.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Weekday {
  Monday,
  Tuesday,
  Wednesday,
  Thursday,
  Friday,
  Saturday,
  Sunday,
}

impl Weekday {
  /// Every day of the week, from Monday.
  pub const ALL: [Weekday; 7] = [
    Weekday::Monday,
    Weekday::Tuesday,
    Weekday::Wednesday,
    Weekday::Thursday,
    Weekday::Friday,
    Weekday::Saturday,
    Weekday::Sunday,
  ];

  /// How many days it is after Monday: 0 for Monday, 6 for Sunday.
  pub fn days_from_monday(self) -> u8 {
    self as u8
  }

  /// The day `days` after Monday, round the week.
  pub fn from_monday(days: i64) -> Self {
    Self::ALL[days.rem_euclid(7) as usize]
  }

  /// The day after it.
  pub fn next(self) -> Self {
    Self::from_monday(self as i64 + 1)
  }

  /// How many days it is after `first`, the first day of a week: 0 to 6.
  pub fn days_from(self, first: Weekday) -> u8 {
    (self as i64 - first as i64).rem_euclid(7) as u8
  }
}

impl crate::app::component::DevtoolsInspectable for Weekday {
  fn inspect(&self, formatter: &mut crate::app::component::DevtoolsFormatter<'_>) {
    formatter.value_debug(self);
  }
}

/// A day of the calendar: a year, a month and a day, with no time and no zone. Years 1 to 9999.
///
/// ```
/// use lurq::core::{Date, Weekday};
///
/// let day = Date::new(2026, 11, 12).unwrap();
/// assert_eq!(day.weekday(), Weekday::Thursday);
/// assert_eq!(day.add_days(20), Date::new(2026, 12, 2).unwrap());
/// assert_eq!(Date::new(2026, 1, 31).unwrap().add_months(1), Date::new(2026, 2, 28).unwrap());
/// assert_eq!(day.iso_week(), (2026, 46));
/// assert_eq!(day.to_string(), "2026-11-12");
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Date {
  year: i32,
  month: u8,
  day: u8,
}

/// The days from 1970-01-01 of a year, a month and a day, by the proleptic Gregorian calendar.
fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
  let year = if month <= 2 { year as i64 - 1 } else { year as i64 };
  let era = if year >= 0 { year } else { year - 399 } / 400;
  let year_of_era = year - era * 400;
  let from_march = (month as i64 + 9) % 12;
  let day_of_year = (153 * from_march + 2) / 5 + day as i64 - 1;
  let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
  era * 146_097 + day_of_era - 719_468
}

/// The year, the month and the day `days` from 1970-01-01.
fn civil_from_days(days: i64) -> (i32, u8, u8) {
  let days = days + 719_468;
  let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
  let day_of_era = days - era * 146_097;
  let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
  let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
  let from_march = (5 * day_of_year + 2) / 153;
  let day = day_of_year - (153 * from_march + 2) / 5 + 1;
  let month = if from_march < 10 {
    from_march + 3
  } else {
    from_march - 9
  };
  let year = year_of_era + era * 400 + if month <= 2 { 1 } else { 0 };
  (year as i32, month as u8, day as u8)
}

/// Whether `year` has a 29th of February.
pub fn is_leap_year(year: i32) -> bool {
  (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// How many days `month` (1 to 12) of `year` has: 0 for a month that is not one.
pub fn days_in_month(year: i32, month: u8) -> u8 {
  match month {
    1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
    4 | 6 | 9 | 11 => 30,
    2 if is_leap_year(year) => 29,
    2 => 28,
    _ => 0,
  }
}

impl Date {
  /// The first day there is: 1 January of the year 1.
  pub const MIN: Date = Date {
    year: 1,
    month: 1,
    day: 1,
  };
  /// The last day there is: 31 December 9999.
  pub const MAX: Date = Date {
    year: 9999,
    month: 12,
    day: 31,
  };

  /// The day `day` of `month` (1 to 12) of `year`, where there is one.
  pub fn new(year: i32, month: u8, day: u8) -> Option<Self> {
    let date = Self { year, month, day };
    ((1..=9999).contains(&year) && (1..=12).contains(&month) && day >= 1 && day <= days_in_month(year, month))
      .then_some(date)
  }

  /// The day `days` from 1 January 1970, kept between [`Date::MIN`] and [`Date::MAX`]: what a count of days of
  /// another library converts to.
  pub fn from_unix_days(days: i64) -> Self {
    let days = days.clamp(Self::MIN.unix_days(), Self::MAX.unix_days());
    let (year, month, day) = civil_from_days(days);
    Self { year, month, day }
  }

  /// How many days it is from 1 January 1970: negative before it.
  pub fn unix_days(self) -> i64 {
    days_from_civil(self.year, self.month as u32, self.day as u32)
  }

  /// Today where the app runs, by the clock and the zone of the system, or of the browser on the web. Where lurq
  /// cannot read the zone, today in UTC.
  pub fn today() -> Self {
    local_today().unwrap_or_else(|| {
      let seconds = crate::time::SystemTime::now()
        .duration_since(crate::time::UNIX_EPOCH)
        .map(|since| since.as_secs() as i64)
        .unwrap_or(0);
      Self::from_unix_days(seconds.div_euclid(86_400))
    })
  }

  pub fn year(self) -> i32 {
    self.year
  }

  /// Its month, 1 to 12.
  pub fn month(self) -> u8 {
    self.month
  }

  /// Its day of the month, from 1.
  pub fn day(self) -> u8 {
    self.day
  }

  /// The day of the week it is.
  pub fn weekday(self) -> Weekday {
    // 1 January 1970 was a Thursday.
    Weekday::from_monday(self.unix_days() + 3)
  }

  /// Which day of its year it is, from 1.
  pub fn ordinal(self) -> u16 {
    (self.unix_days() - days_from_civil(self.year, 1, 1) + 1) as u16
  }

  /// The day `days` after it (before it where negative), kept between [`Date::MIN`] and [`Date::MAX`].
  pub fn add_days(self, days: i64) -> Self {
    Self::from_unix_days(self.unix_days().saturating_add(days))
  }

  /// The same day `months` months after it (before it where negative), or the last day of that month where it is
  /// shorter: a month after 31 January is 28 or 29 February. Kept between [`Date::MIN`] and [`Date::MAX`].
  pub fn add_months(self, months: i32) -> Self {
    let month = YearMonth::of(self).add_months(months);
    Self {
      year: month.year,
      month: month.month,
      day: self.day.min(month.days()),
    }
  }

  /// The same day `years` years after it, or 28 February for a 29th in a year that has none.
  pub fn add_years(self, years: i32) -> Self {
    self.add_months(years.saturating_mul(12))
  }

  /// How many days it is after `other`: negative where it is before it.
  pub fn days_since(self, other: Date) -> i64 {
    self.unix_days() - other.unix_days()
  }

  /// The month it is in.
  pub fn year_month(self) -> YearMonth {
    YearMonth::of(self)
  }

  /// The first day of the week it is in, where weeks start on `first`.
  pub fn start_of_week(self, first: Weekday) -> Self {
    self.add_days(-(self.weekday().days_from(first) as i64))
  }

  /// The last day of the week it is in, where weeks start on `first`.
  pub fn end_of_week(self, first: Weekday) -> Self {
    self.start_of_week(first).add_days(6)
  }

  /// Its ISO 8601 week: the year the week is of, and its number, 1 to 53. A week is of the year its Thursday is in,
  /// so 1 January may be in the last week of the year before.
  pub fn iso_week(self) -> (i32, u8) {
    let thursday = self.add_days(3 - self.weekday().days_from_monday() as i64);
    (thursday.year, ((thursday.ordinal() - 1) / 7 + 1) as u8)
  }

  /// It, kept between `min` and `max` where they are given.
  pub fn clamp_to(self, min: Option<Date>, max: Option<Date>) -> Self {
    let date = min.map_or(self, |min| self.max(min));
    max.map_or(date, |max| date.min(max))
  }
}

impl fmt::Debug for Date {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "Date({self})")
  }
}

/// ISO 8601: `2026-11-12`.
impl fmt::Display for Date {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
  }
}

/// What does not read as a date.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseDateError;

impl fmt::Display for ParseDateError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("not a date of the form YYYY-MM-DD")
  }
}

impl std::error::Error for ParseDateError {}

/// ISO 8601: `2026-11-12`.
impl FromStr for Date {
  type Err = ParseDateError;

  fn from_str(text: &str) -> Result<Self, Self::Err> {
    let mut parts = text.trim().splitn(3, '-');
    let mut number = |digits: usize| {
      parts
        .next()
        .filter(|part| part.len() == digits && part.bytes().all(|byte| byte.is_ascii_digit()))
        .and_then(|part| part.parse::<i32>().ok())
    };
    let (Some(year), Some(month), Some(day)) = (number(4), number(2), number(2)) else {
      return Err(ParseDateError);
    };
    Date::new(year, month as u8, day as u8).ok_or(ParseDateError)
  }
}

impl crate::app::component::DevtoolsInspectable for Date {
  fn inspect(&self, formatter: &mut crate::app::component::DevtoolsFormatter<'_>) {
    formatter.value_display(self);
  }
}

/// A month of a year: what a calendar shows.
///
/// ```
/// use lurq::core::{Date, YearMonth};
///
/// let november = YearMonth::new(2026, 11).unwrap();
/// assert_eq!(november.days(), 30);
/// assert_eq!(november.add_months(2), YearMonth::new(2027, 1).unwrap());
/// assert!(november.contains(Date::new(2026, 11, 12).unwrap()));
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct YearMonth {
  year: i32,
  month: u8,
}

impl YearMonth {
  /// `month` (1 to 12) of `year`, where there is one.
  pub fn new(year: i32, month: u8) -> Option<Self> {
    ((1..=9999).contains(&year) && (1..=12).contains(&month)).then_some(Self { year, month })
  }

  /// The month `date` is in.
  pub fn of(date: Date) -> Self {
    Self {
      year: date.year,
      month: date.month,
    }
  }

  /// The month of today.
  pub fn current() -> Self {
    Self::of(Date::today())
  }

  pub fn year(self) -> i32 {
    self.year
  }

  /// Its month, 1 to 12.
  pub fn month(self) -> u8 {
    self.month
  }

  /// How many days it has.
  pub fn days(self) -> u8 {
    days_in_month(self.year, self.month)
  }

  /// Its first day.
  pub fn first(self) -> Date {
    Date {
      year: self.year,
      month: self.month,
      day: 1,
    }
  }

  /// Its last day.
  pub fn last(self) -> Date {
    Date {
      year: self.year,
      month: self.month,
      day: self.days(),
    }
  }

  /// Whether `date` is one of its days.
  pub fn contains(self, date: Date) -> bool {
    date.year == self.year && date.month == self.month
  }

  /// The month `months` after it (before it where negative), kept between the years 1 and 9999.
  pub fn add_months(self, months: i32) -> Self {
    let index = (self.year as i64 * 12 + self.month as i64 - 1 + months as i64).clamp(12, 9999 * 12 + 11);
    Self {
      year: (index / 12) as i32,
      month: (index % 12 + 1) as u8,
    }
  }

  /// How many months it is after `other`: negative where it is before it.
  pub fn months_since(self, other: YearMonth) -> i32 {
    (self.year - other.year) * 12 + self.month as i32 - other.month as i32
  }

  /// The weeks a calendar shows of it, each of seven days from `first`: six of them, from the week of its first
  /// day, so that every month is as high as another. Days of the months before and after fill the first and the
  /// last.
  pub fn weeks(self, first: Weekday) -> Vec<[Date; 7]> {
    let start = self.first().start_of_week(first);
    (0..6)
      .map(|week| std::array::from_fn(|day| start.add_days(week * 7 + day as i64)))
      .collect()
  }
}

impl fmt::Debug for YearMonth {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "YearMonth({self})")
  }
}

/// ISO 8601: `2026-11`.
impl fmt::Display for YearMonth {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{:04}-{:02}", self.year, self.month)
  }
}

impl crate::app::component::DevtoolsInspectable for YearMonth {
  fn inspect(&self, formatter: &mut crate::app::component::DevtoolsFormatter<'_>) {
    formatter.value_display(self);
  }
}

/// Days from one date to another, both of them in it.
///
/// ```
/// use lurq::core::{Date, DateRange};
///
/// let day = |d| Date::new(2026, 11, d).unwrap();
/// let week = DateRange::new(day(15), day(9));
/// assert_eq!((week.start(), week.end()), (day(9), day(15)));
/// assert_eq!(week.days(), 7);
/// assert!(week.contains(day(12)));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DateRange {
  start: Date,
  end: Date,
}

impl DateRange {
  /// The days from `a` to `b`, whichever is the earlier.
  pub fn new(a: Date, b: Date) -> Self {
    Self {
      start: a.min(b),
      end: a.max(b),
    }
  }

  /// One day.
  pub fn day(date: Date) -> Self {
    Self { start: date, end: date }
  }

  /// The first day.
  pub fn start(self) -> Date {
    self.start
  }

  /// The last day.
  pub fn end(self) -> Date {
    self.end
  }

  /// How many days it holds, the ends with them.
  pub fn days(self) -> i64 {
    self.end.days_since(self.start) + 1
  }

  pub fn contains(self, date: Date) -> bool {
    self.start <= date && date <= self.end
  }
}

/// ISO 8601: `2026-11-09/2026-11-15`.
impl fmt::Display for DateRange {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}/{}", self.start, self.end)
  }
}

impl crate::app::component::DevtoolsInspectable for DateRange {
  fn inspect(&self, formatter: &mut crate::app::component::DevtoolsFormatter<'_>) {
    formatter.value_display(self);
  }
}

/// Today by the system's clock and zone, where lurq can read the zone.
fn local_today() -> Option<Date> {
  local_now().map(|(date, _)| date)
}

/// The seconds since midnight now by the system's clock and zone, where lurq can read the zone: what
/// [`Time::now`](super::Time::now) is.
pub(crate) fn local_now_seconds() -> Option<i64> {
  local_now().map(|(_, seconds)| seconds)
}

/// Now by the system's clock and zone: the date and the seconds since its midnight.
#[cfg(windows)]
fn local_now() -> Option<(Date, i64)> {
  #[repr(C)]
  struct SystemTime {
    year: u16,
    month: u16,
    day_of_week: u16,
    day: u16,
    hour: u16,
    minute: u16,
    second: u16,
    milliseconds: u16,
  }
  #[link(name = "kernel32")]
  unsafe extern "system" {
    fn GetLocalTime(time: *mut SystemTime);
  }
  let mut time = SystemTime {
    year: 0,
    month: 0,
    day_of_week: 0,
    day: 0,
    hour: 0,
    minute: 0,
    second: 0,
    milliseconds: 0,
  };
  // SAFETY: GetLocalTime writes the eight fields of the struct it is given, which is laid out as SYSTEMTIME is.
  unsafe { GetLocalTime(&mut time) };
  let seconds = time.hour as i64 * 3600 + time.minute as i64 * 60 + time.second as i64;
  Date::new(time.year as i32, time.month as u8, time.day as u8).map(|date| (date, seconds))
}

/// Now by the system's clock and zone: the date and the seconds since its midnight.
#[cfg(all(unix, not(target_arch = "wasm32")))]
fn local_now() -> Option<(Date, i64)> {
  use std::os::raw::{c_int, c_long};
  unsafe extern "C" {
    fn time(out: *mut c_long) -> c_long;
    fn localtime_r(time: *const c_long, out: *mut c_int) -> *mut c_int;
  }
  // `struct tm` starts with nine ints (second, minute, hour, day, month from 0, year from 1900, ...); what a
  // platform has after them fits in the rest.
  let mut tm = [0 as c_int; 32];
  // SAFETY: `time` with a null pointer only returns the time; `localtime_r` writes one `struct tm`, smaller than
  // the buffer, and returns null where it fails.
  let written = unsafe {
    let now = time(std::ptr::null_mut());
    localtime_r(&now, tm.as_mut_ptr())
  };
  if written.is_null() {
    return None;
  }
  let seconds = tm[2] as i64 * 3600 + tm[1] as i64 * 60 + tm[0].min(59) as i64;
  Date::new(tm[5] + 1900, (tm[4] + 1) as u8, tm[3] as u8).map(|date| (date, seconds))
}

/// Now by the browser's clock and zone: the date and the seconds since its midnight.
#[cfg(target_arch = "wasm32")]
fn local_now() -> Option<(Date, i64)> {
  let now = js_sys::Date::new_0();
  let seconds = now.get_hours() as i64 * 3600 + now.get_minutes() as i64 * 60 + now.get_seconds() as i64;
  Date::new(
    now.get_full_year() as i32,
    (now.get_month() + 1) as u8,
    now.get_date() as u8,
  )
  .map(|date| (date, seconds))
}

#[cfg(not(any(windows, unix, target_arch = "wasm32")))]
fn local_now() -> Option<(Date, i64)> {
  None
}

#[cfg(test)]
mod tests {
  use super::*;

  fn date(year: i32, month: u8, day: u8) -> Date {
    Date::new(year, month, day).unwrap()
  }

  #[test]
  fn a_date_is_only_one_the_calendar_has() {
    assert!(Date::new(2026, 2, 29).is_none());
    assert!(Date::new(2024, 2, 29).is_some());
    assert!(Date::new(1900, 2, 29).is_none(), "a century is not a leap year");
    assert!(Date::new(2000, 2, 29).is_some(), "unless it is a fourth one");
    assert!(Date::new(2026, 13, 1).is_none());
    assert!(Date::new(2026, 4, 31).is_none());
    assert!(Date::new(0, 1, 1).is_none());
    assert!(Date::new(2026, 1, 0).is_none());
  }

  #[test]
  fn days_from_1970_go_both_ways() {
    assert_eq!(date(1970, 1, 1).unix_days(), 0);
    assert_eq!(date(2000, 3, 1).unix_days(), 11_017);
    assert_eq!(date(1969, 12, 31).unix_days(), -1);
    for days in [-719_162, -1, 0, 1, 10_957, 20_769, 2_932_896] {
      assert_eq!(Date::from_unix_days(days).unix_days(), days);
    }
    assert_eq!(Date::from_unix_days(i64::MIN), Date::MIN);
    assert_eq!(Date::from_unix_days(i64::MAX), Date::MAX);
  }

  #[test]
  fn the_weekday_of_a_date() {
    assert_eq!(date(1970, 1, 1).weekday(), Weekday::Thursday);
    assert_eq!(date(2026, 11, 12).weekday(), Weekday::Thursday);
    assert_eq!(date(2026, 11, 1).weekday(), Weekday::Sunday);
    assert_eq!(date(2000, 1, 1).weekday(), Weekday::Saturday);
    assert_eq!(Weekday::Sunday.days_from(Weekday::Monday), 6);
    assert_eq!(Weekday::Monday.days_from(Weekday::Sunday), 1);
  }

  #[test]
  fn adding_days_months_and_years() {
    assert_eq!(date(2026, 12, 31).add_days(1), date(2027, 1, 1));
    assert_eq!(date(2026, 3, 1).add_days(-1), date(2026, 2, 28));
    assert_eq!(date(2026, 1, 31).add_months(1), date(2026, 2, 28));
    assert_eq!(date(2024, 1, 31).add_months(1), date(2024, 2, 29));
    assert_eq!(date(2026, 11, 12).add_months(-11), date(2025, 12, 12));
    assert_eq!(date(2024, 2, 29).add_years(1), date(2025, 2, 28));
    assert_eq!(Date::MAX.add_days(1), Date::MAX);
    assert_eq!(Date::MIN.add_months(-1), Date::MIN);
  }

  #[test]
  fn the_iso_week() {
    assert_eq!(date(2026, 11, 12).iso_week(), (2026, 46));
    // 1 January 2021 was a Friday: in the last week of 2020, the 53rd.
    assert_eq!(date(2021, 1, 1).iso_week(), (2020, 53));
    // 29 December 2025 is a Monday whose Thursday is 1 January 2026.
    assert_eq!(date(2025, 12, 29).iso_week(), (2026, 1));
    assert_eq!(date(2026, 1, 4).iso_week(), (2026, 1));
    assert_eq!(date(2026, 1, 5).iso_week(), (2026, 2));
  }

  #[test]
  fn the_weeks_of_a_month() {
    let november = YearMonth::new(2026, 11).unwrap();
    let weeks = november.weeks(Weekday::Monday);
    assert_eq!(weeks.len(), 6);
    assert_eq!(weeks[0][0], date(2026, 10, 26));
    assert_eq!(weeks[0][6], date(2026, 11, 1));
    assert_eq!(weeks[5][6], date(2026, 12, 6));
    let sunday = november.weeks(Weekday::Sunday);
    assert_eq!(
      sunday[0][0],
      date(2026, 11, 1),
      "a month that starts on the first day starts its first week"
    );
    assert_eq!(date(2026, 11, 12).start_of_week(Weekday::Monday), date(2026, 11, 9));
    assert_eq!(date(2026, 11, 12).end_of_week(Weekday::Sunday), date(2026, 11, 14));
  }

  #[test]
  fn months_count_and_wrap() {
    let november = YearMonth::new(2026, 11).unwrap();
    assert_eq!(november.add_months(2), YearMonth::new(2027, 1).unwrap());
    assert_eq!(november.add_months(-11), YearMonth::new(2025, 12).unwrap());
    assert_eq!(YearMonth::new(2027, 1).unwrap().months_since(november), 2);
    assert_eq!(november.last(), date(2026, 11, 30));
    assert_eq!(YearMonth::new(2024, 2).unwrap().days(), 29);
  }

  #[test]
  fn iso_text_goes_both_ways() {
    assert_eq!(date(2026, 11, 2).to_string(), "2026-11-02");
    assert_eq!("2026-11-02".parse::<Date>(), Ok(date(2026, 11, 2)));
    assert!("2026-2-30".parse::<Date>().is_err());
    assert!("2026-02-30".parse::<Date>().is_err());
    assert!("26-02-03".parse::<Date>().is_err());
  }

  #[test]
  fn a_range_is_in_order() {
    let range = DateRange::new(date(2026, 11, 15), date(2026, 11, 9));
    assert_eq!(range.start(), date(2026, 11, 9));
    assert_eq!(range.days(), 7);
    assert!(range.contains(date(2026, 11, 15)));
    assert!(!range.contains(date(2026, 11, 16)));
    assert_eq!(DateRange::day(date(2026, 1, 1)).days(), 1);
  }

  #[test]
  fn today_is_a_date() {
    let today = Date::today();
    assert!(today.year() >= 2024);
  }
}
