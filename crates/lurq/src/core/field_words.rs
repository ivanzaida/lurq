//! What fields, pickers and forms say by themselves: the names of their buttons, the lines they draw, how a
//! password's strength and a rating are said, the words of a color, and the size of a file. A part of the app's
//! [`Locale`](super::Locale).

use std::sync::Arc;

use super::{Plural, fill};

/// What fields, pickers and forms say by themselves, in a locale's words. Every field is the app's to replace.
///
/// ```
/// use lurq::core::Locale;
///
/// let locale = Locale::for_tag("pl");
/// assert_eq!(&*locale.fields.show_password, "Pokaż hasło");
/// assert_eq!(locale.count(&locale.fields.fields_need_attention, 5), "5 pól wymaga uwagi");
/// assert_eq!(Locale::for_tag("fr").fields.size(1_300_000), "1.2 Mo");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldWords {
  /// The mark after the label of a field that may be left empty.
  pub optional: Arc<str>,
  /// What the mark after a label that tells more is called.
  pub more_information: Arc<str>,
  /// What a part of a date or a time with nothing in it is read as.
  pub empty: Arc<str>,
  /// The buttons that step a number up and down.
  pub increase: Arc<str>,
  pub decrease: Arc<str>,
  pub show_password: Arc<str>,
  pub hide_password: Arc<str>,
  /// How strong a password is, from the weakest.
  pub weak_password: Arc<str>,
  pub fair_password: Arc<str>,
  pub good_password: Arc<str>,
  pub strong_password: Arc<str>,
  pub clear_search: Arc<str>,
  /// The button of a path field that opens the system's dialog.
  pub browse: Arc<str>,
  pub choose_files: Arc<str>,
  /// What a drop zone says: its title, the line under it, while files are over it, and when they cannot be added.
  pub drop_files: Arc<str>,
  pub or_browse: Arc<str>,
  pub drop_to_add: Arc<str>,
  pub cannot_add: Arc<str>,
  /// What a shortcut recorder is called, what it says empty and while it records, its clear button, and what it
  /// says of a key pressed with no modifier.
  pub shortcut: Arc<str>,
  pub record_shortcut: Arc<str>,
  pub press_shortcut: Arc<str>,
  pub clear_shortcut: Arc<str>,
  pub shortcut_needs_modifier: Arc<str>,
  /// What a rating is called, and what it is read as with no stars.
  pub rating: Arc<str>,
  pub no_rating: Arc<str>,
  /// A rating read out: `{value}` stars of `{count}`, the form by `{count}`: "3.5 of 5 stars".
  pub stars: Plural,
  /// The title of a form's summary of its errors: "2 fields need attention".
  pub fields_need_attention: Plural,
  /// What a color field and its parts are called.
  pub color: Arc<str>,
  pub choose_color: Arc<str>,
  pub pick_from_screen: Arc<str>,
  /// The channels of a color picker, and its list of formats.
  pub hue: Arc<str>,
  pub saturation: Arc<str>,
  pub brightness: Arc<str>,
  pub lightness: Arc<str>,
  pub alpha: Arc<str>,
  pub red: Arc<str>,
  pub green: Arc<str>,
  pub blue: Arc<str>,
  pub format: Arc<str>,
  /// The square of a color picker, which sets both, and what it is read as, with `{saturation}` and `{brightness}`
  /// for their percentages.
  pub saturation_and_brightness: Arc<str>,
  pub saturation_brightness_value: Arc<str>,
  /// What a property grid is called with no title; its reset button and a picker of a property, with `{name}` for
  /// the property; the handle that sets how wide the names are.
  pub properties: Arc<str>,
  pub reset: Arc<str>,
  pub picker: Arc<str>,
  pub names_width: Arc<str>,
  /// The size of a file in each unit, with `{size}` for the number: "4 KB".
  pub bytes: Arc<str>,
  pub kilobytes: Arc<str>,
  pub megabytes: Arc<str>,
  pub gigabytes: Arc<str>,
  pub terabytes: Arc<str>,
}

