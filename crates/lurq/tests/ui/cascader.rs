//! A cascader: its trigger and its columns against the board, opening a column by a press, by the pointer and by
//! the keys, choosing a leaf and where the focus goes on each way out, a choice of any level, lazy children, the
//! clear control, what the handlers are told and when, a disabled and a read only one, the density, a text longer
//! than its room, a fluid field, tooling, and a theme's other anatomy.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    ctx::Ctx,
    theme::{Density, PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  core::Signal,
  layout::{quad::QuadContent, text_style::FontWeight},
  node::{Element, color::Color},
  ui::{Button, Cascader, CascaderPart, Field, TreeItem},
};

use super::stage::Stage;

type Id = &'static str;

const CHEVRON_RIGHT: &str = "\u{E01B}";

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn close_to(a: f32, b: f32) -> bool {
  (a - b).abs() < 0.5
}

fn item(id: Id, label: &str) -> TreeItem<Id> {
  TreeItem::new(id, label).id(id)
}

/// The board's places: Asia, Europe (Poland, Ukraine with Kyiv, Lviv and Odesa, Spain), Americas.
fn places() -> Vec<TreeItem<Id>> {
  vec![
    item("asia", "Asia").items([item("tokyo", "Tokyo"), item("seoul", "Seoul")]),
    item("europe", "Europe").items([
      item("poland", "Poland").items([item("warsaw", "Warsaw"), item("krakow", "Krakow")]),
      item("ukraine", "Ukraine").items([item("kyiv", "Kyiv"), item("lviv", "Lviv"), item("odesa", "Odesa")]),
      item("spain", "Spain").item(item("madrid", "Madrid")),
    ]),
    item("americas", "Americas").item(item("peru", "Peru").item(item("lima", "Lima"))),
  ]
}

type Told = Arc<Mutex<Vec<String>>>;

fn said(told: &Told) -> Vec<String> {
  std::mem::take(&mut *told.lock().unwrap())
}

fn focused(stage: &Stage) -> Option<String> {
  stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned))
}

fn weight_of(stage: &Stage, content: &str) -> FontWeight {
  stage
    .quads()
    .iter()
    .find_map(|quad| match &quad.content {
      QuadContent::Text { text, style, .. } if text == content => Some(style.weight),
      _ => None,
    })
    .unwrap_or_else(|| panic!("no text {content:?} was painted"))
}

/// A stage of `cascader`, 8 in from the corner of a window of 640 by 400 with room for its panel, with a button
/// beside it.
fn staged(cascader: impl Fn(&mut Ctx) -> Element + Send + Sync + 'static) -> Stage {
  let mut stage = Stage::first_frame(640.0, 400.0, move |ctx| {
    Row::new()
      .size(640.0, 400.0)
      .padding(8.0)
      .spacing(8.0)
      .align_items(lurq::layout::Alignment::Start)
      .child(Column::new().width(520.0).child(cascader(ctx)))
      .child(Button::new("After").id("after"))
      .into()
  });
  stage.pass();
  stage
}

/// A mounted cascader of one or none bound to a signal, which tells `told` what its handlers hear.
fn bound(
  value: Option<Id>,
  dress: fn(Cascader<Id, Option<Id>>) -> Cascader<Id, Option<Id>>,
) -> (Stage, Signal<Option<Id>>, Told) {
  let value = Signal::new(value);
  let told: Told = Arc::default();
  let (held, heard) = (value.clone(), told.clone());
  let stage = staged(move |ctx| {
    let (change, open, dismiss, load) = (heard.clone(), heard.clone(), heard.clone(), heard.clone());
    let seen = held.clone();
    dress(
      Cascader::optional(held.clone(), places())
        .label("Place")
        .placeholder("Select a city")
        .id("place")
        .on_change(move |value| {
          change
            .lock()
            .unwrap()
            .push(format!("change {value:?} (the signal has {:?})", seen.get_untracked()))
        })
        .on_open_change(move |is| open.lock().unwrap().push(format!("open {is}")))
        .on_dismiss(move || dismiss.lock().unwrap().push("dismiss".to_owned()))
        .on_load(move |item| load.lock().unwrap().push(format!("load {item}"))),
    )
    .mount(ctx)
  });
  (stage, value, told)
}

