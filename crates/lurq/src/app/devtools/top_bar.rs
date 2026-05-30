use super::style::{BORDER, FILL, MUTED, PRIMARY, SURFACE, SURFACE_2, TEXT, badge, icon, text};
use crate::{
  components::{Column, Rect, Row, Spacer},
  layout::{Alignment, text_style::FontWeight},
  node::{CursorIcon, Element, color::Color},
};

pub(crate) fn top_bar(_node_count: usize) -> Element {
  Row::new()
    .align_items(Alignment::Center)
    .child(
      Row::new()
        .align_items(Alignment::Center)
        .spacing(8.0)
        .child(icon("bug", 16.0, PRIMARY))
        .child(text("lurq DevTools", 14.0, FontWeight::Bold, TEXT))
        .child(badge("v0.1.0", MUTED, SURFACE_2))
        .width(200.0),
    )
    .child(tab("layers", "Components", true))
    .child(tab("activity", "Profiler", false))
    .child(tab("zap", "Signals", false))
    .child(Spacer::new().flex(1.0))
    .child(search_box())
    .child(icon_button("settings"))
    .height(48.0)
    .width(FILL)
    .padding_horizontal(16.0)
    .padding_vertical(0.0)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .into()
}

fn tab(icon_name: &str, label: &str, active: bool) -> Element {
  let color = if active { TEXT } else { MUTED };
  Column::new()
    .align_items(Alignment::Center)
    .child(
      Row::new()
        .align_items(Alignment::Center)
        .spacing(6.0)
        .child(icon(icon_name, 14.0, color))
        .child(text(label, 13.0, FontWeight::Medium, color))
        .height(36.0),
    )
    .child(Rect::new(64.0, 2.0).fill(if active { PRIMARY } else { "#00000000" }))
    .padding_horizontal(16.0)
    .padding_vertical(0.0)
    .cursor(CursorIcon::Pointer)
    .into()
}

fn search_box() -> Element {
  Row::new()
    .align_items(Alignment::Center)
    .spacing(8.0)
    .child(icon("search", 13.0, MUTED))
    .child(text("Search components...", 12.0, FontWeight::Normal, MUTED))
    .child(text("Ctrl+F", 10.0, FontWeight::Normal, MUTED))
    .height(30.0)
    .padding_horizontal(10.0)
    .padding_vertical(0.0)
    .fill(SURFACE_2)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(4.0)
    .into()
}

fn icon_button(icon_name: &str) -> Element {
  Row::new()
    .align_items(Alignment::Center)
    .justify(crate::layout::layout_kind::Justify::Center)
    .child(icon(icon_name, 16.0, MUTED))
    .height(30.0)
    .padding_horizontal(8.0)
    .padding_vertical(0.0)
    .rounded(4.0)
    .cursor(CursorIcon::Pointer)
    .into()
}
