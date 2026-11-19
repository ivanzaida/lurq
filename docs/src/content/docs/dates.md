---
title: Dates
description: How an app's dates are written and read, set once on the app; lurq's own dates, months and ranges, and their arithmetic.
---

# Dates

## How an app writes its dates

Every calendar and date picker of an app asks one thing how a date's text is: the day weeks start on, the names of the months and the days, how a field writes a date and reads what is typed, and what its buttons are called. The app says it once, and it is English until it does:

```rust
use lurq::{app::App, core::DateLocale};

let app = App::new();
app.set_date_locale(DateLocale::for_tag("uk"));
```

`app.date_locale()` reads it. A mounted calendar or picker, and one built in a component's render, is built again when it changes, as it is when the theme changes. The system's language and the [i18n](../i18n/) locale do not change it: an app says what it wants. One that writes dates in its i18n locale sets both where it changes that locale:

```rust
app.i18n().set_locale(tag);
app.set_date_locale(DateLocale::for_tag(tag));
```

A component's own `.locale(..)` is above the app's: `Calendar::new(due).locale(DateLocale::for_tag("de"))`. A component built outside any render (in a test, say) has no app to ask, and is in English unless it is given one.

### Writing dates your way

A date written another way needs no code of the app's own: a [`DateLocale`](#datelocale) has a style, a separator and closures.

```rust
use lurq::core::{DateLocale, DateStyle};

app.set_date_locale(DateLocale::for_tag("de"));                                    // 12.11.2026
app.set_date_locale(DateLocale::for_tag("en"));                                    // Nov 12, 2026
app.set_date_locale(DateLocale::for_tag("en-GB").with_style(DateStyle::Long));     // 12 November 2026
app.set_date_locale(DateLocale::for_tag("en").with_style(DateStyle::Iso));         // 2026-11-12
app.set_date_locale(DateLocale::for_tag("en").with_style(DateStyle::Numeric).with_separator('-')); // 11-12-2026

// A form no style is, with a year of two digits: 12/11/26. The app writes it, and reads it back.
app.set_date_locale(
  DateLocale::for_tag("en-GB")
    .with_format(|date| format!("{}/{:02}/{:02}", date.day(), date.month(), date.year() % 100))
    .with_parse(|text| read_short_year(text)),
);
```

Four styles are the least that covers what is asked of a date's form: its numbers in the locale's order (`DateStyle::Numeric`), the month by its short name (`Short`) or its whole name (`Long`), and ISO 8601 (`Iso`). The order of the day, the month and the year is the locale's, so the four cover `12.11.2026`, `11/12/2026`, `Nov 12, 2026`, `12 November 2026` and `2026-11-12`. Anything else is a closure: `with_format` writes a date (a range is two of them with an en dash between), and `with_parse` reads what is typed in place of the locale's forms; give both where the locale would not read back what the format writes. Whatever the style, what is typed is read in any of the locale's forms and as ISO 8601. `with_first_day(Weekday::Sunday)` starts the weeks on another day, and `with_labels(..)` names the controls.

### Plugging in your own

Where the tables of `DateLocale` are not the app's (its own translations, `chrono`, ICU), the app implements `DateText`, the questions a calendar and a picker ask, and gives it as it would give a `DateLocale`, or as an `Arc` of it:

```rust
use std::sync::Arc;
use lurq::core::{Date, DateText, Weekday};

struct Catalog { /* the app's translations */ }

impl DateText for Catalog {
  fn first_day(&self) -> Weekday { Weekday::Monday }
  fn month_name(&self, month: u8) -> String { self.t(&format!("month.{month}")) }
  fn weekday_name(&self, day: Weekday) -> String { self.t(&format!("day.{day:?}")) }
  fn weekday_short(&self, day: Weekday) -> String { self.t(&format!("day.short.{day:?}")) }
  fn format(&self, date: Date) -> String { self.write(date) }
  fn parse(&self, text: &str) -> Option<Date> { self.read(text) }
  fn previous_month_label(&self) -> String { self.t("calendar.previous") }
}

app.set_date_locale(Arc::new(Catalog::load()));
```

