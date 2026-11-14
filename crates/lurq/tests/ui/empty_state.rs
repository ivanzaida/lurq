//! An empty state: the board's medium one in its edge, with its well, its text and its action, all centred; the
//! table's small one and the phone's large one; an element of the app's in place of the icon; a height it is centred
//! in; that only its actions take the focus; a theme's rules and renderer.

use std::sync::Arc;

use lurq::{
  app::{
    App,
    theme::{PaletteColor, RadiusSize, ThemePalette},
  },
  components::{Column, Rect},
  layout::quad::QuadContent,
  node::{Element, color::Color},
  ui::{Button, ButtonVariant, EmptyState, EmptyStatePart, EmptyStateRenderer, EmptyStateSize, EmptyStateView, icons},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn glyph(name: &str) -> String {
  icons::ALL
    .iter()
    .find(|(held, _)| *held == name)
    .map(|(_, glyph)| glyph.to_string())
    .unwrap_or_else(|| panic!("no icon {name}"))
}

/// A column 320 wide, as the board's empty state is, with the empty state in it.
fn scene(empty: impl Into<Element>) -> Element {
  Column::new().width(320.0).child(empty).into()
}

fn windows() -> EmptyState {
  EmptyState::new("No windows yet")
    .description("Open a window to see it listed here.")
    .icon(icons::INBOX)
    .action(
      Button::new("New window")
        .icon(icons::PLUS)
        .variant(ButtonVariant::Outline)
        .id("new"),
    )
    .id("empty")
}

#[test]
fn the_medium_one_is_in_an_edge_padded_32_with_its_well_text_and_action_16_apart() {
  let mut stage = Stage::new(|_| scene(windows()));
  let (x, y, width, _) = stage.bounds("empty");
  assert_eq!(width, 320.0, "as wide as the room it is given");
  let edge = stage.border("empty").expect("an edge");
  assert_eq!(
    edge.border.and_then(|border| border.top).map(|side| side.color),
    Some(light(PaletteColor::Border))
  );
  let radius = stage.app.theme().radius_value(RadiusSize::Xl);
  assert!(edge.border_radius.is_some_and(|corners| corners.top_left == radius));

  // The well: 40 on the muted fill, 32 under the top, centred.
  let well = stage
    .quads()
    .into_iter()
    .find(|quad| matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Muted)))
    .expect("a well");
  assert_eq!((well.width, well.height), (40.0, 40.0));
  assert!((well.y - y - 32.0).abs() < 0.01, "padded 32: {}", well.y - y);
  assert!((well.x + 20.0 - (x + 160.0)).abs() < 0.01, "centred");
  let radius = stage.app.theme().radius_value(RadiusSize::Lg);
  assert!(well.border_radius.is_some_and(|corners| corners.top_left == radius));
  // Its icon: 20 in the foreground, in the middle of the well.
  let icon = stage.text_quad(&glyph(icons::INBOX)).expect("the icon");
  assert_eq!(
    stage.text(&glyph(icons::INBOX)),
    (light(PaletteColor::Foreground), 20.0)
  );
  assert!((icon.x + icon.width / 2.0 - (well.x + 20.0)).abs() < 0.01);

  // The title 16 under the well, the description 4 under it, both centred.
  let title = stage.text_quad("No windows yet").unwrap();
  assert_eq!(stage.text("No windows yet"), (light(PaletteColor::Foreground), 14.0));
  assert!(
    (title.y - (well.y + 40.0) - 16.0).abs() < 0.5,
    "{}",
    title.y - well.y - 40.0
  );
  let description = stage.text_quad("Open a window to see it listed here.").unwrap();
  assert_eq!(
    stage.text("Open a window to see it listed here."),
    (light(PaletteColor::MutedForeground), 13.0)
  );
  assert!(description.y > title.y + title.height - 0.01);
  for quad in [&title, &description] {
    let middle = quad.x + quad.width / 2.0;
    assert!((middle - (x + 160.0)).abs() < 1.0, "centred: {middle}");
  }

  // The action under the text, centred.
  let (button_x, button_y, button_width, _) = stage.bounds("new");
  assert!(button_y > description.y + description.height + 15.0);
  assert!((button_x + button_width / 2.0 - (x + 160.0)).abs() < 0.5);
}

