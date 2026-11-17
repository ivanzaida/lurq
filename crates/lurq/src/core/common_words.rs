//! What many components say alike: the names of the controls they share ("Close", "Clear", "More"), the lines of
//! a list that is empty or still loading, and a count of things. A part of the app's [`Locale`](super::Locale).

use std::sync::Arc;

use super::Plural;

/// What many components say alike, in a locale's words. Every field is the app's to replace.
///
/// ```
/// use lurq::core::Locale;
///
/// let locale = Locale::for_tag("uk");
/// assert_eq!(&*locale.common.close, "Закрити");
/// assert_eq!(locale.count(&locale.common.selected, 3), "Вибрано: 3");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommonWords {
  pub close: Arc<str>,
  pub clear: Arc<str>,
  /// A button that opens a menu of what else there is.
  pub more: Arc<str>,
  pub cancel: Arc<str>,
  pub save: Arc<str>,
  pub apply: Arc<str>,
  pub remove: Arc<str>,
  /// What asks again for what did not come: a button of a list whose rows failed, of a file that failed to upload.
  pub retry: Arc<str>,
  /// The same, where a sentence says what failed above it: a picture.
  pub try_again: Arc<str>,
  pub yes: Arc<str>,
  pub no: Arc<str>,
  pub back: Arc<str>,
  pub next: Arc<str>,
  pub done: Arc<str>,
  pub skip: Arc<str>,
  pub open: Arc<str>,
  pub download: Arc<str>,
  pub share: Arc<str>,
  /// What a picture that did not load says.
  pub picture_failed: Arc<str>,
  /// What takes a notification away.
  pub dismiss: Arc<str>,
  /// The button that shows what went on in more words.
  pub details: Arc<str>,
  pub show_more: Arc<str>,
  /// What a spinner, and a list that is loading, is called.
  pub loading: Arc<str>,
  /// The line a list draws while it loads.
  pub loading_text: Arc<str>,
  /// The line a list of results draws while they load.
  pub loading_results: Arc<str>,
  pub no_results: Arc<str>,
  pub no_options: Arc<str>,
  /// What a search field says until something is typed in it.
  pub search: Arc<str>,
  pub select_all: Arc<str>,
  /// The mark on what a model made: "AI".
  pub ai: Arc<str>,
  /// A place among a number of them: "3 of 10", with `{at}` and `{total}`.
  pub place_of: Arc<str>,
  /// The button that takes a thing away, with `{name}` for what it is called: "Remove Design".
  pub remove_named: Arc<str>,
  /// How many are chosen: "3 selected".
  pub selected: Plural,
  /// How many more there are than are shown: "3 more".
  pub more_count: Plural,
  /// The same, drawn where there is little room: "+3 more".
  pub plus_more: Plural,
  /// Names, and how many more there are than are named: "Ann, Bo and 3 more", with `{names}`.
  pub and_more: Plural,
}

impl CommonWords {
  pub fn english() -> Self {
    Self {
      close: "Close".into(),
      clear: "Clear".into(),
      more: "More".into(),
      cancel: "Cancel".into(),
      save: "Save".into(),
      apply: "Apply".into(),
      remove: "Remove".into(),
      retry: "Retry".into(),
      try_again: "Try again".into(),
      yes: "Yes".into(),
      no: "No".into(),
      back: "Back".into(),
      next: "Next".into(),
      done: "Done".into(),
      skip: "Skip".into(),
      open: "Open".into(),
      download: "Download".into(),
      share: "Share".into(),
      picture_failed: "The picture didn't load.".into(),
      dismiss: "Dismiss".into(),
      details: "Details".into(),
      show_more: "Show more".into(),
      loading: "Loading".into(),
      loading_text: "Loading…".into(),
      loading_results: "Loading results…".into(),
      no_results: "No results".into(),
      no_options: "No options".into(),
      search: "Search".into(),
      select_all: "Select all".into(),
      ai: "AI".into(),
      place_of: "{at} of {total}".into(),
      remove_named: "Remove {name}".into(),
      selected: Plural::same("{count} selected"),
      more_count: Plural::same("{count} more"),
      plus_more: Plural::same("+{count} more"),
      and_more: Plural::same("{names} and {count} more"),
    }
  }

