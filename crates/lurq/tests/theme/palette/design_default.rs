//! The theme an app has when it sets none: the design, light and dark.

use lurq::{
  app::{
    App, Tree,
    theme::{PaletteColor, Theme, ThemePalette},
    window::{Appearance, SystemSettings},
  },
  components::{Column, Rect},
  node::color::Color,
};

use crate::support::render_pass_with_app;

fn dark_system() -> SystemSettings {
  SystemSettings {
    appearance: Appearance::Dark,
    ..SystemSettings::default()
  }
}

#[test]
fn the_default_theme_has_the_designs_light_palette() {
  let theme = Theme::new();
  assert_eq!(*theme.palette(), ThemePalette::light());
  assert_eq!(theme.palette_color(PaletteColor::Primary), Color::from_hex("#18181b"));
  assert_eq!(
    theme.palette_color(PaletteColor::Foreground),
    Color::from_hex("#09090b")
  );
}

#[test]
fn the_default_theme_is_dark_when_the_system_is() {
  let theme = Theme::new();
  theme.apply_system(&dark_system());
  assert_eq!(*theme.palette(), ThemePalette::dark());
  theme.apply_system(&SystemSettings::default());
  assert_eq!(*theme.palette(), ThemePalette::light());
}

#[test]
fn a_theme_that_does_not_follow_the_system_stays_as_it_is() {
  let theme = Theme::new();
  theme.set_follows_system(false);
  theme.apply_system(&dark_system());
  assert_eq!(*theme.palette(), ThemePalette::light());
}

#[test]
fn a_window_whose_root_paints_no_background_is_cleared_to_the_themes() {
  let mut app = App::new();
  let mut tree = Tree::new();
  tree.set_root(Column::new().child(Rect::new(10.0, 10.0)));
  assert_eq!(
    render_pass_with_app(&mut tree, &mut app).clear_color,
    ThemePalette::light().background
  );

  app.theme().apply_system(&dark_system());
  assert_eq!(
    render_pass_with_app(&mut tree, &mut app).clear_color,
    ThemePalette::dark().background,
    "dark under text that takes the dark foreground"
  );
}

#[test]
fn a_roots_own_background_is_what_its_window_is_cleared_to() {
  let mut app = App::new();
  let mut tree = Tree::new();
  let own = Color::from_hex("#123456");
  tree.set_root(Column::new().background(own).child(Rect::new(10.0, 10.0)));
  assert_eq!(render_pass_with_app(&mut tree, &mut app).clear_color, own);
}
