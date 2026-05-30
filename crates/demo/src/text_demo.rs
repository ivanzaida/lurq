use lurq::{
  layout::{
    Alignment,
    text_style::{FontStyle, FontWeight, TextStyle},
  },
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
    .child(section_title("Font Styles"))
    .child(font_styles())
    .child(section_title("Line Height"))
    .child(line_heights())
    .child(section_title("Wrapping"))
    .child(wrapping())
    .child(section_title("Text Colors"))
    .child(text_colors())
    .padding(CONTENT_PAD)
    .width(FILL_WIDTH)
    .fill(BG)
    .into()
}

fn section_title(label: &str) -> Element {
  text(label, 18.0, FontWeight::Bold, TEXT).width(FILL_WIDTH).into()
}

fn styled_text(
  content: &str,
  font_size: f32,
  weight: FontWeight,
  style: FontStyle,
  line_height: f32,
  color: &str,
) -> lurq::components::Text {
  lurq::components::Text::styled(
    content,
    TextStyle {
      font_size,
      weight,
      style,
      line_height,
      color: Color::from_hex(color),
      ..TextStyle::default()
    },
  )
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
    .padding(24.0)
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
    .padding(24.0)
    .width(FILL_WIDTH)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

fn font_styles() -> Element {
  let styles: &[(&str, FontWeight, FontStyle)] = &[
    ("Normal", FontWeight::Normal, FontStyle::Normal),
    ("Italic", FontWeight::Normal, FontStyle::Italic),
    ("Bold", FontWeight::Bold, FontStyle::Normal),
    ("Bold italic", FontWeight::Bold, FontStyle::Italic),
  ];

  lurq::components::Column::new()
    .spacing(8.0)
    .with_children(styles.iter().map(|(label, weight, font_style)| {
      lurq::components::Row::new()
        .spacing(16.0)
        .align_items(Alignment::Center)
        .child(text(label, 12.0, FontWeight::Normal, TEXT_MUTED).width(110.0))
        .child(styled_text(
          "The quick brown fox",
          18.0,
          *weight,
          *font_style,
          1.2,
          TEXT,
        ))
        .width(FILL_WIDTH)
    }))
    .padding(24.0)
    .width(FILL_WIDTH)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

fn line_heights() -> Element {
  let variants: &[(&str, f32)] = &[("1.0", 1.0), ("1.2", 1.2), ("1.6", 1.6), ("2.0", 2.0)];
  let sample = "Line height controls vertical rhythm for wrapped text across multiple lines.";

  lurq::components::Column::new()
    .spacing(12.0)
    .with_children(variants.iter().map(|(label, line_height)| {
      lurq::components::Row::new()
        .spacing(16.0)
        .align_items(Alignment::Start)
        .child(text(label, 12.0, FontWeight::Normal, TEXT_MUTED).width(40.0))
        .child(styled_text(sample, 16.0, FontWeight::Normal, FontStyle::Normal, *line_height, TEXT).width(360.0))
        .width(FILL_WIDTH)
    }))
    .padding(24.0)
    .width(FILL_WIDTH)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

fn wrapping() -> Element {
  let sample = "Wrapping keeps long typography inside its assigned layout width.";

  lurq::components::Column::new()
    .spacing(12.0)
    .child(
      lurq::components::Row::new()
        .spacing(16.0)
        .align_items(Alignment::Start)
        .child(text("wrap", 12.0, FontWeight::Normal, TEXT_MUTED).width(70.0))
        .child(text(sample, 16.0, FontWeight::Normal, TEXT).width(260.0))
        .width(FILL_WIDTH),
    )
    .child(
      lurq::components::Row::new()
        .spacing(16.0)
        .align_items(Alignment::Start)
        .child(text("nowrap", 12.0, FontWeight::Normal, TEXT_MUTED).width(70.0))
        .child(text(sample, 16.0, FontWeight::Normal, TEXT).nowrap().width(260.0))
        .width(FILL_WIDTH),
    )
    .padding(24.0)
    .width(FILL_WIDTH)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .overflow_visible()
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
    .padding(24.0)
    .width(FILL_WIDTH)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}
