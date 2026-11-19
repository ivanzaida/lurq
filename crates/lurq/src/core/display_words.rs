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
  /// What a link preview says under the address of a page that told nothing about itself.
  pub link_no_preview: Arc<str>,
  /// What a link preview of a video says it is, to tooling, where its length is not known.
  pub link_video: Arc<str>,
  /// The same with its length: `{length}` is where the length goes ("4:10").
  pub link_video_length: Arc<str>,
  /// The name of the button that takes a link preview out of what is being written.
  pub link_remove: Arc<str>,
  /// What a link preview says of a page the server has none at.
  pub link_not_found: Arc<str>,
  /// What a link preview says of an address the app does not allow.
  pub link_denied: Arc<str>,
  /// What a link preview says of a page that took too long.
  pub link_timed_out: Arc<str>,
  /// What a link preview says of a server that answered an error: `{status}` is where its number goes.
  pub link_server_error: Arc<str>,
  /// What a link preview says of a page it could not reach.
  pub link_network_error: Arc<str>,
  /// What a link preview says where it failed in another way.
  pub link_failed: Arc<str>,
}

impl DisplayWords {
  /// The words in the order of the fields.
  fn of(words: [&str; 11]) -> Self {
    let [
      qr_code,
      link_no_preview,
      link_video,
      link_video_length,
      link_remove,
      link_not_found,
      link_denied,
      link_timed_out,
      link_server_error,
      link_network_error,
      link_failed,
    ] = words.map(Arc::from);
    Self {
      qr_code,
      link_no_preview,
      link_video,
      link_video_length,
      link_remove,
      link_not_found,
      link_denied,
      link_timed_out,
      link_server_error,
      link_network_error,
      link_failed,
    }
  }

  pub fn english() -> Self {
    Self::of([
      "QR code",
      "No preview for this page.",
      "Video",
      "Video, {length}",
      "Remove preview",
      "The page was not found.",
      "The app doesn't open this address.",
      "The page took too long to answer.",
      "The site answered {status}.",
      "The page couldn't be reached.",
      "The preview didn't load.",
    ])
  }

  /// The words of a language (`en`, `uk`, `de`, `fr`, `es`, `pl`); of any other, English.
  pub fn for_language(language: &str) -> Self {
    match language {
      "uk" => Self::of([
        "QR-код",
        "Для цієї сторінки немає попереднього перегляду.",
        "Відео",
        "Відео, {length}",
        "Прибрати попередній перегляд",
        "Сторінку не знайдено.",
        "Застосунок не відкриває цю адресу.",
        "Сторінка відповідала надто довго.",
        "Сайт відповів {status}.",
        "Не вдалося дістатися сторінки.",
        "Попередній перегляд не завантажився.",
      ]),
      "de" => Self::of([
        "QR-Code",
        "Keine Vorschau für diese Seite.",
        "Video",
        "Video, {length}",
        "Vorschau entfernen",
        "Die Seite wurde nicht gefunden.",
        "Die App öffnet diese Adresse nicht.",
        "Die Seite hat zu lange gebraucht.",
        "Die Website antwortete mit {status}.",
        "Die Seite war nicht erreichbar.",
        "Die Vorschau wurde nicht geladen.",
      ]),
      "fr" => Self::of([
        "Code QR",
        "Aucun aperçu pour cette page.",
        "Vidéo",
        "Vidéo, {length}",
        "Retirer l’aperçu",
        "La page est introuvable.",
        "L’app n’ouvre pas cette adresse.",
        "La page a mis trop de temps à répondre.",
        "Le site a répondu {status}.",
        "La page est injoignable.",
        "L’aperçu ne s’est pas chargé.",
      ]),
      "es" => Self::of([
        "Código QR",
        "No hay vista previa de esta página.",
        "Vídeo",
        "Vídeo, {length}",
        "Quitar la vista previa",
        "No se encontró la página.",
        "La app no abre esta dirección.",
        "La página tardó demasiado en responder.",
        "El sitio respondió {status}.",
        "No se pudo llegar a la página.",
        "La vista previa no se cargó.",
      ]),
      "pl" => Self::of([
        "Kod QR",
        "Brak podglądu tej strony.",
        "Wideo",
        "Wideo, {length}",
        "Usuń podgląd",
        "Nie znaleziono strony.",
        "Aplikacja nie otwiera tego adresu.",
        "Strona odpowiadała zbyt długo.",
        "Witryna odpowiedziała {status}.",
        "Nie udało się połączyć ze stroną.",
        "Podgląd się nie wczytał.",
      ]),
      _ => Self::english(),
    }
  }

  /// What a link preview says of what went wrong: a page that could not be had in this build says what one
  /// that told nothing says.
  pub fn link_error(&self, error: &crate::resources::LinkPreviewError) -> Arc<str> {
    use crate::resources::LinkPreviewError::*;
    match error {
      NotFound => self.link_not_found.clone(),
      Denied => self.link_denied.clone(),
      TimedOut => self.link_timed_out.clone(),
      Server(status) => Arc::from(super::fill(&self.link_server_error, &[("status", &status.to_string())])),
      Network(_) => self.link_network_error.clone(),
      Unavailable(_) => self.link_no_preview.clone(),
      Other(_) => self.link_failed.clone(),
    }
  }

  /// What a link preview of a video `length` seconds long says it is, to tooling: "Video, 4:10".
  pub fn link_video_text(&self, length: Option<f64>) -> String {
    match length {
      Some(length) => super::fill(
        &self.link_video_length,
        &[("length", &crate::ui::media_time_text(length, false))],
      ),
      None => self.link_video.to_string(),
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
      let words = DisplayWords::for_language(language);
      assert!(!words.qr_code.trim().is_empty(), "{language}");
      assert!(words.link_video_length.contains("{length}"), "{language}");
      assert!(words.link_server_error.contains("{status}"), "{language}");
    }
    assert_eq!(DisplayWords::for_language("xx"), DisplayWords::english());
  }
}
