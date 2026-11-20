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
    BreadcrumbVariant, BreadcrumbView, Case, Interaction, Opacity, icons,
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
  Breadcrumb::default()
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

/// Six steps, of which `max_shown(3)` shows the first, an ellipsis and the last two.
fn long(anchor: ElementRef, open: Signal<bool>) -> Breadcrumb {
  Breadcrumb::new(
    ["Home", "Projects", "lurq", "crates", "ui", "breadcrumb.rs"]
      .into_iter()
      .map(|label| BreadcrumbStep::new(label).id(label.replace('.', "-"))),
  )
  .id("way")
  .max_shown(3)
  .menu(open, anchor)
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
      Breadcrumb::default()
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
      Breadcrumb::default()
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
  // Each of what it shows is dimmed by the theme's opacity for what is disabled, so that a step can be dimmed alone:
  // a layer of what is in it, so the opacity is the layer's and not a quad's.
  let theme = stage.app.theme().clone();
  let case = Case::new().state(BreadcrumbState::Disabled, true);
  let disabled = theme.resolve::<Breadcrumb>(&case, None);
  let enabled = theme.resolve::<Breadcrumb>(&Case::new(), None);
  for part in [
    BreadcrumbPart::Link,
    BreadcrumbPart::Page,
    BreadcrumbPart::Separator,
    BreadcrumbPart::Ellipsis,
  ] {
    assert_eq!(
      disabled.part(part).rest().opacity,
      Some(Opacity::Level(OpacityLevel::Disabled)),
      "{part:?}"
    );
    assert_eq!(enabled.part(part).rest().opacity, None, "{part:?}");
  }
  assert_eq!(disabled.part(BreadcrumbPart::Root).rest().opacity, None, "not twice");
}

#[test]
fn a_disabled_step_is_not_followed_and_tab_passes_it() {
  let (followed, tell) = log();
  let mut stage = Stage::new(move |_| {
    page(
      Breadcrumb::new([
        BreadcrumbStep::new("Home").id("home").disabled(true),
        BreadcrumbStep::new("Settings").id("settings"),
        BreadcrumbStep::new("Theme").id("theme"),
      ])
      .on_select(tell.clone()),
    )
  });
  stage.click("home");
  assert!(followed.lock().unwrap().is_empty(), "a press of it follows nothing");
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!(*followed.lock().unwrap(), ["Settings"], "Tab went past it to Settings");
}

#[test]
fn a_disabled_hidden_step_is_a_disabled_item_of_the_menu() {
  let (followed, tell) = log();
  let mut stage = Stage::new(move |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .padding(20.0)
      .child(
        Breadcrumb::new([
          BreadcrumbStep::new("Home"),
          BreadcrumbStep::new("Projects").id("projects").disabled(true),
          BreadcrumbStep::new("lurq").id("lurq"),
          BreadcrumbStep::new("crates"),
          BreadcrumbStep::new("ui"),
        ])
        .id("way")
        .max_shown(3)
        .on_select(tell.clone())
        .mount(ctx),
      )
      .into()
  });
  stage.click("way-ellipsis");
  stage.click("projects");
  assert!(followed.lock().unwrap().is_empty());
  stage.click("lurq");
  assert_eq!(*followed.lock().unwrap(), ["lurq"]);
}

