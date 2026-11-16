//! The one locale of an app: every word lurq writes by itself, and how it writes a date.
//!
//! A component draws words of its own (the name of a button, a line that says what went wrong, a month). An app
//! in another language, or with another voice, replaces them in one place: it makes a [`Locale`], changes what it
//! wants in it, and gives it to its app ([`App::set_locale`](crate::app::App::set_locale)). Every component reads
//! its words from there ([`current_locale`](crate::ui::current_locale)) unless it is told otherwise itself.
//!
//! What is in it grows with the components: a component's words are a part of the locale, not constants in the
//! component.

use std::sync::Arc;

use parking_lot::RwLock;

use super::{DateLocale, DisplayWords, InputWords, MediaWords, Signal};

/// Every word lurq writes by itself, in one language. Every field is the app's to replace.
///
/// ```
/// use std::sync::Arc;
///
/// use lurq::{app::App, core::Locale};
///
/// let app = App::new();
/// // The words of a language lurq knows, with one of the app's own.
/// let mut locale = Locale::for_tag("uk-UA");
/// locale.media.try_again = Arc::from("Ще раз");
/// app.set_locale(locale);
/// assert_eq!(&*app.locale().media.play, "Відтворити");
/// ```
///
/// lurq knows English (`en`, and `en-US`, `en-GB` for how dates are written), Ukrainian (`uk`), German (`de`),
/// French (`fr`), Spanish (`es`) and Polish (`pl`); any other tag is English.
#[derive(Clone, Debug)]
pub struct Locale {
  /// The tag it was made for: `en`, `uk-UA`.
  pub tag: Arc<str>,
  /// How dates are written and read, and what a calendar and a date picker call their parts.
  pub dates: DateLocale,
  /// What a media and its players say: the names of their controls, and why a media cannot play.
  pub media: MediaWords,
  /// What the controls that take a value say: their names, their buttons', their placeholders, an empty list.
  pub input: InputWords,
  /// What the components that show something are called to tooling.
  pub display: DisplayWords,
}

impl Locale {
  pub fn english() -> Self {
    Self {
      tag: Arc::from("en"),
      dates: DateLocale::english(),
      media: MediaWords::english(),
      input: InputWords::english(),
      display: DisplayWords::english(),
    }
  }

  /// The locale of a tag (`en`, `en-US`, `uk-UA`): by its language where lurq knows it, else English. Dates are
  /// written as [`DateLocale::for_tag`] writes them for the whole tag.
  pub fn for_tag(tag: &str) -> Self {
    let dates = DateLocale::for_tag(tag);
    let known = tag.trim().replace('_', "-").to_ascii_lowercase();
    let language = known.split('-').next().unwrap_or("");
    Self {
      tag: Arc::from(tag.trim()),
      dates,
      media: MediaWords::for_language(language),
      input: InputWords::for_language(language),
      display: DisplayWords::for_language(language),
    }
  }

  /// With `dates` for how dates are written.
  pub fn with_dates(mut self, dates: DateLocale) -> Self {
    self.dates = dates;
    self
  }

  /// With `media` for what a media and its players say.
  pub fn with_media(mut self, media: MediaWords) -> Self {
    self.media = media;
    self
  }

  /// With `input` for what the controls that take a value say.
  pub fn with_input(mut self, input: InputWords) -> Self {
    self.input = input;
    self
  }

  /// With `display` for what the components that show something are called.
  pub fn with_display(mut self, display: DisplayWords) -> Self {
    self.display = display;
    self
  }
}

impl Default for Locale {
  fn default() -> Self {
    Self::english()
  }
}

/// The app's locale, which [`App::set_locale`](crate::app::App::set_locale) sets: read so that what reads it in a
/// render is rendered again when it changes.
#[derive(Clone)]
pub(crate) struct AppLocale {
  current: Arc<RwLock<Arc<Locale>>>,
  version: Signal<u64>,
}

impl AppLocale {
  pub fn new() -> Self {
    Self {
      current: Arc::new(RwLock::new(english_locale())),
      version: Signal::new(0),
    }
  }

  /// The locale, read so that the render reading it is run again when it changes.
  pub fn get(&self) -> Arc<Locale> {
    let _ = self.version.get();
    self.current.read().clone()
  }

  pub fn set(&self, locale: Locale) {
    *self.current.write() = Arc::new(locale);
    self.version.update(|version| *version = version.wrapping_add(1));
  }
}

/// English, made once: the locale outside a render, and of an app that set none.
pub(crate) fn english_locale() -> Arc<Locale> {
  static ENGLISH: std::sync::OnceLock<Arc<Locale>> = std::sync::OnceLock::new();
  ENGLISH.get_or_init(|| Arc::new(Locale::english())).clone()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn a_tag_gives_its_languages_words_and_its_regions_dates() {
    let locale = Locale::for_tag("uk_UA");
    assert_eq!(&*locale.media.pause, "Пауза");
    assert_eq!(&*locale.input.clear, "Очистити");
    assert_eq!(&*locale.display.qr_code, "QR-код");
    assert_eq!(&*locale.dates.tag, &*DateLocale::for_tag("uk_UA").tag);
    // A region of English has English words, and its own way with dates.
    let british = Locale::for_tag("en-GB");
    assert_eq!(british.media, MediaWords::english());
    assert_eq!(british.dates.order, DateLocale::for_tag("en-GB").order);
    assert_eq!(british.input, InputWords::english());
    assert_eq!(Locale::for_tag("xx").media, MediaWords::english());
    assert_eq!(Locale::for_tag("xx").input, InputWords::english());
  }

  #[test]
  fn what_reads_the_apps_locale_reads_what_was_last_set() {
    let held = AppLocale::new();
    assert_eq!(&*held.get().media.play, "Play");
    held.set(Locale::for_tag("de"));
    assert_eq!(&*held.get().media.play, "Wiedergabe");
  }
}
