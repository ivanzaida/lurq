//! Built-in controls take their fill from the theme when the app has not
//! styled them, and paint none where the app's own part sets no fill.

#[allow(deprecated)]
use lurq::{
  app::{App, Tree, theme::PaletteColor},
  components::{Checkbox, Slider},
  core::Signal,
  layout::quad::QuadContent,
  node::{Element, color::Color},
};

const SURFACE: &str = "#1e293b";
const ACCENT: &str = "#7c3aed";
const BORDER: &str = "#334155";

/// Opaque rect fills painted by `root`, laid out headlessly under a theme
/// whose `SurfaceInput`, `Accent` and `Border` are distinct.
fn fills(root: impl Into<Element>) -> Vec<Color> {
  let mut app = App::new();
  let theme = app.theme();
  theme.set_palette_color(PaletteColor::Background, Color::from_hex(SURFACE));
  theme.set_palette_color(PaletteColor::Primary, Color::from_hex(ACCENT));
  theme.set_palette_color(PaletteColor::Border, Color::from_hex(BORDER));
  let mut tree = Tree::new();
  tree.set_root(root.into());
  tree.pass_headless(&mut app);
  tree
    .painted_quads()
    .iter()
    .filter_map(|quad| match quad.content {
      QuadContent::Rect { color, .. } if color.a() > 0 => Some(color),
      _ => None,
    })
    .collect()
}

fn hex(colors: &[&str]) -> Vec<Color> {
  colors.iter().map(|color| Color::from_hex(color)).collect()
}

#[test]
#[allow(deprecated)]
fn unstyled_checkbox_fills_from_the_theme() {
  assert_eq!(fills(Checkbox::new(Signal::new(false))), hex(&[SURFACE]));
  assert_eq!(fills(Checkbox::new(Signal::new(true))), hex(&[ACCENT]));
}

#[test]
#[allow(deprecated)]
fn checkbox_part_without_a_fill_paints_none() {
  let unchecked = Checkbox::new(Signal::new(false)).box_part(|style| style.size(14.0, 14.0));
  assert_eq!(fills(unchecked), []);
  let checked = Checkbox::new(Signal::new(true)).checked_box(|style| style.indicator_size(6.0, 6.0));
  assert_eq!(fills(checked), []);
  // A checked box the app did not style keeps the theme's fill.
  let checked_unstyled = Checkbox::new(Signal::new(true)).box_part(|style| style.size(14.0, 14.0));
  assert_eq!(fills(checked_unstyled), hex(&[ACCENT]));
}

#[test]
#[allow(deprecated)]
fn checkbox_fill_set_by_the_app_is_kept() {
  let unchecked = Checkbox::new(Signal::new(false)).box_part(|style| style.background("#f8fafc"));
  assert_eq!(fills(unchecked), hex(&["#f8fafc"]));
  let checked = Checkbox::new(Signal::new(true)).checked_box(|style| style.background("#111827"));
  assert_eq!(fills(checked), hex(&["#111827"]));
}

#[test]
fn unstyled_slider_fills_from_the_theme() {
  let slider = Slider::new(Signal::new(5)).range(0, 10).width(100.0);
  assert_eq!(fills(slider), hex(&[BORDER, ACCENT]), "track, then thumb");
}

#[test]
fn slider_part_without_a_fill_paints_none() {
  let track = Slider::new(Signal::new(5))
    .range(0, 10)
    .width(100.0)
    .track(|style| style.height(4.0));
  assert_eq!(fills(track), hex(&[ACCENT]), "only the unstyled thumb");
  let thumb = Slider::new(Signal::new(5))
    .range(0, 10)
    .width(100.0)
    .thumb(|style| style.size(10.0, 10.0));
  assert_eq!(fills(thumb), hex(&[BORDER]), "only the unstyled track");
}

/// Opaque rect fills painted by `root` in a theme made from `definition`, in `appearance`, with nothing set.
fn fills_in(
  definition: lurq::app::theme::ThemeDefinition,
  appearance: lurq::app::window::Appearance,
  root: impl Into<Element>,
) -> Vec<Color> {
  let mut app = App::new();
  app.theme().apply(definition);
  app.theme().set_appearance(appearance);
  let mut tree = Tree::new();
  tree.set_root(root.into());
  tree.pass_headless(&mut app);
  tree
    .painted_quads()
    .iter()
    .filter_map(|quad| match quad.content {
      QuadContent::Rect { color, .. } if color.a() > 0 => Some(color),
      _ => None,
    })
    .collect()
}

#[test]
#[allow(deprecated)]
fn unstyled_controls_take_the_design_roles_light_and_dark() {
  use lurq::app::{theme::ThemeDefinition, window::Appearance};
  for appearance in [Appearance::Light, Appearance::Dark] {
    let definition = ThemeDefinition::neutral();
    let palette = definition.palette(appearance).clone();
    let role = |role: PaletteColor| palette.get(role);
    assert_eq!(
      fills_in(definition.clone(), appearance, Checkbox::new(Signal::new(false))),
      [role(PaletteColor::Background)],
      "{appearance:?}"
    );
    assert_eq!(
      fills_in(definition.clone(), appearance, Checkbox::new(Signal::new(true))),
      [role(PaletteColor::Primary)],
      "{appearance:?}"
    );
    assert_eq!(
      fills_in(
        definition,
        appearance,
        Slider::new(Signal::new(5)).range(0, 10).width(100.0)
      ),
      [role(PaletteColor::Border), role(PaletteColor::Primary)],
      "{appearance:?}: track, then thumb"
    );
  }
  let light = ThemeDefinition::neutral()
    .palette(Appearance::Light)
    .get(PaletteColor::Primary);
  let dark = ThemeDefinition::neutral()
    .palette(Appearance::Dark)
    .get(PaletteColor::Primary);
  assert_ne!(light, dark, "the dark appearance is another fill");
}

#[test]
#[allow(deprecated)]
fn the_legacy_theme_keeps_the_controls_of_0_62() {
  use lurq::app::{theme::ThemeDefinition, window::Appearance};
  let legacy = ThemeDefinition::legacy();
  let palette = legacy.palette(Appearance::Light).clone();
  // 0.62 painted the roles of 0.61, which keep their own colors in the legacy palette.
  assert_eq!(
    fills_in(legacy.clone(), Appearance::Light, Checkbox::new(Signal::new(false))),
    [palette.get(PaletteColor::SurfaceInput)]
  );
  assert_eq!(
    fills_in(legacy.clone(), Appearance::Dark, Checkbox::new(Signal::new(true))),
    [palette.get(PaletteColor::Accent)]
  );
  assert_eq!(
    fills_in(
      legacy,
      Appearance::Light,
      Slider::new(Signal::new(5)).range(0, 10).width(100.0)
    ),
    [palette.get(PaletteColor::Border), palette.get(PaletteColor::Accent)]
  );
}

#[test]
fn an_apps_patch_reaches_the_slider_node() {
  use lurq::{
    app::{theme::ThemeDefinition, window::Appearance},
    ui::SliderPart,
  };
  let mut definition = ThemeDefinition::neutral();
  definition
    .ui
    .patch::<Slider>(|rules| rules.base(SliderPart::Thumb, |s| s.background(PaletteColor::Destructive)));
  let destructive = definition.palette(Appearance::Light).get(PaletteColor::Destructive);
  let track = definition.palette(Appearance::Light).get(PaletteColor::Border);
  assert_eq!(
    fills_in(
      definition,
      Appearance::Light,
      Slider::new(Signal::new(5)).range(0, 10).width(100.0)
    ),
    [track, destructive]
  );
}
