//! A theme as one value: the design's, the one of lurq 0.61, and an app's own.

use std::time::Duration;

use lurq::{
  animation::Easing,
  app::{
    theme::{
      BlurSize, ControlSize, Density, IconSize, MotionDuration, MotionEasing, OpacityLevel, PaletteColor, RadiusSize,
      Theme, ThemeDefinition, ThemePalette, TypographyStyle,
    },
    window::{Appearance, SystemSettings},
  },
  node::color::Color,
};

fn dark_system() -> SystemSettings {
  SystemSettings {
    appearance: Appearance::Dark,
    ..SystemSettings::default()
  }
}

#[test]
fn a_new_theme_is_the_neutral_definition() {
  let theme = Theme::new();
  let neutral = ThemeDefinition::neutral();
  let now = theme.definition();
  assert_eq!(now.light, neutral.light);
  assert_eq!(now.dark, neutral.dark);
  assert_eq!(now.radii, neutral.radii);
  assert_eq!(now.durations, neutral.durations);
  assert_eq!(now.focus_ring, neutral.focus_ring);
}

#[test]
fn the_scales_have_the_designs_values() {
  let theme = Theme::new();
  let radii = [
    RadiusSize::Sm,
    RadiusSize::Md,
    RadiusSize::Lg,
    RadiusSize::Xl,
    RadiusSize::Full,
  ];
  assert_eq!(radii.map(|size| theme.radius_value(size)), [4.0, 6.0, 8.0, 12.0, 999.0]);
  assert_eq!(
    [ControlSize::Sm, ControlSize::Md, ControlSize::Lg].map(|size| theme.control_size_value(size)),
    [32.0, 36.0, 40.0]
  );
  assert_eq!(
    [IconSize::Xs, IconSize::Sm, IconSize::Md, IconSize::Lg, IconSize::Xl].map(|size| theme.icon_size_value(size)),
    [12.0, 14.0, 16.0, 20.0, 24.0]
  );
  assert_eq!(
    [Density::Compact, Density::Default, Density::Comfortable, Density::Touch].map(|size| theme.density_value(size)),
    [28.0, 36.0, 44.0, 48.0]
  );
  assert_eq!(theme.duration_value(MotionDuration::Base), Duration::from_millis(160));
  assert_eq!(
    theme.easing_value(MotionEasing::Standard),
    Easing::CubicBezier {
      x1: 0.2,
      y1: 0.0,
      x2: 0.0,
      y2: 1.0
    }
  );
  assert_eq!(theme.opacity_value(OpacityLevel::Disabled), 0.5);
  assert_eq!(theme.blur_value(BlurSize::Regular), 20.0);
  let ring = theme.focus_ring().clone();
  assert_eq!((ring.width, ring.offset, ring.color), (2.0, 2.0, PaletteColor::Ring));
}

#[test]
fn a_scale_takes_a_value_for_a_role_and_for_a_role_an_app_names() {
  let theme = Theme::new();
  theme.set_control_size_value(ControlSize::Md, 44.0);
  assert_eq!(theme.control_size_value(ControlSize::Md), 44.0);
  theme.set_duration_value("page", Duration::from_millis(600));
  assert_eq!(theme.duration_value("page"), Duration::from_millis(600));
  assert_eq!(theme.durations().try_get("absent"), None);
}

#[test]
fn the_legacy_theme_is_lurq_0_61() {
  let theme = Theme::legacy();
  assert_eq!(*theme.palette(), ThemePalette::legacy());
  assert_eq!(
    [RadiusSize::Sm, RadiusSize::Md, RadiusSize::Lg].map(|size| theme.radius_value(size)),
    [3.0, 5.0, 6.0]
  );
  // One palette: it is light whatever the system is in.
  theme.apply_system(&dark_system());
  assert_eq!(*theme.palette(), ThemePalette::legacy());
}

