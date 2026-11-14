//! A time of day: an hour, a minute and a second, with no date and no zone, beside [`Date`](super::Date).

use std::{fmt, str::FromStr};

/// Which half of the day an hour is in, on a clock of twelve hours.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DayPeriod {
  /// From midnight to noon: `AM`.
  Am,
  /// From noon to midnight: `PM`.
  Pm,
}

impl crate::app::component::DevtoolsInspectable for DayPeriod {
  fn inspect(&self, formatter: &mut crate::app::component::DevtoolsFormatter<'_>) {
    formatter.value_debug(self);
  }
}

/// How a locale counts the hours: from 0 to 23, or from 1 to 12 in each half of the day.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HourCycle {
  /// `14:30`.
  H23,
  /// `2:30 PM`.
  H12,
}

/// A time of day: hours 0 to 23, minutes and seconds 0 to 59.
///
/// ```
/// use lurq::core::{DayPeriod, Time};
///
/// let time = Time::new(14, 30, 0).unwrap();
/// assert_eq!(time.to_string(), "14:30:00");
/// assert_eq!("14:30".parse::<Time>(), Ok(time));
/// assert_eq!(time.hour12(), (2, DayPeriod::Pm));
/// assert_eq!(time.add_minutes(45), Time::new(15, 15, 0).unwrap());
/// assert_eq!(Time::new(23, 50, 0).unwrap().add_minutes(20), Time::new(0, 10, 0).unwrap(), "round the clock");
/// assert!(Time::new(9, 0, 0).unwrap() < time);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Time {
  hour: u8,
  minute: u8,
  second: u8,
}

impl Time {
  /// Midnight, `00:00:00`: the least time.
  pub const MIDNIGHT: Time = Time {
    hour: 0,
    minute: 0,
    second: 0,
  };

  /// `23:59:59`: the greatest time.
  pub const MAX: Time = Time {
    hour: 23,
    minute: 59,
    second: 59,
  };

  /// The time of `hour`, `minute` and `second`, where there is one.
  pub fn new(hour: u8, minute: u8, second: u8) -> Option<Self> {
    (hour < 24 && minute < 60 && second < 60).then_some(Self { hour, minute, second })
  }

  /// The time of `hour` (1 to 12) of `period`, as a clock of twelve hours says it.
  pub fn from_hour12(hour: u8, period: DayPeriod, minute: u8, second: u8) -> Option<Self> {
    if !(1..=12).contains(&hour) {
      return None;
    }
    let hour = match period {
      DayPeriod::Am => hour % 12,
      DayPeriod::Pm => hour % 12 + 12,
    };
    Self::new(hour, minute, second)
  }

  /// The time `seconds` after midnight, round the clock.
  pub fn from_seconds(seconds: i64) -> Self {
    let seconds = seconds.rem_euclid(86_400);
    Self {
      hour: (seconds / 3600) as u8,
      minute: (seconds / 60 % 60) as u8,
      second: (seconds % 60) as u8,
    }
  }

  /// How many seconds it is after midnight.
  pub fn seconds(self) -> i64 {
    self.hour as i64 * 3600 + self.minute as i64 * 60 + self.second as i64
  }

  /// Now where the app runs, by the clock and the zone of the system; in UTC where lurq cannot read the zone.
  pub fn now() -> Self {
    super::date::local_now_seconds()
      .map(Self::from_seconds)
      .unwrap_or_else(|| {
        let seconds = std::time::SystemTime::now()
          .duration_since(std::time::UNIX_EPOCH)
          .map(|since| since.as_secs() as i64)
          .unwrap_or(0);
        Self::from_seconds(seconds)
      })
  }

  pub fn hour(self) -> u8 {
    self.hour
  }

  pub fn minute(self) -> u8 {
    self.minute
  }

  pub fn second(self) -> u8 {
    self.second
  }

  /// The hour on a clock of twelve hours, 1 to 12, and the half of the day it is in: 0:30 is 12:30 AM.
  pub fn hour12(self) -> (u8, DayPeriod) {
    let period = if self.hour < 12 { DayPeriod::Am } else { DayPeriod::Pm };
    let hour = match self.hour % 12 {
      0 => 12,
      hour => hour,
    };
    (hour, period)
  }

  /// The half of the day it is in.
  pub fn period(self) -> DayPeriod {
    self.hour12().1
  }

