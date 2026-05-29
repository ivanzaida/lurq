use crate::{
  components::{Column, Row, Text},
  layout::{Alignment, layout_kind::Justify, text_style::FontWeight},
  node::{Element, color::Color, dimension::Dimension},
};

pub(crate) const BG: &str = "#0d0d0d";
pub(crate) const SURFACE: &str = "#161616";
pub(crate) const SURFACE_2: &str = "#1e1e1e";
pub(crate) const SELECTED: &str = "#1a1a2e";
pub(crate) const BORDER: &str = "#27272a";
pub(crate) const TEXT: &str = "#e4e4e7";
pub(crate) const MUTED: &str = "#71717a";
pub(crate) const PRIMARY: &str = "#a855f7";
pub(crate) const BLUE: &str = "#60a5fa";
pub(crate) const GREEN: &str = "#22c55e";
pub(crate) const FILL: Dimension = Dimension::Pct(100.0);

pub(crate) fn section_header(label: &str, count: &str) -> Element {
  Row::new()
    .align_items(Alignment::Center)
    .child(text(label, 11.0, FontWeight::Bold, MUTED))
    .child(crate::components::Spacer::new().flex(1.0))
    .child(text(count, 10.0, FontWeight::Normal, MUTED))
    .width(FILL)
    .height(36.0)
    .pad_xy(12.0, 0.0)
    .border_inside(1.0, Color::from_hex(BORDER))
    .into()
}

pub(crate) fn section_title(label: &str) -> Element {
  Row::new()
    .align_items(Alignment::Center)
    .spacing(6.0)
    .child(text("v", 11.0, FontWeight::Normal, MUTED))
    .child(text(label, 12.0, FontWeight::Bold, TEXT))
    .width(FILL)
    .height(34.0)
    .pad_xy(16.0, 0.0)
    .into()
}

pub(crate) fn info_row(label: &str, value: &str) -> Element {
  Row::new()
    .align_items(Alignment::Center)
    .child(text(label, 11.0, FontWeight::Normal, MUTED).width(120.0))
    .child(text(value, 11.0, FontWeight::Medium, TEXT).nowrap())
    .width(FILL)
    .height(25.0)
    .pad_xy(40.0, 0.0)
    .into()
}

pub(crate) fn empty_state(message: &str) -> Element {
  text(message, 11.0, FontWeight::Normal, MUTED)
    .width(FILL)
    .pad_xy(40.0, 8.0)
    .into()
}

pub(crate) fn empty_section(title: &str, message: &str) -> Element {
  Column::new()
    .child(section_title(title))
    .child(empty_state(message))
    .width(FILL)
    .border_inside(1.0, Color::from_hex(BORDER))
    .into()
}

pub(crate) fn badge(label: &str, color: &str, fill: &str) -> Element {
  Row::new()
    .align_items(Alignment::Center)
    .justify(Justify::Center)
    .child(text(label, 10.0, FontWeight::Medium, color).nowrap())
    .height(18.0)
    .pad_xy(6.0, 0.0)
    .fill(fill)
    .rounded(3.0)
    .into()
}

pub(crate) fn text(content: &str, size: f32, weight: FontWeight, color: &str) -> Text {
  Text::styled(
    content,
    crate::layout::text_style::TextStyle {
      font_size: size,
      weight,
      color: Color::from_hex(color),
      ..Default::default()
    },
  )
}

pub(crate) fn short_tag(tag: &str) -> &str {
  tag.rsplit("::").next().unwrap_or(tag)
}