impl FieldWords {
  pub fn english() -> Self {
    Self {
      optional: "Optional".into(),
      more_information: "More information".into(),
      empty: "Empty".into(),
      increase: "Increase".into(),
      decrease: "Decrease".into(),
      show_password: "Show password".into(),
      hide_password: "Hide password".into(),
      weak_password: "Weak password".into(),
      fair_password: "Fair password".into(),
      good_password: "Good password".into(),
      strong_password: "Strong password".into(),
      clear_search: "Clear search".into(),
      browse: "Browse…".into(),
      choose_files: "Choose files".into(),
      drop_files: "Drop files here".into(),
      or_browse: "or click to browse".into(),
      drop_to_add: "Drop to add".into(),
      cannot_add: "These files can't be added".into(),
      shortcut: "Shortcut".into(),
      record_shortcut: "Record shortcut".into(),
      press_shortcut: "Press shortcut…".into(),
      clear_shortcut: "Clear shortcut".into(),
      shortcut_needs_modifier: "Hold Ctrl, Alt, Shift or the system key with it.".into(),
      rating: "Rating".into(),
      no_rating: "No rating".into(),
      stars: Plural::two("{value} of {count} star", "{value} of {count} stars"),
      fields_need_attention: Plural::two("{count} field needs attention", "{count} fields need attention"),
      color: "Color".into(),
      choose_color: "Choose color".into(),
      pick_from_screen: "Pick a color from the screen".into(),
      hue: "Hue".into(),
      saturation: "Saturation".into(),
      brightness: "Brightness".into(),
      lightness: "Lightness".into(),
      alpha: "Alpha".into(),
      red: "Red".into(),
      green: "Green".into(),
      blue: "Blue".into(),
      format: "Format".into(),
      saturation_and_brightness: "Saturation and brightness".into(),
      saturation_brightness_value: "Saturation {saturation}%, brightness {brightness}%".into(),
      properties: "Properties".into(),
      reset: "Reset {name}".into(),
      picker: "{name} picker".into(),
      names_width: "Width of the names".into(),
      bytes: "{size} B".into(),
      kilobytes: "{size} KB".into(),
      megabytes: "{size} MB".into(),
      gigabytes: "{size} GB".into(),
      terabytes: "{size} TB".into(),
    }
  }

