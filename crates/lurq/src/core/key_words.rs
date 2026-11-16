//! What keys are called where they are written in words: on the keys of a shortcut, in a shortcut recorder, and
//! to what reads the tree. A Mac's glyphs (⌘, ⇧, ↩) are no words and stay what they are. A part of the app's
//! [`Locale`](super::Locale).

use std::sync::Arc;

/// What keys are called, in a locale's words: the labels on a keyboard of that language ("Strg" on a German one)
/// and the names read out. Every field is the app's to replace.
///
/// ```
/// use lurq::core::Locale;
///
/// assert_eq!(&*Locale::for_tag("de").keys.ctrl, "Strg");
/// assert_eq!(&*Locale::for_tag("fr").keys.shift, "Maj");
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyWords {
  /// The modifiers, as written off a Mac.
  pub ctrl: Arc<str>,
  pub alt: Arc<str>,
  pub shift: Arc<str>,
  pub meta: Arc<str>,
  pub cmd: Arc<str>,
  /// Linux's name of the system key, and Windows'.
  pub super_key: Arc<str>,
  pub win: Arc<str>,
  /// What a Mac calls its Ctrl and its Alt in words.
  pub control: Arc<str>,
  pub option: Arc<str>,
  /// Enter, and what a Mac calls it.
  pub enter: Arc<str>,
  pub return_key: Arc<str>,
  pub esc: Arc<str>,
  pub backspace: Arc<str>,
  /// The label of Delete on a key, and its name.
  pub delete: Arc<str>,
  pub delete_name: Arc<str>,
  pub tab: Arc<str>,
  pub space: Arc<str>,
  /// The names of the arrows, which are drawn as arrows.
  pub up: Arc<str>,
  pub down: Arc<str>,
  pub left: Arc<str>,
  pub right: Arc<str>,
  pub page_up: Arc<str>,
  pub page_down: Arc<str>,
  pub home: Arc<str>,
  pub end: Arc<str>,
  pub caps_lock: Arc<str>,
  /// What stands between the steps of a sequence of keys: "G then D".
  pub then: Arc<str>,
}

impl KeyWords {
  pub fn english() -> Self {
    Self {
      ctrl: "Ctrl".into(),
      alt: "Alt".into(),
      shift: "Shift".into(),
      meta: "Meta".into(),
      cmd: "Cmd".into(),
      super_key: "Super".into(),
      win: "Win".into(),
      control: "Control".into(),
      option: "Option".into(),
      enter: "Enter".into(),
      return_key: "Return".into(),
      esc: "Esc".into(),
      backspace: "Backspace".into(),
      delete: "Del".into(),
      delete_name: "Delete".into(),
      tab: "Tab".into(),
      space: "Space".into(),
      up: "Up".into(),
      down: "Down".into(),
      left: "Left".into(),
      right: "Right".into(),
      page_up: "Page Up".into(),
      page_down: "Page Down".into(),
      home: "Home".into(),
      end: "End".into(),
      caps_lock: "Caps Lock".into(),
      then: "then".into(),
    }
  }

  /// The words of a language (`en`, `uk`, `de`, `fr`, `es`, `pl`); of any other, English.
  pub fn for_language(language: &str) -> Self {
    let english = Self::english();
    match language {
      // A Ukrainian and a Polish keyboard have the Latin labels: only what is said in words changes.
      "uk" => Self {
        space: "Пробіл".into(),
        up: "Вгору".into(),
        down: "Вниз".into(),
        left: "Ліворуч".into(),
        right: "Праворуч".into(),
        then: "потім".into(),
        ..english
      },
      "pl" => Self {
        space: "Spacja".into(),
        up: "W górę".into(),
        down: "W dół".into(),
        left: "W lewo".into(),
        right: "W prawo".into(),
        then: "potem".into(),
        ..english
      },
      "de" => Self {
        ctrl: "Strg".into(),
        shift: "Umschalt".into(),
        enter: "Eingabe".into(),
        return_key: "Zeilenschalter".into(),
        backspace: "Rücktaste".into(),
        delete: "Entf".into(),
        delete_name: "Entfernen".into(),
        space: "Leertaste".into(),
        up: "Nach oben".into(),
        down: "Nach unten".into(),
        left: "Nach links".into(),
        right: "Nach rechts".into(),
        page_up: "Bild auf".into(),
        page_down: "Bild ab".into(),
        home: "Pos1".into(),
        end: "Ende".into(),
        caps_lock: "Feststelltaste".into(),
        then: "dann".into(),
        ..english
      },
      "fr" => Self {
        shift: "Maj".into(),
        meta: "Méta".into(),
        control: "Contrôle".into(),
        enter: "Entrée".into(),
        return_key: "Retour".into(),
        esc: "Échap".into(),
        backspace: "Retour arrière".into(),
        delete: "Suppr".into(),
        delete_name: "Supprimer".into(),
        space: "Espace".into(),
        up: "Haut".into(),
        down: "Bas".into(),
        left: "Gauche".into(),
        right: "Droite".into(),
        page_up: "Page préc.".into(),
        page_down: "Page suiv.".into(),
        home: "Début".into(),
        end: "Fin".into(),
        caps_lock: "Verr. maj.".into(),
        then: "puis".into(),
        ..english
      },
      "es" => Self {
        shift: "Mayús".into(),
        option: "Opción".into(),
        enter: "Intro".into(),
        return_key: "Retorno".into(),
        backspace: "Retroceso".into(),
        delete: "Supr".into(),
        delete_name: "Suprimir".into(),
        space: "Espacio".into(),
        up: "Arriba".into(),
        down: "Abajo".into(),
        left: "Izquierda".into(),
        right: "Derecha".into(),
        page_up: "Re Pág".into(),
        page_down: "Av Pág".into(),
        home: "Inicio".into(),
        end: "Fin".into(),
        caps_lock: "Bloq Mayús".into(),
        then: "luego".into(),
        ..english
      },
      _ => english,
    }
  }
}

impl Default for KeyWords {
  fn default() -> Self {
    Self::english()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn every_language_has_every_key() {
    for language in super::super::locale::LANGUAGES {
      let words = KeyWords::for_language(language);
      assert!(!words.ctrl.is_empty() && !words.then.is_empty() && !words.caps_lock.is_empty(), "{language}");
    }
    assert_eq!(KeyWords::for_language("xx"), KeyWords::english());
    assert_eq!(&*KeyWords::for_language("uk").ctrl, "Ctrl");
  }
}
