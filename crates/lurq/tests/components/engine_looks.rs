//! The engine's own styled pieces take their colors from the theme through a recipe: the phone's sheet, the host
//! around it, its tab bar, navigation bar, pull to refresh and swipe row, and the engine's select. Light and dark
//! come from the theme's appearance; the legacy theme keeps the colors of 0.62; an app's patch reaches them.

#[cfg(feature = "router")]
use lurq::components::{NavigationBar, NavigationBarPart, TabBar, TabBarPart};
use lurq::{
  app::{
    App, Tree,
    theme::{PaletteColor, Theme, ThemeDefinition},
    window::Appearance,
  },
  components::{
    ActivityIndicatorProps, Column, PullToRefreshStyle, Select, SelectOption, SheetHostStyle, SheetNode, SheetStyle,
    SwipeAction, SwipeRowPart, SwipeRowStyle,
  },
  core::{ElementRef, Signal},
  layout::quad::QuadContent,
  node::{SelectStyle, color::Color},
  ui::in_theme,
};

fn neutral(appearance: Appearance) -> Theme {
  let theme = Theme::new();
  theme.set_appearance(appearance);
  theme
}

fn legacy(appearance: Appearance) -> Theme {
  let theme = Theme::legacy();
  theme.set_appearance(appearance);
  theme
}

fn role(theme: &Theme, role: PaletteColor) -> Color {
  theme.palette_color(role)
}

#[test]
fn a_sheet_takes_the_roles_of_the_theme_in_each_appearance() {
  for appearance in [Appearance::Light, Appearance::Dark] {
    let theme = neutral(appearance);
    let style = in_theme(&theme, SheetStyle::new);
    assert_eq!(
      style.background,
      role(&theme, PaletteColor::Background),
      "{appearance:?}"
    );
    assert_eq!(style.grabber_color, role(&theme, PaletteColor::Input), "{appearance:?}");
    assert_eq!(style.backdrop, role(&theme, PaletteColor::Scrim), "{appearance:?}");
    assert_eq!(style.dim, 1.0, "the scrim carries its own alpha");
  }
  let light = in_theme(&neutral(Appearance::Light), SheetStyle::new);
  let dark = in_theme(&neutral(Appearance::Dark), SheetStyle::new);
  assert_ne!(
    light.background, dark.background,
    "the dark appearance is another sheet"
  );
}

#[test]
fn the_legacy_theme_keeps_the_sheet_of_0_62() {
  for appearance in [Appearance::Light, Appearance::Dark] {
    let style = in_theme(&legacy(appearance), SheetStyle::new);
    assert_eq!(style.background, Color::from_hex("#FFFFFF"));
    assert_eq!(style.backdrop, Color::from_hex("#000000"));
    assert_eq!(style.dim, 0.3);
    assert_eq!(style.grabber_color, Color::new(60, 60, 67, 77));
  }
  let host = in_theme(&legacy(Appearance::Light), SheetHostStyle::default);
  assert_eq!(host.background, Color::from_hex("#000000"));
}

#[test]
#[allow(deprecated)]
fn the_deprecated_dark_sheet_is_0_62s() {
  let dark = SheetStyle::dark();
  assert_eq!(dark.background, Color::from_hex("#1C1C1E"));
  assert_eq!(dark.grabber_color, Color::new(235, 235, 245, 77));
  assert_eq!((dark.backdrop, dark.dim), (Color::from_hex("#000000"), 0.5));
  let light = in_theme(&neutral(Appearance::Dark), || {
    SheetStyle::for_appearance(Appearance::Light)
  });
  assert_eq!(
    light.background,
    Color::from_hex("#FFFFFF"),
    "0.62's light sheet whatever the theme"
  );
}

#[test]
fn an_apps_patch_reaches_the_sheet() {
  let mut definition = ThemeDefinition::neutral();
  definition
    .ui
    .patch::<SheetStyle>(|rules| rules.base(SheetNode::Panel, |s| s.background(PaletteColor::Card)));
  let theme = Theme::new();
  theme.apply(definition);
  let style = in_theme(&theme, SheetStyle::new);
  assert_eq!(style.background, role(&theme, PaletteColor::Card));
  assert_eq!(
    style.grabber_color,
    role(&theme, PaletteColor::Input),
    "the rest of the recipe stays"
  );
}