  /// The words of a language (`en`, `uk`, `de`, `fr`, `es`, `pl`); of any other, English.
  pub fn for_language(language: &str) -> Self {
    match language {
      "uk" => Self {
        optional: "Необов’язково".into(),
        more_information: "Докладніше".into(),
        empty: "Порожньо".into(),
        increase: "Збільшити".into(),
        decrease: "Зменшити".into(),
        show_password: "Показати пароль".into(),
        hide_password: "Приховати пароль".into(),
        weak_password: "Слабкий пароль".into(),
        fair_password: "Посередній пароль".into(),
        good_password: "Добрий пароль".into(),
        strong_password: "Надійний пароль".into(),
        clear_search: "Очистити пошук".into(),
        browse: "Огляд…".into(),
        choose_files: "Вибрати файли".into(),
        drop_files: "Перетягніть файли сюди".into(),
        or_browse: "або натисніть, щоб вибрати".into(),
        drop_to_add: "Відпустіть, щоб додати".into(),
        cannot_add: "Ці файли не можна додати".into(),
        shortcut: "Сполучення клавіш".into(),
        record_shortcut: "Записати сполучення клавіш".into(),
        press_shortcut: "Натисніть сполучення…".into(),
        clear_shortcut: "Очистити сполучення".into(),
        shortcut_needs_modifier: "Утримуйте разом із нею Ctrl, Alt, Shift або системну клавішу.".into(),
        rating: "Оцінка".into(),
        no_rating: "Без оцінки".into(),
        stars: Plural::three(
          "{value} з {count} зірки",
          "{value} з {count} зірок",
          "{value} з {count} зірок",
        ),
        fields_need_attention: Plural::three(
          "{count} поле потребує уваги",
          "{count} поля потребують уваги",
          "{count} полів потребують уваги",
        ),
        color: "Колір".into(),
        choose_color: "Вибрати колір".into(),
        pick_from_screen: "Взяти колір з екрана".into(),
        hue: "Відтінок".into(),
        saturation: "Насиченість".into(),
        brightness: "Яскравість".into(),
        lightness: "Світлість".into(),
        alpha: "Непрозорість".into(),
        red: "Червоний".into(),
        green: "Зелений".into(),
        blue: "Синій".into(),
        format: "Формат".into(),
        saturation_and_brightness: "Насиченість і яскравість".into(),
        saturation_brightness_value: "Насиченість {saturation}%, яскравість {brightness}%".into(),
        properties: "Властивості".into(),
        reset: "Скинути: {name}".into(),
        picker: "Вибір: {name}".into(),
        names_width: "Ширина назв".into(),
        bytes: "{size} Б".into(),
        kilobytes: "{size} КБ".into(),
        megabytes: "{size} МБ".into(),
        gigabytes: "{size} ГБ".into(),
        terabytes: "{size} ТБ".into(),
      },
      "de" => Self {
        optional: "Optional".into(),
        more_information: "Weitere Informationen".into(),
        empty: "Leer".into(),
        increase: "Erhöhen".into(),
        decrease: "Verringern".into(),
        show_password: "Passwort anzeigen".into(),
        hide_password: "Passwort ausblenden".into(),
        weak_password: "Schwaches Passwort".into(),
        fair_password: "Mittelmäßiges Passwort".into(),
        good_password: "Gutes Passwort".into(),
        strong_password: "Starkes Passwort".into(),
        clear_search: "Suche löschen".into(),
        browse: "Durchsuchen…".into(),
        choose_files: "Dateien auswählen".into(),
        drop_files: "Dateien hier ablegen".into(),
        or_browse: "oder klicken, um sie auszuwählen".into(),
        drop_to_add: "Zum Hinzufügen ablegen".into(),
        cannot_add: "Diese Dateien können nicht hinzugefügt werden".into(),
        shortcut: "Tastenkürzel".into(),
        record_shortcut: "Tastenkürzel aufnehmen".into(),
        press_shortcut: "Tastenkürzel drücken…".into(),
        clear_shortcut: "Tastenkürzel löschen".into(),
        shortcut_needs_modifier: "Halten Sie dabei Strg, Alt, Umschalt oder die Systemtaste gedrückt.".into(),
        rating: "Bewertung".into(),
        no_rating: "Keine Bewertung".into(),
        stars: Plural::two("{value} von {count} Stern", "{value} von {count} Sternen"),
        fields_need_attention: Plural::two(
          "{count} Feld erfordert Aufmerksamkeit",
          "{count} Felder erfordern Aufmerksamkeit",
        ),
        color: "Farbe".into(),
        choose_color: "Farbe auswählen".into(),
        pick_from_screen: "Farbe vom Bildschirm aufnehmen".into(),
        hue: "Farbton".into(),
        saturation: "Sättigung".into(),
        brightness: "Helligkeit".into(),
        lightness: "Hellwert".into(),
        alpha: "Deckkraft".into(),
        red: "Rot".into(),
        green: "Grün".into(),
        blue: "Blau".into(),
        format: "Format".into(),
        saturation_and_brightness: "Sättigung und Helligkeit".into(),
        saturation_brightness_value: "Sättigung {saturation} %, Helligkeit {brightness} %".into(),
        properties: "Eigenschaften".into(),
        reset: "{name} zurücksetzen".into(),
        picker: "Auswahl für {name}".into(),
        names_width: "Breite der Namen".into(),
        bytes: "{size} B".into(),
        kilobytes: "{size} KB".into(),
        megabytes: "{size} MB".into(),
        gigabytes: "{size} GB".into(),
        terabytes: "{size} TB".into(),
      },
      "fr" => Self {
        optional: "Facultatif".into(),
        more_information: "Plus d’informations".into(),
        empty: "Vide".into(),
        increase: "Augmenter".into(),
        decrease: "Diminuer".into(),
        show_password: "Afficher le mot de passe".into(),
        hide_password: "Masquer le mot de passe".into(),
        weak_password: "Mot de passe faible".into(),
        fair_password: "Mot de passe moyen".into(),
        good_password: "Bon mot de passe".into(),
        strong_password: "Mot de passe fort".into(),
        clear_search: "Effacer la recherche".into(),
        browse: "Parcourir…".into(),
        choose_files: "Choisir des fichiers".into(),
        drop_files: "Déposez les fichiers ici".into(),
        or_browse: "ou cliquez pour parcourir".into(),
        drop_to_add: "Déposez pour ajouter".into(),
        cannot_add: "Impossible d’ajouter ces fichiers".into(),
        shortcut: "Raccourci".into(),
        record_shortcut: "Enregistrer un raccourci".into(),
        press_shortcut: "Appuyez sur le raccourci…".into(),
        clear_shortcut: "Effacer le raccourci".into(),
        shortcut_needs_modifier: "Maintenez en même temps Ctrl, Alt, Maj ou la touche système.".into(),
        rating: "Note".into(),
        no_rating: "Aucune note".into(),
        stars: Plural::two("{value} sur {count} étoile", "{value} sur {count} étoiles"),
        fields_need_attention: Plural::two("{count} champ à vérifier", "{count} champs à vérifier"),
        color: "Couleur".into(),
        choose_color: "Choisir une couleur".into(),
        pick_from_screen: "Prélever une couleur à l’écran".into(),
        hue: "Teinte".into(),
        saturation: "Saturation".into(),
        brightness: "Luminosité".into(),
        lightness: "Clarté".into(),
        alpha: "Opacité".into(),
        red: "Rouge".into(),
        green: "Vert".into(),
        blue: "Bleu".into(),
        format: "Format".into(),
        saturation_and_brightness: "Saturation et luminosité".into(),
        saturation_brightness_value: "Saturation {saturation} %, luminosité {brightness} %".into(),
        properties: "Propriétés".into(),
        reset: "Réinitialiser {name}".into(),
        picker: "Sélecteur : {name}".into(),
        names_width: "Largeur des noms".into(),
        bytes: "{size} o".into(),
        kilobytes: "{size} Ko".into(),
        megabytes: "{size} Mo".into(),
        gigabytes: "{size} Go".into(),
        terabytes: "{size} To".into(),
      },
      "es" => Self {
        optional: "Opcional".into(),
        more_information: "Más información".into(),
        empty: "Vacío".into(),
        increase: "Aumentar".into(),
        decrease: "Disminuir".into(),
        show_password: "Mostrar contraseña".into(),
        hide_password: "Ocultar contraseña".into(),
        weak_password: "Contraseña débil".into(),
        fair_password: "Contraseña aceptable".into(),
        good_password: "Contraseña buena".into(),
        strong_password: "Contraseña segura".into(),
        clear_search: "Borrar búsqueda".into(),
        browse: "Examinar…".into(),
        choose_files: "Elegir archivos".into(),
        drop_files: "Suelta los archivos aquí".into(),
        or_browse: "o haz clic para examinar".into(),
        drop_to_add: "Suelta para añadir".into(),
        cannot_add: "No se pueden añadir estos archivos".into(),
        shortcut: "Atajo".into(),
        record_shortcut: "Grabar atajo".into(),
        press_shortcut: "Pulsa el atajo…".into(),
        clear_shortcut: "Borrar atajo".into(),
        shortcut_needs_modifier: "Mantén pulsada a la vez Ctrl, Alt, Mayús o la tecla del sistema.".into(),
        rating: "Valoración".into(),
        no_rating: "Sin valoración".into(),
        stars: Plural::two("{value} de {count} estrella", "{value} de {count} estrellas"),
        fields_need_attention: Plural::two("{count} campo requiere atención", "{count} campos requieren atención"),
        color: "Color".into(),
        choose_color: "Elegir color".into(),
        pick_from_screen: "Tomar un color de la pantalla".into(),
        hue: "Tono".into(),
        saturation: "Saturación".into(),
        brightness: "Brillo".into(),
        lightness: "Luminosidad".into(),
        alpha: "Opacidad".into(),
        red: "Rojo".into(),
        green: "Verde".into(),
        blue: "Azul".into(),
        format: "Formato".into(),
        saturation_and_brightness: "Saturación y brillo".into(),
        saturation_brightness_value: "Saturación {saturation} %, brillo {brightness} %".into(),
        properties: "Propiedades".into(),
        reset: "Restablecer {name}".into(),
        picker: "Selector de {name}".into(),
        names_width: "Ancho de los nombres".into(),
        bytes: "{size} B".into(),
        kilobytes: "{size} KB".into(),
        megabytes: "{size} MB".into(),
        gigabytes: "{size} GB".into(),
        terabytes: "{size} TB".into(),
      },
      "pl" => Self {
        optional: "Opcjonalne".into(),
        more_information: "Więcej informacji".into(),
        empty: "Puste".into(),
        increase: "Zwiększ".into(),
        decrease: "Zmniejsz".into(),
        show_password: "Pokaż hasło".into(),
        hide_password: "Ukryj hasło".into(),
        weak_password: "Słabe hasło".into(),
        fair_password: "Przeciętne hasło".into(),
        good_password: "Dobre hasło".into(),
        strong_password: "Silne hasło".into(),
        clear_search: "Wyczyść wyszukiwanie".into(),
        browse: "Przeglądaj…".into(),
        choose_files: "Wybierz pliki".into(),
        drop_files: "Upuść pliki tutaj".into(),
        or_browse: "lub kliknij, aby wybrać".into(),
        drop_to_add: "Upuść, aby dodać".into(),
        cannot_add: "Nie można dodać tych plików".into(),
        shortcut: "Skrót".into(),
        record_shortcut: "Nagraj skrót".into(),
        press_shortcut: "Naciśnij skrót…".into(),
        clear_shortcut: "Wyczyść skrót".into(),
        shortcut_needs_modifier: "Przytrzymaj przy tym Ctrl, Alt, Shift lub klawisz systemowy.".into(),
        rating: "Ocena".into(),
        no_rating: "Brak oceny".into(),
        stars: Plural::three(
          "{value} z {count} gwiazdki",
          "{value} z {count} gwiazdek",
          "{value} z {count} gwiazdek",
        ),
        fields_need_attention: Plural::three(
          "{count} pole wymaga uwagi",
          "{count} pola wymagają uwagi",
          "{count} pól wymaga uwagi",
        ),
        color: "Kolor".into(),
        choose_color: "Wybierz kolor".into(),
        pick_from_screen: "Pobierz kolor z ekranu".into(),
        hue: "Odcień".into(),
        saturation: "Nasycenie".into(),
        brightness: "Jasność".into(),
        lightness: "Luminancja".into(),
        alpha: "Krycie".into(),
        red: "Czerwony".into(),
        green: "Zielony".into(),
        blue: "Niebieski".into(),
        format: "Format".into(),
        saturation_and_brightness: "Nasycenie i jasność".into(),
        saturation_brightness_value: "Nasycenie {saturation}%, jasność {brightness}%".into(),
        properties: "Właściwości".into(),
        reset: "Resetuj: {name}".into(),
        picker: "Wybór: {name}".into(),
        names_width: "Szerokość nazw".into(),
        bytes: "{size} B".into(),
        kilobytes: "{size} KB".into(),
        megabytes: "{size} MB".into(),
        gigabytes: "{size} GB".into(),
        terabytes: "{size} TB".into(),
      },
      _ => Self::english(),
    }
  }

