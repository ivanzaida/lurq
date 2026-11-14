//! A breadcrumb: its steps and separators as the board draws them, what
//! following a step does, the keyboard, the collapsed form with its menu, and
//! what a theme changes.

use std::sync::{Arc, Mutex};

use lurq::{
  app::{
    App,
    theme::{OpacityLevel, PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  core::{ElementRef, Signal},
  layout::quad::QuadContent,
  node::{Element, color::Color},
  ui::{
    Breadcrumb, BreadcrumbPart, BreadcrumbPartRenderer, BreadcrumbSeparator, BreadcrumbState, BreadcrumbStep,
    BreadcrumbView, Case, Interaction, Opacity, icons,
  },
};

use super::stage::Stage;

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

/// What was followed, in order, and the breadcrumb's handler that says so.
fn log() -> (Arc<Mutex<Vec<String>>>, impl Fn(&str) + Send + Sync + Clone + 'static) {
  let followed = Arc::new(Mutex::new(Vec::new()));
  let told = followed.clone();
  (followed, move |value: &str| told.lock().unwrap().push(value.to_owned()))
}

/// The board's breadcrumb: Settings, Appearance, Theme, each step with an id of its label in lower case.
fn board() -> Breadcrumb {
  Breadcrumb::new()
    .id("way")
    .step(BreadcrumbStep::new("Settings").value("/settings").id("settings"))
    .step(
      BreadcrumbStep::new("Appearance")
        .value("/settings/appearance")
        .id("appearance"),
    )
    .step(
      BreadcrumbStep::new("Theme")
        .value("/settings/appearance/theme")
        .id("theme"),
    )
}

/// A page of 400 by 300 with `breadcrumb` at 20, 20: it has a size of its own so that a menu has room.
fn page(breadcrumb: Breadcrumb) -> Element {
  Column::new().size(400.0, 300.0).padding(20.0).child(breadcrumb).into()
}

/// Six steps, of which `max_items(3)` shows the first, an ellipsis and the last two.
fn long(anchor: ElementRef, open: Signal<bool>) -> Breadcrumb {
  Breadcrumb::new()
    .id("way")
    .steps(
      ["Home", "Projects", "lurq", "crates", "ui", "breadcrumb.rs"]
        .into_iter()
        .map(|label| BreadcrumbStep::new(label).id(label.replace('.', "-"))),
    )
    .max_items(3)
    .menu(anchor, open)
}

#[test]
fn the_steps_are_the_board_s_links_and_page_with_a_chevron_between_each_two() {
  let mut stage = Stage::new(|_| page(board()));
  // The links in the muted foreground, the page in the foreground, all in the body's 14.
  assert_eq!(stage.text("Settings"), (light(PaletteColor::MutedForeground), 14.0));
  assert_eq!(stage.text("Appearance"), (light(PaletteColor::MutedForeground), 14.0));
  assert_eq!(stage.text("Theme"), (light(PaletteColor::Foreground), 14.0));

  // A chevron of 14 in the muted foreground, 8 after a step and 8 before the next.
  let chevron = glyph(icons::CHEVRON_RIGHT);
  let chevrons: Vec<_> = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if *text == chevron))
    .collect();
  assert_eq!(chevrons.len(), 2, "one between each two steps");
  assert_eq!(stage.text(&chevron), (light(PaletteColor::MutedForeground), 14.0));
  let (settings, appearance) = (stage.bounds("settings"), stage.bounds("appearance"));
  let first = &chevrons[0];
  assert_eq!(first.width, 14.0);
  assert!(
    (first.x - (settings.0 + settings.2) - 8.0).abs() < 0.01,
    "8 after the step"
  );
  assert!(
    (appearance.0 - (first.x + first.width) - 8.0).abs() < 0.01,
    "8 before the next"
  );

  // The steps stand on one line, centered on it.
  let (theme, way) = (stage.bounds("theme"), stage.bounds("way"));
  for (_, y, _, height) in [settings, appearance, theme] {
    assert!(((y + height / 2.0) - (way.1 + way.3 / 2.0)).abs() < 0.51);
  }
  assert!(!stage.paints(&glyph(icons::MORE_HORIZONTAL)), "nothing is hidden");
}

