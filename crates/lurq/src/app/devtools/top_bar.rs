use super::style::{BORDER, FILL, MUTED, PRIMARY, SURFACE, SURFACE_2, TEXT, badge, text};
use crate::{
  components::{Column, Rect, Row, Spacer},
  layout::{Alignment, text_style::FontWeight},
  node::{CursorIcon, Element, color::Color},
};

pub(crate) fn top_bar(node_count: usize) -> Element {
  Row::new()
    .align_items(Alignment::Center)
    .child(
      Row::new()
        .align_items(Alignment::Center)
        .spacing(8.0)
        .child(text("bug", 11.0, FontWeight::Bold, PRIMARY))
        .child(text("lurq DevTools", 14.0, FontWeight::Bold, TEXT))
        .child(badge("v0.1.0", MUTED, SURFACE_2))
        .child(badge(&format!("{node_count} nodes"), MUTED, SURFACE_2))
        .width(300.0),
    )
    .child(tab("Components", true))
    .child(tab("Profiler", false))
    .child(tab("Signals", false))
    .child(Spacer::new().flex(1.0))
    .child(search_box())
    .child(icon_button("settings"))
    .height(48.0)
    .width(FILL)
    .pad_xy(16.0, 0.0)
    .fill(SURFACE)
    .border_inside(1.0, Color::from_hex(BORDER))
    .into()
}

fn tab(label: &str, active: bool) -> Element {
  Column::new()
    .align_items(Alignment::Center)
    .child(
      Row::new()
        .align_items(Alignment::Center)
        .spacing(6.0)
        .child(text(label, 13.0, FontWeight::Medium, if active { TEXT } else { MUTED }))
        .height(36.0),
    )
    .child(Rect::new(64.0, 2.0).fill(if active { PRIMARY } else { "#00000000" }))
    .pad_xy(16.0, 0.0)
    .cursor(CursorIcon::Pointer)
    .into()
}

fn search_box() -> Element {
  Row::new()
    .align_items(Alignment::Center)
    .spacing(8.0)
    .child(text("search", 10.0, FontWeight::Normal, MUTED))
    .child(text("Search components...", 12.0, FontWeight::Normal, MUTED))
    .child(text("Ctrl+F", 10.0, FontWeight::Normal, MUTED))
    .height(30.0)
    .pad_xy(10.0, 0.0)
    .fill(SURFACE_2)
    .border_inside(1.0, Color::from_hex(BORDER))
    .rounded(4.0)
    .into()
}

fn icon_button(label: &str) -> Element {
  Row::new()
    .align_items(Alignment::Center)
    .justify(crate::layout::layout_kind::Justify::Center)
    .child(text(label, 10.0, FontWeight::Normal, MUTED))
    .height(30.0)
    .pad_xy(8.0, 0.0)
    .rounded(4.0)
    .cursor(CursorIcon::Pointer)
    .into()
}
