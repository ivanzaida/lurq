//! The app's one locale (`core::Locale`): each of its parts of words reaches what the components draw and what
//! a tool reads, once the app sets it, again when it changes, and a word told to a component by itself wins.

use lurq::{
  app::App,
  components::{Column, Row},
  core::Locale,
  node::Element,
  ui::{ActionBar, BarList, BarListItem, CodeBlock, DropZone, Kbd, KeyPlatform, Pagination},
};

use super::stage::Stage;

/// What the element `id` is called to tooling, which keeps it with `mcp` or `devtools`.
#[cfg(any(feature = "devtools", feature = "mcp"))]
fn named(stage: &mut Stage, id: &str) -> Option<String> {
  stage
    .tree
    .get_element_by_id_mut(id)
    .and_then(|element| element.description("a11y_name"))
    .map(|name| name.to_string())
}

/// A stage of `scene` in an app whose locale is `tag`'s, and the app, to set another.
fn in_locale(tag: &str, scene: impl Fn() -> Element + Send + Sync + 'static) -> (Stage, App) {
  let app = App::new();
  app.set_locale(Locale::for_tag(tag));
  let handle = app.clone();
  let stage = Stage::in_app(app, move |_| Column::new().width(480.0).child(scene()).into());
  (stage, handle)
}

#[test]
fn the_common_words_and_a_count_are_said_in_the_apps_locale() {
  let bar = || ActionBar::new(3).on_clear(|| {}).id("bar");
  let (mut stage, app) = in_locale("uk", move || bar().into());
  assert!(stage.paints("Вибрано: 3"), "how many are chosen, in the app's language");
  #[cfg(any(feature = "devtools", feature = "mcp"))]
  assert_eq!(named(&mut stage, "bar-clear").as_deref(), Some("Зняти виділення"));
  // Another locale for the app: what is drawn is drawn again in it, the count in its own form.
  app.set_locale(Locale::for_tag("fr"));
  stage.pass();
  assert!(stage.paints("3 sélectionnés") && !stage.paints("Вибрано: 3"));
  // Outside an app that set a locale, English; a word told by itself is that word.
  let english = Stage::new(move |_| Column::new().width(480.0).child(bar()).into());
  assert!(english.paints("3 selected"));
  let (told, _) = in_locale("uk", move || {
    bar().count_text("Three picked").clear_label("Drop them").into()
  });
  assert!(told.paints("Three picked") && !told.paints("Вибрано: 3"));
}

#[test]
fn the_words_of_fields_are_said_in_the_apps_locale() {
  let (stage, app) = in_locale("pl", || DropZone::new().id("zone").into());
  assert!(stage.paints("Upuść pliki tutaj") && stage.paints("lub kliknij, aby wybrać"));
  let mut stage = stage;
  app.set_locale(Locale::for_tag("de"));
  stage.pass();
  assert!(stage.paints("Dateien hier ablegen") && !stage.paints("Upuść pliki tutaj"));
  let (told, _) = in_locale("pl", || DropZone::new().title("Put it here").into());
  assert!(told.paints("Put it here") && !told.paints("Upuść pliki tutaj"));
}

#[test]
fn the_words_of_ways_about_an_app_are_said_in_the_apps_locale() {
  let pages = || {
    Row::new()
      .child(Pagination::controlled(3, 12, |_| {}).compact(true).id("p"))
      .into()
  };
  let (mut stage, app) = in_locale("de", pages);
  assert!(stage.paints("Seite 3 von 12"));
  #[cfg(any(feature = "devtools", feature = "mcp"))]
  assert_eq!(named(&mut stage, "p-previous").as_deref(), Some("Vorherige Seite"));
  app.set_locale(Locale::for_tag("uk"));
  stage.pass();
  assert!(stage.paints("Сторінка 3 з 12"));
  let (mut told, _) = in_locale("de", || {
    Row::new()
      .child(
        Pagination::controlled(3, 12, |_| {})
          .compact(true)
          .summary_text("Third")
          .previous_label("Back a page")
          .id("p"),
      )
      .into()
  });
  assert!(told.paints("Third") && !told.paints("Seite 3 von 12"));
  #[cfg(any(feature = "devtools", feature = "mcp"))]
  assert_eq!(named(&mut told, "p-previous").as_deref(), Some("Back a page"));
}

#[test]
fn the_words_of_data_are_said_in_the_apps_locale() {
  let empty = || BarList::new(Vec::<BarListItem>::new()).into();
  let (mut stage, app) = in_locale("es", empty);
  assert!(stage.paints("Sin datos"));
  app.set_locale(Locale::for_tag("uk"));
  stage.pass();
  assert!(stage.paints("Немає даних") && !stage.paints("Sin datos"));
  let (told, _) = in_locale("es", || {
    BarList::new(Vec::<BarListItem>::new()).empty_text("Nothing ran").into()
  });
  assert!(told.paints("Nothing ran") && !told.paints("Sin datos"));
  // A count after a number takes the form its language gives it.
  let locale = Locale::for_tag("uk");
  let said: Vec<String> = [1, 3, 5, 21].map(|count| locale.count(&locale.data.rows, count)).into();
  assert_eq!(said, ["1 рядок", "3 рядки", "5 рядків", "21 рядок"]);
}

#[test]
fn the_words_of_writing_are_said_in_the_apps_locale() {
  let block = || CodeBlock::new("let x = 1;").id("block").into();
  let (mut stage, app) = in_locale("fr", block);
  #[cfg(any(feature = "devtools", feature = "mcp"))]
  assert_eq!(named(&mut stage, "block-copy").as_deref(), Some("Copier le code"));
  app.set_locale(Locale::for_tag("pl"));
  stage.pass();
  #[cfg(any(feature = "devtools", feature = "mcp"))]
  assert_eq!(named(&mut stage, "block-copy").as_deref(), Some("Kopiuj kod"));
  let (mut told, _) = in_locale("fr", || {
    CodeBlock::new("let x = 1;").copy_label("Take it").id("block").into()
  });
  #[cfg(any(feature = "devtools", feature = "mcp"))]
  assert_eq!(named(&mut told, "block-copy").as_deref(), Some("Take it"));
  let _ = (&mut stage, &mut told);
}

#[test]
fn the_names_of_keys_are_said_in_the_apps_locale() {
  let keys = || {
    Row::new()
      .child(Kbd::shortcut("Ctrl+Shift+P").platform(KeyPlatform::Standard))
      .child(Kbd::sequence(["G", "D"]).platform(KeyPlatform::Standard))
      .into()
  };
  let (mut stage, app) = in_locale("de", keys);
  assert!(stage.paints("Strg") && stage.paints("Umschalt") && stage.paints("dann"));
  app.set_locale(Locale::for_tag("fr"));
  stage.pass();
  assert!(stage.paints("Maj") && stage.paints("puis") && !stage.paints("Umschalt"));
  // Ukrainian keyboards have the Latin labels.
  app.set_locale(Locale::for_tag("uk"));
  stage.pass();
  assert!(stage.paints("Ctrl") && stage.paints("Shift") && stage.paints("потім"));
  let (told, _) = in_locale("de", || {
    Kbd::sequence(["G", "D"])
      .platform(KeyPlatform::Standard)
      .separator_text("und dann")
      .into()
  });
  assert!(told.paints("und dann"));
}
