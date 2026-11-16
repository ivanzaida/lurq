//! What the components that show something say by themselves: what tooling calls them where the app gives them no
//! name. A part of the app's [`Locale`](super::Locale).

use std::sync::Arc;

/// What the components that show something say by themselves, in a locale's words. Every field is the app's to
/// replace.
///
/// ```
/// use lurq::core::Locale;
///
/// assert_eq!(&*Locale::for_tag("de").display.qr_code, "QR-Code");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DisplayWords {
  /// What a QR code is called to tooling.
  pub qr_code: Arc<str>,
}

impl DisplayWords {
  /// The words in the order of the fields.
  fn of(words: [&str; 1]) -> Self {
    let [qr_code] = words.map(Arc::from);
    Self { qr_code }
  }

  pub fn english() -> Self {
    Self::of(["QR code"])
  }

  /// The words of a language (`en`, `uk`, `de`, `fr`, `es`, `pl`); of any other, English.
  pub fn for_language(language: &str) -> Self {
    match language {
      "uk" => Self::of(["QR-код"]),
      "de" => Self::of(["QR-Code"]),
      "fr" => Self::of(["Code QR"]),
      "es" => Self::of(["Código QR"]),
      "pl" => Self::of(["Kod QR"]),
      _ => Self::english(),
    }
  }
}

impl Default for DisplayWords {
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
      assert!(
        !DisplayWords::for_language(language).qr_code.trim().is_empty(),
        "{language}"
      );
    }
    assert_eq!(DisplayWords::for_language("xx"), DisplayWords::english());
  }
}
