//! `ui::Icon`: a glyph of the theme's set by name, in the foreground of what
//! it is in.

use lurq::{
  app::{
    App,
    theme::{IconGlyph, IconSize, IconSvg, IconTint, PaletteColor, ThemeIcons, ThemePalette},
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

#[test]
fn a_spinning_icon_turns_while_motion_is_reduced_too() {
  let app = lurq::app::App::new();
  app.theme().set_reduced_motion(Some(true));
  let mut stage = Stage::in_app(app, |_| {
    Row::new()
      .padding(20.0)
      .child(Icon::new(icons::SPINNER).spin(true))
      .into()
  });
  let spinner = glyph(icons::SPINNER);
  stage.pass_after(SPIN_PERIOD / 4);
  let quarter = stage.text_quad(&spinner).expect("it is drawn");
  assert!(
    !quarter.transform.is_identity(),
    "it says that something is under way, and nothing else does"
  );
  stage.pass_after(SPIN_PERIOD / 2);
  assert_ne!(
    quarter.transform,
    stage.text_quad(&spinner).unwrap().transform,
    "and goes on turning"
  );
}

const SQUARE: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect x="4" y="4" width="16" height="16" fill="black" stroke="black"/></svg>"#;

/// The SVG painted in the quads of the stage, if one is.
fn painted_svg(stage: &Stage) -> Option<lurq::svg::SvgData> {
  stage.quads().into_iter().find_map(|quad| match quad.content {
    lurq::layout::quad::QuadContent::Svg { data } => Some(data),
    _ => None,
  })
}

#[test]
fn an_icon_the_theme_draws_with_an_svg_is_that_svg_at_its_size_in_its_color() {
  let app = App::new();
  app.theme().set_icon_svg("mark", IconSvg::filled(SQUARE));
  let mut stage = Stage::in_app(app, |_| {
    Row::new()
      .child(
        Icon::new("mark")
          .size(IconSize::Lg)
          .color(PaletteColor::Success)
          .id("mark"),
      )
      .into()
  });
  let (_, _, width, height) = stage.bounds("mark");
  assert_eq!((width, height), (20.0, 20.0), "a square of its size");
  let data = painted_svg(&stage).expect("the SVG is painted");
  assert_eq!(
    data.fill_override(),
    Some(light(PaletteColor::Success)),
    "its shapes in its color"
  );
  assert_eq!(data.stroke_override(), None, "a filled one leaves its lines alone");
}

#[test]
fn a_stroked_svg_takes_the_foreground_of_what_it_is_in_and_a_colored_one_keeps_its_own() {
  let app = App::new();
  app.theme().set_icon_svg("line", IconSvg::stroked(SQUARE));
  let stage = Stage::in_app(app, |_| Row::new().foreground(RED).child(Icon::new("line")).into());
  let data = painted_svg(&stage).expect("the SVG is painted");
  assert_eq!(data.stroke_override(), Some(RED));
  assert_eq!(data.fill_override(), None);

  let app = App::new();
  app.theme().set_icon_svg("brand", IconSvg::colored(SQUARE));
  let stage = Stage::in_app(app, |_| Row::new().foreground(RED).child(Icon::new("brand")).into());
  let data = painted_svg(&stage).expect("the SVG is painted");
  assert_eq!((data.fill_override(), data.stroke_override()), (None, None));
}

#[test]
fn a_name_has_one_drawing_a_glyph_or_an_svg() {
  let mut set = ThemeIcons::lurq();
  set.set_svg(icons::SEARCH, IconSvg::stroked(SQUARE));
  assert!(set.get(icons::SEARCH).is_none(), "the SVG took the glyph's place");
  assert_eq!(set.svg(icons::SEARCH).map(IconSvg::tint), Some(IconTint::Stroke));
  assert_eq!(set.len(), icons::ALL.len(), "still one icon of that name");
  set.set(icons::SEARCH, IconGlyph::new("brand icons", 'S'));
  assert!(set.svg(icons::SEARCH).is_none());
  // What is no SVG takes its room and draws nothing.
  let app = App::new();
  app.theme().set_icon_svg("broken", IconSvg::stroked("not an svg"));
  let mut stage = Stage::in_app(app, |_| Row::new().child(Icon::new("broken").id("broken")).into());
  assert_eq!(stage.bounds("broken").2, 16.0);
  assert!(painted_svg(&stage).is_none());
}
