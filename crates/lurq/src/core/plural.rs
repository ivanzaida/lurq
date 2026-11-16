//! A word said after a number, in the form its language gives it there, and the places a word has for what goes
//! into it.

use std::sync::Arc;

/// Which form of a word a language puts after a number. English says "1 row" and "2 rows"; Ukrainian and Polish
/// have a third form, for 5 and on ("1 рядок", "2 рядки", "5 рядків"); French says "0 ligne" as it says "1 ligne".
///
/// ```
/// use lurq::core::{PluralForm, PluralRule};
///
/// assert_eq!(PluralRule::for_language("en").form(1), PluralForm::One);
/// assert_eq!(PluralRule::for_language("fr").form(0), PluralForm::One);
/// assert_eq!(PluralRule::for_language("uk").form(22), PluralForm::Few);
/// assert_eq!(PluralRule::for_language("pl").form(21), PluralForm::Many);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PluralRule {
  /// One for 1, the other for every other number: English, German, Spanish.
  #[default]
  OneOther,
  /// One for 0 and 1, the other for more: French.
  ZeroOneOther,
  /// One for 1, 21, 31 (not 11), few for 2 to 4, 22 to 24 (not 12 to 14), many for the rest: Ukrainian.
  EastSlavic,
  /// One for 1 alone, few for 2 to 4, 22 to 24 (not 12 to 14), many for the rest, 21 among them: Polish.
  Polish,
}

/// The form of a word after a number: which of a [`Plural`]'s words is said.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluralForm {
  One,
  Few,
  Many,
  Other,
}

impl PluralRule {
  /// The rule of a language (`en`, `uk`, `de`, `fr`, `es`, `pl`); of any other, English's.
  pub fn for_language(language: &str) -> Self {
    match language {
      "fr" => Self::ZeroOneOther,
      "uk" | "ru" | "be" => Self::EastSlavic,
      "pl" => Self::Polish,
      _ => Self::OneOther,
    }
  }

  /// The form a word takes after `count`.
  pub fn form(self, count: u64) -> PluralForm {
    let (tens, hundreds) = (count % 10, count % 100);
    let few = (2..=4).contains(&tens) && !(12..=14).contains(&hundreds);
    match self {
      Self::OneOther if count == 1 => PluralForm::One,
      Self::OneOther => PluralForm::Other,
      Self::ZeroOneOther if count <= 1 => PluralForm::One,
      Self::ZeroOneOther => PluralForm::Other,
      Self::EastSlavic if tens == 1 && hundreds != 11 => PluralForm::One,
      Self::Polish if count == 1 => PluralForm::One,
      Self::EastSlavic | Self::Polish if few => PluralForm::Few,
      Self::EastSlavic | Self::Polish => PluralForm::Many,
    }
  }
}

/// A sentence with a number in it, in each form its language gives the word after the number. `{count}` is where
/// the number goes; a sentence may have other places too (`{shown}`, `{names}`), which its component fills.
///
/// ```
/// use lurq::core::{Plural, PluralRule};
///
/// let rows = Plural::three("{count} рядок", "{count} рядки", "{count} рядків");
/// assert_eq!(rows.say(PluralRule::EastSlavic, 3), "3 рядки");
/// assert_eq!(rows.say(PluralRule::EastSlavic, 11), "11 рядків");
/// let rows = Plural::two("{count} row", "{count} rows");
/// assert_eq!(rows.say(PluralRule::OneOther, 1), "1 row");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plural {
  pub one: Arc<str>,
  pub few: Arc<str>,
  pub many: Arc<str>,
  /// What a language with two forms says for every number but one.
  pub other: Arc<str>,
}

impl Plural {
  /// The words of a language with two forms (English, German, French, Spanish): after one, and after the others.
  pub fn two(one: &str, other: &str) -> Self {
    Self {
      one: Arc::from(one),
      few: Arc::from(other),
      many: Arc::from(other),
      other: Arc::from(other),
    }
  }

  /// The words of a language with three forms (Ukrainian, Polish): after one, after a few, and after many.
  pub fn three(one: &str, few: &str, many: &str) -> Self {
    Self {
      one: Arc::from(one),
      few: Arc::from(few),
      many: Arc::from(many),
      other: Arc::from(many),
    }
  }

  /// The same words after every number: a sentence its language writes so that no word changes ("Вибрано: 3").
  pub fn same(word: &str) -> Self {
    Self::two(word, word)
  }

  /// The form `rule` gives the word after `count`, its places not filled.
  pub fn form(&self, rule: PluralRule, count: u64) -> &Arc<str> {
    match rule.form(count) {
      PluralForm::One => &self.one,
      PluralForm::Few => &self.few,
      PluralForm::Many => &self.many,
      PluralForm::Other => &self.other,
    }
  }

  /// The sentence after `count`, with the number in its place.
  pub fn say(&self, rule: PluralRule, count: u64) -> String {
    fill(self.form(rule, count), &[("count", &count.to_string())])
  }

  /// Every form, in the order `one`, `few`, `many`, `other`.
  pub fn forms(&self) -> [&Arc<str>; 4] {
    [&self.one, &self.few, &self.many, &self.other]
  }
}

/// `word` with each `{place}` it has put in: `fill("Page {at} of {total}", &[("at", "2"), ("total", "5")])` is
/// "Page 2 of 5". A place the word does not have is left out; a place with no value stays as it is.
///
/// ```
/// assert_eq!(lurq::core::fill("Sort by {column}", &[("column", "Name")]), "Sort by Name");
/// ```
pub fn fill(word: &str, places: &[(&str, &str)]) -> String {
  let mut said = String::with_capacity(word.len() + 16);
  let mut rest = word;
  while let Some(open) = rest.find('{') {
    said.push_str(&rest[..open]);
    let after = &rest[open + 1..];
    match after.find('}').map(|close| (&after[..close], close)) {
      Some((name, close)) if places.iter().any(|(place, _)| *place == name) => {
        let (_, value) = places.iter().find(|(place, _)| *place == name).expect("found above");
        said.push_str(value);
        rest = &after[close + 1..];
      }
      _ => {
        said.push('{');
        rest = after;
      }
    }
  }
  said.push_str(rest);
  said
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn each_language_picks_its_form_after_a_number() {
    let uk = PluralRule::EastSlavic;
    let forms: Vec<_> = [1, 2, 5, 11, 12, 21, 22, 25, 101, 111, 0].map(|n| uk.form(n)).into();
    use PluralForm::{Few, Many, One};
    assert_eq!(forms, [One, Few, Many, Many, Many, One, Few, Many, One, Many, Many]);
    let pl = PluralRule::Polish;
    let forms: Vec<_> = [1, 2, 5, 12, 21, 22, 0].map(|n| pl.form(n)).into();
    assert_eq!(forms, [One, Few, Many, Many, Many, Few, Many]);
    assert_eq!(PluralRule::OneOther.form(0), PluralForm::Other);
    assert_eq!(PluralRule::ZeroOneOther.form(2), PluralForm::Other);
    assert_eq!(PluralRule::for_language("xx"), PluralRule::OneOther);
  }

  #[test]
  fn a_word_has_its_places_filled_and_keeps_the_braces_it_has_no_value_for() {
    assert_eq!(fill("{shown} of {count}", &[("shown", "3"), ("count", "9")]), "3 of 9");
    assert_eq!(fill("{a} {b}", &[("a", "x")]), "x {b}");
    assert_eq!(fill("{ open", &[]), "{ open");
    assert_eq!(Plural::two("{count} row", "{count} rows").say(PluralRule::OneOther, 4), "4 rows");
  }
}
