use lurq::{
  layout::{Alignment, text_style::FontWeight},
  node::{Element, color::Color, dimension::Dimension},
};

use crate::style::{BG, BORDER, PRIMARY, SURFACE, TEXT, TEXT_MUTED, text};

const FILL_WIDTH: Dimension = Dimension::Pct(100.0);
const CONTENT_PAD: f32 = 32.0;
const CARD_RADIUS: f32 = 8.0;

pub(crate) fn visual_content() -> Element {
  lurq::components::Column::new()
    .spacing(24.0)
    .child(text("Visual Styling", 28.0, FontWeight::Bold, TEXT).width(FILL_WIDTH))
    .child(section_title("Color Palette"))
    .child(color_palette())
    .child(section_title("Border Radius"))
    .child(radius_showcase())
    .child(section_title("Clipping (Overflow)"))
    .child(clip_showcase())
    .pad(CONTENT_PAD)
    .width(FILL_WIDTH)
    .fill(BG)
    .into()
}

fn section_title(label: &str) -> Element {
  text(label, 18.0, FontWeight::Bold, TEXT).width(FILL_WIDTH).into()
}

fn color_palette() -> Element {
  let colors: &[(&str, &str)] = &[
    ("red", "#EF4444"),
    ("org", "#F97316"),
    ("yel", "#EAB308"),
    ("grn", "#22C55E"),
    ("blu", "#3B82F6"),
    ("pur", "#8B5CF6"),
  ];

  lurq::components::Column::new()
    .spacing(16.0)
    .child(
      lurq::components::Row::new()
        .spacing(12.0)
        .with_children(colors.iter().map(|(name, hex)| color_swatch(name, hex)))
        .width(FILL_WIDTH),
    )
    .child(alpha_row())
    .pad(24.0)
    .width(FILL_WIDTH)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

fn color_swatch(name: &str, hex: &str) -> Element {
  lurq::components::Column::new()
    .spacing(4.0)
    .align_items(Alignment::Center)
    .justify(lurq::layout::layout_kind::Justify::Center)
    .child(text(hex, 10.0, FontWeight::Normal, "#ffffff"))
    .child(text(name, 11.0, FontWeight::Bold, "#ffffff"))
    .size(Dimension::Pct(100.0), 70.0)
    .fill(hex)
    .rounded(8.0)
    .flex(1.0)
    .into()
}

fn alpha_row() -> Element {
  let alphas: &[(&str, f32)] = &[("100%", 1.0), ("80%", 0.8), ("60%", 0.6), ("40%", 0.4), ("20%", 0.2)];

  lurq::components::Row::new()
    .spacing(12.0)
    .align_items(Alignment::Center)
    .child(text("Alpha:", 12.0, FontWeight::Normal, TEXT_MUTED))
    .with_children(alphas.iter().map(|(label, alpha)| {
      lurq::components::Column::new()
        .align_items(Alignment::Center)
        .justify(lurq::layout::layout_kind::Justify::Center)
        .child(text(label, 11.0, FontWeight::Normal, "#ffffff"))
        .size(60.0, 32.0)
        .fill(PRIMARY)
        .rounded(4.0)
        .opacity(*alpha)
    }))
    .width(FILL_WIDTH)
    .into()
}

fn radius_showcase() -> Element {
  let radii: &[(&str, f32)] = &[
    ("rounded(0) — sharp", 0.0),
    ("rounded(8) — subtle", 8.0),
    ("rounded(16) — rounded", 16.0),
    ("rounded(40) — pill", 40.0),
  ];

  lurq::components::Row::new()
    .spacing(24.0)
    .align_items(Alignment::Center)
    .with_children(radii.iter().map(|(label, radius)| {
      lurq::components::Column::new()
        .spacing(8.0)
        .align_items(Alignment::Center)
        .child(lurq::components::Rect::new(140.0, 50.0).fill(PRIMARY).rounded(*radius))
        .child(text(label, 11.0, FontWeight::Normal, TEXT_MUTED))
        .height(80.0)
        .flex(1.0)
    }))
    .pad_xy(24.0, 16.0)
    .width(FILL_WIDTH)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

fn clip_showcase() -> Element {
  lurq::components::Row::new()
    .spacing(80.0)
    .align_items(Alignment::Center)
    .child(clip_example("Overflow::Visible", false))
    .child(clip_example("Overflow::Hidden", true))
    .pad_xy(60.0, 20.0)
    .width(FILL_WIDTH)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .overflow_visible()
    .into()
}

fn clip_example(label: &str, clip: bool) -> Element {
  let mut parent = lurq::components::Stack::new()
    .child(
      lurq::components::Rect::new(80.0, 50.0)
        .fill("#F59E0B")
        .opacity(0.7)
        .rounded(4.0)
        .absolute_position(60.0, 20.0),
    )
    .size(120.0, 80.0)
    .fill("#0F172A")
    .rounded(4.0)
    .border_inside(1.0, Color::from_hex(BORDER));
  if clip {
    parent = parent.clip();
  } else {
    parent = parent.overflow_visible();
  }

  let mut col = lurq::components::Column::new()
    .spacing(8.0)
    .align_items(Alignment::Center)
    .child(text(label, 12.0, FontWeight::Normal, TEXT_MUTED))
    .child(parent);
  if !clip {
    col = col.overflow_visible();
  }
  col.into()
}