#[test]
fn a_press_of_a_link_follows_it_and_the_page_is_not_followed() {
  let (followed, tell) = log();
  let own = Arc::new(Mutex::new(0));
  let counted = own.clone();
  let mut stage = Stage::new(move |_| {
    let counted = counted.clone();
    page(
      Breadcrumb::new()
        .step(
          BreadcrumbStep::new("Settings")
            .value("/settings")
            .id("settings")
            .on_select(move || *counted.lock().unwrap() += 1),
        )
        .step(
          BreadcrumbStep::new("Theme")
            .id("theme")
            .on_select(|| panic!("the page is not followed")),
        )
        .on_select(tell.clone()),
    )
  });
  stage.click("settings");
  assert_eq!(*own.lock().unwrap(), 1, "the step's own handler");
  assert_eq!(
    *followed.lock().unwrap(),
    ["/settings"],
    "the breadcrumb's, told the value"
  );
  stage.click("theme");
  assert_eq!(followed.lock().unwrap().len(), 1);
}

#[test]
fn a_step_s_value_is_its_label_until_it_is_given_another() {
  let (followed, tell) = log();
  let mut stage = Stage::new(move |_| {
    page(
      Breadcrumb::new()
        .step(BreadcrumbStep::new("Settings").id("settings"))
        .step("Theme")
        .on_select(tell.clone()),
    )
  });
  stage.click("settings");
  assert_eq!(*followed.lock().unwrap(), ["Settings"]);
}

#[test]
fn under_the_pointer_a_link_is_in_the_foreground() {
  let mut stage = Stage::new(|_| page(board()));
  stage.hover("settings");
  assert_eq!(stage.text("Settings").0, light(PaletteColor::Foreground));
  assert_eq!(stage.text("Appearance").0, light(PaletteColor::MutedForeground));
}

#[test]
fn tab_stops_at_each_link_and_not_at_the_page_and_enter_follows_the_focused_one() {
  let (followed, tell) = log();
  let mut stage = Stage::new(move |_| page(board().on_select(tell.clone())));
  assert!(!stage.has_ring(light(PaletteColor::Ring)));
  stage.key("Tab");
  assert!(
    stage.has_ring(light(PaletteColor::Ring)),
    "the ring of the focus, from the keyboard"
  );
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!(*followed.lock().unwrap(), ["/settings/appearance"], "the second link");
  // The page takes no stop: Tab goes on round to the first link.
  stage.key("Tab");
  stage.key("Space");
  assert_eq!(
    *followed.lock().unwrap(),
    ["/settings/appearance", "/settings"],
    "Space follows too"
  );
}

#[test]
fn a_disabled_breadcrumb_is_dimmed_and_follows_nothing() {
  let (followed, tell) = log();
  let mut stage = Stage::new(move |_| page(board().disabled(true).on_select(tell.clone())));
  stage.click("settings");
  stage.key("Tab");
  stage.key("Enter");
  assert!(followed.lock().unwrap().is_empty());
  assert!(!stage.has_ring(light(PaletteColor::Ring)), "no stop of Tab");
  // The root, which holds every step, is dimmed by the theme's opacity for what is disabled: a layer of what is
  // in it, so the opacity is the layer's and not a quad's.
  let theme = stage.app.theme().clone();
  let case = Case::new().state(BreadcrumbState::Disabled, true);
  let root = theme
    .resolve::<Breadcrumb>(&case, None)
    .part(BreadcrumbPart::Root)
    .rest();
  assert_eq!(root.opacity, Some(Opacity::Level(OpacityLevel::Disabled)));
  let enabled = theme
    .resolve::<Breadcrumb>(&Case::new(), None)
    .part(BreadcrumbPart::Root)
    .rest();
  assert_eq!(enabled.opacity, None);
}

