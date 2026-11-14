//! What a calendar takes of a locale: the day weeks start on, the names of months and days, and how a date is
//! written and read.

use std::sync::Arc;

use super::date::{Date, DateRange, Weekday, YearMonth};

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
/// months and of the days, and the short form a field shows.
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
/// ```
///
/// lurq knows English (`en`, with weeks from Monday as the design draws them; `en-US` from Sunday; `en-GB` writing
/// the day first), Ukrainian (`uk`), German (`de`), French (`fr`), Spanish (`es`) and Polish (`pl`); any other tag is
/// English. Every field is the app's to replace.
#[derive(Clone, Debug, PartialEq, Eq)]
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
  /// The order of the day, the month and the year.
  pub order: DateOrder,
  /// What is between the numbers of a date written in numbers: `.` in `12.11.2026`. `None` where the short form
  /// writes the month's name: `Nov 12, 2026`.
  pub separator: Option<char>,
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
      order: DateOrder::MonthDayYear,
      separator: None,
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
        order: DateOrder::DayMonthYear,
        separator: Some('.'),
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
        order: DateOrder::DayMonthYear,
        separator: Some('.'),
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
        order: DateOrder::DayMonthYear,
        separator: Some('/'),
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
        order: DateOrder::DayMonthYear,
        separator: Some('/'),
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
        order: DateOrder::DayMonthYear,
        separator: Some('.'),
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

  /// The short form of a date, which a date picker's field shows: `Nov 12, 2026`, `12.11.2026`.
  pub fn format(&self, date: Date) -> String {
    let (day, month, year) = (date.day(), date.month(), date.year());
    let name = &self.months_short[(month - 1) as usize];
    match (self.separator, self.order) {
      (Some(separator), DateOrder::DayMonthYear) => format!("{day:02}{separator}{month:02}{separator}{year:04}"),
      (Some(separator), DateOrder::MonthDayYear) => format!("{month:02}{separator}{day:02}{separator}{year:04}"),
      (Some(separator), DateOrder::YearMonthDay) => format!("{year:04}{separator}{month:02}{separator}{day:02}"),
      (None, DateOrder::MonthDayYear) => format!("{name} {day}, {year}"),
      (None, DateOrder::DayMonthYear) => format!("{day} {name} {year}"),
      (None, DateOrder::YearMonthDay) => format!("{year} {name} {day}"),
    }
  }

  /// The short form of a range: `Nov 9 – Nov 15, 2026`, the year once where both ends are in it and the month is
  /// written by name; else both ends whole, between them an en dash.
  pub fn format_range(&self, range: DateRange) -> String {
    let (start, end) = (range.start(), range.end());
    if self.separator.is_none() && start.year() == end.year() {
      let name = |date: Date| &self.months_short[(date.month() - 1) as usize];
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
    let text = text.trim();
    let (first, second) = [" – ", " — ", "–", "—", " - ", " to "]
      .iter()
      .find_map(|dash| text.split_once(dash))?;
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
}
