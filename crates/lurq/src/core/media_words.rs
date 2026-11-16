//! What a media and its players say by themselves: the names of their controls, the words they draw, and what a
//! media that cannot play says. A part of the app's [`Locale`](super::Locale).

use std::{fmt, sync::Arc};

/// Why a media cannot play. It says what kind of failure it is, and the [`Locale`](super::Locale) says it in the
/// app's language ([`MediaWords::error`]); what the system or the connection said in its own words is kept beside
/// it, for a log.
///
/// ```
/// use lurq::core::{Locale, MediaError};
///
/// assert_eq!(&*Locale::english().media.error(&MediaError::Unsupported), "Unsupported format");
/// assert_eq!(&*Locale::for_tag("uk").media.error(&MediaError::Server(503)), "Помилка сервера 503");
/// assert_eq!(MediaError::NotFound.to_string(), "Not found");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MediaError {
  /// What it is, is nothing the system plays: bytes that are no video and no sound, a kind it has no decoder for.
  Unsupported,
  /// There is no such file, or the server has none at the address.
  NotFound,
  /// The app's loader does not allow the address.
  Denied,
  /// Loading it took longer than the app's loader waits.
  TimedOut,
  /// It is larger than the app's loader reads whole, and nothing else could read it.
  TooLarge,
  /// The server answered with this status.
  Server(u16),
  /// The connection failed, in the words of what failed.
  Network(Arc<str>),
  /// This build or this system has nothing to play it with, and why.
  Unavailable(Arc<str>),
  /// Anything else, in the system's own words.
  Other(Arc<str>),
}

impl MediaError {
  /// What the system or the connection said in its own words, where it said something: for a log, not for a
  /// person.
  pub fn detail(&self) -> Option<&str> {
    match self {
      Self::Network(said) | Self::Unavailable(said) | Self::Other(said) => Some(said),
      _ => None,
    }
  }
}

/// In English, as [`Locale::english`](super::Locale::english) says it.
impl fmt::Display for MediaError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&MediaWords::english().error(self))
  }
}

impl std::error::Error for MediaError {}

/// What a media and its players say by themselves, in a locale's words. Every field is the app's to replace.
///
/// ```
/// use std::sync::Arc;
///
/// use lurq::core::Locale;
///
/// let mut locale = Locale::for_tag("uk");
/// assert_eq!(&*locale.media.play, "Відтворити");
/// // A word of the app's own in place of one of lurq's.
/// locale.media.try_again = Arc::from("Ще раз");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaWords {
  /// What an audio player and a video player are called to what reads the tree.
  pub audio_player: Arc<str>,
  pub video_player: Arc<str>,
  pub play: Arc<str>,
  pub pause: Arc<str>,
  pub replay: Arc<str>,
  pub previous: Arc<str>,
  pub next: Arc<str>,
  pub shuffle: Arc<str>,
  pub repeat: Arc<str>,
  pub more: Arc<str>,
  /// The name of a video's speed button, and the heading of the menu of speeds.
  pub speed: Arc<str>,
  pub playback_speed: Arc<str>,
  /// The speed things were made at, in a menu of speeds: "Normal".
  pub normal_speed: Arc<str>,
  pub queue: Arc<str>,
  /// What the mark of the track that plays is called in a queue.
  pub now_playing: Arc<str>,
  pub expand: Arc<str>,
  /// What a seek bar and a waveform are called.
  pub seek: Arc<str>,
  pub mute: Arc<str>,
  pub unmute: Arc<str>,
  pub volume: Arc<str>,
  pub captions: Arc<str>,
  pub settings: Arc<str>,
  pub quality: Arc<str>,
  pub picture_in_picture: Arc<str>,
  /// What a video says where it stood while it plays in its small window, and the button that brings it back.
  pub in_picture: Arc<str>,
  pub back_to_window: Arc<str>,
  pub fullscreen: Arc<str>,
  pub exit_fullscreen: Arc<str>,
  pub try_again: Arc<str>,
  /// The touch form's buttons that go ten seconds back and on, and the word after a number of them.
  pub rewind: Arc<str>,
  pub forward: Arc<str>,
  pub seconds: Arc<str>,
  pub minimize: Arc<str>,
  pub back: Arc<str>,
  /// The quality a player picks itself, captions on and off, and the badge of what is live.
  pub auto: Arc<str>,
  pub on: Arc<str>,
  pub off: Arc<str>,
  pub live: Arc<str>,
  /// The title of a video's error; the reason is under it.
  pub unplayable_video: Arc<str>,
  /// What each kind of [`MediaError`] is called.
  pub unsupported: Arc<str>,
  pub not_found: Arc<str>,
  pub denied: Arc<str>,
  pub timed_out: Arc<str>,
  pub too_large: Arc<str>,
  /// The server's answer, with `{status}` where its number goes.
  pub server_error: Arc<str>,
  pub network_error: Arc<str>,
  pub unavailable: Arc<str>,
  pub failed: Arc<str>,
}

