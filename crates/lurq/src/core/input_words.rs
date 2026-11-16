//! What the controls that take a value say by themselves: what they are called where the app gives them no name,
//! the names of their buttons, the words under a line, what an empty list of theirs says. A part of the app's
//! [`Locale`](super::Locale).

use std::sync::Arc;

/// What the controls that take a value say by themselves, in a locale's words. Every field is the app's to replace.
///
/// A word is the area's, not one component's: "Clear" is the cross of a select and the button of a signature pad,
/// "Loading" the spinner of any list whose items are on their way.
///
/// ```
/// use std::sync::Arc;
///
/// use lurq::core::Locale;
///
/// let mut locale = Locale::for_tag("uk");
/// assert_eq!(&*locale.input.clear, "Очистити");
/// // A word of the app's own in place of one of lurq's.
/// locale.input.no_options = Arc::from("Нічого не знайдено");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputWords {
  /// What a signature pad and an angle slider are called to tooling.
  pub signature: Arc<str>,
  pub angle: Arc<str>,
  /// The words under a signature pad's line.
  pub sign_above_the_line: Arc<str>,
  /// What a signature pad says to tooling it holds.
  pub signed: Arc<str>,
  pub not_signed: Arc<str>,
  /// The button that takes back the last change.
  pub undo: Arc<str>,
  /// The control that empties what was given or chosen: a select's cross, a signature pad's button.
  pub clear: Arc<str>,
  /// What the two lists of a transfer are called: what can be chosen, and what is.
  pub available: Arc<str>,
  pub selected: Arc<str>,
  /// The checkbox that checks every item of a list.
  pub select_all: Arc<str>,
  /// The buttons that move what is checked into what is chosen, and back out of it.
  pub add: Arc<str>,
  pub remove: Arc<str>,
  /// What a list with no items says, and a panel of options with none.
  pub no_items: Arc<str>,
  pub no_options: Arc<str>,
  /// What the spinner of what is on its way is called.
  pub loading: Arc<str>,
}

impl InputWords {
  /// The words in the order of the fields.
  fn of(words: [&str; 15]) -> Self {
    let [
      signature,
      angle,
      sign_above_the_line,
      signed,
      not_signed,
      undo,
      clear,
      available,
      selected,
      select_all,
      add,
      remove,
      no_items,
      no_options,
      loading,
    ] = words.map(Arc::from);
    Self {
      signature,
      angle,
      sign_above_the_line,
      signed,
      not_signed,
      undo,
      clear,
      available,
      selected,
      select_all,
      add,
      remove,
      no_items,
      no_options,
      loading,
    }
  }

  pub fn english() -> Self {
    Self::of([
      "Signature",
      "Angle",
      "Sign above the line",
      "Signed",
      "Not signed",
      "Undo",
      "Clear",
      "Available",
      "Selected",
      "Select all",
      "Add",
      "Remove",
      "No items",
      "No options",
      "Loading",
    ])
  }

  /// The words of a language (`en`, `uk`, `de`, `fr`, `es`, `pl`); of any other, English.
  pub fn for_language(language: &str) -> Self {
    match language {
      "uk" => Self::of([
        "Підпис",
        "Кут",
        "Підпишіть над лінією",
        "Підписано",
        "Не підписано",
        "Скасувати",
        "Очистити",
        "Доступні",
        "Вибрані",
        "Вибрати все",
        "Додати",
        "Прибрати",
        "Немає елементів",
        "Немає варіантів",
        "Завантаження",
      ]),
      "de" => Self::of([
        "Unterschrift",
        "Winkel",
        "Über der Linie unterschreiben",
        "Unterschrieben",
        "Nicht unterschrieben",
        "Rückgängig",
        "Löschen",
        "Verfügbar",
        "Ausgewählt",
        "Alle auswählen",
        "Hinzufügen",
        "Entfernen",
        "Keine Einträge",
        "Keine Optionen",
        "Wird geladen",
      ]),
      "fr" => Self::of([
        "Signature",
        "Angle",
        "Signez au-dessus de la ligne",
        "Signé",
        "Non signé",
        "Annuler",
        "Effacer",
        "Disponibles",
        "Sélectionnés",
        "Tout sélectionner",
        "Ajouter",
        "Retirer",
        "Aucun élément",
        "Aucune option",
        "Chargement",
      ]),
      "es" => Self::of([
        "Firma",
        "Ángulo",
        "Firme sobre la línea",
        "Firmado",
        "Sin firmar",
        "Deshacer",
        "Borrar",
        "Disponibles",
        "Seleccionados",
        "Seleccionar todo",
        "Añadir",
        "Quitar",
        "Sin elementos",
        "Sin opciones",
        "Cargando",
      ]),
      "pl" => Self::of([
        "Podpis",
        "Kąt",
        "Podpisz nad linią",
        "Podpisano",
        "Nie podpisano",
        "Cofnij",
        "Wyczyść",
        "Dostępne",
        "Wybrane",
        "Zaznacz wszystko",
        "Dodaj",
        "Usuń",
        "Brak elementów",
        "Brak opcji",
        "Ładowanie",
      ]),
      _ => Self::english(),
    }
  }
}

impl Default for InputWords {
  fn default() -> Self {
    Self::english()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn every_language_has_every_word_and_another_is_english() {
    for language in ["en", "uk", "de", "fr", "es", "pl"] {
      let words = InputWords::for_language(language);
      let all = [
        &words.signature,
        &words.angle,
        &words.sign_above_the_line,
        &words.signed,
        &words.not_signed,
        &words.undo,
        &words.clear,
        &words.available,
        &words.selected,
        &words.select_all,
        &words.add,
        &words.remove,
        &words.no_items,
        &words.no_options,
        &words.loading,
      ];
      assert!(all.iter().all(|word| !word.trim().is_empty()), "{language}");
      // Two words of one language that say two things are two words.
      assert_ne!(words.signed, words.not_signed, "{language}");
      assert_ne!(words.add, words.remove, "{language}");
      assert_ne!(words.available, words.selected, "{language}");
    }
    assert_eq!(InputWords::for_language("xx"), InputWords::english());
    assert_eq!(&*InputWords::for_language("de").select_all, "Alle auswählen");
  }
}