#[test]
fn with_more_steps_than_max_items_it_shows_the_first_an_ellipsis_and_the_last_ones() {
  let (anchor, open) = (ElementRef::new(), Signal::new(false));
  let mut stage = Stage::new(move |_| page(long(anchor.clone(), open.clone())));
  for shown in ["Home", "ui", "breadcrumb.rs"] {
    assert!(stage.paints(shown), "{shown} is shown");
  }
  for hidden in ["Projects", "lurq", "crates"] {
    assert!(!stage.paints(hidden), "{hidden} is hidden");
  }
  // The ellipsis: a square of 24 in the board's accent with the small radius, a 16 icon in the foreground.
  let (x, _, width, height) = stage.bounds("way-ellipsis");
  assert_eq!((width, height), (24.0, 24.0));
  assert_eq!(stage.fill_color("way-ellipsis"), Some(light(PaletteColor::Highlight)));
  let fill = stage.fill("way-ellipsis").unwrap();
  assert!(fill.border_radius.is_some_and(|radius| radius.top_left == 4.0));
  let more = glyph(icons::MORE_HORIZONTAL);
  assert_eq!(stage.text(&more), (light(PaletteColor::Foreground), 16.0));
  // It stands where the hidden steps would: after the first and a chevron.
  let home = stage.bounds("Home");
  assert!(x > home.0 + home.2 + 8.0 + 14.0);
  let ui = stage.bounds("ui");
  assert!(ui.0 > x + width);
  let chevrons = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if *text == glyph(icons::CHEVRON_RIGHT)))
    .count();
  assert_eq!(chevrons, 3, "Home › … › ui › breadcrumb.rs");
}

#[test]
fn max_items_is_never_fewer_than_the_first_and_the_last() {
  let stage = Stage::new(|_| {
    page(
      Breadcrumb::new()
        .steps(["Settings", "Appearance", "Theme"])
        .max_items(0)
        .menu(ElementRef::new(), Signal::new(false)),
    )
  });
  assert!(stage.paints("Settings") && stage.paints("Theme"));
  assert!(!stage.paints("Appearance"));

  let stage = Stage::new(|_| {
    page(
      Breadcrumb::new()
        .steps(["Settings", "Appearance", "Theme"])
        .max_items(3),
    )
  });
  assert!(stage.paints("Appearance"), "no more than three: none hidden");
  assert!(!stage.paints(&glyph(icons::MORE_HORIZONTAL)));
}

#[test]
fn the_ellipsis_opens_a_menu_of_the_hidden_steps_and_choosing_one_follows_it() {
  let (followed, tell) = log();
  let (anchor, open) = (ElementRef::new(), Signal::new(false));
  let held = open.clone();
  let mut stage = Stage::new(move |_| page(long(anchor.clone(), held.clone()).on_select(tell.clone())));
  stage.click("way-ellipsis");
  assert!(open.get(), "a press opens it");
  for hidden in ["Projects", "lurq", "crates"] {
    assert!(stage.paints(hidden), "{hidden} is in the menu");
  }
  // The menu hangs under the ellipsis.
  let (_, y, _, height) = stage.bounds("way-ellipsis");
  assert!(stage.text_quad("Projects").unwrap().y > y + height);

  stage.click("lurq");
  assert_eq!(*followed.lock().unwrap(), ["lurq"]);
  assert!(!open.get(), "choosing a step closes the menu");
  stage.pass();
  assert!(!stage.paints("Projects"));

  // A second press of the ellipsis closes it as it opened it.
  stage.click("way-ellipsis");
  assert!(open.get());
  stage.click("way-ellipsis");
  assert!(!open.get());
}

#[test]
fn from_the_keyboard_the_ellipsis_is_a_stop_of_tab_and_its_menu_takes_the_focus() {
  let (followed, tell) = log();
  let (anchor, open) = (ElementRef::new(), Signal::new(false));
  let held = open.clone();
  let mut stage = Stage::new(move |_| page(long(anchor.clone(), held.clone()).on_select(tell.clone())));
  // Home, then the ellipsis: Enter opens the menu at its first item.
  stage.key("Tab");
  stage.key("Tab");
  stage.key("Enter");
  assert!(open.get());
  assert_eq!(stage.fill_color("Projects"), Some(light(PaletteColor::Highlight)));
  stage.key("ArrowDown");
  stage.key("Enter");
  assert_eq!(*followed.lock().unwrap(), ["lurq"]);
  assert!(!open.get());
  // The focus is back on the ellipsis: Enter opens the menu again.
  stage.key("Enter");
  assert!(open.get(), "the ellipsis had the focus back");
  stage.key("Escape");
  assert!(!open.get());
}

