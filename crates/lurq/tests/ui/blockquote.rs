//! `ui::Blockquote`: the design's quote (`mlxnG`).

use lurq::{
  app::{
    App,
    theme::{PaletteColor, ThemePalette},
  },
  components::{Column, Text},
  layout::{quad::QuadContent, text_style::FontStyle},
  node::color::Color,
  ui::{Blockquote, BlockquotePart},
};

use super::stage::Stage;

const SAID: &str = "A design system is a set of decisions you no longer have to make.";

fn light(role: PaletteColor) -> Color {
  ThemePalette::light().get(role)
}

fn staged(scene: impl Fn() -> Blockquote + Send + Sync + 'static) -> Stage {
  let app = App::new();
  app
    .theme()
    .set_preferred_appearance(Some(lurq::app::window::Appearance::Light));
  Stage::in_app(app, move |_| Column::new().width(380.0).child(scene()).into())
}

#[test]
fn the_quote_is_the_boards() {
  let mut stage = staged(|| Blockquote::new(SAID).id("quote"));
  let (x, y, width, height) = stage.bounds("quote");
  assert_eq!((x, y, width), (0.0, 0.0, 380.0), "it fills its room");
  // Two lines of 21 at 380, padded 4 above and below: 50, as the board's.
  assert!((height - 50.0).abs() < 1.0, "{height}");
  // The line at its start: 2 wide, in Border, along the quote and no other side.
  let line = stage
    .quads()
    .into_iter()
    .find_map(|quad| {
      let at = (quad.x, quad.y, quad.width, quad.height) == (x, y, width, height);
      let side = quad
        .border
        .and_then(|border| border.left.filter(|_| border.top.is_none()))?;
      at.then_some((side.width, side.color))
    })
    .expect("the line at its start");
  assert_eq!(line, (2.0, light(PaletteColor::Border)));
  // The text: 14, italic, on lines of 1.5, in MutedForeground, 16 in.
  let (style, text_x) = stage
    .quads()
    .into_iter()
    .find_map(|quad| match quad.content {
      QuadContent::Text { text, style, .. } if text == SAID => Some((style, quad.x)),
      _ => None,
    })
    .expect("the text");
  assert_eq!(text_x, 16.0);
  assert_eq!(style.font_size, 14.0);
  assert_eq!(style.style, FontStyle::Italic);
  assert_eq!(style.line_height, 1.5);
  assert_eq!(style.color, light(PaletteColor::MutedForeground));
}

#[test]
fn a_quote_holds_paragraphs_under_its_text_or_alone() {
  let mut stage = staged(|| {
    Blockquote::new("")
      .id("quote")
      .child(Text::new("First paragraph.").id("one"))
      .child(Text::new("Second paragraph.").id("two"))
  });
  let (_, one_y, _, one_height) = stage.bounds("one");
  let (two_x, two_y, ..) = stage.bounds("two");
  assert_eq!(one_y, 4.0, "no text over them");
  assert_eq!(two_x, 16.0);
  assert!((two_y - (one_y + one_height + 8.0)).abs() < 0.01, "8 apart");
  // What it holds takes its color.
  let (color, _) = stage.text("First paragraph.");
  assert_eq!(color, light(PaletteColor::MutedForeground));
}

#[test]
fn a_theme_draws_it_otherwise() {
  let stage = staged(|| Blockquote::new(SAID).style(BlockquotePart::Label, |s| s.foreground(PaletteColor::Foreground)));
  let (color, _) = stage.text(SAID);
  assert_eq!(color, light(PaletteColor::Foreground));
}

#[cfg(any(feature = "mcp", feature = "devtools"))]
#[test]
fn to_tooling_it_is_a_blockquote_called_by_its_label() {
  let mut stage = staged(|| Blockquote::new(SAID).label("Ivan, on design systems").id("quote"));
  let mut said = |key: &str| {
    stage
      .tree
      .get_element_by_id_mut("quote")
      .and_then(|element| element.description(key))
  };
  assert_eq!(said("a11y_role").as_deref(), Some("blockquote"));
  assert_eq!(said("a11y_name").as_deref(), Some("Ivan, on design systems"));
}