  /// The time `minutes` after it (before it where negative), round the clock.
  pub fn add_minutes(self, minutes: i64) -> Self {
    Self::from_seconds(self.seconds() + minutes * 60)
  }

  /// The time `seconds` after it (before it where negative), round the clock.
  pub fn add_seconds(self, seconds: i64) -> Self {
    Self::from_seconds(self.seconds() + seconds)
  }

  /// It with its seconds at 0.
  pub fn without_seconds(self) -> Self {
    Self { second: 0, ..self }
  }

  /// It, kept between `min` and `max` where they are given.
  pub fn clamp_to(self, min: Option<Time>, max: Option<Time>) -> Self {
    let time = min.map_or(self, |min| self.max(min));
    max.map_or(time, |max| time.min(max))
  }
}

impl fmt::Debug for Time {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "Time({self})")
  }
}

/// ISO 8601: `14:30:00`.
impl fmt::Display for Time {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{:02}:{:02}:{:02}", self.hour, self.minute, self.second)
  }
}

/// What is not a time of ISO 8601.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseTimeError;

impl fmt::Display for ParseTimeError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str("not a time of the form HH:MM or HH:MM:SS")
  }
}

impl std::error::Error for ParseTimeError {}

/// ISO 8601: `14:30` or `14:30:00`, two digits each.
impl FromStr for Time {
  type Err = ParseTimeError;

  fn from_str(text: &str) -> Result<Self, Self::Err> {
    let parts: Vec<&str> = text.trim().split(':').collect();
    if !(2..=3).contains(&parts.len()) {
      return Err(ParseTimeError);
    }
    let mut numbers = parts.iter().map(|part| {
      (part.len() == 2 && part.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| part.parse::<u8>().ok())
        .flatten()
    });
    let hour = numbers.next().flatten().ok_or(ParseTimeError)?;
    let minute = numbers.next().flatten().ok_or(ParseTimeError)?;
    let second = match numbers.next() {
      Some(second) => second.ok_or(ParseTimeError)?,
      None => 0,
    };
    Time::new(hour, minute, second).ok_or(ParseTimeError)
  }
}

impl crate::app::component::DevtoolsInspectable for Time {
  fn inspect(&self, formatter: &mut crate::app::component::DevtoolsFormatter<'_>) {
    formatter.value_display(self);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn time(hour: u8, minute: u8, second: u8) -> Time {
    Time::new(hour, minute, second).unwrap()
  }

  #[test]
  fn a_time_is_an_hour_a_minute_and_a_second_of_the_day() {
    assert_eq!(Time::new(24, 0, 0), None);
    assert_eq!(Time::new(12, 60, 0), None);
    assert_eq!(time(9, 5, 7).to_string(), "09:05:07");
    assert_eq!(time(9, 5, 7).seconds(), 9 * 3600 + 5 * 60 + 7);
    assert_eq!(Time::from_seconds(-60), time(23, 59, 0));
    assert!(time(9, 0, 0) < time(9, 0, 1));
  }

  #[test]
  fn a_clock_of_twelve_hours_says_it_with_its_half_of_the_day() {
    assert_eq!(time(0, 30, 0).hour12(), (12, DayPeriod::Am));
    assert_eq!(time(12, 0, 0).hour12(), (12, DayPeriod::Pm));
    assert_eq!(time(23, 0, 0).hour12(), (11, DayPeriod::Pm));
    assert_eq!(Time::from_hour12(12, DayPeriod::Am, 15, 0), Some(time(0, 15, 0)));
    assert_eq!(Time::from_hour12(12, DayPeriod::Pm, 0, 0), Some(time(12, 0, 0)));
    assert_eq!(Time::from_hour12(13, DayPeriod::Pm, 0, 0), None);
  }

  #[test]
  fn iso_8601_is_read_and_written() {
    assert_eq!("07:45".parse::<Time>(), Ok(time(7, 45, 0)));
    assert_eq!("07:45:30".parse::<Time>(), Ok(time(7, 45, 30)));
    for wrong in ["7:45", "07:60", "24:00", "07-45", "07:45:3", ""] {
      assert_eq!(wrong.parse::<Time>(), Err(ParseTimeError), "{wrong}");
    }
    assert_eq!(time(10, 0, 0).clamp_to(Some(time(11, 0, 0)), None), time(11, 0, 0));
  }
}
