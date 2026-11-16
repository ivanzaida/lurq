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

use super::{
  CommonWords, DataWords, DateLocale, EditingWords, FieldWords, KeyWords, MediaWords, NavigationWords, Plural,
  PluralRule, Signal,
};

/// The languages lurq has words for.
#[cfg(test)]
pub(crate) const LANGUAGES: [&str; 6] = ["en", "uk", "de", "fr", "es", "pl"];

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
  /// What many components say alike: "Close", "Clear", "More", "No results", a count of things chosen.
  pub common: CommonWords,
  /// What fields, pickers and forms say: their buttons, a password's strength, a color's channels, a file's size.
  pub fields: FieldWords,
  /// What the ways about an app say: windows, panels, tabs, sidebars, pages, slides, steps, notifications.
  pub navigation: NavigationWords,
  /// What tables, lists and charts say: counts of rows, a column's menus, filters, sums, a chart's names.
  pub data: DataWords,
  /// What the tools of writing say: find and replace, a rich text editor, a block of code, a command palette.
  pub editing: EditingWords,
  /// What keys are called where they are written in words.
  pub keys: KeyWords,
  /// Which form of a word the language puts after a number: what [`Locale::count`] says a [`Plural`] with.
  pub plural: PluralRule,
}

impl Locale {
  pub fn english() -> Self {
    Self {
      tag: Arc::from("en"),
      dates: DateLocale::english(),
      media: MediaWords::english(),
      common: CommonWords::english(),
      fields: FieldWords::english(),
      navigation: NavigationWords::english(),
      data: DataWords::english(),
      editing: EditingWords::english(),
      keys: KeyWords::english(),
      plural: PluralRule::OneOther,
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
      common: CommonWords::for_language(language),
      fields: FieldWords::for_language(language),
      navigation: NavigationWords::for_language(language),
      data: DataWords::for_language(language),
      editing: EditingWords::for_language(language),
      keys: KeyWords::for_language(language),
      plural: PluralRule::for_language(language),
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

  /// `word` said after `count`, in the form this locale's language gives it there: "3 rows", "3 рядки".
  ///
  /// ```
  /// use lurq::core::Locale;
  ///
  /// let locale = Locale::for_tag("pl");
  /// assert_eq!(locale.count(&locale.data.rows, 1), "1 wiersz");
  /// assert_eq!(locale.count(&locale.data.rows, 4), "4 wiersze");
  /// assert_eq!(locale.count(&locale.data.rows, 5), "5 wierszy");
  /// ```
  pub fn count(&self, word: &Plural, count: usize) -> String {
    word.say(self.plural, count as u64)
  }

  /// The form of `word` this locale's language gives it after `count`, its places not filled: for a sentence
  /// with other places than the count.
  pub fn form<'a>(&self, word: &'a Plural, count: usize) -> &'a Arc<str> {
    word.form(self.plural, count as u64)
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
    assert_eq!(&*locale.dates.tag, &*DateLocale::for_tag("uk_UA").tag);
    // A region of English has English words, and its own way with dates.
    let british = Locale::for_tag("en-GB");
    assert_eq!(british.media, MediaWords::english());
    assert_eq!(british.dates.order, DateLocale::for_tag("en-GB").order);
    assert_eq!(Locale::for_tag("xx").media, MediaWords::english());
  }

  #[test]
  fn what_reads_the_apps_locale_reads_what_was_last_set() {
    let held = AppLocale::new();
    assert_eq!(&*held.get().media.play, "Play");
    held.set(Locale::for_tag("de"));
    assert_eq!(&*held.get().media.play, "Wiedergabe");
  }
}
