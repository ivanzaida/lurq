use lurq::{
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, color::Color, dimension::Dimension, padding::Padding},
};

use crate::style::{ACCENT, BG, BORDER, PRIMARY, SECONDARY, SURFACE, SURFACE_DARK, TEXT, TEXT_MUTED, text};

const CONTENT_PAD: f32 = 32.0;
const FILL_WIDTH: Dimension = Dimension::Pct(100.0);
const CARD_RADIUS: f32 = 8.0;
const PANEL_RADIUS: f32 = 6.0;
const BLUE_TINT: &str = "#3b82f626";
const CYAN_TINT: &str = "#06b6d426";

pub(crate) fn sizing_content() -> Element {
  lurq::components::Column::new()
    .spacing(24.0)
    .child(text("Sizing & Spacing", 28.0, FontWeight::Bold, TEXT).width(FILL_WIDTH))
    .child(section_title("Dimension Types"))
    .child(dimension_types())
    .child(section_title("Padding Showcase"))
    .child(padding_showcase())
    .child(section_title("Spacer"))
    .child(spacer_showcase())
    .padding(CONTENT_PAD)
    .width(FILL_WIDTH)
    .fill(BG)
    .into()
}

fn section_title(label: &str) -> Element {
  text(label, 18.0, FontWeight::Bold, TEXT).width(FILL_WIDTH).into()
}

fn dimension_types() -> Element {
  lurq::components::Column::new()
    .spacing(12.0)
    .child(dimension_row(
      "Fixed (120px)",
      lurq::components::Row::new()
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .child(text("120px", 11.0, FontWeight::Medium, TEXT_MUTED))
        .size(120.0, 24.0)
        .fill(PRIMARY)
        .into(),
    ))
    .child(dimension_row(
      "Percentage (80%)",
      lurq::components::Row::new()
        .child(
          lurq::components::Row::new()
            .align_items(Alignment::Center)
            .justify(Justify::Center)
            .child(text("80% of parent", 11.0, FontWeight::Medium, TEXT_MUTED))
            .width(Dimension::Pct(80.0))
            .height(24.0)
            .fill(SECONDARY),
        )
        .width(FILL_WIDTH)
        .height(24.0)
        .into(),
    ))
    .child(dimension_row(
      "Auto (fits content)",
      lurq::components::Row::new()
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .child(text("Auto sized", 11.0, FontWeight::Medium, TEXT_MUTED))
        .padding_horizontal(18.0)
        .padding_vertical(0.0)
        .height(24.0)
        .fill(ACCENT)
        .into(),
    ))
    .padding(24.0)
    .width(FILL_WIDTH)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

fn dimension_row(label: &str, visual: Element) -> Element {
  lurq::components::Row::new()
    .spacing(16.0)
    .align_items(Alignment::Center)
    .child(text(label, 12.0, FontWeight::Medium, TEXT_MUTED).width(130.0))
    .child(
      lurq::components::Row::new()
        .align_items(Alignment::Center)
        .child(visual)
        .height(32.0)
        .flex(1.0),
    )
    .width(FILL_WIDTH)
    .height(32.0)
    .into()
}

fn padding_showcase() -> Element {
  lurq::components::Column::new()
    .spacing(16.0)
    .child(padding_sample(
      "Uniform 20px padding",
      BLUE_TINT,
      PRIMARY,
      Padding::all(Dimension::Px(20.0)),
      80.0,
    ))
    .child(padding_sample(
      "40px horiz, 10px vert",
      CYAN_TINT,
      ACCENT,
      Padding::symmetric(Dimension::Px(40.0), Dimension::Px(10.0)),
      60.0,
    ))
    .padding(24.0)
    .width(FILL_WIDTH)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

fn padding_sample(label: &str, fill: &str, stroke: &str, padding: Padding, height: f32) -> Element {
  lurq::components::Row::new()
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(
      lurq::components::Row::new()
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .child(text(label, 13.0, FontWeight::Normal, TEXT))
        .width(FILL_WIDTH)
        .height(24.0)
        .fill(SURFACE_DARK)
        .rounded(3.0),
    )
    .padding(padding)
    .width(FILL_WIDTH)
    .height(height)
    .fill(fill)
    .border_inside(1.0, Color::from_hex(stroke))
    .rounded(PANEL_RADIUS)
    .into()
}

fn spacer_showcase() -> Element {
  lurq::components::Row::new()
    .align_items(Alignment::Center)
    .child(spacer_button("Left"))
    .child(
      lurq::components::Row::new()
        .align_items(Alignment::Center)
        .justify(Justify::Center)
        .child(text("+ spacer with flex(1) +", 11.0, FontWeight::Medium, TEXT_MUTED))
        .height(30.0)
        .flex(1.0),
    )
    .child(spacer_button("Right"))
    .padding(24.0)
    .width(FILL_WIDTH)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(CARD_RADIUS)
    .into()
}

fn spacer_button(label: &str) -> Element {
  lurq::components::Row::new()
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(text(label, 11.0, FontWeight::Bold, TEXT))
    .size(100.0, 48.0)
    .fill(PRIMARY)
    .rounded(PANEL_RADIUS)
    .into()
}
