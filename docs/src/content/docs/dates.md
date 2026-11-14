---
title: Dates
description: lurq's own dates, months and ranges, their arithmetic, and how a locale writes and reads them.
---

# Dates

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

## Date

A year, a month and a day of the Gregorian calendar, years 1 to 9999 (`Date::MIN` to `Date::MAX`). `Date::new(year, month, day)` is `None` for a day that does not exist, such as 30 February. Dates are ordered, hashed and copied. They are written and read as ISO 8601 by `Display` and `FromStr`.

| `Date` | |
| --- | --- |
| `today()` | Today, by the clock and the zone of the system. In the browser, and wherever lurq cannot read the zone, today in UTC |
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

## YearMonth

A month of a year: the month a calendar shows. `YearMonth::new(year, month)`, `YearMonth::of(date)` and `YearMonth::current()` make one. It has `days()`, its `first()` and `last()` dates, `contains(date)`, `add_months(n)` and `months_since(other)`. `weeks(first)` gives the six weeks a calendar draws it in, from the week its first day is in, each of seven dates from `first`: the days before and after it are of the months beside it. It is written `2026-11`.

## DateRange

Two dates and the days between them, both ends in it. `DateRange::new(a, b)` orders them, so either may come first, and `DateRange::day(date)` is a range of one day. It has `start()`, `end()`, `days()`, the count of its days with both ends, and `contains(date)`. It is written `2026-11-09/2026-11-15`, as ISO 8601 writes an interval.

## Weekday

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

Plain `en` starts its weeks on Monday, as lurq's design draws its calendar.

- `format(date)` writes the short form, and `format_range(range)` writes a range with an en dash between its ends: `Nov 9 – Nov 15, 2026`, with the year once where both ends are in it.
- `parse(text)` reads the short form, numbers in the locale's order with any separator, and ISO 8601. A month may be written by name or by the start of its name, in the locale's words or in English. The year must have four digits, so what is still being typed (`12.11.20`) is no date yet. `parse_range(text)` reads two dates with a dash or `to` between them, and the first may leave out its year.
- `month_name(month)` and `month_title(year_month)` give what a calendar's heading says, starting with a capital. `day_label(date)` gives what a day is called to what reads the tree. `week()` gives the seven days from the first.

Every field is public, so a locale lurq does not know can be written out: `tag`, `first_day`, `months`, `months_short`, `weekdays`, `weekdays_short`, `order` (a `DateOrder`) and `separator` (a `char` between numbers, or `None` where the month is written by name).

`ui::Calendar`, `ui::DatePicker` and `ui::DateRangePicker` take a locale with `.locale(..)`. When mounted, they use the app's [i18n](../i18n/) locale (`app.i18n().locale()`) and are built again when it changes. Without `ctx` and with no locale given, they are in English. See [the design system](../design-system/#calendar).