fn plain(cascader: Cascader<Id, Option<Id>>) -> Cascader<Id, Option<Id>> {
  cascader
}

#[test]
fn the_trigger_and_the_columns_are_the_boards() {
  let mut stage = staged(|_| {
    Cascader::controlled("kyiv", places(), |_| {})
      .inline(true)
      .id("place")
      .into()
  });
  // The trigger: a select's box, 260 by 36, the path in it 12 in at 14.
  assert_eq!(stage.bounds("place"), (8.0, 8.0, 260.0, 36.0));
  assert_eq!(stage.fill_color("place"), Some(light(PaletteColor::Background)));
  let text = stage
    .text_quad("Europe / Ukraine / Kyiv")
    .expect("the path to what is chosen");
  assert_eq!(text.x, 20.0);
  assert_eq!(
    stage.text("Europe / Ukraine / Kyiv"),
    (light(PaletteColor::Foreground), 14.0)
  );
  // The panel: 4 under the trigger on the popover's fill, three columns of 124 side by side.
  let (x, y, width, height) = stage.bounds("place-panel");
  assert_eq!((x, y, width), (8.0, 48.0, 372.0));
  assert_eq!(stage.fill_color("place-panel"), Some(light(PaletteColor::Popover)));
  // As high as the longest column: three options of 30 and 4 over and under them.
  assert!(close_to(height, 3.0 * 30.0 + 8.0), "{height}");
  for (column, left) in [(0, 8.0), (1, 132.0), (2, 256.0)] {
    let (cx, _, cw, _) = stage.bounds(&format!("place-columns-column-{column}"));
    assert!(close_to(cx, left) && close_to(cw, 124.0), "column {column}: {cx} {cw}");
  }
  // An option: 30 high, 4 in from its column, 8 to its label at 13; on the path in the highlight at 500.
  let (ox, oy, ow, oh) = stage.bounds("asia");
  assert!(close_to(ox, 12.0) && close_to(oy, 52.0) && close_to(ow, 116.0) && close_to(oh, 30.0));
  assert!(close_to(stage.text_quad("Asia").unwrap().x, 20.0));
  assert_eq!(stage.text("Asia"), (light(PaletteColor::Foreground), 13.0));
  for on_path in ["europe", "ukraine", "kyiv"] {
    assert_eq!(
      stage.fill_color(on_path),
      Some(light(PaletteColor::Highlight)),
      "{on_path}"
    );
  }
  for label in ["Europe", "Ukraine", "Kyiv"] {
    assert_eq!(weight_of(&stage, label), FontWeight::Medium, "{label}");
  }
  assert_eq!(weight_of(&stage, "Lviv"), FontWeight::Normal);
  assert_eq!(
    stage.text("Kyiv").0,
    light(PaletteColor::Foreground),
    "the last as the others"
  );
  // A chevron of 14 at the end of each option that holds others: three continents and three countries.
  let chevrons: Vec<_> = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if text == CHEVRON_RIGHT))
    .collect();
  assert_eq!(chevrons.len(), 6);
  assert!(chevrons.iter().all(|quad| close_to(quad.width, 14.0)));
}