impl MediaWords {
  /// The words in the order of the fields.
  fn of(words: [&str; 48]) -> Self {
    let [
      audio_player,
      video_player,
      play,
      pause,
      replay,
      previous,
      next,
      shuffle,
      repeat,
      more,
      speed,
      playback_speed,
      normal_speed,
      queue,
      now_playing,
      expand,
      seek,
      mute,
      unmute,
      volume,
      captions,
      settings,
      quality,
      picture_in_picture,
      in_picture,
      back_to_window,
      fullscreen,
      exit_fullscreen,
      try_again,
      rewind,
      forward,
      seconds,
      minimize,
      back,
      auto,
      on,
      off,
      live,
      unplayable_video,
      unsupported,
      not_found,
      denied,
      timed_out,
      too_large,
      server_error,
      network_error,
      unavailable,
      failed,
    ] = words.map(Arc::from);
    Self {
      audio_player,
      video_player,
      play,
      pause,
      replay,
      previous,
      next,
      shuffle,
      repeat,
      more,
      speed,
      playback_speed,
      normal_speed,
      queue,
      now_playing,
      expand,
      seek,
      mute,
      unmute,
      volume,
      captions,
      settings,
      quality,
      picture_in_picture,
      in_picture,
      back_to_window,
      fullscreen,
      exit_fullscreen,
      try_again,
      rewind,
      forward,
      seconds,
      minimize,
      back,
      auto,
      on,
      off,
      live,
      unplayable_video,
      unsupported,
      not_found,
      denied,
      timed_out,
      too_large,
      server_error,
      network_error,
      unavailable,
      failed,
    }
  }

  pub fn english() -> Self {
    Self::of([
      "Audio player",
      "Video player",
      "Play",
      "Pause",
      "Replay",
      "Previous",
      "Next",
      "Shuffle",
      "Repeat",
      "More",
      "Speed",
      "Playback speed",
      "Normal",
      "Queue",
      "Now playing",
      "Expand",
      "Seek",
      "Mute",
      "Unmute",
      "Volume",
      "Captions",
      "Settings",
      "Quality",
      "Picture in picture",
      "Playing in picture in picture",
      "Back to the window",
      "Fullscreen",
      "Exit fullscreen",
      "Try again",
      "Back 10 seconds",
      "Forward 10 seconds",
      "seconds",
      "Minimize",
      "Back",
      "Auto",
      "On",
      "Off",
      "LIVE",
      "This video can't be played",
      "Unsupported format",
      "Not found",
      "Not allowed",
      "Timed out",
      "Too large",
      "Server error {status}",
      "Network error",
      "Can't play on this device",
      "Can't be played",
    ])
  }