#[test]
fn a_bare_theme_leaves_the_colors_of_0_62() {
  let theme = Theme::new();
  theme.apply(ThemeDefinition::bare());
  let style = in_theme(&theme, SheetStyle::new);
  assert_eq!(style.background, Color::from_hex("#FFFFFF"));
  assert_eq!(style.dim, 0.3);
}

#[test]
fn a_pull_to_refresh_spins_in_the_muted_foreground() {
  for appearance in [Appearance::Light, Appearance::Dark] {
    let theme = neutral(appearance);
    let style = in_theme(&theme, PullToRefreshStyle::new);
    assert_eq!(
      style.spinner_color,
      role(&theme, PaletteColor::MutedForeground),
      "{appearance:?}"
    );
    let spinner = in_theme(&theme, ActivityIndicatorProps::new);
    assert_eq!(
      spinner.color, style.spinner_color,
      "the spinner it draws is in the same color"
    );
  }
  let old = in_theme(&legacy(Appearance::Dark), PullToRefreshStyle::new);
  assert_eq!(old.spinner_color, Color::new(60, 60, 67, 153));
  #[allow(deprecated)]
  let dark = PullToRefreshStyle::dark();
  assert_eq!(dark.spinner_color, Color::new(235, 235, 245, 153));
}

#[test]
fn a_swipe_rows_actions_take_the_roles_of_the_theme() {
  for appearance in [Appearance::Light, Appearance::Dark] {
    let theme = neutral(appearance);
    let (plain, destructive) = in_theme(&theme, || {
      (
        SwipeAction::new("Archive", || {}),
        SwipeAction::destructive("Delete", || {}),
      )
    });
    assert_eq!(
      plain.background,
      role(&theme, PaletteColor::MutedForeground),
      "{appearance:?}"
    );
    assert_eq!(plain.color, role(&theme, PaletteColor::Background), "{appearance:?}");
    assert_eq!(
      destructive.background,
      role(&theme, PaletteColor::Destructive),
      "{appearance:?}"
    );
    assert_eq!(
      destructive.color,
      role(&theme, PaletteColor::DestructiveForeground),
      "{appearance:?}"
    );
  }
  let (plain, destructive) = in_theme(&legacy(Appearance::Light), || {
    (
      SwipeAction::new("Archive", || {}),
      SwipeAction::destructive("Delete", || {}),
    )
  });
  assert_eq!(
    (plain.background, plain.color),
    (Color::from_hex("#8E8E93"), Color::from_hex("#FFFFFF"))
  );
  assert_eq!(destructive.background, Color::from_hex("#FF3B30"));

  let mut definition = ThemeDefinition::neutral();
  definition
    .ui
    .patch::<SwipeRowStyle>(|rules| rules.base(SwipeRowPart::Action, |s| s.background(PaletteColor::Info)));
  let theme = Theme::new();
  theme.apply(definition);
  let archive = in_theme(&theme, || SwipeAction::new("Archive", || {}));
  assert_eq!(archive.background, role(&theme, PaletteColor::Info));
}

#[cfg(feature = "router")]
#[test]
fn a_tab_bar_takes_the_roles_of_the_theme_in_each_appearance() {
  for appearance in [Appearance::Light, Appearance::Dark] {
    let theme = neutral(appearance);
    let bar = in_theme(&theme, TabBar::new);
    assert_eq!(bar.background, role(&theme, PaletteColor::Background), "{appearance:?}");
    assert_eq!(bar.border, role(&theme, PaletteColor::Border), "{appearance:?}");
    assert_eq!(bar.tint, role(&theme, PaletteColor::Foreground), "{appearance:?}");
    assert_eq!(bar.color, role(&theme, PaletteColor::MutedForeground), "{appearance:?}");
    assert_eq!(
      bar.badge_background,
      role(&theme, PaletteColor::Destructive),
      "{appearance:?}"
    );
    assert_eq!(
      bar.badge_color,
      role(&theme, PaletteColor::DestructiveForeground),
      "{appearance:?}"
    );
    assert_eq!(
      bar.floating_selection,
      role(&theme, PaletteColor::Muted),
      "{appearance:?}"
    );
  }
}