#[test]
fn a_press_opens_columns_and_a_leaf_chooses_closes_and_gives_the_focus_back() {
  let (mut stage, value, told) = bound(Some("kyiv"), plain);
  assert!(!stage.paints("Lviv"));
  stage.click("place");
  assert_eq!(said(&told), ["open true"]);
  // The columns of the path to what is chosen, and the focus on it.
  assert!(stage.paints("Lviv") && stage.paints("Poland"));
  assert_eq!(focused(&stage).as_deref(), Some("kyiv"));
  // A press of an option that holds others opens its column, and chooses nothing.
  stage.click("poland");
  assert!(stage.paints("Warsaw") && !stage.paints("Lviv"));
  assert!(said(&told).is_empty());
  assert_eq!(value.get(), Some("kyiv"));
  // A press of a leaf chooses it: the panel closes first, then the handler hears the value the signal has.
  stage.click("warsaw");
  assert_eq!(value.get(), Some("warsaw"));
  assert_eq!(
    said(&told),
    ["open false", r#"change Some("warsaw") (the signal has Some("warsaw"))"#]
  );
  assert!(!stage.paints("Krakow"));
  assert!(stage.paints("Europe / Poland / Warsaw"));
  assert_eq!(focused(&stage).as_deref(), Some("place"));
  // What the app sets is shown, and told to nobody.
  value.set(Some("lima"));
  stage.pass();
  assert!(stage.paints("Americas / Peru / Lima"));
  assert!(said(&told).is_empty());
}

#[test]
fn the_keys_open_the_panel_go_across_the_columns_and_choose() {
  let (mut stage, value, told) = bound(Some("kyiv"), plain);
  stage.key("Tab");
  assert_eq!(focused(&stage).as_deref(), Some("place"));
  stage.key("ArrowDown");
  assert_eq!(said(&told), ["open true"]);
  assert_eq!(focused(&stage).as_deref(), Some("kyiv"));
  // Left back to the country, Up to the one before, whose column opens; Right into it.
  stage.key("ArrowLeft");
  assert_eq!(focused(&stage).as_deref(), Some("ukraine"));
  stage.key("ArrowUp");
  assert_eq!(focused(&stage).as_deref(), Some("poland"));
  assert!(stage.paints("Warsaw"));
  stage.key("ArrowRight");
  assert_eq!(focused(&stage).as_deref(), Some("warsaw"));
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("krakow"));
  // Moving chooses nothing; Enter on a leaf does, and closes the panel.
  assert_eq!(value.get(), Some("kyiv"));
  stage.key("Enter");
  assert_eq!(value.get(), Some("krakow"));
  assert_eq!(focused(&stage).as_deref(), Some("place"));
  // Enter on an option that holds others goes into it.
  stage.key("ArrowDown");
  stage.key("ArrowLeft");
  stage.key("ArrowLeft");
  assert_eq!(focused(&stage).as_deref(), Some("europe"));
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("americas"));
  stage.key("Enter");
  assert_eq!(focused(&stage).as_deref(), Some("peru"));
  assert_eq!(value.get(), Some("krakow"));
  // Escape closes it, chooses nothing and gives the focus back.
  said(&told);
  stage.key("Escape");
  assert_eq!(said(&told), ["dismiss", "open false"]);
  assert_eq!(focused(&stage).as_deref(), Some("place"));
  assert_eq!(value.get(), Some("krakow"));
  // Opened again, it is on what is chosen, not on where it was left.
  stage.key("ArrowDown");
  assert_eq!(focused(&stage).as_deref(), Some("krakow"));
}