  /// The words of a language (`en`, `uk`, `de`, `fr`, `es`, `pl`); of any other, English.
  pub fn for_language(language: &str) -> Self {
    match language {
      "uk" => Self::of([
        "Аудіоплеєр",
        "Відеоплеєр",
        "Відтворити",
        "Пауза",
        "Відтворити знову",
        "Попередній",
        "Наступний",
        "Перемішати",
        "Повтор",
        "Ще",
        "Швидкість",
        "Швидкість відтворення",
        "Звичайна",
        "Черга",
        "Зараз грає",
        "Розгорнути",
        "Перемотування",
        "Вимкнути звук",
        "Увімкнути звук",
        "Гучність",
        "Субтитри",
        "Налаштування",
        "Якість",
        "Картинка в картинці",
        "Відтворюється в режимі «картинка в картинці»",
        "Повернути у вікно",
        "На весь екран",
        "Вийти з повноекранного режиму",
        "Спробувати ще раз",
        "Назад на 10 секунд",
        "Вперед на 10 секунд",
        "секунд",
        "Згорнути",
        "Назад",
        "Авто",
        "Увімк.",
        "Вимк.",
        "НАЖИВО",
        "Це відео не вдається відтворити",
        "Непідтримуваний формат",
        "Не знайдено",
        "Не дозволено",
        "Час очікування вичерпано",
        "Завеликий файл",
        "Помилка сервера {status}",
        "Помилка мережі",
        "Не відтворюється на цьому пристрої",
        "Не вдається відтворити",
      ]),
      "de" => Self::of([
        "Audioplayer",
        "Videoplayer",
        "Wiedergabe",
        "Pause",
        "Erneut abspielen",
        "Vorheriger Titel",
        "Nächster Titel",
        "Zufallswiedergabe",
        "Wiederholen",
        "Mehr",
        "Geschwindigkeit",
        "Wiedergabegeschwindigkeit",
        "Normal",
        "Warteschlange",
        "Läuft gerade",
        "Erweitern",
        "Position",
        "Stummschalten",
        "Stummschaltung aufheben",
        "Lautstärke",
        "Untertitel",
        "Einstellungen",
        "Qualität",
        "Bild im Bild",
        "Wiedergabe im Bild-im-Bild-Modus",
        "Zurück zum Fenster",
        "Vollbild",
        "Vollbild beenden",
        "Erneut versuchen",
        "10 Sekunden zurück",
        "10 Sekunden vor",
        "Sekunden",
        "Minimieren",
        "Zurück",
        "Auto",
        "Ein",
        "Aus",
        "LIVE",
        "Dieses Video kann nicht abgespielt werden",
        "Nicht unterstütztes Format",
        "Nicht gefunden",
        "Nicht erlaubt",
        "Zeitüberschreitung",
        "Zu groß",
        "Serverfehler {status}",
        "Netzwerkfehler",
        "Auf diesem Gerät nicht abspielbar",
        "Wiedergabe nicht möglich",
      ]),
      "fr" => Self::of([
        "Lecteur audio",
        "Lecteur vidéo",
        "Lire",
        "Pause",
        "Relire",
        "Précédent",
        "Suivant",
        "Lecture aléatoire",
        "Répéter",
        "Plus",
        "Vitesse",
        "Vitesse de lecture",
        "Normale",
        "File d’attente",
        "Lecture en cours",
        "Agrandir",
        "Position de lecture",
        "Couper le son",
        "Rétablir le son",
        "Volume",
        "Sous-titres",
        "Paramètres",
        "Qualité",
        "Image dans l’image",
        "Lecture en mode image dans l’image",
        "Revenir à la fenêtre",
        "Plein écran",
        "Quitter le plein écran",
        "Réessayer",
        "Reculer de 10 secondes",
        "Avancer de 10 secondes",
        "secondes",
        "Réduire",
        "Retour",
        "Auto",
        "Activé",
        "Désactivé",
        "EN DIRECT",
        "Impossible de lire cette vidéo",
        "Format non pris en charge",
        "Introuvable",
        "Non autorisé",
        "Délai dépassé",
        "Trop volumineux",
        "Erreur du serveur {status}",
        "Erreur réseau",
        "Lecture impossible sur cet appareil",
        "Lecture impossible",
      ]),
      "es" => Self::of([
        "Reproductor de audio",
        "Reproductor de vídeo",
        "Reproducir",
        "Pausar",
        "Volver a reproducir",
        "Anterior",
        "Siguiente",
        "Aleatorio",
        "Repetir",
        "Más",
        "Velocidad",
        "Velocidad de reproducción",
        "Normal",
        "Cola",
        "Reproduciendo",
        "Ampliar",
        "Posición",
        "Silenciar",
        "Activar sonido",
        "Volumen",
        "Subtítulos",
        "Ajustes",
        "Calidad",
        "Imagen en imagen",
        "Reproduciendo en imagen en imagen",
        "Volver a la ventana",
        "Pantalla completa",
        "Salir de pantalla completa",
        "Reintentar",
        "Retroceder 10 segundos",
        "Avanzar 10 segundos",
        "segundos",
        "Minimizar",
        "Atrás",
        "Auto",
        "Activado",
        "Desactivado",
        "EN DIRECTO",
        "No se puede reproducir este vídeo",
        "Formato no compatible",
        "No encontrado",
        "No permitido",
        "Tiempo de espera agotado",
        "Demasiado grande",
        "Error del servidor {status}",
        "Error de red",
        "No se puede reproducir en este dispositivo",
        "No se puede reproducir",
      ]),
      "pl" => Self::of([
        "Odtwarzacz audio",
        "Odtwarzacz wideo",
        "Odtwórz",
        "Wstrzymaj",
        "Odtwórz ponownie",
        "Poprzedni",
        "Następny",
        "Losowo",
        "Powtarzaj",
        "Więcej",
        "Szybkość",
        "Szybkość odtwarzania",
        "Normalna",
        "Kolejka",
        "Teraz odtwarzane",
        "Rozwiń",
        "Przewijanie",
        "Wycisz",
        "Wyłącz wyciszenie",
        "Głośność",
        "Napisy",
        "Ustawienia",
        "Jakość",
        "Obraz w obrazie",
        "Odtwarzanie w trybie obraz w obrazie",
        "Wróć do okna",
        "Pełny ekran",
        "Zamknij pełny ekran",
        "Spróbuj ponownie",
        "Cofnij o 10 sekund",
        "Do przodu o 10 sekund",
        "sekund",
        "Minimalizuj",
        "Wstecz",
        "Auto",
        "Wł.",
        "Wył.",
        "NA ŻYWO",
        "Nie można odtworzyć tego filmu",
        "Nieobsługiwany format",
        "Nie znaleziono",
        "Niedozwolone",
        "Przekroczono limit czasu",
        "Za duży plik",
        "Błąd serwera {status}",
        "Błąd sieci",
        "Nie można odtworzyć na tym urządzeniu",
        "Nie można odtworzyć",
      ]),
      _ => Self::english(),
    }
  }