#[cfg(feature = "router")]
#[test]
#[allow(deprecated)]
fn the_legacy_theme_keeps_the_tab_bar_of_0_62() {
  let bar = in_theme(&legacy(Appearance::Dark), TabBar::new);
  assert_eq!(bar.background, Color::new(249, 249, 249, 224));
  assert_eq!(bar.border, Color::new(60, 60, 67, 74));
  assert_eq!(bar.tint, Color::from_hex("#007AFF"));
  assert_eq!(bar.color, Color::from_hex("#999999"));
  assert_eq!(bar.contrast_color, Color::from_hex("#515154"));
  assert_eq!(bar.badge_background, Color::from_hex("#FF3B30"));
  assert_eq!(bar.floating_selection, Color::new(120, 120, 128, 41));
  let dark = TabBar::dark();
  assert_eq!(
    (dark.background, dark.tint),
    (Color::new(22, 22, 22, 204), Color::from_hex("#0A84FF"))
  );
  assert_eq!(
    dark.badge_background,
    Color::from_hex("#FF3B30"),
    "0.62's dark bar has 0.62's badge"
  );

  let mut definition = ThemeDefinition::neutral();
  definition
    .ui
    .patch::<TabBar>(|rules| rules.base(TabBarPart::SelectedItem, |s| s.foreground(PaletteColor::Primary)));
  let theme = Theme::new();
  theme.apply(definition);
  assert_eq!(in_theme(&theme, TabBar::new).tint, role(&theme, PaletteColor::Primary));
}

#[cfg(feature = "router")]
#[test]
fn a_navigation_bar_takes_the_roles_of_the_theme_in_each_appearance() {
  for appearance in [Appearance::Light, Appearance::Dark] {
    let theme = neutral(appearance);
    let bar = in_theme(&theme, NavigationBar::new);
    assert_eq!(bar.background, role(&theme, PaletteColor::Background), "{appearance:?}");
    assert_eq!(bar.border, role(&theme, PaletteColor::Border), "{appearance:?}");
    assert_eq!(bar.title, role(&theme, PaletteColor::Foreground), "{appearance:?}");
    assert_eq!(bar.tint, role(&theme, PaletteColor::Foreground), "{appearance:?}");
  }
}

#[cfg(feature = "router")]
#[test]
#[allow(deprecated)]
fn the_legacy_theme_keeps_the_navigation_bar_of_0_62() {
  let bar = in_theme(&legacy(Appearance::Light), NavigationBar::new);
  assert_eq!(bar.background, Color::new(249, 249, 249, 224));
  assert_eq!(bar.tint, Color::from_hex("#007aff"));
  assert_eq!(bar.title, Color::from_hex("#000000"));
  assert_eq!(bar.border, Color::from_hex("#c6c6c8"));
  let dark = NavigationBar::dark();
  assert_eq!(
    (dark.title, dark.border),
    (Color::from_hex("#ffffff"), Color::from_hex("#38383a"))
  );

  let mut definition = ThemeDefinition::neutral();
  definition
    .ui
    .patch::<NavigationBar>(|rules| rules.base(NavigationBarPart::Action, |s| s.foreground(PaletteColor::Primary)));
  let theme = Theme::new();
  theme.apply(definition);
  assert_eq!(
    in_theme(&theme, NavigationBar::new).tint,
    role(&theme, PaletteColor::Primary)
  );
}

