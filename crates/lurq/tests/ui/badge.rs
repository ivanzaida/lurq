//! A badge: its variants and sizes against the board, a dot, an icon, a dot
//! alone, a count and its maximum, a badge on the corner of another element,
//! that it takes no focus, and a theme's rules and renderer.

use std::sync::Arc;

use lurq::{
  app::{
    App,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Row},
  layout::quad::{Quad, QuadContent},
  node::color::Color,
  ui::{Badge, BadgePart, BadgeRenderer, BadgeShape, BadgeSize, BadgeVariant, BadgeView, Button},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

/// A column with room around what it holds, so that what reaches past a badge's element is inside it.
fn scene(children: Vec<lurq::node::Element>) -> lurq::node::Element {
  children
    .into_iter()
    .fold(Column::new().padding(24.0).spacing(16.0), |column, child| {
      column.child(Row::new().overflow_visible().child(child))
    })
    .into()
}

/// The filled rectangle of `side` by `side` inside the box of `id`: a dot.
fn dot_in(stage: &mut Stage, id: &str, side: f32) -> Quad {
  let (x, y, width, height) = stage.bounds(id);
  stage
    .quads()
    .into_iter()
    .find(|quad| {
      matches!(&quad.content, QuadContent::Rect { color, .. } if color.a() > 0)
        && (quad.width, quad.height) == (side, side)
        && quad.x >= x - 0.01
        && quad.y >= y - 0.01
        && quad.x + quad.width <= x + width + 0.01
        && quad.y + quad.height <= y + height + 0.01
    })
    .unwrap_or_else(|| panic!("#{id} should have a dot of {side}"))
}

fn color_of(quad: &Quad) -> Color {
  match quad.content {
    QuadContent::Rect { color, .. } => color,
    _ => panic!("a filled rectangle"),
  }
}

#[test]
fn a_badge_is_a_round_label_22_high_in_the_primary_color() {
  let mut stage = Stage::new(|_| scene(vec![Badge::new("Badge").id("badge").into()]));
  let (x, _, _, height) = stage.bounds("badge");
  assert_eq!(height, 22.0);
  assert_eq!(stage.fill_color("badge"), Some(light(PaletteColor::Primary)));
  let fill = stage.fill("badge").unwrap();
  assert!(
    fill.border_radius.is_some_and(|radius| radius.top_left >= 11.0),
    "round"
  );
  assert_eq!(stage.text("Badge"), (light(PaletteColor::PrimaryForeground), 12.0));
  let label = stage.text_quad("Badge").unwrap();
  assert!((label.x - x - 8.0).abs() < 0.01, "padded 8: {}", label.x - x);
}

#[test]
fn each_variant_has_the_fill_and_the_ink_of_the_board() {
  let variants = [
    (
      BadgeVariant::Default,
      "Default",
      PaletteColor::Primary,
      PaletteColor::PrimaryForeground,
    ),
    (
      BadgeVariant::Secondary,
      "Secondary",
      PaletteColor::Secondary,
      PaletteColor::SecondaryForeground,
    ),
    (
      BadgeVariant::Outline,
      "Outline",
      PaletteColor::Background,
      PaletteColor::Foreground,
    ),
    (
      BadgeVariant::Success,
      "Success",
      PaletteColor::SuccessMuted,
      PaletteColor::Success,
    ),
    (
      BadgeVariant::Warning,
      "Warning",
      PaletteColor::WarningMuted,
      PaletteColor::Warning,
    ),
    (BadgeVariant::Info, "Info", PaletteColor::InfoMuted, PaletteColor::Info),
    (
      BadgeVariant::Destructive,
      "Failed",
      PaletteColor::DestructiveMuted,
      PaletteColor::Destructive,
    ),
  ];
  let shown = variants.clone();
  let mut stage = Stage::new(move |_| {
    scene(
      shown
        .iter()
        .map(|(variant, text, ..)| Badge::new(text).variant(*variant).id(*text).into())
        .collect(),
    )
  });
  for (_, text, fill, ink) in variants {
    assert_eq!(stage.fill_color(text), Some(light(fill)), "{text}");
    assert_eq!(stage.text(text).0, light(ink), "{text}");
  }
  // The outline alone has an edge, in the color of a border.
  let edge = stage.border("Outline").expect("an edge");
  assert_eq!(
    edge.border.and_then(|border| border.top).map(|side| side.color),
    Some(light(PaletteColor::Border))
  );
  assert!(stage.border("Default").is_none());
}

#[test]
fn the_sizes_are_18_22_and_26_high_and_the_square_has_the_small_radius() {
  let mut stage = Stage::new(|_| {
    scene(vec![
      Badge::new("Small").size(BadgeSize::Sm).id("sm").into(),
      Badge::new("Medium").id("md").into(),
      Badge::new("Large").size(BadgeSize::Lg).id("lg").into(),
      Badge::new("v0.60")
        .variant(BadgeVariant::Secondary)
        .shape(BadgeShape::Square)
        .id("square")
        .into(),
    ])
  });
  for (id, text, height, padding, font) in [
    ("sm", "Small", 18.0, 6.0, 11.0),
    ("md", "Medium", 22.0, 8.0, 12.0),
    ("lg", "Large", 26.0, 10.0, 13.0),
  ] {
    let (x, _, _, h) = stage.bounds(id);
    assert_eq!(h, height, "{id}");
    assert_eq!(stage.text(text).1, font, "{id}");
    let label = stage.text_quad(text).unwrap();
    assert!((label.x - x - padding).abs() < 0.01, "{id} padded {padding}");
  }
  let radius = stage.fill("square").unwrap().border_radius.unwrap().top_left;
  assert_eq!(radius, 4.0, "radius-sm");
}

#[test]
fn a_dot_before_the_text_takes_the_variant_and_the_badge_is_outlined() {
  let mut stage = Stage::new(|_| {
    scene(vec![
      Badge::new("Online")
        .variant(BadgeVariant::Success)
        .dot(true)
        .id("online")
        .into(),
      Badge::new("Away")
        .variant(BadgeVariant::Warning)
        .dot(true)
        .id("away")
        .into(),
    ])
  });
  let (x, y, _, height) = stage.bounds("online");
  assert_eq!(height, 22.0);
  assert_eq!(stage.fill_color("online"), Some(light(PaletteColor::Background)));
  assert!(stage.border("online").is_some(), "an edge");
  assert_eq!(stage.text("Online").0, light(PaletteColor::Foreground));
  let dot = dot_in(&mut stage, "online", 7.0);
  assert_eq!(color_of(&dot), light(PaletteColor::Success));
  assert!((dot.x - x - 8.0).abs() < 0.01, "8 from the start");
  assert!((dot.y - y - 7.5).abs() < 0.01, "in the middle");
  let label = stage.text_quad("Online").unwrap();
  assert!((label.x - (dot.x + 7.0) - 6.0).abs() < 0.01, "the text 6 after the dot");
  let (_, _, width, _) = stage.bounds("online");
  let end = x + width;
  assert!(
    (end - (label.x + label.width) - 10.0).abs() < 0.5,
    "padded 10 at the end"
  );
  assert_eq!(color_of(&dot_in(&mut stage, "away", 7.0)), light(PaletteColor::Warning));
}

#[test]
fn an_icon_stands_4_before_the_text() {
  let mut stage = Stage::new(|_| {
    scene(vec![
      Badge::new("Passing")
        .variant(BadgeVariant::Success)
        .icon(lurq::ui::icons::CHECK)
        .id("passing")
        .into(),
    ])
  });
  let (x, ..) = stage.bounds("passing");
  let label = stage.text_quad("Passing").unwrap();
  assert!(
    (label.x - x - (8.0 + 12.0 + 4.0)).abs() < 0.01,
    "padding, an icon of 12, a gap of 4"
  );
}

#[test]
fn a_dot_alone_is_a_circle_and_draws_no_text() {
  let mut stage = Stage::new(|_| {
    scene(vec![
      Badge::new("Unread")
        .variant(BadgeVariant::Info)
        .dot_only(true)
        .id("unread")
        .into(),
      Badge::new("Small").size(BadgeSize::Sm).dot_only(true).id("sm").into(),
    ])
  });
  let (.., width, height) = stage.bounds("unread");
  assert_eq!((width, height), (8.0, 8.0));
  assert_eq!(stage.fill_color("unread"), Some(light(PaletteColor::Info)));
  assert!(
    stage
      .fill("unread")
      .unwrap()
      .border_radius
      .is_some_and(|radius| radius.top_left >= 4.0)
  );
  assert!(!stage.paints("Unread"), "what it says is what it is called, not drawn");
  let (.., width, height) = stage.bounds("sm");
  assert_eq!((width, height), (6.0, 6.0));
  assert_eq!(stage.fill_color("sm"), Some(light(PaletteColor::Primary)));
}

#[test]
fn a_count_is_a_small_round_badge_capped_at_its_maximum_and_nothing_at_zero() {
  let mut stage = Stage::new(|_| {
    scene(vec![
      Badge::count(8).id("eight").into(),
      Badge::count(1).id("one").into(),
      Badge::count(120).id("many").into(),
      Badge::count(12).max(9).id("nine").into(),
      Badge::count(0).id("zero").into(),
      Badge::count(0).show_zero(true).id("shown").into(),
      Badge::count(4).variant(BadgeVariant::Secondary).id("quiet").into(),
    ])
  });
  let (.., height) = stage.bounds("eight");
  assert_eq!(height, 18.0);
  assert_eq!(stage.fill_color("eight"), Some(light(PaletteColor::Destructive)));
  assert_eq!(stage.text("8"), (light(PaletteColor::DestructiveForeground), 11.0));
  let (.., width, height) = stage.bounds("one");
  assert!(
    width >= 18.0 && height == 18.0,
    "at least as wide as it is high: {width}"
  );
  assert!(stage.paints("99+"));
  assert!(!stage.paints("120"));
  assert!(stage.paints("9+"));
  assert!(stage.paints("0"), "shown when told");
  assert!(
    stage.tree.get_element_by_id_mut("zero").is_none(),
    "a count of 0 is nothing"
  );
  assert_eq!(stage.fill_color("quiet"), Some(light(PaletteColor::Secondary)));
}

#[test]
fn a_badge_on_an_element_sits_on_its_top_end_corner_with_a_ring_of_the_background() {
  let mut stage = Stage::new(|_| {
    let square = |id: &str| Row::new().size(36.0, 36.0).id(id.to_owned());
    scene(vec![
      Badge::new("New")
        .variant(BadgeVariant::Destructive)
        .dot_only(true)
        .id("dot")
        .on(square("bell"))
        .into(),
      Badge::count(3)
        .label("notifications")
        .id("count")
        .on(square("inbox"))
        .into(),
      Badge::count(0).id("none").on(square("quiet")).into(),
      Badge::count(1200).max(999).id("wide").on(square("mail")).into(),
    ])
  });
  // The board's Indicator: a dot of 12 at 27, -3 on a box of 36, its round end centred 3 in from the corner.
  let (x, y, width, height) = stage.bounds("bell");
  assert_eq!(
    (width, height),
    (36.0, 36.0),
    "the badge does not make the element larger"
  );
  let (dx, dy, dw, dh) = stage.bounds("dot");
  assert_eq!((dw, dh), (12.0, 12.0));
  assert!(
    (dx - x - 27.0).abs() < 0.01 && (dy - y + 3.0).abs() < 0.01,
    "at {}, {}",
    dx - x,
    dy - y
  );
  assert_eq!(stage.fill_color("dot"), Some(light(PaletteColor::Destructive)));
  assert!(
    stage.has_ring(light(PaletteColor::Background)),
    "a ring of the background around it"
  );

  // A count of 18: its middle on the same point, 3 in from the corner, however wide its number makes it, so
  // it reaches as far to either side of the point and 6 over the top.
  let (x, y, width, _) = stage.bounds("inbox");
  let (cx, cy, cw, _) = stage.bounds("count");
  assert!(
    (cx + cw / 2.0 - (x + width - 3.0)).abs() < 0.01,
    "its middle 3 in from the end: {}",
    cx + cw / 2.0 - (x + width)
  );
  assert!((cy - y + 6.0).abs() < 0.01, "6 over the top");
  assert!(stage.paints("3"));
  let (wx, _, ww, _) = stage.bounds("wide");
  let (x, _, width, _) = stage.bounds("mail");
  assert!(ww > 18.0, "wider than it is high: {ww}");
  assert!((wx + ww / 2.0 - (x + width - 3.0)).abs() < 0.01, "a wider one too");

  // A count that is not drawn leaves the element alone.
  assert!(stage.tree.get_element_by_id_mut("none").is_none());
  let (.., width, height) = stage.bounds("quiet");
  assert_eq!((width, height), (36.0, 36.0));
}

#[test]
fn a_badge_takes_no_focus_and_tab_passes_it_by() {
  let mut stage = Stage::new(|_| {
    scene(vec![
      Badge::new("Beta").id("beta").into(),
      Badge::count(2).on(Button::new("Inbox").id("inbox")).into(),
    ])
  });
  // The first stop of Tab is the button the count is on: the badges are not stops.
  stage.key("Tab");
  assert!(stage.has_ring(light(PaletteColor::Ring)));
  let focused = stage
    .tree
    .focused_element()
    .and_then(|element| element.id().map(str::to_owned));
  assert_eq!(focused.as_deref(), Some("inbox"), "Tab reaches the button, not a badge");
}

#[test]
fn a_theme_restyles_a_badge_and_draws_another() {
  let app = App::new();
  app.theme().patch_recipe::<Badge>(|rules| {
    rules
      .base(BadgePart::Root, |s| s.height(30.0))
      .variant(BadgeVariant::Success, BadgePart::Root, |s| {
        s.background(PaletteColor::Success).foreground(PaletteColor::Background)
      })
  });
  let mut stage = Stage::in_app(app, |_| {
    scene(vec![
      Badge::new("Passing")
        .variant(BadgeVariant::Success)
        .id("passing")
        .into(),
    ])
  });
  let (.., height) = stage.bounds("passing");
  assert_eq!(height, 30.0);
  assert_eq!(stage.fill_color("passing"), Some(light(PaletteColor::Success)));
  assert_eq!(stage.text("Passing").0, light(PaletteColor::Background));

  // A renderer that draws the text in capitals, in the root the recipe still styles.
  let app = App::new();
  let renderer: BadgeRenderer = Arc::new(|view: BadgeView| {
    let part = view.part(BadgePart::Label);
    view.root().child(part.text(&view.text.to_uppercase())).into()
  });
  app.theme().set_renderer::<Badge>(renderer);
  let mut stage = Stage::in_app(app, |_| scene(vec![Badge::new("Beta").id("beta").into()]));
  assert!(stage.paints("BETA"));
  assert!(!stage.paints("Beta"));
  assert_eq!(stage.fill_color("beta"), Some(light(PaletteColor::Primary)));
}