  /// The words of a language (`en`, `uk`, `de`, `fr`, `es`, `pl`); of any other, English.
  pub fn for_language(language: &str) -> Self {
    match language {
      "uk" => Self {
        close: "Закрити".into(),
        clear: "Очистити".into(),
        more: "Ще".into(),
        cancel: "Скасувати".into(),
        save: "Зберегти".into(),
        apply: "Застосувати".into(),
        remove: "Вилучити".into(),
        retry: "Повторити".into(),
        try_again: "Спробувати ще раз".into(),
        yes: "Так".into(),
        no: "Ні".into(),
        back: "Назад".into(),
        next: "Далі".into(),
        done: "Готово".into(),
        skip: "Пропустити".into(),
        open: "Відкрити".into(),
        download: "Завантажити".into(),
        share: "Поділитися".into(),
        picture_failed: "Не вдалося завантажити зображення.".into(),
        dismiss: "Відхилити".into(),
        details: "Докладніше".into(),
        show_more: "Показати більше".into(),
        loading: "Завантаження".into(),
        loading_text: "Завантаження…".into(),
        loading_results: "Завантаження результатів…".into(),
        no_results: "Нічого не знайдено".into(),
        no_options: "Немає варіантів".into(),
        search: "Пошук".into(),
        select_all: "Вибрати все".into(),
        ai: "ШІ".into(),
        place_of: "{at} з {total}".into(),
        remove_named: "Вилучити {name}".into(),
        selected: Plural::same("Вибрано: {count}"),
        more_count: Plural::same("ще {count}"),
        plus_more: Plural::same("Ще {count}"),
        and_more: Plural::same("{names} та ще {count}"),
      },
      "de" => Self {
        close: "Schließen".into(),
        clear: "Löschen".into(),
        more: "Mehr".into(),
        cancel: "Abbrechen".into(),
        save: "Speichern".into(),
        apply: "Übernehmen".into(),
        remove: "Entfernen".into(),
        retry: "Wiederholen".into(),
        try_again: "Erneut versuchen".into(),
        yes: "Ja".into(),
        no: "Nein".into(),
        back: "Zurück".into(),
        next: "Weiter".into(),
        done: "Fertig".into(),
        skip: "Überspringen".into(),
        open: "Öffnen".into(),
        download: "Herunterladen".into(),
        share: "Teilen".into(),
        picture_failed: "Das Bild wurde nicht geladen.".into(),
        dismiss: "Verwerfen".into(),
        details: "Details".into(),
        show_more: "Mehr anzeigen".into(),
        loading: "Wird geladen".into(),
        loading_text: "Wird geladen…".into(),
        loading_results: "Ergebnisse werden geladen…".into(),
        no_results: "Keine Ergebnisse".into(),
        no_options: "Keine Optionen".into(),
        search: "Suchen".into(),
        select_all: "Alle auswählen".into(),
        ai: "KI".into(),
        place_of: "{at} von {total}".into(),
        remove_named: "{name} entfernen".into(),
        selected: Plural::same("{count} ausgewählt"),
        more_count: Plural::same("{count} weitere"),
        plus_more: Plural::same("+{count} weitere"),
        and_more: Plural::same("{names} und {count} weitere"),
      },
      "fr" => Self {
        close: "Fermer".into(),
        clear: "Effacer".into(),
        more: "Plus".into(),
        cancel: "Annuler".into(),
        save: "Enregistrer".into(),
        apply: "Appliquer".into(),
        remove: "Supprimer".into(),
        retry: "Réessayer".into(),
        try_again: "Réessayer".into(),
        yes: "Oui".into(),
        no: "Non".into(),
        back: "Retour".into(),
        next: "Suivant".into(),
        done: "Terminé".into(),
        skip: "Passer".into(),
        open: "Ouvrir".into(),
        download: "Télécharger".into(),
        share: "Partager".into(),
        picture_failed: "L’image ne s’est pas chargée.".into(),
        dismiss: "Ignorer".into(),
        details: "Détails".into(),
        show_more: "Afficher plus".into(),
        loading: "Chargement".into(),
        loading_text: "Chargement…".into(),
        loading_results: "Chargement des résultats…".into(),
        no_results: "Aucun résultat".into(),
        no_options: "Aucune option".into(),
        search: "Rechercher".into(),
        select_all: "Tout sélectionner".into(),
        ai: "IA".into(),
        place_of: "{at} sur {total}".into(),
        remove_named: "Supprimer {name}".into(),
        selected: Plural::two("{count} sélectionné", "{count} sélectionnés"),
        more_count: Plural::two("{count} autre", "{count} autres"),
        plus_more: Plural::two("+{count} autre", "+{count} autres"),
        and_more: Plural::two("{names} et {count} autre", "{names} et {count} autres"),
      },
      "es" => Self {
        close: "Cerrar".into(),
        clear: "Borrar".into(),
        more: "Más".into(),
        cancel: "Cancelar".into(),
        save: "Guardar".into(),
        apply: "Aplicar".into(),
        remove: "Quitar".into(),
        retry: "Reintentar".into(),
        try_again: "Reintentar".into(),
        yes: "Sí".into(),
        no: "No".into(),
        back: "Atrás".into(),
        next: "Siguiente".into(),
        done: "Listo".into(),
        skip: "Omitir".into(),
        open: "Abrir".into(),
        download: "Descargar".into(),
        share: "Compartir".into(),
        picture_failed: "No se ha cargado la imagen.".into(),
        dismiss: "Descartar".into(),
        details: "Detalles".into(),
        show_more: "Mostrar más".into(),
        loading: "Cargando".into(),
        loading_text: "Cargando…".into(),
        loading_results: "Cargando resultados…".into(),
        no_results: "Sin resultados".into(),
        no_options: "Sin opciones".into(),
        search: "Buscar".into(),
        select_all: "Seleccionar todo".into(),
        ai: "IA".into(),
        place_of: "{at} de {total}".into(),
        remove_named: "Quitar {name}".into(),
        selected: Plural::two("{count} seleccionado", "{count} seleccionados"),
        more_count: Plural::same("{count} más"),
        plus_more: Plural::same("+{count} más"),
        and_more: Plural::same("{names} y {count} más"),
      },
      "pl" => Self {
        close: "Zamknij".into(),
        clear: "Wyczyść".into(),
        more: "Więcej".into(),
        cancel: "Anuluj".into(),
        save: "Zapisz".into(),
        apply: "Zastosuj".into(),
        remove: "Usuń".into(),
        retry: "Ponów".into(),
        try_again: "Spróbuj ponownie".into(),
        yes: "Tak".into(),
        no: "Nie".into(),
        back: "Wstecz".into(),
        next: "Dalej".into(),
        done: "Gotowe".into(),
        skip: "Pomiń".into(),
        open: "Otwórz".into(),
        download: "Pobierz".into(),
        share: "Udostępnij".into(),
        picture_failed: "Nie udało się wczytać obrazu.".into(),
        dismiss: "Odrzuć".into(),
        details: "Szczegóły".into(),
        show_more: "Pokaż więcej".into(),
        loading: "Ładowanie".into(),
        loading_text: "Ładowanie…".into(),
        loading_results: "Ładowanie wyników…".into(),
        no_results: "Brak wyników".into(),
        no_options: "Brak opcji".into(),
        search: "Szukaj".into(),
        select_all: "Zaznacz wszystko".into(),
        ai: "AI".into(),
        place_of: "{at} z {total}".into(),
        remove_named: "Usuń {name}".into(),
        selected: Plural::same("Zaznaczono: {count}"),
        more_count: Plural::same("jeszcze {count}"),
        plus_more: Plural::same("+{count} więcej"),
        and_more: Plural::same("{names} i jeszcze {count}"),
      },
      _ => Self::english(),
    }
  }
}

impl Default for CommonWords {
  fn default() -> Self {
    Self::english()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn every_language_has_every_word_with_its_places() {
    for language in super::super::locale::LANGUAGES {
      let words = CommonWords::for_language(language);
      assert!(
        words.place_of.contains("{at}") && words.place_of.contains("{total}"),
        "{language}"
      );
      assert!(words.remove_named.contains("{name}"), "{language}");
      for plural in [&words.selected, &words.more_count, &words.plus_more, &words.and_more] {
        assert!(plural.forms().iter().all(|form| form.contains("{count}")), "{language}");
      }
      assert!(
        words.and_more.forms().iter().all(|form| form.contains("{names}")),
        "{language}"
      );
      assert!(!words.close.is_empty() && !words.ai.is_empty(), "{language}");
    }
    assert_eq!(CommonWords::for_language("xx"), CommonWords::english());
  }
}
