//! Built-in controls take their fill from the theme when the app has not
//! styled them, and paint none where the app's own part sets no fill.

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
  theme.set_palette_color(PaletteColor::SurfaceInput, Color::from_hex(SURFACE));
  theme.set_palette_color(PaletteColor::Accent, Color::from_hex(ACCENT));
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
fn unstyled_checkbox_fills_from_the_theme() {
  assert_eq!(fills(Checkbox::new(Signal::new(false))), hex(&[SURFACE]));
  assert_eq!(fills(Checkbox::new(Signal::new(true))), hex(&[ACCENT]));
}

#[test]
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
