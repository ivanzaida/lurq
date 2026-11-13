//! A card: its surface, header, content and footer where the board has
//! them, its variants and its small size, a title that wraps, a card that is
//! pressed as a whole and one that is read, and a theme's rule and renderer.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::{
    App,
    theme::{OpacityLevel, PaletteColor, ShadowStyle, ThemePalette},
  },
  components::{Column, Row},
  layout::quad::{Quad, QuadContent},
  node::{BoxShadowValue, color::Color},
  ui::{
    Button, ButtonVariant, Card, CardPart, CardRenderer, CardSize, CardState, CardVariant, CardView, Case, Interaction,
    Opacity,
  },
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// Counts what is pressed.
fn counter() -> (Arc<AtomicUsize>, impl Fn() + Send + Sync + Clone + 'static) {
  let count = Arc::new(AtomicUsize::new(0));
  let add = count.clone();
  (count, move || {
    add.fetch_add(1, Ordering::SeqCst);
  })
}

/// The color of the edge of the element with `id`, where it has one.
fn edge(stage: &mut Stage, id: &str) -> Option<Color> {
  stage
    .border(id)
    .and_then(|quad| quad.border)
    .and_then(|border| border.top)
    .map(|side| side.color)
}

/// The board's card: a title, a description, a row of content and two buttons in the footer.
fn appearance() -> Card {
  Card::new()
    .width(360.0)
    .id("card")
    .title("Appearance")
    .description("How lurq looks on this device.")
    .child(Row::new().size(312.0, 40.0).id("content"))
    .footer(Button::new("Cancel").variant(ButtonVariant::Outline).id("cancel"))
    .footer(Button::new("Save").id("save"))
}

/// The line between the body and the footer: drawn along the bottom of a box as wide as the card, or as a
/// rectangle of 1 in the border's color.
fn line_over_the_footer(stage: &Stage, x: f32, width: f32) -> Option<Quad> {
  stage.quads().into_iter().find(|quad| {
    let as_edge = quad
      .border
      .and_then(|border| border.bottom.filter(|_| border.top.is_none()))
      .is_some_and(|side| side.color == light(PaletteColor::Border) && side.width == 1.0);
    let as_fill = matches!(&quad.content, QuadContent::Rect { color, .. } if *color == light(PaletteColor::Border))
      && quad.height == 1.0;
    (as_edge || as_fill) && (quad.x - x).abs() < 0.5 && (quad.width - width).abs() < 0.5
  })
}