| `DateText` | Default |
| --- | --- |
| `first_day()` | none: the first column of a calendar |
| `week()` | the seven days from `first_day` |
| `month_name(month)`, `month_title(year_month)` | none; `month_title` is the name and the year: `November 2026` |
| `weekday_name(day)`, `weekday_short(day)` | none: `Thursday`, and the head of its column, `Th` |
| `weekday_medium(day)` | the first three letters of `weekday_name`: what a day of an event calendar says before its number, `Thu` |
| `format(date)`, `format_range(range)` | none; a range is two dates with an en dash between |
| `day_label(date)` | the weekday's name and the date: `Thursday, Nov 12, 2026`, what a day is called to tooling |
| `parse(text)`, `parse_range(text)` | ISO 8601; a range is two dates with a dash or `to` between |
| `previous_month_label()`, `next_month_label()`, `choose_date_label()`, `choose_dates_label()`, `clear_label()` | English: `Previous month`, `Next month`, `Choose date`, `Choose dates`, `Clear` |
| `hour_cycle()`, `day_period_name(period)` | `HourCycle::H23`; `AM` and `PM` |
| `format_time(time, seconds)`, `parse_time(text)` | `14:30` (or `2:30 PM` on a clock of twelve hours), and back: hours and minutes and perhaps seconds, and a half of the day by its name or `am`/`pm` |
| `date_order()`, `date_separator()`, `time_separator()` | the order and what stands between the segments of a date field (`MonthDayYear`, `/`) and of a time field (`:`) |
| `month_short(month)` | the first three letters of `month_name`: a month picker's cell |
| `segment_label(segment)`, `segment_placeholder(segment)` | what a segment of a field is called to tooling (`Day`, `Hour`) and what an empty one shows (`dd`, `mm`, `yyyy`, `hh`, `mm`, `ss`, `--`) |
| `previous_year_label()`, `next_year_label()`, `previous_years_label()`, `next_years_label()`, `choose_time_label()` | English: `Previous year`, `Next year`, `Previous years`, `Next years`, `Choose time` |
| `year_range_title(first, last)` | a year picker's heading: `2020 – 2031` |
| `previous_week_label()`, `next_week_label()`, `previous_day_label()`, `next_day_label()`, `today_label()` | English: `Previous week`, `Next week`, `Previous day`, `Next day`, `Today`: the buttons of an event calendar's header |

The names of the controls are words lurq writes itself, not dates, and are in `DateText` so that one thing gives all of a calendar's text; a component's `previous_label`, `next_label`, `trigger_label`, `clear_label` and `panel_label` are above them. `DateLocale` gives them in its language.

## The time of day

`core::Time` is a time of day with no date and no zone: what a time field and a time picker hold.

```rust
use lurq::core::{DayPeriod, Time};

let time = Time::new(14, 30, 0).unwrap();
assert_eq!(time.to_string(), "14:30:00");
assert_eq!("14:30".parse::<Time>(), Ok(time));
assert_eq!(time.hour12(), (2, DayPeriod::Pm));
assert_eq!(time.add_minutes(45), Time::new(15, 15, 0).unwrap());
```

| `Time` | |
| --- | --- |
| `new(hour, minute, second)` | `None` for what is no time: hours 0 to 23, minutes and seconds 0 to 59 |
| `from_hour12(hour, period, minute, second)`, `hour12()`, `period()` | On a clock of twelve hours: 12:30 AM is `00:30` |
| `now()` | Now by the system's clock and zone; in the browser, and wherever lurq cannot read the zone, in UTC |
| `hour()`, `minute()`, `second()`, `seconds()` | Its parts, and the seconds since midnight |
| `add_minutes(n)`, `add_seconds(n)`, `from_seconds(n)` | Round the clock: 20 minutes after 23:50 is 00:10 |
| `without_seconds()`, `clamp_to(min, max)` | |
| `Time::MIDNIGHT`, `Time::MAX` | `00:00:00` and `23:59:59` |

Times are ordered, hashed and copied, and written and read as ISO 8601 (`14:30` or `14:30:00`). `HourCycle` says how a locale counts the hours (`H23`, `H12`), and `DayPeriod` the halves of the day (`Am`, `Pm`). A `DateLocale` writes and reads times as its language does: `en-US` on a clock of twelve hours, the others on one of 24.

## The date type

`lurq::core` has a date of its own, with no time and no zone: what a calendar, a date picker and an app's due dates and bookings hold. It needs no feature and no other library.

```rust
use lurq::core::{Date, DateRange, Weekday, YearMonth};

let day = Date::new(2026, 11, 12).unwrap();
assert_eq!(day.weekday(), Weekday::Thursday);
assert_eq!(day.add_days(20), Date::new(2026, 12, 2).unwrap());
assert_eq!(day.to_string(), "2026-11-12");
assert_eq!("2026-11-12".parse::<Date>(), Ok(day));

let november = YearMonth::of(day);
assert_eq!(november.days(), 30);
assert_eq!(november.weeks(Weekday::Monday).len(), 6);

let week = DateRange::new(day.add_days(3), day);
assert_eq!(week.start(), day);
assert_eq!(week.days(), 4);
```

### Date

A year, a month and a day of the Gregorian calendar, years 1 to 9999 (`Date::MIN` to `Date::MAX`). `Date::new(year, month, day)` is `None` for a day that does not exist, such as 30 February. Dates are ordered, hashed and copied. They are written and read as ISO 8601 by `Display` and `FromStr`.

| `Date` | |
| --- | --- |
| `today()` | Today, by the clock and the zone of the system, or of the browser on the web. Wherever lurq cannot read the zone, today in UTC |
| `year()`, `month()`, `day()` | Its parts. The month is 1 to 12 |
| `weekday()`, `ordinal()` | Its day of the week, and its day of the year, from 1 |
| `add_days(n)`, `add_months(n)`, `add_years(n)` | A day after it, or before it where `n` is negative. A month after 31 January is the last day of February |
| `days_since(other)` | How many days it is after `other`, negative where it is before |
| `year_month()` | The month it is in |
| `start_of_week(first)`, `end_of_week(first)` | The ends of its week, where weeks start on `first` |
| `iso_week()` | Its ISO 8601 week: the year the week is of and its number, 1 to 53. A week is of the year its Thursday is in |
| `clamp_to(min, max)` | It, kept between the dates given |
| `from_unix_days(n)`, `unix_days()` | The count of days from 1 January 1970: what another library's date converts through |

