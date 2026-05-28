use lurq::{
  layout::{Alignment, text_style::FontWeight},
  node::{Element, color::Color, dimension::Dimension},
};

use crate::style::{BG, BORDER, SURFACE, TEXT, TEXT_MUTED, text};

const FILL_WIDTH: Dimension = Dimension::Pct(100.0);
const CONTENT_PAD: f32 = 32.0;
const CARD_RADIUS: f32 = 8.0;

pub(crate) fn text_content() -> Element {
  lurq::components::Column::new()
    .spacing(24.0)
    .child(text("Typography", 28.0, FontWeight::Bold, TEXT).width(FILL_WIDTH))
    .child(section_title("Font Sizes"))
    .child(font_sizes())
    .child(section_title("Font Weights"))
    .child(font_weights())
    .child(section_title("Text Colors"))
    .child(text_colors())
    .pad(CONTENT_PAD)
    .width(FILL_WIDTH)
    .fill(BG)
    .into()
}

fn section_title(label: &str) -> Element {
  text(label, 18.0, FontWeight::Bold, TEXT).width(FILL_WIDTH).into()
}

fn font_sizes() -> Element {
  let sizes: &[(f32, &str)] = &[
    (48.0, "48px"),
    (32.0, "32px"),
    (24.0, "24px"),
    (20.0, "20px"),
    (16.0, "16px"),
    (14.0, "14px"),
    (12.0, "12px"),
  ];

  lurq::components::Column::new()
    .spacing(8.0)
    .with_children(sizes.iter().map(|(size, label)| {
      lurq::components::Row::new()
        .spacing(16.0)
        .align_items(Alignment::Center)
        .child(text(label, 12.0, FontWeight::Normal, TEXT_MUTED).width(40.0))
        .child(text("The quick brown fox", *size, FontWeight::Normal, TEXT))
        .width(FILL_WIDTH)
    }))
    .pad(24.0)
    .width(FILL_WIDTH)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

fn font_weights() -> Element {
  let weights: &[(&str, FontWeight)] = &[
    ("Thin (200)", FontWeight::Thin),
    ("Light (300)", FontWeight::Light),
    ("Normal (400)", FontWeight::Normal),
    ("Bold (700)", FontWeight::Bold),
    ("Black (900)", FontWeight::Black),
  ];

  lurq::components::Column::new()
    .spacing(8.0)
    .with_children(weights.iter().map(|(label, weight)| {
      lurq::components::Row::new()
        .spacing(16.0)
        .align_items(Alignment::Center)
        .child(text(label, 12.0, FontWeight::Normal, TEXT_MUTED).width(110.0))
        .child(text("The quick brown fox", 16.0, *weight, TEXT))
        .width(FILL_WIDTH)
    }))
    .pad(24.0)
    .width(FILL_WIDTH)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

fn text_colors() -> Element {
  let colors: &[(&str, &str)] = &[
    ("White text (#F8FAFC)", "#F8FAFC"),
    ("Primary blue (#3B82F6)", "#3B82F6"),
    ("Success green (#22C55E)", "#22C55E"),
    ("Warning amber (#F59E0B)", "#F59E0B"),
    ("Error red (#EF4444)", "#EF4444"),
    ("Muted gray (#64748B)", "#64748B"),
  ];

  lurq::components::Column::new()
    .spacing(6.0)
    .with_children(
      colors
        .iter()
        .map(|(content, color)| text(content, 15.0, FontWeight::Normal, color).width(FILL_WIDTH)),
    )
    .pad(24.0)
    .width(FILL_WIDTH)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}