#[test]
fn a_card_is_the_boards_surface_with_its_header_content_and_footer() {
  let mut stage = Stage::new(|_| Column::new().padding(8.0).child(appearance()).into());
  let (x, y, width, height) = stage.bounds("card");
  assert_eq!(width, 360.0);

  // The card's color, an edge of the border's color, corners of 12 and the small shadow.
  assert_eq!(stage.fill_color("card"), Some(light(PaletteColor::Card)));
  assert!(
    stage
      .fill("card")
      .unwrap()
      .border_radius
      .is_some_and(|radius| radius.top_left == 12.0),
    "radius-xl"
  );
  let surface = stage.border("card").expect("an outlined card has an edge");
  let side = surface.border.and_then(|border| border.top).unwrap();
  assert_eq!((side.width, side.color), (1.0, light(PaletteColor::Border)));
  assert!(
    stage
      .quads()
      .iter()
      .any(|quad| matches!(quad.content, QuadContent::BoxShadow(_))),
    "the card casts a shadow"
  );

  // The title 16/600 in the card's ink, 24 from the top and the start, the description 14 muted under it.
  assert_eq!(stage.text("Appearance"), (light(PaletteColor::CardForeground), 16.0));
  assert_eq!(
    stage.text("How lurq looks on this device."),
    (light(PaletteColor::MutedForeground), 14.0)
  );
  let title = stage.text_quad("Appearance").unwrap();
  let description = stage.text_quad("How lurq looks on this device.").unwrap();
  assert!((title.x - (x + 24.0)).abs() < 0.5);
  assert!((title.y - (y + 24.0)).abs() < 2.0, "24 under the top: {} {y}", title.y);
  assert!((description.x - title.x).abs() < 0.5);
  assert!(description.y > title.y + title.height, "under the title");

  // The content 24 from the start and 24 under the header.
  let (content_x, content_y, ..) = stage.bounds("content");
  assert!((content_x - (x + 24.0)).abs() < 0.5);
  assert!(
    content_y - (description.y + description.height) >= 22.0,
    "24 under the header"
  );

  // The buttons at the end, 8 apart, 24 from the end and 16 from the bottom, under a line across the card.
  let (cancel_x, _, cancel_width, _) = stage.bounds("cancel");
  let (save_x, save_y, save_width, save_height) = stage.bounds("save");
  assert!((save_x + save_width - (x + width - 24.0)).abs() < 0.5);
  assert!((save_x - (cancel_x + cancel_width) - 8.0).abs() < 0.5);
  assert!((save_y + save_height - (y + height - 16.0)).abs() < 0.5);
  let line = line_over_the_footer(&stage, x, width).expect("a line sets the footer off");
  let line_bottom = line.y + line.height;
  assert!((save_y - 16.0 - line_bottom).abs() < 1.0, "16 under the line");
  assert!(
    (line_bottom - (content_y + 40.0 + 24.0)).abs() < 1.0,
    "24 under the content"
  );
}

#[test]
fn a_card_with_children_alone_is_a_padded_surface() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .child(
        Card::new()
          .width(200.0)
          .id("card")
          .child(Row::new().size(50.0, 20.0).id("inside")),
      )
      .into()
  });
  let (x, y, width, height) = stage.bounds("card");
  let (inside_x, inside_y, ..) = stage.bounds("inside");
  assert_eq!((inside_x - x, inside_y - y), (24.0, 24.0));
  assert_eq!((width, height), (200.0, 68.0));
  assert!(line_over_the_footer(&stage, x, width).is_none(), "no footer, no line");
}

#[test]
fn a_card_is_as_wide_as_the_room_it_is_given() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .width(300.0)
      .padding(10.0)
      .child(Card::new().id("card").title("Storage"))
      .into()
  });
  assert_eq!(stage.bounds("card").2, 280.0);
}

#[test]
fn a_long_title_wraps_in_the_room_the_action_leaves_it() {
  const LONG: &str = "Notifications from every workspace you are a member of, as they come";
  let mut stage = Stage::new(|_| {
    Column::new()
      .spacing(16.0)
      .child(Card::new().width(240.0).id("short").title("Notifications"))
      .child(
        Card::new()
          .width(240.0)
          .id("long")
          .title(LONG)
          .action(Button::new("Edit").variant(ButtonVariant::Ghost).id("edit")),
      )
      .into()
  });
  let (_, _, _, short) = stage.bounds("short");
  let (x, y, width, long) = stage.bounds("long");
  assert!(long > short + 10.0, "on more lines: {long} {short}");

  // The action at the end of the header, 24 from the end and from the top.
  let (edit_x, edit_y, edit_width, _) = stage.bounds("edit");
  assert!((edit_x + edit_width - (x + width - 24.0)).abs() < 0.5);
  assert!((edit_y - (y + 24.0)).abs() < 0.5);
  // No line of the title runs under the action or out of the card.
  let lines: Vec<Quad> = stage
    .quads()
    .into_iter()
    .filter(|quad| matches!(&quad.content, QuadContent::Text { .. }) && quad.y > y && quad.y < y + long)
    .filter(|quad| quad.x < edit_x - 1.0)
    .collect();
  assert!(!lines.is_empty());
  for line in lines {
    assert!(
      line.x + line.width <= edit_x - 16.0 + 0.5,
      "beside the action: {} + {}",
      line.x,
      line.width
    );
  }
}