#[test]
fn applying_a_definition_changes_the_look_at_once_and_bumps_the_theme_once() {
  let theme = Theme::new();
  let mut versions = 0;
  let before = theme.definition();
  theme.apply(ThemeDefinition::legacy());
  versions += 1;
  assert_eq!(theme.palette_color(PaletteColor::Primary), Color::from_hex("#2563eb"));
  assert_eq!(theme.radius_value(RadiusSize::Md), 5.0);
  theme.apply(before);
  versions += 1;
  assert_eq!(theme.palette_color(PaletteColor::Primary), Color::from_hex("#18181b"));
  assert_eq!(theme.radius_value(RadiusSize::Md), 6.0);
  assert_eq!(versions, 2);
}

#[test]
fn applying_a_definition_leaves_what_the_system_set() {
  let theme = Theme::new();
  theme.apply_system(&SystemSettings {
    appearance: Appearance::Dark,
    text_scale: 1.5,
    bold_text: true,
    ..SystemSettings::default()
  });
  let body = theme.typography_style(TypographyStyle::Body).font_size;

  let mut brand = ThemeDefinition::neutral();
  brand.light.primary = Color::from_hex("#7c3aed");
  brand.dark.as_mut().unwrap().primary = Color::from_hex("#a78bfa");
  theme.apply(brand);

  assert_eq!(theme.appearance(), Appearance::Dark);
  assert_eq!(
    theme.palette_color(PaletteColor::Primary),
    Color::from_hex("#a78bfa"),
    "the dark one"
  );
  assert_eq!(theme.text_scale(), 1.5);
  assert!(theme.bold_text());
  assert_eq!(
    theme.typography_style(TypographyStyle::Body).font_size,
    body,
    "still scaled"
  );
}

#[test]
fn a_color_set_for_a_role_is_set_in_both_appearances() {
  let theme = Theme::new();
  let brand = Color::from_hex("#7c3aed");
  theme.set_palette_color(PaletteColor::Primary, brand);
  assert_eq!(theme.palette_color(PaletteColor::Primary), brand);
  theme.apply_system(&dark_system());
  assert_eq!(
    theme.palette_color(PaletteColor::Primary),
    brand,
    "kept when the system turns dark"
  );
  assert_eq!(
    theme.palette_color(PaletteColor::Background),
    ThemePalette::dark().background,
    "and nothing else of the dark palette is touched"
  );
}

#[test]
fn a_color_set_for_one_appearance_is_set_in_that_one() {
  let theme = Theme::new();
  let violet = Color::from_hex("#a78bfa");
  theme.set_palette_color_for(Appearance::Dark, PaletteColor::Primary, violet);
  assert_eq!(
    theme.palette_color(PaletteColor::Primary),
    ThemePalette::light().primary
  );
  theme.apply_system(&dark_system());
  assert_eq!(theme.palette_color(PaletteColor::Primary), violet);
}

#[test]
fn a_palette_set_while_in_one_appearance_is_that_appearances() {
  let theme = Theme::new();
  let mut own = ThemePalette::light();
  own.background = Color::from_hex("#fff7ed");
  theme.set_palette(own.clone());
  theme.apply_system(&dark_system());
  assert_eq!(*theme.palette(), ThemePalette::dark(), "the dark one is as it was");
  theme.apply_system(&SystemSettings::default());
  assert_eq!(*theme.palette(), own, "and the light one is still the app's");
}

#[test]
fn a_definition_read_back_holds_what_was_set_on_the_theme() {
  let theme = Theme::new();
  theme.set_radius_value(RadiusSize::Md, 10.0);
  theme.set_palette_color(PaletteColor::Primary, Color::from_hex("#7c3aed"));
  let definition = theme.definition();
  assert_eq!(definition.radii.md, 10.0);
  assert_eq!(definition.light.primary, Color::from_hex("#7c3aed"));
  assert_eq!(definition.dark.unwrap().primary, Color::from_hex("#7c3aed"));
}