#[test]
fn a_mounted_breadcrumb_keeps_the_menu_of_its_hidden_steps() {
  let (followed, tell) = log();
  let mut stage = Stage::new(move |ctx| {
    Column::new()
      .size(400.0, 300.0)
      .padding(20.0)
      .child(
        Breadcrumb::new(["Home", "Projects", "lurq", "crates", "ui", "breadcrumb.rs"])
          .id("way")
          .max_shown(3)
          .on_select(tell.clone())
          .mount(ctx),
      )
      .into()
  });
  assert!(!stage.paints("Projects"));
  stage.click("way-ellipsis");
  assert!(
    stage.paints("Projects"),
    "the ellipsis opened the menu with no signal of the app's"
  );
  stage.key("ArrowDown");
  stage.key("Enter");
  assert_eq!(*followed.lock().unwrap(), ["lurq"]);
  stage.pass();
  assert!(!stage.paints("Projects"), "choosing a step closed it");
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn its_label_and_the_ellipsis_s_name_can_be_said_otherwise() {
  let mut stage = Stage::new(|_| {
    page(
      Breadcrumb::new(["Home", "Projects", "lurq", "ui"])
        .id("way")
        .max_shown(3)
        .label("Pfad")
        .more_label("Mehr"),
    )
  });
  let said = |stage: &mut Stage, id: &str| {
    stage
      .tree
      .get_element_by_id_mut(id)
      .and_then(|element| element.description("a11y_name"))
  };
  assert_eq!(said(&mut stage, "way").as_deref(), Some("Pfad"));
  assert_eq!(said(&mut stage, "way-ellipsis").as_deref(), Some("Mehr"));
  let mut plain = Stage::new(|_| {
    page(
      Breadcrumb::new(["Home", "Projects", "lurq", "ui"])
        .id("way")
        .max_shown(3),
    )
  });
  assert_eq!(said(&mut plain, "way").as_deref(), Some("Breadcrumb"));
  assert_eq!(
    said(&mut plain, "way-ellipsis").as_deref(),
    Some("Show the hidden steps")
  );
}

#[test]
fn with_more_steps_than_max_shown_it_shows_the_first_an_ellipsis_and_the_last_ones() {
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
fn max_shown_is_never_fewer_than_the_first_and_the_last() {
  let stage = Stage::new(|_| {
    page(
      Breadcrumb::default()
        .steps(["Settings", "Appearance", "Theme"])
        .max_shown(0)
        .menu(Signal::new(false), ElementRef::new()),
    )
  });
  assert!(stage.paints("Settings") && stage.paints("Theme"));
  assert!(!stage.paints("Appearance"));

  let stage = Stage::new(|_| {
    page(
      Breadcrumb::default()
        .steps(["Settings", "Appearance", "Theme"])
        .max_shown(3),
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
      Breadcrumb::default()
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

/// The board's path control: Macintosh HD, projects, lurq, crates, each segment with an id.
fn bar() -> Breadcrumb {
  Breadcrumb::new([
    BreadcrumbStep::new("Macintosh HD")
      .icon(icons::DRIVE)
      .value("/")
      .id("hd"),
    BreadcrumbStep::new("projects")
      .icon(icons::FOLDER)
      .value("/projects")
      .id("projects"),
    BreadcrumbStep::new("lurq")
      .icon(icons::FOLDER)
      .value("/projects/lurq")
      .id("lurq"),
    BreadcrumbStep::new("crates")
      .icon(icons::FOLDER)
      .value("/projects/lurq/crates")
      .id("crates"),
  ])
  .variant(BreadcrumbVariant::Path)
  .id("bar")
}

/// The color of the veil over the element with `id`: an inset shadow over its whole box.
fn veil_of(stage: &mut Stage, id: &str) -> Option<Color> {
  let (x, y, width, height) = stage.bounds(id);
  stage.quads().into_iter().find_map(|quad| match &quad.content {
    QuadContent::BoxShadow(shadow)
      if shadow.inset && (quad.x, quad.y, quad.width, quad.height) == (x, y, width, height) =>
    {
      Some(shadow.color)
    }
    _ => None,
  })
}

/// A press of the pointer on the text `content`.
fn press_text(stage: &mut Stage, content: &str) {
  let quad = stage
    .text_quad(content)
    .unwrap_or_else(|| panic!("{content:?} should be painted"));
  let (x, y) = (quad.x + quad.width / 2.0, quad.y + quad.height / 2.0);
  stage.tree.mouse_move(x, y);
  stage.tree.mouse_down(x, y, lurq::app::events::MouseButton::Left);
  stage.tree.mouse_up(x, y, lurq::app::events::MouseButton::Left);
  stage.pass();
}

/// A path of seven parts, mounted in a window `width` wide with 10 around it: one frame painted.
fn folding(width: f32, followed: impl Fn(&str) + Send + Sync + Clone + 'static) -> Stage {
  Stage::first_frame(width, 300.0, move |ctx| {
    Column::new()
      .size(width, 300.0)
      .padding(10.0)
      .child(
        Breadcrumb::path("H:\\projects\\lurq\\crates\\gallery\\src\\pages")
          .id("bar")
          .on_select(followed.clone())
          .mount(ctx),
      )
      .into()
  })
}

#[test]
fn a_path_control_is_the_board_s_bar_of_segments_with_the_last_in_a_box() {
  let mut stage = Stage::new(|_| page(bar()));
  // The bar: 28 high on the muted fill with the medium radius, 4 in from its sides.
  let way = stage.bounds("bar");
  assert_eq!(way.3, 28.0);
  assert_eq!(stage.fill_color("bar"), Some(light(PaletteColor::Muted)));
  assert!(
    stage
      .fill("bar")
      .unwrap()
      .border_radius
      .is_some_and(|radius| radius.top_left == 6.0),
    "radius-md"
  );
  // A segment: 22 high in the middle of the bar, 6 in, an icon of 13 in the muted foreground 5 before a text of 12.
  let hd = stage.bounds("hd");
  assert_eq!((hd.0 - way.0, hd.1 - way.1, hd.3), (4.0, 3.0, 22.0));
  let drive = stage.text_quad(&glyph(icons::DRIVE)).unwrap();
  assert_eq!(
    stage.text(&glyph(icons::DRIVE)),
    (light(PaletteColor::MutedForeground), 13.0)
  );
  assert!((drive.x - hd.0 - 6.0).abs() < 0.01, "6 in");
  let label = stage.text_quad("Macintosh HD").unwrap();
  assert_eq!(stage.text("Macintosh HD"), (light(PaletteColor::Foreground), 12.0));
  assert!((label.x - (drive.x + 13.0) - 5.0).abs() < 0.01, "5 after the icon");
  assert!(
    (hd.0 + hd.2 - (label.x + label.width) - 6.0).abs() < 0.01,
    "6 after the text"
  );
  assert_eq!(stage.fill_color("hd"), None, "a segment has no fill at rest");

  // A chevron of 12 in the muted foreground between each two, touching both.
  let chevron = glyph(icons::CHEVRON_RIGHT);
  let chevrons: Vec<_> = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { text, .. } if *text == chevron))
    .collect();
  assert_eq!(chevrons.len(), 3);
  assert_eq!(stage.text(&chevron), (light(PaletteColor::MutedForeground), 12.0));
  let projects = stage.bounds("projects");
  assert!((chevrons[0].x - (hd.0 + hd.2)).abs() < 0.01, "after the segment");
  assert!((projects.0 - (chevrons[0].x + 12.0)).abs() < 0.01, "before the next");

  // The last: on the background's fill in an edge of 1, with the small radius; 4 from the end of the bar.
  let crates = stage.bounds("crates");
  assert_eq!(crates.3, 22.0);
  assert_eq!(stage.fill_color("crates"), Some(light(PaletteColor::Background)));
  let edge = stage.border("crates").expect("the last segment has an edge");
  let side = edge.border.and_then(|border| border.top).unwrap();
  assert_eq!((side.width, side.color), (1.0, light(PaletteColor::Border)));
  assert!(edge.border_radius.is_some_and(|radius| radius.top_left == 4.0));
  assert!((way.0 + way.2 - (crates.0 + crates.2) - 4.0).abs() < 0.01);
  assert_eq!(stage.text("crates"), (light(PaletteColor::Foreground), 12.0));
}

#[test]
fn a_segment_of_a_path_control_is_veiled_under_the_pointer_and_its_press_tells_its_folder() {
  let (followed, tell) = log();
  let mut stage = Stage::new(move |_| page(bar().on_select(tell.clone())));
  assert_eq!(veil_of(&mut stage, "lurq"), None);
  stage.hover("lurq");
  assert_eq!(veil_of(&mut stage, "lurq"), Some(light(PaletteColor::StateHover)));
  stage.click("lurq");
  assert_eq!(*followed.lock().unwrap(), ["/projects/lurq"]);
  stage.click("crates");
  assert_eq!(followed.lock().unwrap().len(), 1, "the last is where one is");
  // From the keyboard: Tab goes on from the segment that was pressed, with the ring of the focus.
  stage.tree.mouse_move(390.0, 290.0);
  stage.key("Tab");
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  stage.key("Enter");
  assert_eq!(followed.lock().unwrap().len(), 2, "Enter follows the focused segment");
}

#[test]
fn the_steps_of_a_path_are_its_parts_each_told_as_the_path_as_far_as_it() {
  let read = |path: &str| -> Vec<(String, String)> {
    BreadcrumbStep::path(path)
      .iter()
      .map(|step| (step.label().to_owned(), step.value_of().to_owned()))
      .collect()
  };
  let pairs = |pairs: &[(&str, &str)]| -> Vec<(String, String)> {
    pairs
      .iter()
      .map(|(label, value)| ((*label).to_owned(), (*value).to_owned()))
      .collect()
  };
  assert_eq!(
    read("H:\\projects\\lurq"),
    pairs(&[
      ("H:", "H:\\"),
      ("projects", "H:\\projects"),
      ("lurq", "H:\\projects\\lurq")
    ])
  );
  assert_eq!(read("C:"), pairs(&[("C:", "C:\\")]));
  assert_eq!(
    read("/usr/local/bin/"),
    pairs(&[
      ("/", "/"),
      ("usr", "/usr"),
      ("local", "/usr/local"),
      ("bin", "/usr/local/bin")
    ]),
    "a separator at the end is no part"
  );
  assert_eq!(
    read("\\\\nas\\media\\films"),
    pairs(&[("\\\\nas\\media", "\\\\nas\\media"), ("films", "\\\\nas\\media\\films")]),
    "a share is one place"
  );
  assert_eq!(
    read("~/notes/todo.md"),
    pairs(&[("~", "~"), ("notes", "~/notes"), ("todo.md", "~/notes/todo.md")])
  );
  assert_eq!(
    read("crates/lurq"),
    pairs(&[("crates", "crates"), ("lurq", "crates/lurq")])
  );
  assert!(read("").is_empty());

  // A drive, the root and a share have the icon of a drive, a home its own, every other part a folder's.
  let icons_of = |path: &str| -> Vec<String> {
    BreadcrumbStep::path(path)
      .iter()
      .map(|step| {
        step
          .icon_name()
          .map(|name| name.as_str().to_owned())
          .unwrap_or_default()
      })
      .collect()
  };
  assert_eq!(icons_of("H:\\projects"), [icons::DRIVE, icons::FOLDER]);
  assert_eq!(icons_of("/usr"), [icons::DRIVE, icons::FOLDER]);
  assert_eq!(icons_of("~/notes"), [icons::HOME, icons::FOLDER]);
}

#[test]
fn a_path_s_press_tells_the_folder_and_the_still_form_drops_nothing() {
  let (followed, tell) = log();
  let mut stage = Stage::new(move |_| page(Breadcrumb::path("/usr/local/bin").id("bar").on_select(tell.clone())));
  for part in ["usr", "local", "bin"] {
    assert!(stage.paints(part), "{part} is shown");
  }
  assert!(!stage.paints(&glyph(icons::MORE_HORIZONTAL)), "nothing is hidden");
  press_text(&mut stage, "local");
  assert_eq!(*followed.lock().unwrap(), ["/usr/local"]);
  press_text(&mut stage, "/");
  assert_eq!(followed.lock().unwrap().last().map(String::as_str), Some("/"));
}

#[test]
fn a_mounted_path_folds_its_middle_into_the_menu_where_it_has_no_room_from_its_first_frame() {
  let middle = ["projects", "lurq", "crates", "gallery", "src"];
  // With room for all of it, all of it is shown and there is no ellipsis.
  let mut wide = folding(800.0, |_| {});
  let first = wide.picture();
  wide.frame();
  assert_eq!(first, wide.picture(), "the first frame is the next one");
  for part in middle {
    assert!(wide.paints(part), "{part} has room");
  }
  assert!(!wide.paints(&glyph(icons::MORE_HORIZONTAL)));

  // In a narrower room the parts nearest the first go, the first and the last stay, and the ellipsis is there.
  for width in [420.0, 340.0, 260.0] {
    let mut stage = folding(width, |_| {});
    let first = stage.picture();
    stage.frame();
    assert_eq!(first, stage.picture(), "at {width}: the first frame is the next one");
    assert!(stage.paints("H:") && stage.paints("pages"), "at {width}: the ends stay");
    assert!(stage.paints(&glyph(icons::MORE_HORIZONTAL)), "at {width}: the ellipsis");
    let shown: Vec<bool> = middle.iter().map(|part| stage.paints(part)).collect();
    assert!(!shown[0], "at {width}: the part after the first went first");
    assert!(
      shown.windows(2).all(|pair| !pair[0] || pair[1]),
      "at {width}: the parts that went are the ones nearest the first: {shown:?}"
    );
    let (x, _, bar_width, _) = stage.bounds("bar");
    assert!(
      x + bar_width <= width - 10.0 + 0.5,
      "at {width}: the bar is in its room"
    );
  }
  // The narrower, the fewer.
  let count = |width: f32| {
    let stage = folding(width, |_| {});
    middle.iter().filter(|part| stage.paints(part)).count()
  };
  assert!(count(420.0) > count(260.0));
}

#[test]
fn the_ellipsis_of_a_folded_path_opens_the_parts_that_went_and_choosing_one_tells_its_folder() {
  let (followed, tell) = log();
  let mut stage = folding(260.0, tell);
  assert!(!stage.paints("projects"));
  stage.click("bar-ellipsis");
  assert!(stage.paints("projects"), "the menu lists what went");
  press_text(&mut stage, "projects");
  assert_eq!(*followed.lock().unwrap(), ["H:\\projects"]);
  stage.pass();
  assert!(!stage.paints("projects"), "choosing closed the menu");
}

#[test]
fn a_last_step_longer_than_its_room_is_cut_and_the_steps_before_it_keep_theirs() {
  let name = "a-folder-with-a-name-far-longer-than-the-room-it-is-given";
  let mut stage = Stage::first_frame(240.0, 200.0, move |ctx| {
    Column::new()
      .size(240.0, 200.0)
      .padding(10.0)
      .child(Breadcrumb::path(&format!("H:\\projects\\{name}")).id("bar").mount(ctx))
      .into()
  });
  stage.frame();
  assert!(stage.paints("H:"), "the first keeps its room");
  assert!(!stage.paints("projects"), "the middle went into the menu first");
  assert!(!stage.paints(name), "the last has no room for all of itself");
  let cut = stage.cut_text_quad(name).expect("the last is cut with an ellipsis");
  let (x, _, width, _) = stage.bounds("bar");
  assert!(x + width <= 230.5, "the bar is in its room");
  assert!(cut.x + cut.width <= x + width, "the cut text is in the bar");

  // The plain breadcrumb, not mounted, has the same rule for its page.
  let mut plain = Stage::new(move |_| {
    Column::new()
      .size(240.0, 200.0)
      .padding(10.0)
      .child(Breadcrumb::new(["Home", name]).id("way"))
      .into()
  });
  assert!(plain.paints("Home"));
  assert!(plain.cut_text_quad(name).is_some() && !plain.paints(name));
  let (x, _, width, _) = plain.bounds("way");
  assert!(x + width <= 230.5);
}

#[test]
fn a_path_that_overflows_and_has_a_most_shown_hides_by_both() {
  let mut stage = Stage::first_frame(800.0, 300.0, move |ctx| {
    Column::new()
      .size(800.0, 300.0)
      .padding(10.0)
      .child(
        Breadcrumb::path("H:\\projects\\lurq\\crates\\gallery\\src\\pages")
          .id("bar")
          .max_shown(4)
          .mount(ctx),
      )
      .into()
  });
  stage.frame();
  // Room for all of it, and no more than four shown: the first, the ellipsis, the last three.
  for part in ["H:", "gallery", "src", "pages"] {
    assert!(stage.paints(part), "{part} is shown");
  }
  for part in ["projects", "lurq", "crates"] {
    assert!(!stage.paints(part), "{part} is over the most shown");
  }
  assert!(stage.paints(&glyph(icons::MORE_HORIZONTAL)));
  stage.click("bar-ellipsis");
  assert!(stage.paints("projects") && stage.paints("crates"));
}

#[cfg(any(feature = "devtools", feature = "mcp"))]
#[test]
fn a_path_is_a_navigation_called_path_to_tooling() {
  let mut stage = Stage::new(|_| page(Breadcrumb::path("/usr/local").id("bar")));
  let said = |stage: &mut Stage, key: &str| {
    stage
      .tree
      .get_element_by_id_mut("bar")
      .and_then(|element| element.description(key))
  };
  assert_eq!(said(&mut stage, "a11y_role").as_deref(), Some("navigation"));
  assert_eq!(said(&mut stage, "a11y_name").as_deref(), Some("Path"));
}

#[test]
fn a_step_in_a_state_of_its_own_is_styled_by_it_and_the_others_are_not() {
  let app = App::new();
  app.theme().patch_recipe::<Breadcrumb>(|rules| {
    rules
      .when(lurq::ui::Selector::new().state("ancestor"), BreadcrumbPart::Page, |s| {
        s.foreground(PaletteColor::MutedForeground)
      })
      .when(lurq::ui::Selector::new().state("mono"), BreadcrumbPart::Link, |s| {
        s.foreground(PaletteColor::Primary)
      })
  });
  let stage = Stage::in_app(app, |_| {
    page(
      Breadcrumb::default()
        .id("way")
        .step(BreadcrumbStep::new("Projects").value("/projects").id("projects"))
        .step(
          BreadcrumbStep::new("KON-12")
            .value("/kon-12")
            .state("mono", true)
            .id("key"),
        )
        .step(BreadcrumbStep::new("src").state("ancestor", true).id("folder")),
    )
  });
  assert_eq!(
    stage.text("Projects").0,
    light(PaletteColor::MutedForeground),
    "a link in no state"
  );
  assert_eq!(
    stage.text("KON-12").0,
    light(PaletteColor::Primary),
    "a link in its own state"
  );
  assert_eq!(
    stage.text("src").0,
    light(PaletteColor::MutedForeground),
    "the page drawn as the steps before it"
  );
  assert_eq!(
    BreadcrumbStep::new("x")
      .state("mono", true)
      .state("mono", false)
      .states(),
    &[] as &[lurq::ui::Name]
  );
}