#[test]
fn of_any_level_an_option_that_holds_others_is_chosen_and_the_panel_stays_open() {
  let (mut stage, value, told) = bound(None, |cascader| cascader.leaves_only(false));
  stage.click("place");
  said(&told);
  stage.click("europe");
  assert_eq!(value.get(), Some("europe"));
  assert!(stage.paints("Ukraine"), "its column is open");
  assert_eq!(
    said(&told),
    [r#"change Some("europe") (the signal has Some("europe"))"#]
  );
  // A leaf closes it.
  stage.click("ukraine");
  stage.click("lviv");
  assert_eq!(value.get(), Some("lviv"));
  assert!(!stage.paints("Odesa"));
}

#[test]
fn the_pointer_opens_a_column_where_it_is_asked_to() {
  let (mut stage, value, _) = bound(None, |cascader| cascader.open_on_hover(true));
  stage.click("place");
  stage.hover("europe");
  assert!(stage.paints("Ukraine"));
  stage.hover("ukraine");
  assert!(stage.paints("Odesa"));
  // A leaf under the pointer chooses nothing.
  stage.hover("odesa");
  assert_eq!(value.get(), None);
  assert!(stage.paints("Odesa"));
}

#[test]
fn a_lazy_item_opens_a_column_with_a_spinner_and_the_app_is_told() {
  let told: Told = Arc::default();
  let loaded = Signal::new(false);
  let (heard, given) = (told.clone(), loaded.clone());
  let mut stage = staged(move |ctx| {
    let heard = heard.clone();
    let mut later = item("later", "Later").lazy(true);
    if given.get() {
      later = later.item(item("soon", "Soon"));
    }
    Cascader::optional(Signal::new(None), [item("now", "Now"), later])
      .id("place")
      .on_load(move |item| heard.lock().unwrap().push(format!("load {item}")))
      .mount(ctx)
  });
  stage.click("place");
  stage.click("later");
  assert_eq!(said(&told), ["load later"]);
  assert!(
    stage.bounds("place-columns-column-1").2 > 0.0,
    "a column for what it holds"
  );
  loaded.set(true);
  stage.pass();
  assert!(stage.paints("Soon"));
}

#[test]
fn the_cross_clears_what_is_chosen() {
  let (mut stage, value, told) = bound(Some("kyiv"), |cascader| cascader.clearable(true));
  stage.click("place-clear");
  assert_eq!(value.get(), None);
  assert_eq!(said(&told), ["change None (the signal has None)"]);
  assert!(stage.paints("Select a city"));
}

#[test]
fn a_disabled_or_read_only_cascader_does_not_open() {
  for dress in [
    (|cascader: Cascader<Id, Option<Id>>| cascader.disabled(true)) as fn(_) -> _,
    |cascader| cascader.read_only(true),
  ] {
    let (mut stage, _, told) = bound(Some("kyiv"), dress);
    stage.click("place");
    assert!(said(&told).is_empty());
    assert!(!stage.paints("Lviv"));
  }
}

#[test]
fn an_options_height_follows_the_density() {
  for (density, height) in [
    (Density::Compact, 22.0),
    (Density::Default, 30.0),
    (Density::Comfortable, 38.0),
    (Density::Touch, 42.0),
  ] {
    let app = App::new();
    app.theme().set_density(density);
    let mut stage = Stage::first_frame_in(app, 640.0, 400.0, |_| {
      Column::new()
        .size(640.0, 400.0)
        .child(Cascader::controlled("kyiv", places(), |_| {}).inline(true).id("place"))
        .into()
    });
    stage.pass();
    assert_eq!(stage.bounds("asia").3, height, "{density:?}");
    // The columns as high as three options and their room.
    assert!(
      close_to(stage.bounds("place-panel").3, 3.0 * height + 8.0),
      "{density:?}"
    );
  }
}

#[test]
fn a_path_longer_than_the_trigger_ends_in_an_ellipsis() {
  let mut stage = staged(|_| {
    Cascader::controlled(
      "deep",
      [item("a", "A continent with a long name")
        .item(item("b", "A country with a longer name").item(item("deep", "And a city whose name is longest")))],
      |_| {},
    )
    .id("place")
    .into()
  });
  let (x, _, width, _) = stage.bounds("place");
  let quad = stage
    .cut_text_quad("A continent with a long name / A country with a longer name / And a city whose name is longest")
    .expect("the path, cut");
  assert!(
    quad.x + quad.width <= x + width - 12.0 - 16.0 - 8.0 + 0.5,
    "it leaves the chevron its room"
  );
}

#[test]
fn in_a_fluid_field_the_trigger_is_the_fluid_box() {
  let mut stage = staged(|ctx| {
    Field::new("Place", Cascader::controlled("kyiv", places(), |_| {}).id("place"))
      .fluid(true)
      .mount(ctx)
  });
  let (_, y, _, height) = stage.bounds("place");
  assert_eq!(height, 56.0);
  // The path on the line under the field's label.
  let path = stage.text_quad("Europe / Ukraine / Kyiv").unwrap();
  let label = stage.text_quad("Place").unwrap();
  assert!(path.y > label.y + label.height - 0.5 && path.y > y + 20.0);
}

#[test]
fn a_theme_restyles_its_parts() {
  let mut stage = staged(|_| {
    Cascader::controlled("kyiv", places(), |_| {})
      .inline(true)
      .style(CascaderPart::Column, |s| s.min_width(150.0))
      .style(CascaderPart::Option, |s| s.height(36.0))
      .id("place")
      .into()
  });
  assert_eq!(stage.bounds("place-panel").2, 450.0);
  assert_eq!(stage.bounds("asia").3, 36.0);
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn tooling_reads_a_combobox_that_opens_listboxes_of_options() {
  let (mut stage, ..) = bound(Some("kyiv"), plain);
  stage.click("place");
  let mut said = |id: &str, what: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description(what))
  };
  assert_eq!(said("place", "a11y_role").as_deref(), Some("combobox"));
  assert_eq!(said("place", "a11y_haspopup").as_deref(), Some("listbox"));
  assert_eq!(said("place", "a11y_expanded").as_deref(), Some("true"));
  assert_eq!(said("place", "a11y_value").as_deref(), Some("Europe / Ukraine / Kyiv"));
  assert_eq!(said("place-columns-column-1", "a11y_role").as_deref(), Some("listbox"));
  assert_eq!(said("place-columns-column-1", "a11y_name").as_deref(), Some("Europe"));
  assert_eq!(said("ukraine", "a11y_selected").as_deref(), Some("true"));
  assert_eq!(said("ukraine", "a11y_expanded").as_deref(), Some("true"));
  assert_eq!(said("poland", "a11y_selected").as_deref(), Some("false"));
}