  /// What `error` is called: the short line a player shows a person.
  pub fn error(&self, error: &MediaError) -> Arc<str> {
    match error {
      MediaError::Unsupported => self.unsupported.clone(),
      MediaError::NotFound => self.not_found.clone(),
      MediaError::Denied => self.denied.clone(),
      MediaError::TimedOut => self.timed_out.clone(),
      MediaError::TooLarge => self.too_large.clone(),
      MediaError::Server(status) => Arc::from(self.server_error.replace("{status}", &status.to_string())),
      MediaError::Network(_) => self.network_error.clone(),
      MediaError::Unavailable(_) => self.unavailable.clone(),
      MediaError::Other(_) => self.failed.clone(),
    }
  }
}

impl Default for MediaWords {
  fn default() -> Self {
    Self::english()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn every_language_has_every_word_and_the_server_error_has_its_place_for_the_status() {
    for language in ["en", "uk", "de", "fr", "es", "pl"] {
      let words = MediaWords::for_language(language);
      assert!(words.server_error.contains("{status}"), "{language}");
      assert!(words.error(&MediaError::Server(404)).contains("404"), "{language}");
      assert!(!words.play.is_empty() && !words.failed.is_empty(), "{language}");
    }
    assert_eq!(MediaWords::for_language("xx"), MediaWords::english());
  }

  #[test]
  fn an_error_keeps_what_the_system_said_and_shows_what_the_locale_says() {
    let error = MediaError::Network(Arc::from("No such host is known. (os error 11001)"));
    assert_eq!(error.detail(), Some("No such host is known. (os error 11001)"));
    assert_eq!(error.to_string(), "Network error");
    assert_eq!(MediaError::Unsupported.detail(), None);
  }
}
