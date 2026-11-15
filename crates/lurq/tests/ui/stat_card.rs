//! A stat card: the board's card, its label, its value and its change as a badge in its tone, the small size with
//! media beside the value, a long label, a card that is pressed, what tooling reads, and a theme's rules and
//! renderer.

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use lurq::{
  app::theme::{PaletteColor, ThemePalette},
  components::{Column, Row, Text},
  layout::text_style::FontWeight,
  node::{color::Color, dimension::Dimension},
  ui::{StatCard, StatCardPart, StatCardRenderer, StatCardSize, StatCardVariant, StatCardView},
};

use super::stage::Stage;

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn weight_of(stage: &Stage, content: &str) -> FontWeight {
  stage
    .quads()
    .iter()
    .find_map(|quad| match &quad.content {
      lurq::layout::quad::QuadContent::Text { text, style, .. } if text == content => Some(style.weight),
      _ => None,
    })
    .unwrap()
}

/// The board's card: frames per second, up 12.5% against 0.59.
fn board() -> StatCard {
  StatCard::new("Frames per second", "144")
    .delta("+12.5%")
    .variant(StatCardVariant::Success)
    .description("vs 0.59")
    .id("card")
}

fn stage_of(card: impl Fn() -> StatCard + Send + Sync + 'static) -> Stage {
  Stage::new(move |_| Column::new().width(200.0).child(card()).into())
}

#[test]
fn the_card_says_its_label_its_value_and_its_change() {
  let mut stage = stage_of(board);
  let (x, y, width, _) = stage.bounds("card");
  assert_eq!(width, 200.0);
  assert_eq!(stage.fill_color("card"), Some(light(PaletteColor::Card)));
  let label = stage.text_quad("Frames per second").unwrap();
  assert!(
    (label.x - (x + 20.0)).abs() < 0.5 && (label.y - (y + 20.0)).abs() < 2.0,
    "20 in"
  );
  assert_eq!(
    stage.text("Frames per second"),
    (light(PaletteColor::MutedForeground), 13.0)
  );
  assert_eq!(stage.text("144"), (light(PaletteColor::CardForeground), 28.0));
  assert_eq!(weight_of(&stage, "144"), FontWeight::SemiBold);
  // The change: a badge in the success tone, what it is measured against 8 after it in the mono type.
  assert_eq!(stage.text("+12.5%").0, light(PaletteColor::Success));
  let (delta, against) = (stage.text_quad("+12.5%").unwrap(), stage.text_quad("vs 0.59").unwrap());
  assert!(against.x > delta.x + delta.width);
  assert_eq!(stage.text("vs 0.59"), (light(PaletteColor::MutedForeground), 11.0));
  assert!(stage.font_of("vs 0.59").to_lowercase().contains("mono"));
  // 8 between the label, the value and the change.
  let value = stage.text_quad("144").unwrap();
  assert!(value.y > label.y + label.height && delta.y > value.y + value.height);

  // A change for the worse is in the destructive tone.
  let stage = stage_of(|| {
    StatCard::new("Errors", "12")
      .delta("+3")
      .variant(StatCardVariant::Destructive)
  });
  assert_eq!(stage.text("+3").0, light(PaletteColor::Destructive));
}

#[test]
fn the_small_card_holds_media_beside_its_value() {
  let mut stage = stage_of(|| {
    StatCard::new("Frames per second", "144")
      .size(StatCardSize::Sm)
      .media(Row::new().size(Dimension::Px(80.0), Dimension::Px(28.0)).id("media"))
      .id("card")
  });
  let (x, y, width, height) = stage.bounds("card");
  let label = stage.text_quad("Frames per second").unwrap();
  assert!(
    (label.x - (x + 16.0)).abs() < 0.5 && (label.y - (y + 16.0)).abs() < 2.0,
    "16 in"
  );
  assert_eq!(stage.text("Frames per second").1, 12.0);
  assert_eq!(stage.text("144").1, 24.0);
  let (media_x, media_y, media_width, media_height) = stage.bounds("media");
  assert!(
    (media_x + media_width - (x + width - 16.0)).abs() < 0.5,
    "at the end of the row"
  );
  assert!(
    (media_y + media_height - (y + height - 16.0)).abs() < 1.0,
    "at its foot"
  );
}

#[test]
fn a_long_label_wraps_and_a_card_that_is_pressed_tells_its_handlers() {
  let mut stage = stage_of(|| StatCard::new("Frames per second on the second monitor, at 144 Hz", "141"));
  let label = stage
    .text_quad("Frames per second on the second monitor, at 144 Hz")
    .unwrap();
  assert!(label.height > 20.0 && label.width <= 160.5, "it wraps in the card");

  let pressed = Arc::new(AtomicUsize::new(0));
  let count = pressed.clone();
  let mut stage = stage_of(move || {
    let count = count.clone();
    board().on_click(move |_| {
      count.fetch_add(1, Ordering::Relaxed);
    })
  });
  stage.click("card");
  assert_eq!(pressed.load(Ordering::Relaxed), 1);
  stage.key("Enter");
  assert_eq!(pressed.load(Ordering::Relaxed), 2, "Enter presses it with the focus");
}

#[test]
fn a_rule_and_a_renderer_of_the_theme_change_it() {
  let mut stage = stage_of(|| board().style(StatCardPart::Root, |s| s.padding_x(8.0).padding_y(8.0)));
  let (x, ..) = stage.bounds("card");
  assert!((stage.text_quad("144").unwrap().x - (x + 8.0)).abs() < 0.5);
  let app = lurq::app::App::new();
  let renderer: StatCardRenderer =
    Arc::new(|view: StatCardView| Text::new(&format!("{} = {}", view.label, view.value)).into());
  app.theme().set_renderer::<StatCard>(renderer);
  let stage = Stage::in_app(app, |_| board().into());
  assert!(stage.paints("Frames per second = 144"));
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn a_stat_card_tells_tooling_its_number() {
  let mut stage = stage_of(board);
  let mut said = |name: &str| {
    stage
      .tree
      .get_element_by_id_mut("card")
      .and_then(|element| element.description(name))
  };
  assert_eq!(said("a11y_role").as_deref(), Some("group"));
  assert_eq!(said("a11y_name").as_deref(), Some("Frames per second"));
  assert_eq!(said("a11y_description").as_deref(), Some("144, +12.5% vs 0.59"));
}