#[test]
fn the_typography_is_the_designs_and_the_legacy_one_is_lurq_0_61() {
  use lurq::layout::text_style::FontWeight;
  let theme = Theme::new();
  let sizes = [
    TypographyStyle::Heading,
    TypographyStyle::Title,
    TypographyStyle::Body,
    TypographyStyle::Description,
    TypographyStyle::Label,
    TypographyStyle::Button,
    TypographyStyle::Caption,
    TypographyStyle::Mono,
  ]
  .map(|style| theme.typography_style(style).font_size);
  assert_eq!(sizes, [24.0, 20.0, 14.0, 14.0, 13.0, 13.0, 12.0, 13.0]);
  let heading = theme.typography_style(TypographyStyle::Heading);
  assert_eq!((heading.weight, heading.letter_spacing), (FontWeight::SemiBold, -0.5));

  let legacy = Theme::legacy();
  assert_eq!(legacy.typography_style(TypographyStyle::Body).font_size, 16.0);
  assert_eq!(legacy.typography_style(TypographyStyle::Mono).font_size, 16.0);
  assert_eq!(
    legacy.typography_style(TypographyStyle::Heading).weight,
    FontWeight::Bold
  );
}

#[test]
fn one_setting_changes_the_font_of_the_whole_interface() {
  let theme = Theme::new();
  let system = theme.sans_family();
  theme.set_sans_family("Inter");
  for style in [TypographyStyle::Body, TypographyStyle::Heading, TypographyStyle::Button] {
    assert_eq!(&*theme.typography_style(style).font_family, "Inter");
  }
  assert_eq!(
    &*theme.typography_style(TypographyStyle::Mono).font_family,
    "monospace",
    "code keeps its own"
  );
  assert_ne!(&*system, "Inter");

  theme.set_mono_family("JetBrains Mono");
  assert_eq!(
    &*theme.typography_style(TypographyStyle::Mono).font_family,
    "JetBrains Mono"
  );
  assert_eq!(&*theme.typography_style(TypographyStyle::Body).font_family, "Inter");
}

#[test]
fn a_style_in_a_font_of_its_own_keeps_it_when_the_interfaces_font_changes() {
  let theme = Theme::new();
  let mut display = theme.typography_style(TypographyStyle::Heading);
  display.font_family = "Georgia".into();
  theme.set_typography_style(TypographyStyle::Heading, display);
  theme.set_sans_family("Inter");
  assert_eq!(
    &*theme.typography_style(TypographyStyle::Heading).font_family,
    "Georgia"
  );
  assert_eq!(&*theme.typography_style(TypographyStyle::Title).font_family, "Inter");
}

#[test]
fn the_legacy_theme_has_the_elevations_of_0_61() {
  use lurq::app::theme::{ShadowStyle, ThemeShadows};
  assert_eq!(Theme::legacy().shadow_style(ShadowStyle::Md), ThemeShadows::legacy().md);
  assert_ne!(Theme::new().shadow_style(ShadowStyle::Md), ThemeShadows::legacy().md);
  assert_eq!(
    Theme::new().shadow_style(ShadowStyle::Sm),
    ThemeShadows::legacy().sm,
    "the smallest is the same"
  );
}

#[test]
fn the_density_a_theme_is_at_is_part_of_its_definition() {
  use lurq::app::theme::Density;
  let theme = Theme::new();
  assert_eq!(theme.density(), Density::Default);
  assert_eq!(theme.density_value(theme.density()), 36.0, "the height of a row");
  theme.set_density(Density::Compact);
  assert_eq!(theme.density_value(theme.density()), 28.0);
  let compact = theme.definition();
  assert_eq!(compact.density, Density::Compact);
  theme.apply(ThemeDefinition::neutral());
  assert_eq!(theme.density(), Density::Default, "a definition says the density too");
  theme.apply(compact);
  assert_eq!(theme.density(), Density::Compact);
}