/// Its words are the app's locale's, read again when the app changes it; a locale of its own is above the app's, and
/// a word told by itself is above both. The mark between the steps of the path is no word.
#[test]
fn its_words_are_the_app_s_locale_s_and_a_word_told_by_itself_stays() {
  use lurq::core::Locale;
  let app = App::new();
  app.set_locale(Locale::for_tag("uk"));
  let handle = app.clone();
  let mut stage = Stage::first_frame_in(app, 640.0, 400.0, |_| {
    Column::new()
      .padding(8.0)
      .child(
        Cascader::controlled_optional(None::<Id>, Vec::<TreeItem<Id>>::new(), |_| {})
          .inline(true)
          .id("empty"),
      )
      .child(
        Cascader::controlled_optional(Some("kyiv"), places(), |_| {})
          .clearable(true)
          .id("place"),
      )
      .into()
  });
  stage.pass();
  assert!(stage.paints("Немає варіантів"));
  assert!(stage.paints("Europe / Ukraine / Kyiv"));
  #[cfg(any(feature = "mcp", feature = "devtools"))]
  assert_eq!(
    stage
      .tree
      .get_element_by_id_mut("place-clear")
      .and_then(|element| element.description("a11y_name"))
      .as_deref(),
    Some("Очистити")
  );
  handle.set_locale(Locale::for_tag("fr"));
  stage.pass();
  assert!(stage.paints("Aucune option") && !stage.paints("Немає варіантів"));
  #[cfg(any(feature = "mcp", feature = "devtools"))]
  assert_eq!(
    stage
      .tree
      .get_element_by_id_mut("place-clear")
      .and_then(|element| element.description("a11y_name"))
      .as_deref(),
    Some("Effacer")
  );

  // A locale of its own, in an app of none, and a word told by itself, whichever comes first.
  let own = staged(|_| {
    Column::new()
      .child(
        Cascader::controlled_optional(None::<Id>, Vec::<TreeItem<Id>>::new(), |_| {})
          .empty_text("Nada")
          .locale(&Locale::for_tag("es"))
          .inline(true),
      )
      .child(
        Cascader::controlled_optional(None::<Id>, Vec::<TreeItem<Id>>::new(), |_| {})
          .locale(&Locale::for_tag("es"))
          .inline(true),
      )
      .into()
  });
  assert!(own.paints("Nada") && own.paints("Sin opciones"));
  // With no locale it is English.
  let plain = staged(|_| {
    Cascader::controlled_optional(None::<Id>, Vec::<TreeItem<Id>>::new(), |_| {})
      .inline(true)
      .into()
  });
  assert!(plain.paints("No options"));
}