#[test]
fn the_variants_and_the_small_size() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .spacing(16.0)
      .child(
        Card::new()
          .width(196.0)
          .size(CardSize::Sm)
          .id("small")
          .title("Outline")
          .description("A hairline border."),
      )
      .child(
        Card::new()
          .width(196.0)
          .variant(CardVariant::Elevated)
          .id("elevated")
          .title("Elevated"),
      )
      .child(
        Card::new()
          .width(196.0)
          .variant(CardVariant::Filled)
          .id("filled")
          .title("Filled"),
      )
      .into()
  });

  // The small card is padded 16, with a title of 14 and a description of 13.
  let (x, y, ..) = stage.bounds("small");
  let title = stage.text_quad("Outline").unwrap();
  assert!((title.x - (x + 16.0)).abs() < 0.5);
  assert!((title.y - (y + 16.0)).abs() < 2.0);
  assert_eq!(stage.text("Outline").1, 14.0);
  assert_eq!(stage.text("A hairline border.").1, 13.0);

  // The elevated card has no edge and the medium shadow; the filled one the muted color and no edge.
  assert_eq!(edge(&mut stage, "elevated"), None);
  assert_eq!(stage.fill_color("elevated"), Some(light(PaletteColor::Card)));
  assert_eq!(edge(&mut stage, "filled"), None);
  assert_eq!(stage.fill_color("filled"), Some(light(PaletteColor::Muted)));
  let theme = stage.app.theme().clone();
  let shadow = |variant: CardVariant| {
    theme
      .resolve::<Card>(&Case::new().variant(variant), None)
      .part(CardPart::Root)
      .rest()
      .shadow
  };
  assert_eq!(
    shadow(CardVariant::Outline),
    Some(BoxShadowValue::Theme(ShadowStyle::Sm))
  );
  assert_eq!(
    shadow(CardVariant::Elevated),
    Some(BoxShadowValue::Theme(ShadowStyle::Md))
  );
  assert_eq!(shadow(CardVariant::Filled), None);
}

#[test]
fn a_pressed_card_is_one_stop_and_its_footer_keeps_its_own_presses() {
  let (cards, card) = counter();
  let (buttons, button) = counter();
  let mut stage = Stage::new(move |_| {
    let (card, button) = (card.clone(), button.clone());
    Column::new()
      .padding(8.0)
      .child(
        Card::new()
          .width(300.0)
          .id("card")
          .title("lurq")
          .description("A Rust UI toolkit with GPU-backed rendering.")
          .on_click(move |_| card())
          .footer(Button::new("Star").id("star").on_click(move |_| button())),
      )
      .into()
  });
  stage.click("card");
  assert_eq!(cards.load(Ordering::SeqCst), 1, "a press of the card");
  stage.click("star");
  assert_eq!(buttons.load(Ordering::SeqCst), 1);
  assert_eq!(
    cards.load(Ordering::SeqCst),
    1,
    "a press of the button is the button's alone"
  );

  // Tab reaches the card, which draws the ring, and Enter presses it; the next Tab reaches the button.
  stage.key("Tab");
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  stage.key("Enter");
  assert_eq!(cards.load(Ordering::SeqCst), 2);
  stage.key("Tab");
  stage.key(" ");
  assert_eq!(buttons.load(Ordering::SeqCst), 2);
  assert_eq!(cards.load(Ordering::SeqCst), 2);
}

#[test]
fn a_card_that_is_read_is_no_stop_and_what_is_in_it_keeps_its_own() {
  let (saved, save) = counter();
  let mut stage = Stage::new(move |_| {
    let save = save.clone();
    Column::new()
      .padding(8.0)
      .child(
        Card::new()
          .width(300.0)
          .id("card")
          .title("Profile")
          .footer(Button::new("Save").id("save").on_click(move |_| save())),
      )
      .into()
  });
  // The first stop of Tab is the button in the footer: Enter presses it.
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!(saved.load(Ordering::SeqCst), 1);
  // The pointer over it changes nothing.
  stage.hover("card");
  assert_eq!(edge(&mut stage, "card"), Some(light(PaletteColor::Border)));
}