/// The fill of the engine's select's trigger, built in `definition`'s theme and painted in `appearance`.
fn trigger_fill(definition: ThemeDefinition, appearance: Appearance) -> Color {
  let mut app = App::new();
  app.theme().apply(definition);
  app.theme().set_appearance(appearance);
  let select = ElementRef::new();
  let style = in_theme(&app.theme(), SelectStyle::new);
  let mut tree = Tree::new();
  tree.set_root(
    Column::new().child(
      Select::new(Signal::new("a".to_owned()))
        .options([SelectOption::new("a".to_owned(), "A")])
        .width(200.0)
        .style(style)
        .ref_element(select.clone()),
    ),
  );
  tree.pass_headless(&mut app);
  let bounds = select.bounds();
  tree
    .painted_quads()
    .iter()
    .filter(|quad| (quad.x, quad.y, quad.width, quad.height) == (bounds.x, bounds.y, bounds.width, bounds.height))
    .find_map(|quad| match quad.content {
      QuadContent::Rect { color, .. } if color.a() > 0 => Some(color),
      _ => None,
    })
    .expect("the trigger is painted a fill")
}

#[test]
fn the_engines_select_takes_its_colors_from_its_recipe() {
  for appearance in [Appearance::Light, Appearance::Dark] {
    let theme = neutral(appearance);
    assert_eq!(
      trigger_fill(ThemeDefinition::neutral(), appearance),
      role(&theme, PaletteColor::Background),
      "{appearance:?}"
    );
  }
  // The legacy theme's select is on 0.62's role for an input, which keeps its own color there.
  let legacy_theme = legacy(Appearance::Light);
  #[allow(deprecated)]
  let surface = role(&legacy_theme, PaletteColor::SurfaceInput);
  assert_eq!(trigger_fill(ThemeDefinition::legacy(), Appearance::Light), surface);

  let mut definition = ThemeDefinition::neutral();
  definition
    .ui
    .patch::<SelectStyle>(|rules| rules.base(lurq::ui::SelectPart::Trigger, |s| s.background(PaletteColor::Muted)));
  assert_eq!(
    trigger_fill(definition, Appearance::Light),
    role(&neutral(Appearance::Light), PaletteColor::Muted)
  );
}

/// The fill and the opacity a theme gives a part of the stack's shades, as the outlet reads them.
#[cfg(feature = "router")]
fn shade(theme: &Theme, part: lurq::components::StackPart) -> (Color, f32) {
  use lurq::{
    components::StackMotion,
    node::BackgroundColor,
    ui::{Case, Opacity},
  };
  let style = theme.resolve::<StackMotion>(&Case::new(), None).part(part).rest();
  let color = match style.background.expect("a fill") {
    BackgroundColor::Color(color) => color,
    BackgroundColor::Palette(role) => theme.palette_color(role),
    BackgroundColor::CurrentForeground => panic!("no fill of its own"),
  };
  let opacity = match style.opacity.expect("an opacity") {
    Opacity::Value(value) => value,
    Opacity::Level(level) => theme.opacities().try_get(level).expect("a level of the theme"),
  };
  (color, opacity)
}

#[cfg(feature = "router")]
#[test]
fn a_stacks_shades_are_the_scrim_light_and_dark() {
  use lurq::components::StackPart;
  for appearance in [Appearance::Light, Appearance::Dark] {
    let theme = neutral(appearance);
    let scrim = role(&theme, PaletteColor::Scrim);
    assert_eq!(shade(&theme, StackPart::Shade), (scrim, 1.0), "{appearance:?}");
    assert_eq!(shade(&theme, StackPart::MenuShade), (scrim, 1.0), "{appearance:?}");
  }
  assert_ne!(
    role(&neutral(Appearance::Light), PaletteColor::Scrim),
    role(&neutral(Appearance::Dark), PaletteColor::Scrim),
    "the dark scrim is darker"
  );
}

#[cfg(feature = "router")]
#[test]
fn the_legacy_theme_keeps_the_stacks_black_shades_of_0_62() {
  use lurq::components::StackPart;
  let theme = legacy(Appearance::Dark);
  let black = Color::from_hex("#000000");
  assert_eq!(
    shade(&theme, StackPart::Shade),
    (black, 1.0),
    "at the move's dim, as 0.62"
  );
  assert_eq!(shade(&theme, StackPart::MenuShade), (black, 0.2));
}
