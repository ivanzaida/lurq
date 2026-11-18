//! The light palette's fills are light and its lines crisp (`design/decisions.md`, "A lighter light theme"), and
//! what is drawn on them can still be read: text 4.5:1, a control's edge and the focus ring 3:1 (WCAG 2.2, 1.4.3 and
//! 1.4.11).

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  node::color::Color,
};

fn luminance(color: Color) -> f64 {
  let channel = |value: u8| {
    let value = f64::from(value) / 255.0;
    if value <= 0.039_28 {
      value / 12.92
    } else {
      ((value + 0.055) / 1.055).powf(2.4)
    }
  };
  0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
}

fn contrast(palette: &ThemePalette, on: PaletteColor, under: PaletteColor) -> f64 {
  let (a, b) = (luminance(palette.get(&on)), luminance(palette.get(&under)));
  (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

#[test]
fn the_light_palettes_fills_and_lines_are_the_decisions() {
  let light = ThemePalette::light();
  let hex = Color::from_hex;
  for (role, value) in [
    (PaletteColor::Canvas, "#F7F7F8"),
    (PaletteColor::Muted, "#F1F1F4"),
    (PaletteColor::Secondary, "#F1F1F4"),
    (PaletteColor::Highlight, "#F1F1F4"),
    (PaletteColor::SecondaryHover, "#E6E6EB"),
    (PaletteColor::Border, "#D4D4DA"),
    (PaletteColor::Input, "#8E8E99"),
    (PaletteColor::Bar, "#FFFFFF"),
  ] {
    assert_eq!(light.get(&role), hex(value), "{}", role.as_str());
  }
}

#[test]
fn a_bar_is_white_with_a_line_in_light_and_the_muted_fill_in_dark() {
  let (light, dark) = (ThemePalette::light(), ThemePalette::dark());
  assert_eq!(light.get(PaletteColor::Bar), light.get(PaletteColor::Background));
  assert_eq!(
    dark.get(PaletteColor::Bar),
    dark.get(PaletteColor::Muted),
    "the dark bars are as they were"
  );
  let legacy = ThemePalette::legacy();
  assert_eq!(
    legacy.get(PaletteColor::Bar),
    legacy.get(PaletteColor::Muted),
    "and 0.62's look keeps its own"
  );
}

#[test]
fn text_on_the_light_fills_has_four_and_a_half_to_one() {
  let light = ThemePalette::light();
  for under in [
    PaletteColor::Background,
    PaletteColor::Bar,
    PaletteColor::Canvas,
    PaletteColor::Muted,
    PaletteColor::Highlight,
    PaletteColor::SecondaryHover,
  ] {
    for on in [PaletteColor::Foreground, PaletteColor::MutedForeground] {
      let ratio = contrast(&light, on.clone(), under.clone());
      assert!(ratio >= 4.5, "{} on {}: {ratio:.2}", on.as_str(), under.as_str());
    }
  }
  let ratio = contrast(&light, PaletteColor::SecondaryForeground, PaletteColor::Secondary);
  assert!(ratio >= 4.5, "a secondary button's label: {ratio:.2}");
}

#[test]
fn a_fields_edge_and_the_focus_ring_have_three_to_one_against_the_light_surfaces() {
  let light = ThemePalette::light();
  for under in [
    PaletteColor::Background,
    PaletteColor::Bar,
    PaletteColor::Canvas,
    PaletteColor::Card,
  ] {
    for edge in [PaletteColor::Input, PaletteColor::Ring] {
      let ratio = contrast(&light, edge.clone(), under.clone());
      assert!(ratio >= 3.0, "{} on {}: {ratio:.2}", edge.as_str(), under.as_str());
    }
  }
}
