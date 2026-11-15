//! The time a player reads out: how far it is, and how long the whole is.
//!
//! This file is the agreed surface of the component (its props and events), drawn plainly until the component is
//! given its own parts and recipe from the board (`Media/Time`, `kQtnm`). The players are built on these names:
//! keep them.

use std::sync::Arc;

use super::theme::current_theme;
use crate::{
  app::{
    events::MouseEvent,
    theme::{PaletteColor, Theme},
  },
  components::{Row, Text},
  layout::Alignment,
  node::{Element, EventHandler, IntoMouseEventHandler},
};

/// The speeds a player offers, slowest first; 1 is the speed a recording was made at.
pub const MEDIA_RATES: [f32; 6] = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0];

/// A speed as a menu names it: "Normal" for 1, and "0.75×", "1.5×", "2×" for the others.
///
/// ```
/// use lurq::ui::media_rate_text;
///
/// assert_eq!(media_rate_text(1.0), "Normal");
/// assert_eq!(media_rate_text(1.5), "1.5×");
/// assert_eq!(media_rate_text(2.0), "2×");
/// ```
pub fn media_rate_text(rate: f32) -> String {
  if (rate - 1.0).abs() < f32::EPSILON {
    "Normal".to_string()
  } else {
    format!("{rate}×")
  }
}

/// A number of seconds as a player reads it out: `1:24`, and `1:02:45` with `hours` or from an hour up. A time
/// that is not a number, or below nothing, reads `0:00`.
///
/// ```
/// use lurq::ui::media_time_text;
///
/// assert_eq!(media_time_text(84.0, false), "1:24");
/// assert_eq!(media_time_text(3765.0, false), "1:02:45");
/// assert_eq!(media_time_text(84.0, true), "0:01:24");
/// ```
pub fn media_time_text(seconds: f64, hours: bool) -> String {
  let whole = if seconds.is_finite() && seconds > 0.0 {
    seconds.floor() as u64
  } else {
    0
  };
  let (h, m, s) = (whole / 3600, whole / 60 % 60, whole % 60);
  if hours || h > 0 {
    format!("{h}:{m:02}:{s:02}")
  } else {
    format!("{m}:{s:02}")
  }
}

/// The time a player reads out.
///
/// ```
/// use lurq::ui::MediaTime;
///
/// let time = MediaTime::new(84.0, Some(250.0));
/// // What is left, alone: "-2:46".
/// let left = MediaTime::new(84.0, Some(250.0)).remaining(true);
/// // Nothing is known of the length yet: "-:-- / -:--".
/// let unknown = MediaTime::new(0.0, None);
/// ```
pub struct MediaTime {
  position: f64,
  duration: Option<f64>,
  remaining: bool,
  on_media: bool,
  on_click: Vec<EventHandler<MouseEvent>>,
  id: Option<Arc<str>>,
  key: Option<Arc<str>>,
}

impl MediaTime {
  /// The time at `position` seconds of `duration`; of no duration where the length is not known yet.
  pub fn new(position: impl Into<f64>, duration: Option<f64>) -> Self {
    Self {
      position: position.into(),
      duration,
      remaining: false,
      on_media: false,
      on_click: Vec::new(),
      id: None,
      key: None,
    }
  }

  /// Whether it reads what is left, with a minus before it, and nothing else.
  pub fn remaining(mut self, remaining: bool) -> Self {
    self.remaining = remaining;
    self
  }

  /// Whether it stands over a picture: white, the same in a light and a dark theme.
  pub fn on_media(mut self, on_media: bool) -> Self {
    self.on_media = on_media;
    self
  }

  /// Called on a press: a player turns the reading between elapsed and remaining.
  pub fn on_click(mut self, handler: impl IntoMouseEventHandler) -> Self {
    self.on_click.push(handler.into_event_handler());
    self
  }

  pub fn id(mut self, id: impl Into<Arc<str>>) -> Self {
    self.id = Some(id.into());
    self
  }

  pub fn key(mut self, key: impl Into<Arc<str>>) -> Self {
    self.key = Some(key.into());
    self
  }

  /// Builds the time in `theme`.
  pub fn build(self, _theme: &Theme) -> Element {
    let hours = self.duration.is_some_and(|duration| duration >= 3600.0);
    let read = |seconds: f64| media_time_text(seconds, hours);
    let muted = PaletteColor::MutedForeground;
    let mut row = Row::new().align_items(Alignment::Center).spacing(4.0);
    row = match (self.duration, self.remaining) {
      (Some(duration), true) => {
        row.child(Text::new(&format!("-{}", read((duration - self.position).max(0.0)))).nowrap())
      }
      (Some(duration), false) => row
        .child(Text::new(&read(self.position)).nowrap())
        .child(Text::new("/").nowrap().color(muted.clone()))
        .child(Text::new(&read(duration)).nowrap().color(muted)),
      (None, _) => row
        .child(Text::new("-:--").nowrap().color(muted.clone()))
        .child(Text::new("/").nowrap().color(muted.clone()))
        .child(Text::new("-:--").nowrap().color(muted)),
    };
    for handler in self.on_click {
      row = row.on_click(handler);
    }
    if let Some(id) = self.id {
      row = row.id(id);
    }
    if let Some(key) = self.key {
      row = row.key(key);
    }
    row.into()
  }
}

impl From<MediaTime> for Element {
  fn from(time: MediaTime) -> Self {
    time.build(&current_theme())
  }
}