The results of the arithmetic stay between `Date::MIN` and `Date::MAX`.

An app that keeps its dates in `chrono` or `time` converts them at the edge, by their year, month and day or by their days from 1970:

```rust
use lurq::core::Date;

// chrono's NaiveDate counts its days from the year 1.
fn from_chrono(date: chrono::NaiveDate) -> Date {
  Date::from_unix_days(date.num_days_from_ce() as i64 - 719_163)
}
```

`is_leap_year(year)` and `days_in_month(year, month)` are beside it.

### YearMonth

A month of a year: the month a calendar shows. `YearMonth::new(year, month)`, `YearMonth::of(date)` and `YearMonth::current()` make one. It has `days()`, its `first()` and `last()` dates, `contains(date)`, `add_months(n)` and `months_since(other)`. `weeks(first)` gives the six weeks a calendar draws it in, from the week its first day is in, each of seven dates from `first`: the days before and after it are of the months beside it. It is written `2026-11`.

### DateRange

Two dates and the days between them, both ends in it. `DateRange::new(a, b)` orders them, so either may come first, and `DateRange::day(date)` is a range of one day. It has `start()`, `end()`, `days()`, the count of its days with both ends, and `contains(date)`. It is written `2026-11-09/2026-11-15`, as ISO 8601 writes an interval.

### Weekday

`Weekday::Monday` to `Weekday::Sunday`, and `Weekday::ALL` from Monday. `days_from_monday()` gives 0 to 6, `from_monday(n)` goes the other way and wraps, `next()` is the day after, and `days_from(first)` is how far it is from the first day of a week that starts on `first`.

## DateLocale

How a locale writes dates and reads them back: the day its weeks start on, the names of its months and days, and the short form a field shows.

```rust
use lurq::core::{Date, DateLocale, Weekday, YearMonth};

let day = Date::new(2026, 11, 12).unwrap();
let english = DateLocale::for_tag("en");
assert_eq!(english.format(day), "Nov 12, 2026");
assert_eq!(english.month_title(YearMonth::of(day)), "November 2026");
assert_eq!(english.day_label(day), "Thursday, Nov 12, 2026");
assert_eq!(english.parse("November 12, 2026"), Some(day));
assert_eq!(english.parse("2026-11-12"), Some(day));

let ukrainian = DateLocale::for_tag("uk");
assert_eq!(ukrainian.format(day), "12.11.2026");
assert_eq!(ukrainian.month_title(YearMonth::of(day)), "Листопад 2026");

// English names, with weeks from Sunday.
let own = DateLocale::for_tag("en").with_first_day(Weekday::Sunday);
```

`DateLocale::for_tag(tag)` knows these tags; any other is English:

| Tag | Weeks from | Writes |
| --- | --- | --- |
| `en` | Monday | `Nov 12, 2026` |
| `en-US` (and `en-CA`, `en-PH`) | Sunday | `Nov 12, 2026` |
| `en-GB` (and `en-IE`, `en-AU`, `en-NZ`, `en-IN`, `en-ZA`) | Monday | `12 Nov 2026` |
| `uk`, `de`, `pl` | Monday | `12.11.2026` |
| `fr`, `es` | Monday | `12/11/2026` |

Plain `en` starts its weeks on Monday, as lurq's design draws its calendar. Each names a calendar's controls in its language: `Попередній місяць`, `Vorheriger Monat`.

- `format(date)` writes the date in the locale's style, and `format_range(range)` writes a range with an en dash between its ends: `Nov 9 – Nov 15, 2026`, with the year once where both ends are in it.
- `parse(text)` reads the short form, numbers in the locale's order with any separator, and ISO 8601. A month may be written by name or by the start of its name, in the locale's words or in English. The year must have four digits, so what is still being typed (`12.11.20`) is no date yet. `parse_range(text)` reads two dates with a dash or `to` between them, and the first may leave out its year.
- `month_name(month)` and `month_title(year_month)` give what a calendar's heading says, starting with a capital. `day_label(date)` gives what a day is called to what reads the tree. `week()` gives the seven days from the first.

Every field is public, so a locale lurq does not know can be written out: `tag`, `first_day`, `months`, `months_short`, `weekdays`, `weekdays_short`, `weekdays_medium` (the days as an event calendar's day has them: `Mon`), `order` (a `DateOrder`), `style` (a `DateStyle`), `separator` (the `char` between numbers), `formatter` and `parser` (the closures of `with_format` and `with_parse`) and `labels` (a `DateLabels`).

`DateLocale` is lurq's `DateText`: `ui::Calendar`, `ui::DatePicker` and `ui::DateRangePicker` take it, or the app's own, with `.locale(..)`, and the app takes it with `set_date_locale`. See [the design system](../design-system/#calendar).