#[test]
fn a_pressed_card_lifts_under_the_pointer() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(
        Card::new()
          .width(200.0)
          .id("card")
          .title("Interactive")
          .on_click(|_| ()),
      )
      .into()
  });
  assert_eq!(edge(&mut stage, "card"), Some(light(PaletteColor::Border)));
  stage.hover("card");
  assert_eq!(
    edge(&mut stage, "card"),
    Some(light(PaletteColor::Ring)),
    "an edge of the ring's color"
  );
  let theme = stage.app.theme().clone();
  let recipe = theme.resolve::<Card>(&Case::new().state(CardState::Pressable, true), None);
  assert_eq!(
    recipe.part(CardPart::Root).styles().hovered.shadow,
    Some(BoxShadowValue::Theme(ShadowStyle::Md)),
    "the medium shadow"
  );
}

#[test]
fn a_disabled_card_is_dimmed_and_takes_nothing() {
  let (cards, card) = counter();
  let mut stage = Stage::new(move |_| {
    let card = card.clone();
    Column::new()
      .padding(8.0)
      .child(
        Card::new()
          .width(200.0)
          .id("card")
          .title("Archived")
          .disabled(true)
          .on_click(move |_| card()),
      )
      .into()
  });
  stage.click("card");
  stage.key("Tab");
  stage.key("Enter");
  assert_eq!(cards.load(Ordering::SeqCst), 0);
  assert!(!stage.has_ring(light(PaletteColor::Ring)));
  let theme = stage.app.theme().clone();
  let root = theme
    .resolve::<Card>(&Case::new().state(CardState::Disabled, true), None)
    .part(CardPart::Root)
    .rest();
  assert_eq!(root.opacity, Some(Opacity::Level(OpacityLevel::Disabled)));
}

#[test]
fn a_forced_focus_draws_the_ring_of_a_pressed_card() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .padding(8.0)
      .child(
        Card::new()
          .width(200.0)
          .title("Focused")
          .on_click(|_| ())
          .force(Some(Interaction::FocusVisible)),
      )
      .into()
  });
  stage.pass();
  assert!(stage.has_ring(light(PaletteColor::Ring)));
}

#[test]
fn a_rule_and_a_renderer_of_a_theme_change_it() {
  let mut stage = Stage::new(|_| {
    Column::new()
      .child(
        Card::new()
          .width(200.0)
          .id("card")
          .title("Ruled")
          .style(CardPart::Root, |s| s.background(PaletteColor::Secondary))
          .style(CardPart::Title, |s| s.font_size(20.0)),
      )
      .into()
  });
  assert_eq!(stage.fill_color("card"), Some(light(PaletteColor::Secondary)));
  assert_eq!(stage.text("Ruled").1, 20.0);

  // A card that draws its body alone, without its footer: other nodes, the same recipe.
  let app = App::new();
  let renderer: CardRenderer = Arc::new(|mut view: CardView| {
    let body = view.body();
    let mut root = view.read_root();
    if let Some(body) = body {
      root = root.child(body);
    }
    root.into()
  });
  app.theme().set_renderer::<Card>(renderer);
  let mut stage = Stage::in_app(app, |_| {
    Column::new()
      .child(
        Card::new()
          .width(200.0)
          .id("card")
          .title("Body alone")
          .footer(Button::new("Hidden")),
      )
      .into()
  });
  assert!(stage.paints("Body alone"));
  assert!(!stage.paints("Hidden"));
  assert_eq!(
    stage.fill_color("card"),
    Some(light(PaletteColor::Card)),
    "the recipe still styles it"
  );
}