#[test]
fn a_step_has_its_icon_4_before_its_text() {
  let stage = Stage::new(|_| {
    page(
      Breadcrumb::new()
        .step(BreadcrumbStep::new("Home").icon(icons::HOME))
        .step("Settings"),
    )
  });
  let (icon, label) = (
    stage.text_quad(&glyph(icons::HOME)).unwrap(),
    stage.text_quad("Home").unwrap(),
  );
  assert_eq!(icon.width, 14.0);
  assert!((label.x - (icon.x + icon.width) - 4.0).abs() < 0.01);
  assert_eq!(stage.text(&glyph(icons::HOME)).0, light(PaletteColor::MutedForeground));
}

#[test]
fn a_slash_stands_between_the_steps_in_place_of_the_chevron() {
  let stage = Stage::new(|_| page(board().separator(BreadcrumbSeparator::slash())));
  assert_eq!(stage.text("/"), (light(PaletteColor::MutedForeground), 14.0));
  assert!(!stage.paints(&glyph(icons::CHEVRON_RIGHT)));
}

#[test]
fn a_forced_state_draws_the_links_with_nothing_pointing_at_them() {
  let stage = Stage::new(|_| page(board().force(Some(Interaction::Hovered))));
  assert_eq!(stage.text("Settings").0, light(PaletteColor::Foreground));
  let stage = Stage::new(|_| page(board().force(Some(Interaction::FocusVisible))));
  assert!(stage.has_ring(light(PaletteColor::Ring)));
}

#[test]
fn a_theme_changes_the_separator_and_the_recipe() {
  let app = App::new();
  let separator: BreadcrumbPartRenderer =
    Arc::new(|view: &BreadcrumbView| view.part(BreadcrumbPart::Separator).text("·").into());
  app
    .theme()
    .set_part_renderer::<Breadcrumb, BreadcrumbPartRenderer>(BreadcrumbPart::Separator, separator);
  app.theme().patch_recipe::<Breadcrumb>(|rules| {
    rules
      .base(BreadcrumbPart::Root, |s| s.gap(12.0))
      .base(BreadcrumbPart::Page, |s| {
        s.foreground(PaletteColor::Primary).opacity(Opacity::Value(1.0))
      })
  });
  let mut stage = Stage::in_app(app, |_| page(board()));
  assert!(stage.paints("·"));
  assert!(!stage.paints(&glyph(icons::CHEVRON_RIGHT)));
  assert_eq!(stage.text("Theme").0, light(PaletteColor::Primary));
  let (settings, dot) = (stage.bounds("settings"), stage.text_quad("·").unwrap());
  assert!(
    (dot.x - (settings.0 + settings.2) - 12.0).abs() < 0.01,
    "the theme's gap"
  );
}

#[test]
fn a_theme_s_renderer_builds_the_breadcrumb_and_its_links_still_follow() {
  let (followed, tell) = log();
  let app = App::new();
  // The steps one under the other: other nodes, the same behavior.
  app.theme().set_renderer::<Breadcrumb>(Arc::new(|view: BreadcrumbView| {
    let mut column = Column::new().spacing(2.0);
    for index in view.shown() {
      column = column.child(view.step(index));
    }
    Row::new().child(column).into()
  }));
  let mut stage = Stage::in_app(app, move |_| page(board().on_select(tell.clone())));
  let (settings, appearance) = (stage.bounds("settings"), stage.bounds("appearance"));
  assert!(appearance.1 > settings.1, "one under the other");
  stage.click("appearance");
  assert_eq!(*followed.lock().unwrap(), ["/settings/appearance"]);
}
