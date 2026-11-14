//! `ui::Icon`: a glyph of the theme's set by name, in the foreground of what
//! it is in.

use lurq::{
  app::{
    App,
    theme::{IconGlyph, IconSize, PaletteColor, ThemeIcons, ThemePalette},
  },
  components::{Column, Row},
  node::color::Color,
  ui::{Icon, SPIN_PERIOD, icons},
};

use super::stage::Stage;

const RED: Color = Color::new(200, 30, 30, 255);

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// The character lurq's font has for an icon.
fn glyph(name: &str) -> String {
  icons::ALL
    .iter()
    .find_map(|(known, glyph)| (*known == name).then(|| glyph.to_string()))
    .unwrap_or_else(|| panic!("lurq's font has no icon {name:?}"))
}

#[test]
fn an_icon_is_a_glyph_of_lurqs_font_16_high_in_the_themes_foreground() {
  let mut stage = Stage::new(|_| Row::new().child(Icon::new(icons::SEARCH).id("search")).into());
  let search = glyph(icons::SEARCH);
  let (ink, size) = stage.text(&search);
  assert_eq!(size, 16.0, "the middle step of the icon sizes");
  assert_eq!(ink, light(PaletteColor::Foreground));
  assert_eq!(stage.font_of(&search), icons::FAMILY, "the font every app has loaded");
  let (_, _, width, height) = stage.bounds("search");
  assert_eq!((width, height), (16.0, 16.0), "a square of its size");
}

#[test]
fn an_icon_takes_the_foreground_of_what_it_is_in_unless_it_names_a_color() {
  let stage = Stage::new(|_| {
    Row::new()
      .foreground(RED)
      .child(Icon::new(icons::CHECK))
      .child(Icon::new(icons::CLOSE).color(PaletteColor::Success))
      .into()
  });
  assert_eq!(stage.text(&glyph(icons::CHECK)).0, RED);
  assert_eq!(stage.text(&glyph(icons::CLOSE)).0, light(PaletteColor::Success));
}

#[test]
fn an_icon_is_a_step_of_the_themes_sizes_or_a_number_of_pixels() {
  let app = App::new();
  app.theme().set_icon_size_value(IconSize::Xl, 28.0);
  let mut stage = Stage::in_app(app, |_| {
    Column::new()
      .child(Icon::new(icons::PLUS).size(IconSize::Xs).id("xs"))
      .child(Icon::new(icons::MINUS).size(IconSize::Xl).id("xl"))
      .child(Icon::new(icons::CLOSE).size(18.0).id("px"))
      .into()
  });
  let sides = ["xs", "xl", "px"].map(|id| stage.bounds(id).2);
  assert_eq!(sides, [12.0, 28.0, 18.0], "the theme's step, as the theme has it");
  assert_eq!(stage.text(&glyph(icons::CLOSE)).1, 18.0);
}

#[test]
fn a_theme_draws_a_name_with_another_glyph_and_adds_names_of_its_own() {
  let app = App::new();
  let theme = app.theme().clone();
  theme.set_icon("rocket", IconGlyph::new("brand icons", 'R'));
  let mut stage = Stage::in_app(app, |_| {
    Row::new()
      .child(Icon::new("rocket"))
      .child(Icon::new(icons::SEARCH))
      .into()
  });
  assert_eq!(stage.font_of("R"), "brand icons", "a name the theme added");
  assert!(stage.paints(&glyph(icons::SEARCH)));

  // The theme changes while the app runs: the icon on screen is drawn again.
  theme.set_icon(icons::SEARCH, IconGlyph::new("brand icons", 'S'));
  stage.pass();
  assert!(stage.paints("S"));
  assert!(!stage.paints(&glyph(icons::SEARCH)));
}

#[test]
fn a_name_the_theme_has_no_glyph_for_takes_its_room_and_draws_nothing() {
  let app = App::new();
  app.theme().set_icons(ThemeIcons::empty());
  let mut stage = Stage::in_app(app, |_| {
    Row::new()
      .child(Icon::new(icons::SEARCH).size(IconSize::Lg).id("gone"))
      .child(Icon::new(icons::CHECK).id("after"))
      .into()
  });
  assert!(!stage.paints(&glyph(icons::SEARCH)));
  let (x, _, width, height) = stage.bounds("gone");
  assert_eq!((width, height), (20.0, 20.0));
  assert_eq!(
    stage.bounds("after").0,
    x + 20.0,
    "what is after it is where it would be"
  );
}

#[test]
fn a_spinning_icon_turns_about_its_middle_once_in_its_period() {
  let mut stage = Stage::new(|_| {
    Row::new()
      .padding(20.0)
      .child(Icon::new(icons::SPINNER).spin(true))
      .child(Icon::new(icons::CHECK))
      .into()
  });
  let spinner = glyph(icons::SPINNER);
  stage.pass_after(SPIN_PERIOD / 4);
  let quarter = stage.text_quad(&spinner).unwrap();
  assert!(!quarter.transform.is_identity(), "a quarter of a turn on");
  let still = stage.text_quad(&glyph(icons::CHECK)).unwrap();
  assert!(
    still.transform.is_identity(),
    "an icon that does not spin stays as it is"
  );
  stage.pass_after(SPIN_PERIOD / 2);
  let half = stage.text_quad(&spinner).unwrap();
  assert_ne!(quarter.transform, half.transform, "it goes on turning");
  // The quad's corner goes round the middle of the icon, which is 16 by 16 at 20, 20: it stays as far from
  // it, and a quarter of a period moves it a quarter of the way round.
  let from_middle = |quad: &lurq::layout::quad::Quad| (quad.x - 28.0, quad.y - 28.0);
  let (first, second) = (from_middle(&quarter), from_middle(&half));
  let far = |(x, y): (f32, f32)| (x * x + y * y).sqrt();
  assert!(
    (far(first) - 128.0_f32.sqrt()).abs() < 0.01,
    "about its middle: {first:?}"
  );
  assert!((far(second) - far(first)).abs() < 0.01);
  assert!(
    (second.0 + first.1).abs() < 0.01 && (second.1 - first.0).abs() < 0.01,
    "a quarter of a turn on, clockwise: {first:?} then {second:?}"
  );
}