  /// A size in bytes as a person reads it, in these words' units: "512 B", "4 KB", "1.2 MB". One decimal under
  /// ten, where it says something; the number is written with a point in every language.
  pub fn size(&self, bytes: u64) -> String {
    let units = [&self.kilobytes, &self.megabytes, &self.gigabytes, &self.terabytes];
    if bytes < 1024 {
      return fill(&self.bytes, &[("size", &bytes.to_string())]);
    }
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit < units.len() - 1 {
      value /= 1024.0;
      unit += 1;
    }
    let tenths = (value * 10.0).round() / 10.0;
    let number = if tenths < 10.0 && tenths.fract() != 0.0 {
      format!("{tenths:.1}")
    } else {
      (value.round() as u64).to_string()
    };
    fill(units[unit], &[("size", &number)])
  }
}

impl Default for FieldWords {
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
      let words = FieldWords::for_language(language);
      for unit in [
        &words.bytes,
        &words.kilobytes,
        &words.megabytes,
        &words.gigabytes,
        &words.terabytes,
      ] {
        assert!(unit.contains("{size}"), "{language}");
      }
      for word in [&words.reset, &words.picker] {
        assert!(word.contains("{name}"), "{language}");
      }
      let value = &words.saturation_brightness_value;
      assert!(
        value.contains("{saturation}") && value.contains("{brightness}"),
        "{language}"
      );
      assert!(
        words
          .stars
          .forms()
          .iter()
          .all(|form| form.contains("{value}") && form.contains("{count}"))
      );
      assert!(
        words
          .fields_need_attention
          .forms()
          .iter()
          .all(|form| form.contains("{count}"))
      );
      assert!(!words.optional.is_empty() && !words.terabytes.is_empty(), "{language}");
    }
    assert_eq!(FieldWords::for_language("xx"), FieldWords::english());
  }

  #[test]
  fn a_size_is_said_in_the_units_of_its_language() {
    let english = FieldWords::english();
    assert_eq!(english.size(512), "512 B");
    assert_eq!(english.size(4096), "4 KB");
    assert_eq!(english.size(1_300_000), "1.2 MB");
    assert_eq!(FieldWords::for_language("uk").size(3 * 1024 * 1024 * 1024), "3 ГБ");
  }
}