#[test]
fn the_small_one_is_the_tables_with_the_icon_alone_and_no_edge() {
  let mut stage = Stage::new(|_| {
    scene(
      EmptyState::new("No crates match")
        .description("Check the spelling or clear the filter.")
        .icon(icons::SEARCH_CLOSE)
        .size(EmptyStateSize::Sm)
        .id("empty"),
    )
  });
  assert!(stage.border("empty").is_none(), "no edge");
  assert!(
    !stage
      .quads()
      .iter()
      .any(|quad| matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Muted))),
    "no well"
  );
  let (_, y, ..) = stage.bounds("empty");
  let icon = stage.text_quad(&glyph(icons::SEARCH_CLOSE)).unwrap();
  assert_eq!(
    stage.text(&glyph(icons::SEARCH_CLOSE)),
    (light(PaletteColor::MutedForeground), 22.0)
  );
  assert!((icon.y - y - 32.0).abs() < 0.01, "padded 32");
  let title = stage.text_quad("No crates match").unwrap();
  assert!((title.y - (icon.y + 22.0) - 10.0).abs() < 0.5, "10 under the icon");
}

#[test]
fn the_large_one_is_the_phones_with_a_round_well_of_64() {
  let mut stage = Stage::new(|_| {
    scene(
      EmptyState::new("You're all caught up")
        .description("New review requests will show up here.")
        .icon(icons::INBOX)
        .size(EmptyStateSize::Lg)
        .id("empty"),
    )
  });
  assert!(stage.border("empty").is_none(), "no edge");
  let well = stage
    .quads()
    .into_iter()
    .find(|quad| matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Muted)))
    .expect("a well");
  assert_eq!((well.width, well.height), (64.0, 64.0));
  assert!(
    well.border_radius.is_some_and(|corners| corners.top_left >= 32.0),
    "round"
  );
  assert_eq!(stage.text(&glyph(icons::INBOX)).1, 28.0);
  assert_eq!(stage.text("You're all caught up").1, 18.0);
  assert_eq!(stage.text("New review requests will show up here.").1, 15.0);
}

#[test]
fn an_edge_is_drawn_or_not_as_told() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(320.0)
      .child(EmptyState::new("Plain").bordered(false).id("plain"))
      .child(
        EmptyState::new("Edged")
          .size(EmptyStateSize::Sm)
          .bordered(true)
          .id("edged"),
      )
      .into()
  });
  assert!(stage.border("plain").is_none());
  assert!(stage.border("edged").is_some());
}

#[test]
fn an_element_of_the_apps_stands_in_place_of_the_icon() {
  let mut stage = Stage::new(|_| {
    scene(
      EmptyState::new("Nothing yet")
        .icon(icons::INBOX)
        .media(Rect::new(96.0, 72.0).background(PaletteColor::Primary).id("picture"))
        .id("empty"),
    )
  });
  let (x, y, width, height) = stage.bounds("picture");
  assert_eq!((width, height), (96.0, 72.0));
  let (root_x, root_y, ..) = stage.bounds("empty");
  assert!((y - root_y - 32.0).abs() < 0.01);
  assert!((x + 48.0 - (root_x + 160.0)).abs() < 0.5, "centred");
  assert!(!stage.paints(&glyph(icons::INBOX)), "the icon is not drawn");
}

#[test]
fn given_a_height_what_it_holds_is_centred_in_it() {
  let mut stage = Stage::new(|_| scene(EmptyState::new("Empty").height(240.0).bordered(false).id("empty")));
  let (_, y, _, height) = stage.bounds("empty");
  assert_eq!(height, 240.0);
  let title = stage.text_quad("Empty").unwrap();
  let middle = title.y + title.height / 2.0;
  assert!((middle - (y + 120.0)).abs() < 0.5, "centred: {}", middle - y);
}

#[test]
fn only_its_actions_take_the_focus() {
  let mut stage = Stage::new(|_| scene(windows()));
  stage.key("Tab");
  let focused = stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned));
  assert_eq!(focused.as_deref(), Some("new"));
}

#[test]
fn a_theme_restyles_an_empty_state_and_draws_another() {
  let app = App::new();
  app.theme().patch_recipe::<EmptyState>(|rules| {
    rules.base(EmptyStatePart::Title, |s| {
      s.font_size(20.0).foreground(PaletteColor::Primary)
    })
  });
  let stage = Stage::in_app(app, |_| scene(windows()));
  assert_eq!(stage.text("No windows yet"), (light(PaletteColor::Primary), 20.0));

  // A renderer that draws the title alone, in the root the recipe still styles.
  let app = App::new();
  let renderer: EmptyStateRenderer = Arc::new(|view: EmptyStateView| {
    let part = view.part(EmptyStatePart::Title);
    view.root().child(part.text(&view.title.to_uppercase())).into()
  });
  app.theme().set_renderer::<EmptyState>(renderer);
  let mut stage = Stage::in_app(app, |_| scene(windows()));
  assert!(stage.paints("NO WINDOWS YET"));
  assert!(!stage.paints("Open a window to see it listed here."));
  assert!(stage.border("empty").is_some());
}
